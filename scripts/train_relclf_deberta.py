#!/usr/bin/env python3
"""Fine-tune DeBERTa-v3-xsmall as a cross-encoder relation classifier.

Replaces the MiniLM + logistic regression approach with a proper fine-tuned
sequence classifier that learns entity-aware representations.

Requirements:
    pip install transformers[torch] optimum onnxruntime onnx scikit-learn

Usage:
    python scripts/prepare_training_data.py   # generate training_data.json first
    python scripts/train_relclf_deberta.py
"""

import json
import logging
import sys
from collections import Counter
from copy import deepcopy
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, Dataset
from transformers import (
    AutoModelForSequenceClassification,
    AutoTokenizer,
    get_linear_schedule_with_warmup,
)

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger(__name__)

SCRIPT_DIR = Path(__file__).resolve().parent
DATA_PATH = SCRIPT_DIR / "training_data.json"
OUTPUT_DIR = Path("models/relation_classifier_deberta")

MODEL_NAME = "microsoft/deberta-v3-xsmall"
MAX_SEQ_LEN = 128
BATCH_SIZE = 8
GRAD_ACCUM_STEPS = 2  # effective batch size = 16
EPOCHS = 15
LR = 2e-5
WARMUP_RATIO = 0.1
PATIENCE = 4  # early stopping patience (epochs without val F1 improvement)
SEED = 42

ENTITY_MARKERS = ["[E1]", "[/E1]", "[E2]", "[/E2]"]

LABEL_NAMES = [
    "chose", "rejected", "replaced", "depends_on", "fixed",
    "introduced", "deprecated", "caused", "constrained_by", "none",
]
LABEL2ID = {name: i for i, name in enumerate(LABEL_NAMES)}
ID2LABEL = {i: name for i, name in enumerate(LABEL_NAMES)}
NUM_LABELS = len(LABEL_NAMES)


# ---------------------------------------------------------------------------
# Dataset
# ---------------------------------------------------------------------------

class RelationDataset(Dataset):
    def __init__(self, records: list[dict], tokenizer, max_len: int = MAX_SEQ_LEN):
        self.texts = [r["text"] for r in records]
        self.labels = [LABEL2ID[r["label"]] for r in records]
        self.tokenizer = tokenizer
        self.max_len = max_len

    def __len__(self):
        return len(self.texts)

    def __getitem__(self, idx):
        encoding = self.tokenizer(
            self.texts[idx],
            truncation=True,
            max_length=self.max_len,
            padding="max_length",
            return_tensors="pt",
        )
        return {
            "input_ids": encoding["input_ids"].squeeze(0),
            "attention_mask": encoding["attention_mask"].squeeze(0),
            "label": torch.tensor(self.labels[idx], dtype=torch.long),
        }


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def load_data(path: Path) -> tuple[list[dict], list[dict]]:
    with open(path) as f:
        records = json.load(f)
    train = [r for r in records if r.get("split") == "train"]
    val = [r for r in records if r.get("split") in ("val", "eval", "dev", "test")]
    return train, val


def compute_class_weights(labels: list[int], num_classes: int) -> torch.Tensor:
    """Inverse frequency weights, normalized so they sum to num_classes."""
    counts = Counter(labels)
    total = len(labels)
    weights = torch.zeros(num_classes, dtype=torch.float32)
    for cls_id in range(num_classes):
        count = counts.get(cls_id, 1)
        weights[cls_id] = total / (num_classes * count)
    return weights


def evaluate(model, dataloader, device) -> tuple[float, dict]:
    """Return macro F1 and per-class metrics."""
    from sklearn.metrics import classification_report, f1_score

    model.eval()
    all_preds = []
    all_labels = []
    with torch.no_grad():
        for batch in dataloader:
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            labels = batch["label"]

            outputs = model(input_ids=input_ids, attention_mask=attention_mask)
            preds = outputs.logits.argmax(dim=-1).cpu().numpy()
            all_preds.extend(preds)
            all_labels.extend(labels.numpy())

    macro_f1 = f1_score(all_labels, all_preds, average="macro", zero_division=0)
    report = classification_report(
        all_labels, all_preds,
        target_names=LABEL_NAMES,
        labels=list(range(NUM_LABELS)),
        zero_division=0,
        output_dict=True,
    )
    return macro_f1, report


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------

def train_model(
    train_records: list[dict],
    val_records: list[dict],
    device: torch.device,
) -> tuple:
    """Fine-tune DeBERTa and return (model, tokenizer)."""
    log.info("Loading tokenizer and model: %s", MODEL_NAME)
    tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)

    # Add entity markers as special tokens
    num_added = tokenizer.add_special_tokens(
        {"additional_special_tokens": ENTITY_MARKERS}
    )
    log.info("Added %d special tokens: %s", num_added, ENTITY_MARKERS)

    model = AutoModelForSequenceClassification.from_pretrained(
        MODEL_NAME,
        num_labels=NUM_LABELS,
        id2label=ID2LABEL,
        label2id=LABEL2ID,
    )
    # Resize embeddings for new special tokens
    model.resize_token_embeddings(len(tokenizer))
    model.to(device)

    # Datasets and loaders
    train_dataset = RelationDataset(train_records, tokenizer)
    val_dataset = RelationDataset(val_records, tokenizer)
    train_loader = DataLoader(train_dataset, batch_size=BATCH_SIZE, shuffle=True)
    val_loader = DataLoader(val_dataset, batch_size=BATCH_SIZE, shuffle=False)

    # Class-weighted loss — boost "none" class weight to improve none detection
    train_labels = [LABEL2ID[r["label"]] for r in train_records]
    class_weights = compute_class_weights(train_labels, NUM_LABELS).to(device)
    # Override none weight: detecting "no relation" is critical for filtering
    none_idx = LABEL2ID["none"]
    class_weights[none_idx] = max(class_weights[none_idx], 1.5)
    log.info("Class weights: %s", {LABEL_NAMES[i]: f"{w:.2f}" for i, w in enumerate(class_weights)})
    criterion = nn.CrossEntropyLoss(weight=class_weights)

    # Optimizer and scheduler
    optimizer = torch.optim.AdamW(model.parameters(), lr=LR, weight_decay=0.01)
    total_steps = (len(train_loader) // GRAD_ACCUM_STEPS) * EPOCHS
    warmup_steps = int(total_steps * WARMUP_RATIO)
    scheduler = get_linear_schedule_with_warmup(optimizer, warmup_steps, total_steps)

    log.info("Training: %d steps/epoch, %d total, %d warmup, grad_accum=%d", len(train_loader), total_steps, warmup_steps, GRAD_ACCUM_STEPS)

    best_f1 = 0.0
    best_state = None
    patience_counter = 0

    for epoch in range(1, EPOCHS + 1):
        model.train()
        total_loss = 0.0
        optimizer.zero_grad()

        for step, batch in enumerate(train_loader, 1):
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            labels = batch["label"].to(device)

            outputs = model(input_ids=input_ids, attention_mask=attention_mask)
            loss = criterion(outputs.logits, labels) / GRAD_ACCUM_STEPS
            loss.backward()

            if step % GRAD_ACCUM_STEPS == 0 or step == len(train_loader):
                torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                optimizer.step()
                scheduler.step()
                optimizer.zero_grad()

            total_loss += loss.item() * GRAD_ACCUM_STEPS

        avg_loss = total_loss / len(train_loader)

        # Validate
        macro_f1, report = evaluate(model, val_loader, device)
        none_f1 = report.get("none", {}).get("f1-score", 0.0)

        log.info(
            "Epoch %d/%d — loss: %.4f, val macro-F1: %.4f, none-F1: %.4f",
            epoch, EPOCHS, avg_loss, macro_f1, none_f1,
        )

        if macro_f1 > best_f1:
            best_f1 = macro_f1
            best_state = deepcopy(model.state_dict())
            patience_counter = 0
            log.info("  -> New best model (macro-F1=%.4f)", best_f1)
        else:
            patience_counter += 1
            if patience_counter >= PATIENCE:
                log.info("Early stopping at epoch %d (no improvement for %d epochs)", epoch, PATIENCE)
                break

    # Restore best model
    model.load_state_dict(best_state)
    log.info("Restored best model with macro-F1=%.4f", best_f1)

    return model, tokenizer


# ---------------------------------------------------------------------------
# ONNX Export
# ---------------------------------------------------------------------------

def export_onnx(model, tokenizer, output_dir: Path, device: torch.device):
    """Export model to ONNX and quantize to INT8."""
    from onnxruntime.quantization import QuantType, quantize_dynamic

    output_dir.mkdir(parents=True, exist_ok=True)

    model.eval()
    model.to("cpu")

    # Dummy input for tracing
    dummy_text = "[E1]React[/E1] was chosen over [E2]Vue[/E2] for the frontend."
    dummy = tokenizer(
        dummy_text,
        truncation=True,
        max_length=MAX_SEQ_LEN,
        padding="max_length",
        return_tensors="pt",
    )

    onnx_fp32_path = output_dir / "model_fp32.onnx"
    onnx_int8_path = output_dir / "model_int8.onnx"

    log.info("Exporting to ONNX (FP32)...")
    torch.onnx.export(
        model,
        (dummy["input_ids"], dummy["attention_mask"]),
        str(onnx_fp32_path),
        input_names=["input_ids", "attention_mask"],
        output_names=["logits"],
        dynamic_axes={
            "input_ids": {0: "batch", 1: "seq"},
            "attention_mask": {0: "batch", 1: "seq"},
            "logits": {0: "batch"},
        },
        opset_version=14,
        do_constant_folding=True,
        dynamo=False,
    )
    log.info("FP32 ONNX saved: %s (%d KB)", onnx_fp32_path, onnx_fp32_path.stat().st_size // 1024)

    # INT8 dynamic quantization
    log.info("Quantizing to INT8...")
    quantize_dynamic(
        str(onnx_fp32_path),
        str(onnx_int8_path),
        weight_type=QuantType.QInt8,
    )
    log.info("INT8 ONNX saved: %s (%d KB)", onnx_int8_path, onnx_int8_path.stat().st_size // 1024)

    # Clean up FP32 model
    onnx_fp32_path.unlink()
    log.info("Removed FP32 model (keeping INT8 only)")

    # Save tokenizer (HuggingFace format for rust `tokenizers` crate)
    tokenizer.save_pretrained(str(output_dir))
    log.info("Tokenizer saved to %s", output_dir)

    # Save label map
    label_map = {str(i): name for i, name in enumerate(LABEL_NAMES)}
    label_map_path = output_dir / "label_map.json"
    with open(label_map_path, "w") as f:
        json.dump(label_map, f, indent=2)
    log.info("Label map saved to %s", label_map_path)


def verify_onnx(model, tokenizer, output_dir: Path, device: torch.device):
    """Load ONNX model and compare outputs to PyTorch model."""
    import onnxruntime as ort

    onnx_path = output_dir / "model_int8.onnx"
    log.info("Verifying ONNX model: %s", onnx_path)

    sess = ort.InferenceSession(str(onnx_path), providers=["CPUExecutionProvider"])

    test_texts = [
        "[E1]React[/E1] was chosen over [E2]Vue[/E2] for the frontend.",
        "[E1]Redis[/E1] replaced [E2]Memcached[/E2] as the caching layer.",
        "[E1]Terraform[/E1] modules manage infra. [E2]Kubernetes[/E2] handles orchestration.",
    ]

    model.eval()
    model.to("cpu")

    log.info("Comparing PyTorch vs ONNX outputs on %d test examples:", len(test_texts))
    for text in test_texts:
        enc = tokenizer(
            text,
            truncation=True,
            max_length=MAX_SEQ_LEN,
            padding="max_length",
            return_tensors="pt",
        )

        # PyTorch
        with torch.no_grad():
            pt_logits = model(
                input_ids=enc["input_ids"],
                attention_mask=enc["attention_mask"],
            ).logits.numpy()[0]

        # ONNX
        ort_logits = sess.run(None, {
            "input_ids": enc["input_ids"].numpy(),
            "attention_mask": enc["attention_mask"].numpy(),
        })[0][0]

        pt_pred = LABEL_NAMES[np.argmax(pt_logits)]
        ort_pred = LABEL_NAMES[np.argmax(ort_logits)]
        max_diff = np.max(np.abs(pt_logits - ort_logits))

        log.info(
            "  Text: %.60s... | PT: %-14s | ONNX: %-14s | max_diff: %.4f",
            text, pt_pred, ort_pred, max_diff,
        )

    log.info("ONNX verification complete.")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    from sklearn.metrics import classification_report

    torch.manual_seed(SEED)
    np.random.seed(SEED)

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    log.info("Device: %s", device)

    # Load data
    log.info("Loading training data from %s", DATA_PATH)
    train_records, val_records = load_data(DATA_PATH)
    log.info("Train: %d, Val: %d", len(train_records), len(val_records))

    train_dist = Counter(r["label"] for r in train_records)
    log.info("Train distribution: %s", dict(sorted(train_dist.items())))

    # Train
    model, tokenizer = train_model(train_records, val_records, device)

    # Final evaluation
    log.info("=" * 60)
    log.info("FINAL EVALUATION")
    log.info("=" * 60)
    val_dataset = RelationDataset(val_records, tokenizer)
    val_loader = DataLoader(val_dataset, batch_size=BATCH_SIZE, shuffle=False)
    macro_f1, report_dict = evaluate(model, val_loader, device)

    # Print full text report
    model.eval()
    all_preds, all_labels = [], []
    with torch.no_grad():
        for batch in val_loader:
            outputs = model(
                input_ids=batch["input_ids"].to(device),
                attention_mask=batch["attention_mask"].to(device),
            )
            all_preds.extend(outputs.logits.argmax(dim=-1).cpu().numpy())
            all_labels.extend(batch["label"].numpy())

    report_text = classification_report(
        all_labels, all_preds,
        target_names=LABEL_NAMES,
        labels=list(range(NUM_LABELS)),
        zero_division=0,
    )
    log.info("Classification Report:\n%s", report_text)
    log.info("Macro F1: %.4f", macro_f1)
    none_f1 = report_dict.get("none", {}).get("f1-score", 0.0)
    log.info("None class F1: %.4f (target: > 0.60)", none_f1)

    if none_f1 < 0.60:
        log.warning("None class F1 is below 0.60 target — consider more none examples or tuning.")

    # Export
    log.info("=" * 60)
    log.info("ONNX EXPORT")
    log.info("=" * 60)
    export_onnx(model, tokenizer, OUTPUT_DIR, device)

    # Verify
    verify_onnx(model, tokenizer, OUTPUT_DIR, device)

    # Summary
    log.info("=" * 60)
    log.info("DONE — artifacts in %s", OUTPUT_DIR)
    for p in sorted(OUTPUT_DIR.iterdir()):
        size = p.stat().st_size
        if size > 1024:
            log.info("  %s (%d KB)", p.name, size // 1024)
        else:
            log.info("  %s (%d B)", p.name, size)


if __name__ == "__main__":
    main()

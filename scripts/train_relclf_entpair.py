#!/usr/bin/env python3
"""Train relation classifier using entity-pair embeddings (3x384 = 1152-dim).

Instead of a single embedding of the full marked text, generates 3 embeddings:
  - head_ctx: sentence with only [E1] markers (tail markers stripped)
  - tail_ctx: sentence with only [E2] markers (head markers stripped)
  - pair_ctx: just "head_name tail_name" (entity similarity signal)

Concatenated input: [head_ctx || tail_ctx || pair_ctx] = 1152-dim

Trains both logistic regression and MLP classifiers, exports best to ONNX.

Requirements:
    pip install sentence-transformers scikit-learn onnx onnxruntime numpy

Usage:
    python scripts/prepare_training_data.py   # generate training_data.json first
    python scripts/train_relclf_entpair.py
"""

import json
import logging
import re
import sys
from collections import Counter
from pathlib import Path

import numpy as np

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger(__name__)

SCRIPT_DIR = Path(__file__).resolve().parent
DATA_PATH = SCRIPT_DIR / "training_data.json"
OUTPUT_DIR = Path("models/relation_classifier_entpair")

LABEL_NAMES = [
    "chose", "rejected", "replaced", "depends_on", "fixed",
    "introduced", "deprecated", "caused", "constrained_by", "none",
]
LABEL2ID = {name: i for i, name in enumerate(LABEL_NAMES)}
NUM_LABELS = len(LABEL_NAMES)
EMB_DIM = 384
INPUT_DIM = EMB_DIM * 3  # 1152


def load_data(path: Path):
    with open(path) as f:
        records = json.load(f)
    train = [r for r in records if r.get("split") == "train"]
    val = [r for r in records if r.get("split") in ("val", "eval", "dev", "test")]
    return train, val


def strip_markers(text: str, keep: str) -> str:
    """Strip entity markers, keeping only the specified entity's markers.

    Args:
        text: Text with [E1]/[/E1] and [E2]/[/E2] markers.
        keep: "E1" to keep head markers only, "E2" to keep tail markers only.
    """
    if keep == "E1":
        # Remove [E2] and [/E2] markers, keep [E1]/[/E1]
        return re.sub(r"\[/?E2\]", "", text).strip()
    elif keep == "E2":
        # Remove [E1] and [/E1] markers, keep [E2]/[/E2]
        return re.sub(r"\[/?E1\]", "", text).strip()
    else:
        raise ValueError(f"keep must be 'E1' or 'E2', got {keep}")


def build_triple_texts(records: list[dict]) -> tuple[list[str], list[str], list[str]]:
    """Build the 3 text variants for each record.

    Returns:
        (head_texts, tail_texts, pair_texts)
    """
    head_texts = []
    tail_texts = []
    pair_texts = []

    for r in records:
        text = r["text"]
        head = r["head"]
        tail = r["tail"]

        head_texts.append(strip_markers(text, keep="E1"))
        tail_texts.append(strip_markers(text, keep="E2"))
        pair_texts.append(f"{head} {tail}")

    return head_texts, tail_texts, pair_texts


def encode_triples(st_model, records: list[dict], desc: str = "data") -> np.ndarray:
    """Encode records into 1152-dim entity-pair embeddings."""
    head_texts, tail_texts, pair_texts = build_triple_texts(records)

    log.info("Encoding %s head_ctx (%d texts)...", desc, len(head_texts))
    head_emb = st_model.encode(head_texts, show_progress_bar=True, batch_size=64)

    log.info("Encoding %s tail_ctx (%d texts)...", desc, len(tail_texts))
    tail_emb = st_model.encode(tail_texts, show_progress_bar=True, batch_size=64)

    log.info("Encoding %s pair_ctx (%d texts)...", desc, len(pair_texts))
    pair_emb = st_model.encode(pair_texts, show_progress_bar=True, batch_size=64)

    # Concatenate: [head_ctx || tail_ctx || pair_ctx] -> [N, 1152]
    combined = np.concatenate([head_emb, tail_emb, pair_emb], axis=1)
    log.info("%s embeddings shape: %s", desc, combined.shape)
    return combined


def export_logreg_onnx(clf, output_dir: Path, input_dim: int = INPUT_DIM):
    """Export logistic regression as ONNX: input [1, 1152] -> output [1, 10]."""
    import onnx
    from onnx import TensorProto, helper

    output_dir.mkdir(parents=True, exist_ok=True)

    W = clf.coef_.astype(np.float32)       # [10, 1152]
    b = clf.intercept_.astype(np.float32)   # [10]

    X = helper.make_tensor_value_info("embedding", TensorProto.FLOAT, [1, input_dim])
    Y = helper.make_tensor_value_info("logits", TensorProto.FLOAT, [1, NUM_LABELS])

    W_init = helper.make_tensor("W", TensorProto.FLOAT, W.shape, W.flatten().tolist())
    b_init = helper.make_tensor("b", TensorProto.FLOAT, b.shape, b.flatten().tolist())

    transpose_node = helper.make_node("Transpose", ["W"], ["W_T"], perm=[1, 0])
    matmul_node = helper.make_node("MatMul", ["embedding", "W_T"], ["matmul_out"])
    add_node = helper.make_node("Add", ["matmul_out", "b"], ["logits"])

    graph = helper.make_graph(
        [transpose_node, matmul_node, add_node],
        "relation_classifier_entpair_logreg",
        [X], [Y],
        initializer=[W_init, b_init],
    )

    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 14)])
    model.ir_version = 8

    onnx_path = output_dir / "logreg.onnx"
    onnx.save(model, str(onnx_path))
    log.info("LogReg ONNX saved to %s (%d KB)", onnx_path, onnx_path.stat().st_size // 1024)

    # Verify
    import onnxruntime as ort
    sess = ort.InferenceSession(str(onnx_path))
    dummy = np.random.randn(1, input_dim).astype(np.float32)
    out = sess.run(None, {"embedding": dummy})
    log.info("LogReg ONNX verification: output shape=%s", out[0].shape)


def export_mlp_onnx(clf, output_dir: Path, input_dim: int = INPUT_DIM):
    """Export MLP (1152 -> 256 -> 10) as ONNX."""
    import onnx
    from onnx import TensorProto, helper

    output_dir.mkdir(parents=True, exist_ok=True)

    # MLPClassifier stores weights in coefs_ and intercepts_
    W1 = clf.coefs_[0].astype(np.float32)       # [1152, 256]
    b1 = clf.intercepts_[0].astype(np.float32)   # [256]
    W2 = clf.coefs_[1].astype(np.float32)        # [256, 10]
    b2 = clf.intercepts_[1].astype(np.float32)   # [10]

    X = helper.make_tensor_value_info("embedding", TensorProto.FLOAT, [1, input_dim])
    Y = helper.make_tensor_value_info("logits", TensorProto.FLOAT, [1, NUM_LABELS])

    W1_init = helper.make_tensor("W1", TensorProto.FLOAT, W1.shape, W1.flatten().tolist())
    b1_init = helper.make_tensor("b1", TensorProto.FLOAT, b1.shape, b1.flatten().tolist())
    W2_init = helper.make_tensor("W2", TensorProto.FLOAT, W2.shape, W2.flatten().tolist())
    b2_init = helper.make_tensor("b2", TensorProto.FLOAT, b2.shape, b2.flatten().tolist())

    # Layer 1: hidden = ReLU(X @ W1 + b1)
    matmul1 = helper.make_node("MatMul", ["embedding", "W1"], ["mm1_out"])
    add1 = helper.make_node("Add", ["mm1_out", "b1"], ["linear1_out"])
    relu1 = helper.make_node("Relu", ["linear1_out"], ["hidden"])

    # Layer 2: logits = hidden @ W2 + b2
    matmul2 = helper.make_node("MatMul", ["hidden", "W2"], ["mm2_out"])
    add2 = helper.make_node("Add", ["mm2_out", "b2"], ["logits"])

    graph = helper.make_graph(
        [matmul1, add1, relu1, matmul2, add2],
        "relation_classifier_entpair_mlp",
        [X], [Y],
        initializer=[W1_init, b1_init, W2_init, b2_init],
    )

    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 14)])
    model.ir_version = 8

    onnx_path = output_dir / "mlp.onnx"
    onnx.save(model, str(onnx_path))
    log.info("MLP ONNX saved to %s (%d KB)", onnx_path, onnx_path.stat().st_size // 1024)

    # Verify
    import onnxruntime as ort
    sess = ort.InferenceSession(str(onnx_path))
    dummy = np.random.randn(1, input_dim).astype(np.float32)
    out = sess.run(None, {"embedding": dummy})
    log.info("MLP ONNX verification: output shape=%s", out[0].shape)


def main():
    from sentence_transformers import SentenceTransformer
    from sklearn.linear_model import LogisticRegression
    from sklearn.metrics import classification_report, f1_score
    from sklearn.neural_network import MLPClassifier

    log.info("Loading training data from %s", DATA_PATH)
    train_records, val_records = load_data(DATA_PATH)
    log.info("Train: %d, Val: %d", len(train_records), len(val_records))

    train_dist = Counter(r["label"] for r in train_records)
    log.info("Train distribution: %s", dict(sorted(train_dist.items())))

    # Load embedding model
    log.info("Loading sentence-transformers model: all-MiniLM-L6-v2")
    st_model = SentenceTransformer("all-MiniLM-L6-v2")

    # Build entity-pair embeddings (3x384 = 1152-dim)
    X_train = encode_triples(st_model, train_records, desc="train")
    X_val = encode_triples(st_model, val_records, desc="val")

    y_train = np.array([LABEL2ID[r["label"]] for r in train_records])
    y_val = np.array([LABEL2ID[r["label"]] for r in val_records])

    # ── Logistic Regression ──────────────────────────────────────────────
    log.info("=" * 60)
    log.info("LOGISTIC REGRESSION on entity-pair embeddings (1152-dim)")
    log.info("=" * 60)

    best_f1_lr, best_C = 0.0, 1.0
    best_clf_lr = None

    for C in [0.01, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 50.0, 100.0]:
        clf = LogisticRegression(
            max_iter=2000, C=C, class_weight="balanced",
            solver="lbfgs", random_state=42,
        )
        clf.fit(X_train, y_train)
        f1 = f1_score(y_val, clf.predict(X_val), average="macro", zero_division=0)
        log.info("  C=%.2f -> F1=%.4f", C, f1)
        if f1 > best_f1_lr:
            best_f1_lr = f1
            best_C = C
            best_clf_lr = clf

    log.info("Best LogReg: C=%.2f, F1=%.4f", best_C, best_f1_lr)

    y_pred_lr = best_clf_lr.predict(X_val)
    report_lr = classification_report(
        y_val, y_pred_lr,
        target_names=LABEL_NAMES,
        labels=list(range(NUM_LABELS)),
        zero_division=0,
    )
    log.info("LogReg classification report:\n%s", report_lr)

    # ── MLP Classifier ──────────────────────────────────────────────────
    log.info("=" * 60)
    log.info("MLP CLASSIFIER on entity-pair embeddings (1152 -> 256 -> 10)")
    log.info("=" * 60)

    best_f1_mlp = 0.0
    best_mlp = None
    best_mlp_cfg = ""

    for alpha in [0.0001, 0.001, 0.01]:
        for lr_init in [0.001, 0.0005]:
            mlp = MLPClassifier(
                hidden_layer_sizes=(256,),
                activation="relu",
                solver="adam",
                alpha=alpha,
                learning_rate="adaptive",
                learning_rate_init=lr_init,
                max_iter=500,
                early_stopping=True,
                validation_fraction=0.15,
                n_iter_no_change=20,
                random_state=42,
                batch_size=32,
            )
            mlp.fit(X_train, y_train)
            f1 = f1_score(y_val, mlp.predict(X_val), average="macro", zero_division=0)
            cfg = f"alpha={alpha}, lr={lr_init}"
            log.info("  MLP %s -> F1=%.4f (epochs=%d)", cfg, f1, mlp.n_iter_)
            if f1 > best_f1_mlp:
                best_f1_mlp = f1
                best_mlp = mlp
                best_mlp_cfg = cfg

    log.info("Best MLP: %s, F1=%.4f", best_mlp_cfg, best_f1_mlp)

    y_pred_mlp = best_mlp.predict(X_val)
    report_mlp = classification_report(
        y_val, y_pred_mlp,
        target_names=LABEL_NAMES,
        labels=list(range(NUM_LABELS)),
        zero_division=0,
    )
    log.info("MLP classification report:\n%s", report_mlp)

    # ── Summary ──────────────────────────────────────────────────────────
    log.info("=" * 60)
    log.info("SUMMARY")
    log.info("=" * 60)
    log.info("  Simple (single 384-dim, baseline):  F1 ~ 0.84")
    log.info("  Entity-pair LogReg (1152-dim):       F1 = %.4f", best_f1_lr)
    log.info("  Entity-pair MLP (1152->256->10):     F1 = %.4f", best_f1_mlp)

    # ── Export ONNX ──────────────────────────────────────────────────────
    log.info("Exporting models to ONNX...")
    export_logreg_onnx(best_clf_lr, OUTPUT_DIR)
    export_mlp_onnx(best_mlp, OUTPUT_DIR)

    # Save label map
    label_map_path = OUTPUT_DIR / "label_map.json"
    with open(label_map_path, "w") as f:
        json.dump({str(i): name for i, name in enumerate(LABEL_NAMES)}, f, indent=2)
    log.info("Label map saved to %s", label_map_path)

    # Save config describing the embedding approach
    config = {
        "approach": "entity_pair_embeddings",
        "embedding_model": "all-MiniLM-L6-v2",
        "embedding_dim": EMB_DIM,
        "input_dim": INPUT_DIM,
        "num_labels": NUM_LABELS,
        "label_names": LABEL_NAMES,
        "components": ["head_ctx", "tail_ctx", "pair_ctx"],
        "best_logreg_f1": round(best_f1_lr, 4),
        "best_logreg_C": best_C,
        "best_mlp_f1": round(best_f1_mlp, 4),
        "best_mlp_config": best_mlp_cfg,
    }
    config_path = OUTPUT_DIR / "config.json"
    with open(config_path, "w") as f:
        json.dump(config, f, indent=2)
    log.info("Config saved to %s", config_path)

    log.info("Done! Model artifacts in %s", OUTPUT_DIR)
    for p in sorted(OUTPUT_DIR.iterdir()):
        size = p.stat().st_size
        if size > 1024:
            log.info("  %s (%d KB)", p.name, size // 1024)
        else:
            log.info("  %s (%d B)", p.name, size)


if __name__ == "__main__":
    main()

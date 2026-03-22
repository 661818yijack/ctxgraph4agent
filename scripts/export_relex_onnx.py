#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "gliner==0.2.26",
#     "torch>=2.0.0",
#     "onnx>=1.14.0",
#     "onnxruntime>=1.16.0",
#     "transformers>=4.30.0",
#     "numpy>=1.21.0",
# ]
# ///
"""Export knowledgator/gliner-relex-large-v0.5 to ONNX format for ctxgraph.

This model is a UniEncoderSpanRelex architecture (span_mode="markerV0") which
produces both entity span logits AND relation extraction outputs.

Architecture: DeBERTa-v3-large backbone with:
  - Span representation layer (markerV0)
  - GCN-based relations layer for adjacency prediction
  - Pair representation layer for relation scoring

ONNX Inputs (6 tensors):
  - input_ids:      [batch, seq_len]       int64  tokenised input
  - attention_mask:  [batch, seq_len]       int64  1 for real tokens, 0 for padding
  - words_mask:      [batch, seq_len]       int64  maps subtokens to word indices
  - text_lengths:    [batch, 1]             int64  number of words per sample
  - span_idx:        [batch, num_spans, 2]  int64  (start, end) word indices per span
  - span_mask:       [batch, num_spans]     int64  1 for valid spans, 0 for padding

ONNX Outputs (4 tensors):
  - logits:      [batch, num_words, max_width, num_ent_classes]  entity span scores
  - rel_idx:     [batch, num_pairs, 2]                           indices into selected entities
  - rel_logits:  [batch, num_pairs, num_rel_classes]             relation type scores
  - rel_mask:    [batch, num_pairs]                              1 for valid pairs

Usage:
    pip install "gliner==0.2.26" torch onnx onnxruntime
    python scripts/export_relex_onnx.py

    # With quantization:
    python scripts/export_relex_onnx.py --quantize

    # Custom output directory:
    python scripts/export_relex_onnx.py --output ~/.cache/ctxgraph/models/gliner-relex
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path

MODEL_ID = "knowledgator/gliner-relex-large-v0.5"
DEFAULT_OUTPUT = str(
    Path.home() / ".cache" / "ctxgraph" / "models" / "gliner-relex-large-v0.5"
)


def export(
    model_name: str,
    output_dir: str,
    quantize: bool = False,
    opset: int = 19,
) -> None:
    try:
        from gliner import GLiNER
    except ImportError:
        print("Error: gliner package not installed.")
        print("Run: pip install 'gliner==0.2.26' torch onnx onnxruntime")
        sys.exit(1)

    try:
        import onnxruntime as ort
    except ImportError:
        ort = None

    out = Path(output_dir)
    onnx_dir = out / "onnx"
    onnx_dir.mkdir(parents=True, exist_ok=True)

    # ── Load model ──────────────────────────────────────────────────────
    print(f"Loading model: {model_name}")
    print("(This downloads ~1.7GB on first run)")
    model = GLiNER.from_pretrained(model_name)

    # Verify architecture
    config = getattr(model, "config", None)
    if config:
        span_mode = getattr(config, "span_mode", "unknown")
        relations_layer = getattr(config, "relations_layer", None)
        model_backbone = getattr(config, "model_name", "unknown")
        print(f"  backbone:        {model_backbone}")
        print(f"  span_mode:       {span_mode}")
        print(f"  relations_layer: {relations_layer}")
        print(f"  hidden_size:     {getattr(config, 'hidden_size', '?')}")
        print(f"  max_width:       {getattr(config, 'max_width', '?')}")
        print(f"  max_len:         {getattr(config, 'max_len', '?')}")

        if span_mode != "markerV0":
            print(
                f"WARNING: Expected span_mode='markerV0', got '{span_mode}'. "
                "This script is designed for the markerV0 (span-level) relex model."
            )

    # Verify the model type has export_to_onnx support
    # In gliner 0.2.26, export_to_onnx() lives on the GLiNER facade
    # (UniEncoderSpanRelexGLiNER), not on the inner PyTorch model.
    class_name = type(model).__name__
    print(f"  GLiNER variant:  {class_name}")

    if not hasattr(model, "export_to_onnx"):
        print(f"ERROR: {class_name} does not have export_to_onnx method.")
        print("Make sure you have gliner==0.2.26 installed.")
        sys.exit(1)

    # ── Patch diag_embed for ONNX compatibility ────────────────────────
    import torch

    _orig_diag_embed = torch.diag_embed

    def _safe_diag_embed(input, offset=0, dim1=-2, dim2=-1):
        if offset == 0 and dim1 == -2 and dim2 == -1:
            n = input.shape[-1]
            eye = torch.eye(n, dtype=input.dtype, device=input.device)
            return input.unsqueeze(-1) * eye
        return _orig_diag_embed(input, offset, dim1, dim2)

    torch.diag_embed = _safe_diag_embed
    print("Patched torch.diag_embed for ONNX compatibility")

    # ── Build dummy batch with full schema labels + relation types ────
    entity_labels = [
        "Person", "Component", "Service", "Language", "Database",
        "Infrastructure", "Decision", "Constraint", "Metric", "Pattern",
    ]
    relation_labels = [
        "chose", "rejected", "replaced", "depends_on", "fixed",
        "introduced", "deprecated", "caused", "constrained_by",
    ]

    dummy_text = (
        "Sarah chose PostgreSQL over MySQL for the AuthService. "
        "Bob deprecated the legacy SOAP endpoint and introduced a REST API. "
        "The CI pipeline depends on Docker and GitHub Actions replaced Jenkins."
    )

    # Build batch manually with both entity AND relation types
    tokens, _, _ = model.prepare_inputs([dummy_text])
    input_x = model.prepare_base_input(tokens)

    collator = model.data_collator_class(
        model.config,
        data_processor=model.data_processor,
        return_tokens=False,
        return_entities=False,
        return_id_to_classes=False,
        prepare_labels=False,
    )

    loader = torch.utils.data.DataLoader(
        input_x, batch_size=1, shuffle=False,
        collate_fn=lambda batch: collator(
            batch,
            entity_types=entity_labels,
            relation_types=relation_labels,
        ),
    )
    batch = next(iter(loader))
    for k, v in list(batch.items()):
        if isinstance(v, torch.Tensor):
            batch[k] = v.to("cpu")

    print("Dummy batch shapes:")
    for k, v in batch.items():
        if isinstance(v, torch.Tensor):
            print(f"  {k}: {v.shape} {v.dtype}")

    # ── Export to ONNX ──────────────────────────────────────────────────
    print(f"\nExporting to ONNX (opset={opset})...")
    onnx_filename = "model.onnx"
    quantized_filename = "model_quantized.onnx"
    onnx_path = onnx_dir / onnx_filename

    core = model.model.to("cpu").eval()
    spec = model._get_onnx_input_spec()
    all_inputs = tuple(batch[name] for name in spec["input_names"])
    wrapper = model._create_onnx_wrapper(core)

    model._run_torch_onnx_export(
        wrapper, all_inputs,
        spec["input_names"], spec["output_names"],
        spec["dynamic_axes"], onnx_path, opset,
    )

    # Save tokenizer and config alongside the model
    model.data_processor.transformer_tokenizer.save_pretrained(str(onnx_dir))
    gliner_config_src = None
    for candidate in [
        Path(model.config._name_or_path) / "gliner_config.json",
    ]:
        if candidate.exists():
            gliner_config_src = candidate
            break

    if gliner_config_src is None:
        # Try HF cache
        from huggingface_hub import hf_hub_download
        gliner_config_src = Path(hf_hub_download(
            repo_id=model_name, filename="gliner_config.json"
        ))

    if gliner_config_src and gliner_config_src.exists():
        shutil.copy2(gliner_config_src, onnx_dir / "gliner_config.json")

    # Quantize if requested
    quantized_path = None
    if quantize:
        try:
            from onnxruntime.quantization import quantize_dynamic, QuantType
            q_path = onnx_dir / quantized_filename
            quantize_dynamic(str(onnx_path), str(q_path), weight_type=QuantType.QInt8)
            quantized_path = str(q_path)
        except Exception as e:
            print(f"Quantization failed: {e}")

    result = {"onnx_path": str(onnx_path)}
    if quantized_path:
        result["quantized_path"] = quantized_path

    torch.diag_embed = _orig_diag_embed

    onnx_path = Path(result["onnx_path"])
    quantized_path = result.get("quantized_path")

    print(f"Saved ONNX model: {onnx_path}")
    if quantized_path:
        print(f"Saved quantized:  {quantized_path}")

    # ── Verify ONNX model ──────────────────────────────────────────────
    if ort is not None and onnx_path.exists():
        print("\nVerifying ONNX model...")
        session = ort.InferenceSession(str(onnx_path))

        print("  Inputs:")
        for inp in session.get_inputs():
            print(f"    {inp.name}: {inp.shape} ({inp.type})")

        print("  Outputs:")
        for out_node in session.get_outputs():
            print(f"    {out_node.name}: {out_node.shape} ({out_node.type})")

        input_names = {inp.name for inp in session.get_inputs()}
        output_names = {o.name for o in session.get_outputs()}

        expected_inputs = {
            "input_ids", "attention_mask", "words_mask",
            "text_lengths", "span_idx", "span_mask",
        }
        expected_outputs = {"logits", "rel_idx", "rel_logits", "rel_mask"}

        if input_names == expected_inputs:
            print("  Input schema matches expected 6-input span-relex format.")
        else:
            missing = expected_inputs - input_names
            extra = input_names - expected_inputs
            if missing:
                print(f"  WARNING: Missing expected inputs: {missing}")
            if extra:
                print(f"  WARNING: Extra inputs: {extra}")

        if output_names == expected_outputs:
            print("  Output schema matches expected 4-output relex format.")
        else:
            missing = expected_outputs - output_names
            extra = output_names - expected_outputs
            if missing:
                print(f"  WARNING: Missing expected outputs: {missing}")
            if extra:
                print(f"  WARNING: Extra outputs: {extra}")
    else:
        print("onnxruntime not installed — skipping verification.")

    # ── Copy tokenizer to top-level model dir ───────────────────────────
    # export_to_onnx saves tokenizer inside onnx_dir; copy to parent too
    tokenizer_in_onnx = onnx_dir / "tokenizer.json"
    tokenizer_dst = out / "tokenizer.json"

    if tokenizer_in_onnx.exists():
        shutil.copy2(tokenizer_in_onnx, tokenizer_dst)
        print(f"\nCopied tokenizer: {tokenizer_dst}")
    else:
        # Fallback: try to download directly
        try:
            from huggingface_hub import hf_hub_download

            hf_hub_download(
                repo_id=model_name,
                filename="tokenizer.json",
                local_dir=str(out),
            )
            print(f"Downloaded tokenizer: {tokenizer_dst}")
        except Exception as e:
            print(f"WARNING: Could not obtain tokenizer.json: {e}")
            print(f"Please manually copy tokenizer.json to: {tokenizer_dst}")

    # ── Save metadata ───────────────────────────────────────────────────
    metadata = {
        "model_id": model_name,
        "architecture": "UniEncoderSpanRelex",
        "backbone": getattr(config, "model_name", "unknown") if config else "unknown",
        "span_mode": getattr(config, "span_mode", "unknown") if config else "unknown",
        "relations_layer": getattr(config, "relations_layer", None) if config else None,
        "hidden_size": getattr(config, "hidden_size", None) if config else None,
        "max_width": getattr(config, "max_width", None) if config else None,
        "max_len": getattr(config, "max_len", None) if config else None,
        "opset": opset,
        "quantized": quantize,
        "onnx_inputs": [
            "input_ids", "attention_mask", "words_mask",
            "text_lengths", "span_idx", "span_mask",
        ],
        "onnx_outputs": ["logits", "rel_idx", "rel_logits", "rel_mask"],
    }
    meta_path = out / "export_metadata.json"
    with open(meta_path, "w") as f:
        json.dump(metadata, f, indent=2)
    print(f"Saved metadata:   {meta_path}")

    # ── Summary ─────────────────────────────────────────────────────────
    if onnx_path.exists():
        size_mb = onnx_path.stat().st_size / (1024 * 1024)
        print(f"\nONNX model size: {size_mb:.1f} MB")

    print(f"\nDone! Files saved to: {out}")
    print(f"  {onnx_dir / onnx_filename}")
    if quantized_path:
        print(f"  {quantized_path}")
    print(f"  {tokenizer_dst}")
    print(f"  {meta_path}")


def main():
    parser = argparse.ArgumentParser(
        description="Export gliner-relex-large-v0.5 to ONNX for ctxgraph"
    )
    parser.add_argument(
        "--model",
        default=MODEL_ID,
        help=f"HuggingFace model ID (default: {MODEL_ID})",
    )
    parser.add_argument(
        "--output",
        default=DEFAULT_OUTPUT,
        help=f"Output directory (default: {DEFAULT_OUTPUT})",
    )
    parser.add_argument(
        "--quantize",
        action="store_true",
        help="Also produce INT8 quantized model (~50-75%% smaller)",
    )
    parser.add_argument(
        "--opset",
        type=int,
        default=19,
        help="ONNX opset version (default: 19)",
    )
    args = parser.parse_args()
    export(args.model, args.output, args.quantize, args.opset)


if __name__ == "__main__":
    main()

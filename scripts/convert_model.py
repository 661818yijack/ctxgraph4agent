#!/usr/bin/env python3
"""Convert GLiNER multitask model to ONNX format for ctxgraph.

This script converts the `knowledgator/gliner-multitask-large-v0.5` model
from PyTorch to ONNX format, ready for use with gline-rs RelationPipeline.

This model uses span_mode="token_level" (4 ONNX inputs: input_ids,
attention_mask, words_mask, text_lengths) — the only format compatible
with gline-rs RelationPipeline.

Usage (local):
    pip install gliner onnx onnxruntime
    python scripts/convert_model.py

Usage (Google Colab):
    !pip install gliner onnx onnxruntime
    !python scripts/convert_model.py
    # Then download the output files from ~/.cache/ctxgraph/models/

Output:
    ~/.cache/ctxgraph/models/gliner-multitask-large-v0.5/onnx/model.onnx
    ~/.cache/ctxgraph/models/gliner-multitask-large-v0.5/tokenizer.json
"""

import shutil
import sys
from pathlib import Path


def main():
    try:
        from gliner import GLiNER
    except ImportError:
        print("Error: gliner package not installed.")
        print("Run: pip install gliner onnx onnxruntime")
        return 1

    model_name = "knowledgator/gliner-multitask-large-v0.5"
    cache_dir = Path.home() / ".cache" / "ctxgraph" / "models" / "gliner-multitask-large-v0.5"
    onnx_dir = cache_dir / "onnx"

    print(f"Loading model: {model_name}")
    print("(This downloads ~1.3GB on first run)")
    model = GLiNER.from_pretrained(model_name)

    # Verify this is a token-level model (required by gline-rs)
    config = getattr(model, "config", None)
    if config:
        span_mode = getattr(config, "span_mode", "unknown")
        print(f"Model span_mode: {span_mode}")
        if span_mode != "token_level":
            print(f"WARNING: Expected span_mode='token_level', got '{span_mode}'")
            print("gline-rs RelationPipeline requires a token-level model.")
            print("The exported ONNX may not be compatible.")

    print("Converting to ONNX...")
    onnx_dir.mkdir(parents=True, exist_ok=True)
    onnx_path = str(onnx_dir / "model.onnx")
    model.to_onnx(onnx_path)
    print(f"Saved ONNX model to: {onnx_path}")

    # Verify ONNX model inputs
    try:
        import onnxruntime as ort
        session = ort.InferenceSession(onnx_path)
        input_names = [inp.name for inp in session.get_inputs()]
        print(f"ONNX inputs: {input_names}")

        expected = {"input_ids", "attention_mask", "words_mask", "text_lengths"}
        actual = set(input_names)
        if actual == expected:
            print("Input schema matches gline-rs TokenPipeline (4 inputs).")
        elif actual.issuperset(expected):
            print(f"WARNING: Model has extra inputs: {actual - expected}")
            print("gline-rs may reject this model if inputs don't match exactly.")
        else:
            print(f"WARNING: Missing expected inputs: {expected - actual}")
            print("This model may not be compatible with gline-rs.")
    except ImportError:
        print("onnxruntime not installed — skipping input verification.")
        print("Run: pip install onnxruntime")

    # Copy tokenizer
    hf_cache = Path.home() / ".cache" / "huggingface" / "hub"
    tokenizer_src = None

    for d in hf_cache.glob(
        "models--knowledgator--gliner-multitask-large-v0.5/snapshots/*/tokenizer.json"
    ):
        tokenizer_src = d
        break

    if tokenizer_src and tokenizer_src.exists():
        tokenizer_dst = cache_dir / "tokenizer.json"
        shutil.copy2(tokenizer_src, tokenizer_dst)
        print(f"Copied tokenizer to: {tokenizer_dst}")
    else:
        # Try downloading tokenizer directly
        try:
            from huggingface_hub import hf_hub_download
            tokenizer_dst = cache_dir / "tokenizer.json"
            downloaded = hf_hub_download(
                repo_id=model_name,
                filename="tokenizer.json",
                local_dir=str(cache_dir),
            )
            print(f"Downloaded tokenizer to: {tokenizer_dst}")
        except Exception:
            print("Warning: tokenizer.json not found in HF cache.")
            print(f"Please manually copy tokenizer.json to: {cache_dir / 'tokenizer.json'}")

    # Print file sizes
    onnx_file = Path(onnx_path)
    if onnx_file.exists():
        size_mb = onnx_file.stat().st_size / (1024 * 1024)
        print(f"\nONNX model size: {size_mb:.1f} MB")

    print()
    print("Done! The model is ready for ctxgraph relation extraction.")
    print(f"Model dir: {cache_dir}")
    print()
    print("To use with ctxgraph, ensure these files exist:")
    print(f"  {onnx_dir / 'model.onnx'}")
    print(f"  {cache_dir / 'tokenizer.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

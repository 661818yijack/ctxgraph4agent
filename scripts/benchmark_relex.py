#!/usr/bin/env python3
"""Benchmark gliner-relex-large-v0.5 against the 50 benchmark episodes.

Usage:
    python scripts/benchmark_relex.py [--threshold 0.5] [--rel-threshold 0.5]
    python scripts/benchmark_relex.py --onnx  # test ONNX model via onnxruntime
"""
import json
import sys
import time
from pathlib import Path

def compute_f1(predicted: set, expected: set):
    if not predicted and not expected:
        return 1.0, 1.0, 1.0
    tp = len(predicted & expected)
    p = tp / len(predicted) if predicted else 0.0
    r = tp / len(expected) if expected else 0.0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
    return p, r, f1


def fuzzy_match_entity(pred_text: str, exp_text: str) -> bool:
    """Check if predicted entity text is a fuzzy match for expected."""
    p, e = pred_text.lower(), exp_text.lower()
    if p == e:
        return True
    # Substring match (either direction)
    if p in e or e in p:
        return True
    return False


def compute_fuzzy_rel_f1(predicted_rels: set, expected_rels: set):
    """Compute relation F1 with fuzzy entity name matching."""
    if not predicted_rels and not expected_rels:
        return 1.0, 1.0, 1.0

    # Parse triples
    pred_triples = []
    for r in predicted_rels:
        parts = r.split(":")
        if len(parts) == 3:
            pred_triples.append((parts[0], parts[1], parts[2]))

    exp_triples = []
    for r in expected_rels:
        parts = r.split(":")
        if len(parts) == 3:
            exp_triples.append((parts[0], parts[1], parts[2]))

    # Match: same relation type + fuzzy head/tail match
    matched_pred = set()
    matched_exp = set()
    for pi, (ph, pr, pt) in enumerate(pred_triples):
        for ei, (eh, er, et) in enumerate(exp_triples):
            if ei in matched_exp:
                continue
            if pr == er and fuzzy_match_entity(ph, eh) and fuzzy_match_entity(pt, et):
                matched_pred.add(pi)
                matched_exp.add(ei)
                break

    tp = len(matched_exp)
    p = tp / len(pred_triples) if pred_triples else 0.0
    r = tp / len(exp_triples) if exp_triples else 0.0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
    return p, r, f1


def main():
    threshold = 0.5
    rel_threshold = 0.5
    for arg in sys.argv[1:]:
        if arg.startswith("--threshold="):
            threshold = float(arg.split("=")[1])
        elif arg.startswith("--rel-threshold="):
            rel_threshold = float(arg.split("=")[1])

    fixture = Path(__file__).parent.parent / "crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json"
    episodes = json.loads(fixture.read_text())

    # Entity types and relation types from the schema
    # Map: description -> canonical type key (matches Rust entity_label_descriptions())
    entity_desc_to_key = {
        "person or engineer": "Person",
        "software library or framework": "Component",
        "cloud service or API": "Service",
        "programming language": "Language",
        "database or data store": "Database",
        "server or cloud platform": "Infrastructure",
        "architectural decision": "Decision",
        "technical constraint": "Constraint",
        "performance metric": "Metric",
        "design pattern": "Pattern",
    }

    use_descriptions = True
    for arg in sys.argv[1:]:
        if arg == "--no-descriptions":
            use_descriptions = False

    if use_descriptions:
        entity_labels = list(entity_desc_to_key.keys())
        print("Using entity descriptions as labels (zero-shot mode)")
    else:
        entity_labels = list(entity_desc_to_key.values())
        print("Using entity type names as labels")

    relation_labels = [
        "chose", "rejected", "replaced", "depends_on", "fixed",
        "introduced", "deprecated", "caused", "constrained_by",
    ]

    use_onnx = "--onnx" in sys.argv
    onnx_path = Path.home() / ".cache/ctxgraph/models/gliner-relex-large-v0.5/onnx"

    from gliner import GLiNER
    if use_onnx:
        print(f"Loading ONNX model from {onnx_path}...")
        model = GLiNER.from_pretrained(
            str(onnx_path),
            load_onnx_model=True,
            onnx_model_file="model.onnx",
        )
        print("ONNX model loaded.")
    else:
        print("Loading gliner-relex-large-v0.5 (PyTorch)...")
        model = GLiNER.from_pretrained("knowledgator/gliner-relex-large-v0.5")
        print("PyTorch model loaded.")
    model.eval()

    total_entity_f1 = 0.0
    total_relation_f1 = 0.0
    total_fuzzy_rel_f1 = 0.0
    total_text_f1 = 0.0
    total_time = 0.0

    errors = 0
    for i, ep in enumerate(episodes):
        text = ep["text"]
        t0 = time.time()
        try:
            result = model.inference(
                [text], entity_labels,
                relations=relation_labels,
                threshold=threshold,
                relation_threshold=rel_threshold,
                return_relations=True,
            )
            pred_entities = result[0][0] if result[0] else []
            pred_relations = result[1][0] if result[1] else []
        except Exception as e:
            print(f"Episode {i:2d}: INFERENCE ERROR: {e}")
            pred_entities = []
            pred_relations = []
            errors += 1
        elapsed = time.time() - t0
        total_time += elapsed

        # Map predicted entity labels back to canonical types
        def canon_type(label):
            if use_descriptions:
                return entity_desc_to_key.get(label, label)
            return label

        # Entity F1 (name:type)
        pred_ent_set = {f"{e['text']}:{canon_type(e['label'])}" for e in pred_entities}
        exp_ent_set = {f"{e['name']}:{e['entity_type']}" for e in ep["expected_entities"]}
        _, _, ent_f1 = compute_f1(pred_ent_set, exp_ent_set)

        # Text-only entity F1
        pred_text_set = {e["text"].lower() for e in pred_entities}
        exp_text_set = {e["name"].lower() for e in ep["expected_entities"]}
        _, _, text_f1 = compute_f1(pred_text_set, exp_text_set)

        # Relation F1 (head:relation:tail)
        pred_rel_set = {f"{r['head']['text']}:{r['relation']}:{r['tail']['text']}" for r in pred_relations}
        exp_rel_set = {f"{r['head']}:{r['relation']}:{r['tail']}" for r in ep["expected_relations"]}
        rp, rr, rel_f1 = compute_f1(pred_rel_set, exp_rel_set)
        frp, frr, frel_f1 = compute_fuzzy_rel_f1(pred_rel_set, exp_rel_set)

        print(f"Episode {i:2d}: entities F1={ent_f1:.3f} text-only={text_f1:.3f} | relations strict={rel_f1:.3f} fuzzy={frel_f1:.3f} (P={frp:.3f} R={frr:.3f})")
        if rel_f1 < 1.0:
            missed = exp_rel_set - pred_rel_set
            spurious = pred_rel_set - exp_rel_set
            if missed:
                print(f"  MISSED: {sorted(missed)}")
            if spurious:
                print(f"  SPURIOUS: {sorted(spurious)}")

        total_entity_f1 += ent_f1
        total_relation_f1 += rel_f1
        total_fuzzy_rel_f1 += frel_f1
        total_text_f1 += text_f1

    n = len(episodes)
    avg_ent = total_entity_f1 / n
    avg_text = total_text_f1 / n
    avg_rel = total_relation_f1 / n
    avg_frel = total_fuzzy_rel_f1 / n
    combined = (avg_ent + avg_rel) / 2.0
    combined_fuzzy = (avg_text + avg_frel) / 2.0

    print()
    print("=== RELEX BENCHMARK RESULTS ===")
    print(f"Average entity F1 (name+type): {avg_ent:.3f}")
    print(f"Average entity F1 (name only):  {avg_text:.3f}")
    print(f"Average relation F1 (strict):   {avg_rel:.3f}")
    print(f"Average relation F1 (fuzzy):    {avg_frel:.3f}")
    print(f"Combined F1 (strict):           {combined:.3f}")
    print(f"Combined F1 (fuzzy):            {combined_fuzzy:.3f}")
    print(f"Target:                         0.800")
    print(f"Thresholds: entity={threshold}, relation={rel_threshold}")
    print(f"Backend: {'ONNX' if use_onnx else 'PyTorch'}")
    print(f"Total time: {total_time:.1f}s ({total_time/n:.2f}s/episode)")
    if errors:
        print(f"Inference errors: {errors}/{n}")
    print("================================")


if __name__ == "__main__":
    main()

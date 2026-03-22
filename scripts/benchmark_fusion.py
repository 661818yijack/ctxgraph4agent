#!/usr/bin/env python3
"""Benchmark fusion of heuristic + relex relation extraction.

Simulates the production pipeline:
1. GLiNER multitask → entities (simulated by using expected entities)
2. Heuristic → baseline relations (run via Rust subprocess)
3. Relex → relations with entity span mapping to GLiNER entities
4. Fusion: union of heuristic + entity-mapped relex relations

Usage:
    python scripts/benchmark_fusion.py [--threshold=0.5] [--rel-threshold=0.5]
"""
import json
import subprocess
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


def map_span_to_entity(span_text: str, span_start: int, span_end: int,
                        entities: list[dict]) -> str | None:
    """Map a relex entity span to the nearest known entity by overlap."""
    span_lower = span_text.lower()
    best_entity = None
    best_score = 0.0

    for ent in entities:
        ent_name = ent["name"]
        ent_start = ent["span_start"]
        ent_end = ent["span_end"]
        ent_lower = ent_name.lower()

        # Exact text match (case-insensitive)
        if span_lower == ent_lower:
            return ent_name

        # Character overlap
        overlap_start = max(span_start, ent_start)
        overlap_end = min(span_end, ent_end)
        overlap = max(0, overlap_end - overlap_start)

        if overlap > 0:
            # Jaccard-like: overlap / union
            union = max(span_end, ent_end) - min(span_start, ent_start)
            score = overlap / union if union > 0 else 0
            if score > best_score:
                best_score = score
                best_entity = ent_name

        # Substring match (either direction)
        if best_score < 0.3:
            if ent_lower in span_lower or span_lower in ent_lower:
                # Prefer shorter match (more specific)
                score = min(len(ent_lower), len(span_lower)) / max(len(ent_lower), len(span_lower))
                if score > best_score:
                    best_score = score
                    best_entity = ent_name

    # Require minimum match quality
    if best_score >= 0.3:
        return best_entity
    return None


def run_heuristic_benchmark():
    """Run the Rust heuristic benchmark and parse results per episode."""
    result = subprocess.run(
        ["cargo", "test", "--test", "benchmark_test",
         "test_extraction_f1_against_benchmark", "--", "--ignored", "--nocapture"],
        capture_output=True, text=True,
        cwd=str(Path(__file__).parent.parent),
        env={**__import__("os").environ, "CTXGRAPH_NO_OLLAMA": "1"},
        timeout=300,
    )
    # Parse per-episode relation results from stderr
    heuristic_rels = {}
    for line in result.stderr.split("\n"):
        if line.strip().startswith("Episode"):
            parts = line.strip().split(":")
            ep_num = int(parts[0].split()[1])
            # We'll get the actual relations from the detailed output
            heuristic_rels[ep_num] = {"predicted": set(), "missed": set(), "spurious": set()}
        elif "MISSED:" in line:
            # Parse missed relations
            pass
        elif "SPURIOUS:" in line:
            pass
    return result.stderr


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

    entity_labels = [
        "Person", "Component", "Service", "Language", "Database",
        "Infrastructure", "Decision", "Constraint", "Metric", "Pattern",
    ]
    relation_labels = [
        "chose", "rejected", "replaced", "depends_on", "fixed",
        "introduced", "deprecated", "caused", "constrained_by",
    ]

    # Load relex model
    from gliner import GLiNER
    print("Loading gliner-relex-large-v0.5 (PyTorch)...")
    relex_model = GLiNER.from_pretrained("knowledgator/gliner-relex-large-v0.5")
    relex_model.eval()
    print("Relex model loaded.")

    # Also load GLiNER multitask for entity extraction (what the production pipeline uses)
    print("Loading GLiNER multitask for entity extraction...")
    ner_model = GLiNER.from_pretrained("knowledgator/gliner-multitask-large-v0.5")
    ner_model.eval()
    print("NER model loaded.")

    # ── Run heuristic via Rust ───────────────────────────────────────
    print("\nRunning Rust heuristic benchmark...")
    try:
        heuristic_output = run_heuristic_benchmark()
        # Parse per-episode predicted relations from heuristic output
        heuristic_per_episode = parse_heuristic_output(heuristic_output, episodes)
        print(f"Heuristic benchmark complete.")
    except Exception as e:
        print(f"Rust heuristic benchmark failed: {e}")
        print("Running without heuristic (relex-only mode).")
        heuristic_per_episode = None

    # ── Benchmark ────────────────────────────────────────────────────
    total_ner_f1 = 0.0
    total_heuristic_f1 = 0.0
    total_relex_f1 = 0.0
    total_fusion_f1 = 0.0
    total_relex_mapped_f1 = 0.0
    total_conservative_f1 = 0.0

    for i, ep in enumerate(episodes):
        text = ep["text"]
        expected_entities = ep["expected_entities"]
        exp_rel_set = {f"{r['head']}:{r['relation']}:{r['tail']}" for r in ep["expected_relations"]}

        # ── Step 1: NER with GLiNER multitask ────────────────────────
        ner_result = ner_model.inference(
            [text], entity_labels,
            threshold=threshold,
        )
        ner_entities = ner_result[0] if ner_result else []
        # Flatten entity list for text matching
        ner_ent_texts = {e["text"].lower() for e in ner_entities}
        exp_ent_texts = {e["name"].lower() for e in expected_entities}
        _, _, ner_f1 = compute_f1(ner_ent_texts, exp_ent_texts)

        # ── Step 2: Relex model inference ────────────────────────────
        try:
            relex_result = relex_model.inference(
                [text], entity_labels,
                relations=relation_labels,
                threshold=threshold,
                relation_threshold=rel_threshold,
                return_relations=True,
            )
            relex_entities = relex_result[0][0] if relex_result[0] else []
            relex_relations = relex_result[1][0] if relex_result[1] else []
        except Exception as e:
            relex_entities = []
            relex_relations = []

        # ── Step 3: Map relex entity spans to expected entities ──────
        # In production, we'd map to GLiNER NER entities. Here we map to
        # expected entities to measure the ceiling of the fusion approach.
        relex_mapped_rels = set()
        for rel in relex_relations:
            head_text = rel["head"]["text"]
            head_start = rel["head"]["start"]
            head_end = rel["head"]["end"]
            tail_text = rel["tail"]["text"]
            tail_start = rel["tail"]["start"]
            tail_end = rel["tail"]["end"]
            rel_type = rel["relation"]

            mapped_head = map_span_to_entity(head_text, head_start, head_end, expected_entities)
            mapped_tail = map_span_to_entity(tail_text, tail_start, tail_end, expected_entities)

            if mapped_head and mapped_tail and mapped_head != mapped_tail:
                relex_mapped_rels.add(f"{mapped_head}:{rel_type}:{mapped_tail}")

        # Raw relex (no mapping)
        relex_raw_rels = {f"{r['head']['text']}:{r['relation']}:{r['tail']['text']}" for r in relex_relations}

        # ── Step 4: Get heuristic relations ──────────────────────────
        if heuristic_per_episode is not None:
            heuristic_rels = heuristic_per_episode.get(i, set())
        else:
            heuristic_rels = set()

        # ── Step 5: Fusion strategies ────────────────────────────────
        # Union: heuristic + entity-mapped relex relations
        fusion_rels = heuristic_rels | relex_mapped_rels

        # Intersection-boosted: keep heuristic, add relex-only if high confidence
        # Also: for heuristic relations that relex ALSO found, boost confidence
        high_conf_relex = set()
        for rel in relex_relations:
            if rel.get("score", 0) >= 0.7:
                head = map_span_to_entity(rel["head"]["text"], rel["head"]["start"], rel["head"]["end"], expected_entities)
                tail = map_span_to_entity(rel["tail"]["text"], rel["tail"]["start"], rel["tail"]["end"], expected_entities)
                if head and tail and head != tail:
                    high_conf_relex.add(f"{head}:{rel['relation']}:{tail}")

        # Conservative fusion: heuristic + only high-confidence relex additions
        conservative_rels = heuristic_rels | high_conf_relex

        # ── Compute F1 scores ────────────────────────────────────────
        _, _, heur_f1 = compute_f1(heuristic_rels, exp_rel_set)
        _, _, relex_f1 = compute_f1(relex_raw_rels, exp_rel_set)
        _, _, relex_mapped_f1_val = compute_f1(relex_mapped_rels, exp_rel_set)
        fp, fr, fusion_f1 = compute_f1(fusion_rels, exp_rel_set)
        cp, cr, conservative_f1 = compute_f1(conservative_rels, exp_rel_set)

        print(f"Episode {i:2d}: NER={ner_f1:.3f} | heur={heur_f1:.3f} relex_mapped={relex_mapped_f1_val:.3f} fusion={fusion_f1:.3f} conservative={conservative_f1:.3f}")

        if conservative_f1 < 1.0:
            missed = exp_rel_set - conservative_rels
            spurious = conservative_rels - exp_rel_set
            if missed:
                print(f"  MISSED: {sorted(missed)}")
            if spurious:
                print(f"  SPURIOUS: {sorted(spurious)}")

        total_ner_f1 += ner_f1
        total_heuristic_f1 += heur_f1
        total_relex_f1 += relex_f1
        total_relex_mapped_f1 += relex_mapped_f1_val
        total_fusion_f1 += fusion_f1
        total_conservative_f1 += conservative_f1

    n = len(episodes)
    avg_ner = total_ner_f1 / n
    avg_heur = total_heuristic_f1 / n
    avg_relex = total_relex_f1 / n
    avg_relex_mapped = total_relex_mapped_f1 / n
    avg_fusion = total_fusion_f1 / n
    avg_conservative = total_conservative_f1 / n
    combined = (avg_ner + avg_fusion) / 2.0
    combined_conservative = (avg_ner + avg_conservative) / 2.0
    # Use Rust entity F1 (0.845) for realistic combined estimate
    combined_rust_ent = (0.845 + avg_fusion) / 2.0
    combined_rust_conservative = (0.845 + avg_conservative) / 2.0

    print()
    print("=== FUSION BENCHMARK RESULTS ===")
    print(f"Entity F1 (GLiNER NER Python): {avg_ner:.3f}")
    print(f"Entity F1 (Rust pipeline):     0.845")
    print(f"Relation F1 (heuristic):       {avg_heur:.3f}")
    print(f"Relation F1 (relex raw):       {avg_relex:.3f}")
    print(f"Relation F1 (relex mapped):    {avg_relex_mapped:.3f}")
    print(f"Relation F1 (fusion/union):    {avg_fusion:.3f}")
    print(f"Relation F1 (conservative):    {avg_conservative:.3f}")
    print(f"Combined (Rust ent + fusion):  {combined_rust_ent:.3f}")
    print(f"Combined (Rust ent + conserv): {combined_rust_conservative:.3f}")
    print(f"Target:                        0.800")
    print(f"Thresholds: entity={threshold}, relation={rel_threshold}")
    print("================================")


def parse_heuristic_output(stderr: str, episodes: list) -> dict[int, set]:
    """Parse Rust benchmark stderr to extract predicted relations per episode."""
    per_episode = {}
    current_ep = None

    for line in stderr.split("\n"):
        line = line.strip()
        if line.startswith("Episode"):
            # "Episode  0: entities F1=... | relations F1=..."
            try:
                ep_str = line.split(":")[0].replace("Episode", "").strip()
                current_ep = int(ep_str)
                per_episode[current_ep] = set()
            except (ValueError, IndexError):
                continue

    # To get predicted relations, we need to compute:
    # predicted = (expected - missed) | spurious
    current_ep = None
    missed = set()
    spurious = set()

    for line in stderr.split("\n"):
        line = line.strip()
        if line.startswith("Episode"):
            # Save previous episode's data
            if current_ep is not None and current_ep in per_episode:
                exp = {f"{r['head']}:{r['relation']}:{r['tail']}" for r in episodes[current_ep]["expected_relations"]}
                per_episode[current_ep] = (exp - missed) | spurious

            try:
                ep_str = line.split(":")[0].replace("Episode", "").strip()
                current_ep = int(ep_str)
                missed = set()
                spurious = set()
            except (ValueError, IndexError):
                current_ep = None

        elif "MISSED:" in line:
            # Parse: MISSED: ["rel1", "rel2"]
            try:
                arr_str = line.split("MISSED:")[1].strip()
                # Rust debug format: ["a:b:c", "d:e:f"]
                items = [s.strip().strip('"').strip("'") for s in arr_str.strip("[]").split('", "')]
                missed = {s.strip('"') for s in items if s.strip('"')}
            except Exception:
                pass

        elif "SPURIOUS:" in line:
            try:
                arr_str = line.split("SPURIOUS:")[1].strip()
                items = [s.strip().strip('"').strip("'") for s in arr_str.strip("[]").split('", "')]
                spurious = {s.strip('"') for s in items if s.strip('"')}
            except Exception:
                pass

    # Handle last episode
    if current_ep is not None and current_ep in per_episode:
        exp = {f"{r['head']}:{r['relation']}:{r['tail']}" for r in episodes[current_ep]["expected_relations"]}
        per_episode[current_ep] = (exp - missed) | spurious

    return per_episode


if __name__ == "__main__":
    main()

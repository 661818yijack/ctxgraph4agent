#!/usr/bin/env python3
"""Quick test: can local Ollama models extract/clean entities?

Tests two approaches:
1. Full extraction: LLM finds entities from scratch (like Graphiti)
2. Cleanup: LLM fixes GLiNER's entity names

Tests on the hardest episodes where GLiNER gets entities wrong.
"""
import json, time, requests
from pathlib import Path

OLLAMA_URL = "http://localhost:11434/api/generate"

# Hard episodes where GLiNER fails (entity F1 < 0.8)
HARD_CASES = [
    {
        "text": "Set up Terraform modules to provision all AWS resources with IAM roles following least privilege principle.",
        "expected": ["Terraform", "AWS", "IAM", "least privilege"],
        "gliner_gets": ["Terraform modules", "AWS", "IAM roles", "least privilege"],
    },
    {
        "text": "Rewrote RecommendationService in Java 21 with virtual threads, cutting GC pause times by 60%.",
        "expected": ["RecommendationService", "Java", "virtual threads", "GC pause times"],
        "gliner_gets": ["RecommendationService", "Java 21", "Java 11", "virtual threads"],
    },
    {
        "text": "All services must meet p99 < 100ms SLA. Built SLO Dashboard to track compliance across regions.",
        "expected": ["SLO Dashboard", "100ms SLA", "p99"],
        "gliner_gets": ["services", "SLO", "p99", "100ms SLA"],
    },
    {
        "text": "ADR-010: Adopted Istio service mesh with mTLS for zero-trust networking between all services.",
        "expected": ["Istio", "mTLS", "zero-trust"],
        "gliner_gets": ["ADR-010", "Istio", "service mesh", "mTLS", "zero-trust"],
    },
    {
        "text": "Migrated from Nomad to Kubernetes for container orchestration. ADR-004 documents the decision. Helm charts manage all deployments.",
        "expected": ["Kubernetes", "Nomad", "Helm"],
        "gliner_gets": ["ADR-004", "Kubernetes", "Nomad", "Helm"],
    },
]

ENTITY_TYPES = "Person, Component, Service, Language, Database, Infrastructure, Decision, Constraint, Metric, Pattern"

# ── Approach 1: Full extraction (like Graphiti) ──────────────────
EXTRACT_PROMPT = """Extract named entities from this text. Return ONLY a JSON array of objects with "name" and "type" fields.

Entity types: {types}

Rules:
- Use the shortest precise name (e.g., "Terraform" not "Terraform modules")
- Only extract real technology names, people, services, patterns
- Don't extract generic words like "services", "resources", "modules"

Text: {text}

Output ONLY a JSON array, no other text."""

# ── Approach 2: Cleanup (fix GLiNER output) ──────────────────────
CLEANUP_PROMPT = """A NER model extracted these entities from the text below. Some entity names may have extra words or be incorrect. Fix them.

Extracted entities: {entities}

Text: {text}

Rules:
- Fix entity names to be the shortest precise form (e.g., "Terraform modules" → "Terraform")
- Remove generic entities that aren't real names (e.g., "services", "resources")
- Keep entities that are correct as-is
- Return ONLY a JSON array of corrected entity name strings

Output ONLY a JSON array of strings, no other text."""


def call_ollama(model: str, prompt: str, timeout: int = 30) -> tuple[str, float]:
    t0 = time.time()
    resp = requests.post(OLLAMA_URL, json={
        "model": model, "prompt": prompt, "stream": False,
        "options": {"temperature": 0.0, "num_predict": 256}
    }, timeout=timeout)
    lat = time.time() - t0
    return resp.json().get("response", ""), lat


def extract_names(response: str) -> set[str]:
    """Extract entity names from LLM response."""
    import re
    names = set()
    # Try JSON array of objects
    try:
        data = json.loads(response.strip())
        if isinstance(data, list):
            for item in data:
                if isinstance(item, dict) and "name" in item:
                    names.add(item["name"])
                elif isinstance(item, str):
                    names.add(item)
            return names
    except json.JSONDecodeError:
        pass
    # Try extracting from markdown code block
    m = re.search(r'```(?:json)?\s*\n?(.*?)\n?```', response, re.DOTALL)
    if m:
        try:
            data = json.loads(m.group(1).strip())
            if isinstance(data, list):
                for item in data:
                    if isinstance(item, dict) and "name" in item:
                        names.add(item["name"])
                    elif isinstance(item, str):
                        names.add(item)
                return names
        except json.JSONDecodeError:
            pass
    return names


def compute_f1(predicted: set, expected: set) -> tuple[float, float, float]:
    pred_lower = {p.lower() for p in predicted}
    exp_lower = {e.lower() for e in expected}
    if not pred_lower and not exp_lower:
        return 1.0, 1.0, 1.0
    tp = len(pred_lower & exp_lower)
    p = tp / len(pred_lower) if pred_lower else 0.0
    r = tp / len(exp_lower) if exp_lower else 0.0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
    return p, r, f1


def main():
    models = ["qwen2.5:1.5b", "qwen2.5:3b", "qwen2.5:7b"]

    for model in models:
        print(f"\n{'='*60}")
        print(f"Model: {model}")
        print(f"{'='*60}")

        # ── Test 1: Full extraction ──
        print(f"\n  --- Full Extraction (like Graphiti) ---")
        total_f1 = 0.0
        for i, case in enumerate(HARD_CASES):
            prompt = EXTRACT_PROMPT.format(types=ENTITY_TYPES, text=case["text"])
            try:
                response, lat = call_ollama(model, prompt)
                predicted = extract_names(response)
                expected = set(case["expected"])
                p, r, f1 = compute_f1(predicted, expected)
                total_f1 += f1
                print(f"    Case {i}: F1={f1:.3f} ({lat:.1f}s) pred={sorted(predicted)}")
                if f1 < 1.0:
                    missed = {e for e in expected if e.lower() not in {p.lower() for p in predicted}}
                    extra = {p for p in predicted if p.lower() not in {e.lower() for e in expected}}
                    if missed: print(f"      missed: {sorted(missed)}")
                    if extra: print(f"      extra:  {sorted(extra)}")
            except Exception as e:
                print(f"    Case {i}: ERROR — {e}")
        print(f"  >> Full extraction avg F1: {total_f1/len(HARD_CASES):.3f}")

        # ── Test 2: Cleanup ──
        print(f"\n  --- Cleanup (fix GLiNER output) ---")
        total_f1 = 0.0
        for i, case in enumerate(HARD_CASES):
            prompt = CLEANUP_PROMPT.format(
                entities=json.dumps(case["gliner_gets"]),
                text=case["text"]
            )
            try:
                response, lat = call_ollama(model, prompt)
                predicted = extract_names(response)
                expected = set(case["expected"])
                p, r, f1 = compute_f1(predicted, expected)
                total_f1 += f1

                gliner_set = set(case["gliner_gets"])
                _, _, gliner_f1 = compute_f1(gliner_set, expected)

                delta = f1 - gliner_f1
                arrow = "↑" if delta > 0 else "↓" if delta < 0 else "="
                print(f"    Case {i}: GLiNER={gliner_f1:.3f} → cleaned={f1:.3f} {arrow} ({lat:.1f}s)")
                if f1 < 1.0:
                    print(f"      cleaned: {sorted(predicted)}")
                    missed = {e for e in expected if e.lower() not in {p.lower() for p in predicted}}
                    if missed: print(f"      missed:  {sorted(missed)}")
            except Exception as e:
                print(f"    Case {i}: ERROR — {e}")
        print(f"  >> Cleanup avg F1: {total_f1/len(HARD_CASES):.3f}")

        # ── GLiNER baseline ──
        gliner_total = 0.0
        for case in HARD_CASES:
            _, _, f1 = compute_f1(set(case["gliner_gets"]), set(case["expected"]))
            gliner_total += f1
        print(f"  >> GLiNER baseline:  {gliner_total/len(HARD_CASES):.3f}")


if __name__ == "__main__":
    main()

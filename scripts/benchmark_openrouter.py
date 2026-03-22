#!/usr/bin/env python3
"""Benchmark open-source LLMs via OpenRouter for relation extraction.

Tests multiple models against the 50-episode benchmark to find the best
model for local deployment. Uses OpenRouter's OpenAI-compatible API.

Usage:
    python scripts/benchmark_openrouter.py [--models=model1,model2] [--episodes=5]

Env:
    OPENROUTER_API_KEY  — required (or reads from .env)
"""
import json
import os
import sys
import time
from pathlib import Path

import requests

# ── Load .env ──────────────────────────────────────────────────────
def load_dotenv():
    env_path = Path(__file__).parent.parent / ".env"
    if env_path.exists():
        for line in env_path.read_text().splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" in line:
                key, _, val = line.partition("=")
                key = key.strip()
                val = val.split("#")[0].strip()  # strip inline comments
                if val and key not in os.environ:
                    os.environ[key] = val

load_dotenv()

# ── Models to benchmark ───────────────────────────────────────────
# Format: (openrouter_model_id, display_name, local_equivalent)
MODELS = [
    # Qwen family — strong at structured output
    ("qwen/qwen-2.5-7b-instruct", "Qwen 2.5 7B", "qwen2.5:7b"),
    ("qwen/qwen-2.5-72b-instruct", "Qwen 2.5 72B", "qwen2.5:72b"),
    ("qwen/qwen3-8b", "Qwen 3 8B", "qwen3:8b"),
    ("qwen/qwen3-32b", "Qwen 3 32B", "qwen3:32b"),

    # DeepSeek — good reasoning
    ("deepseek/deepseek-chat-v3-0324", "DeepSeek V3 0324", "N/A"),
    ("deepseek/deepseek-r1-0528", "DeepSeek R1", "N/A"),

    # Llama family
    ("meta-llama/llama-3.3-70b-instruct", "Llama 3.3 70B", "llama3.3:70b"),
    ("meta-llama/llama-4-maverick", "Llama 4 Maverick", "N/A"),

    # Mistral
    ("mistralai/mistral-small-3.2-24b-instruct", "Mistral Small 3.2 24B", "mistral-small3.2:24b"),

    # Gemma
    ("google/gemma-3-27b-it", "Gemma 3 27B", "gemma3:27b"),

    # Phi (small/efficient)
    ("microsoft/phi-4", "Phi-4 14B", "phi4:14b"),
]

# ── Prompts (matching api.rs) ─────────────────────────────────────
RELATION_TYPES = [
    "chose", "rejected", "replaced", "depends_on", "fixed",
    "introduced", "deprecated", "caused", "constrained_by",
]

SYSTEM_PROMPT = """You are a precise software architecture knowledge graph extractor. Extract directed relationships between entities.

Relation types with direction rules and examples:
- chose: Person/Service adopted a technology. head=chooser, tail=chosen. "Alice chose PostgreSQL" → chose(Alice,PostgreSQL)
- rejected: Person/Service rejected an alternative. head=rejector, tail=rejected. "decided against MongoDB" → rejected(Alice,MongoDB)
- replaced: NEW replaced OLD. head=NEW, tail=OLD. "migrated from MySQL to PostgreSQL" → replaced(PostgreSQL,MySQL)
- depends_on: Consumer depends on provider. head=consumer, tail=provider. "PaymentService uses Redis" → depends_on(PaymentService,Redis)
- fixed: Fixer fixed something. head=fixer, tail=fixed. "Bob patched AuthService" → fixed(Bob,AuthService)
- introduced: Added a new component. head=introducer, tail=introduced. "added Prometheus" → introduced(BillingService,Prometheus)
- deprecated: Removed/phased out. head=deprecator, tail=deprecated. "sunset the SOAP endpoint" → deprecated(Bob,SOAP)
- caused: Causal effect. head=cause, tail=effect. "Redis improved p99 latency" → caused(Redis,p99 latency)
- constrained_by: Constrained by requirement. head=constrained, tail=constraint. "must comply with SLA" → constrained_by(Service,SLA)

Critical rules:
1. "replaced": head=NEW, tail=OLD. "from X to Y" → head=Y, tail=X.
2. "depends_on": head=consumer, tail=provider.
3. "X over Y" in a choice context → chose(chooser,X) + rejected(chooser,Y).
4. Only use relation types: """ + ", ".join(RELATION_TYPES) + """

Output a JSON array of objects: [{"head":"<entity>","relation":"<type>","tail":"<entity>"}]
Use exact entity names from the provided list. Only extract relationships explicitly supported by the text."""


def build_user_prompt(text: str, entities: list[dict]) -> str:
    entity_lines = "\n".join(f"- {e['name']} [{e['entity_type']}]" for e in entities)
    return f"""Entities:
{entity_lines}

Text: {text}

Extract relationships using ONLY these types: {', '.join(RELATION_TYPES)}
Output ONLY a JSON array, no other text."""


# ── API call ──────────────────────────────────────────────────────
OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"


def call_model(api_key: str, model_id: str, system: str, user: str,
               timeout: int = 60) -> tuple[str, float]:
    """Call OpenRouter and return (content, latency_seconds)."""
    t0 = time.time()
    resp = requests.post(
        OPENROUTER_URL,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "HTTP-Referer": "https://github.com/pchaganti/ctxgraph",
        },
        json={
            "model": model_id,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user},
            ],
            "temperature": 0.0,
            "max_tokens": 512,
        },
        timeout=timeout,
    )
    latency = time.time() - t0

    if resp.status_code != 200:
        raise RuntimeError(f"HTTP {resp.status_code}: {resp.text[:200]}")

    data = resp.json()
    if "error" in data:
        raise RuntimeError(f"API error: {data['error']}")

    content = data["choices"][0]["message"].get("content") or ""
    return content, latency


# ── Parsing ───────────────────────────────────────────────────────
def parse_relations(content: str, entities: list[dict]) -> set[str]:
    """Parse LLM JSON response into relation triples."""
    entity_names = {e["name"].lower(): e["name"] for e in entities}

    def match_entity(name: str) -> str | None:
        nl = name.lower()
        # Exact
        if nl in entity_names:
            return entity_names[nl]
        # Substring
        for ek, ev in entity_names.items():
            if ek in nl or nl in ek:
                return ev
        return None

    relations = set()

    # Extract JSON from markdown code blocks if present
    text = content.strip()
    if "```" in text:
        # Find content between ```json and ``` or ``` and ```
        import re
        m = re.search(r'```(?:json)?\s*\n?(.*?)\n?```', text, re.DOTALL)
        if m:
            text = m.group(1).strip()

    # Try full JSON array
    try:
        triples = json.loads(text)
        if isinstance(triples, list):
            for t in triples:
                if not isinstance(t, dict):
                    continue
                h = match_entity(t.get("head", ""))
                r = t.get("relation", "")
                tl = match_entity(t.get("tail", ""))
                if h and tl and r in RELATION_TYPES and h != tl:
                    relations.add(f"{h}:{r}:{tl}")
            return relations
    except (json.JSONDecodeError, KeyError):
        pass

    # JSON lines fallback
    for line in text.splitlines():
        line = line.strip().rstrip(",")
        if not line.startswith("{"):
            continue
        try:
            t = json.loads(line)
            h = match_entity(t.get("head", ""))
            r = t.get("relation", "")
            tl = match_entity(t.get("tail", ""))
            if h and tl and r in RELATION_TYPES and h != tl:
                relations.add(f"{h}:{r}:{tl}")
        except (json.JSONDecodeError, KeyError):
            pass

    return relations


# ── F1 ────────────────────────────────────────────────────────────
def compute_f1(predicted: set, expected: set) -> tuple[float, float, float]:
    if not predicted and not expected:
        return 1.0, 1.0, 1.0
    tp = len(predicted & expected)
    p = tp / len(predicted) if predicted else 0.0
    r = tp / len(expected) if expected else 0.0
    f1 = 2 * p * r / (p + r) if (p + r) > 0 else 0.0
    return p, r, f1


# ── Main ──────────────────────────────────────────────────────────
def main():
    api_key = os.environ.get("OPENROUTER_API_KEY") or os.environ.get("CTXGRAPH_OPENROUTER_KEY")
    if not api_key:
        # Try the commented-out OpenRouter key from .env
        env_path = Path(__file__).parent.parent / ".env"
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line.startswith("# CTXGRAPH_API_KEY=sk-or-") or line.startswith("#CTXGRAPH_API_KEY=sk-or-"):
                    api_key = line.split("=", 1)[1].strip()
                    break
    if not api_key:
        print("ERROR: Set OPENROUTER_API_KEY env var or uncomment OpenRouter key in .env")
        sys.exit(1)

    # Parse args
    max_episodes = 50
    selected_models = None
    for arg in sys.argv[1:]:
        if arg.startswith("--episodes="):
            max_episodes = int(arg.split("=")[1])
        elif arg.startswith("--models="):
            selected_models = arg.split("=")[1].split(",")

    # Filter models
    models = MODELS
    if selected_models:
        models = [m for m in MODELS if any(s in m[0] or s in m[1].lower() for s in selected_models)]

    # Load fixtures
    fixture = Path(__file__).parent.parent / "crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json"
    episodes = json.loads(fixture.read_text())[:max_episodes]

    print(f"Benchmarking {len(models)} models on {len(episodes)} episodes via OpenRouter")
    print(f"{'='*80}")

    results = {}

    for model_id, display_name, local_name in models:
        print(f"\n{'─'*80}")
        print(f"Model: {display_name} ({model_id})")
        print(f"Local: {local_name}")
        print(f"{'─'*80}")

        total_f1 = 0.0
        total_p = 0.0
        total_r = 0.0
        total_latency = 0.0
        errors = 0
        parse_failures = 0

        for i, ep in enumerate(episodes):
            text = ep["text"]
            entities = ep["expected_entities"]
            exp_rels = {f"{r['head']}:{r['relation']}:{r['tail']}" for r in ep["expected_relations"]}
            user_prompt = build_user_prompt(text, entities)

            try:
                content, latency = call_model(api_key, model_id, SYSTEM_PROMPT, user_prompt)
                predicted = parse_relations(content, entities)
                p, r, f1 = compute_f1(predicted, exp_rels)

                total_f1 += f1
                total_p += p
                total_r += r
                total_latency += latency

                status = "OK" if f1 >= 0.5 else "LOW" if f1 > 0 else "MISS"
                print(f"  Ep{i:2d}: F1={f1:.3f} P={p:.3f} R={r:.3f} ({latency:.1f}s) [{status}]", end="")

                if f1 < 1.0:
                    missed = exp_rels - predicted
                    spurious = predicted - exp_rels
                    parts = []
                    if missed:
                        parts.append(f"missed={list(missed)}")
                    if spurious:
                        parts.append(f"spurious={list(spurious)}")
                    if parts:
                        print(f" {'; '.join(parts)}", end="")
                print()

                if not predicted and exp_rels:
                    parse_failures += 1

            except Exception as e:
                errors += 1
                print(f"  Ep{i:2d}: ERROR — {e}")
                continue

            # Rate limit: ~2 req/s to be polite
            time.sleep(0.5)

        n = len(episodes) - errors
        if n > 0:
            avg_f1 = total_f1 / n
            avg_p = total_p / n
            avg_r = total_r / n
            avg_latency = total_latency / n
        else:
            avg_f1 = avg_p = avg_r = avg_latency = 0.0

        results[model_id] = {
            "name": display_name,
            "local": local_name,
            "f1": avg_f1,
            "precision": avg_p,
            "recall": avg_r,
            "latency": avg_latency,
            "errors": errors,
            "parse_failures": parse_failures,
        }

        print(f"\n  >> {display_name}: F1={avg_f1:.3f} P={avg_p:.3f} R={avg_r:.3f} "
              f"latency={avg_latency:.1f}s errors={errors} parse_fail={parse_failures}")

    # ── Summary ───────────────────────────────────────────────────
    print(f"\n{'='*80}")
    print("BENCHMARK RESULTS — Relation Extraction via OpenRouter")
    print(f"{'='*80}")
    print(f"{'Model':<30} {'F1':>6} {'Prec':>6} {'Recall':>6} {'Lat(s)':>7} {'Err':>4} {'Local'}")
    print(f"{'─'*30} {'─'*6} {'─'*6} {'─'*6} {'─'*7} {'─'*4} {'─'*20}")

    # Sort by F1 descending
    ranked = sorted(results.items(), key=lambda x: x[1]["f1"], reverse=True)
    for model_id, r in ranked:
        print(f"{r['name']:<30} {r['f1']:>6.3f} {r['precision']:>6.3f} {r['recall']:>6.3f} "
              f"{r['latency']:>7.1f} {r['errors']:>4} {r['local']}")

    print(f"\nHeuristic baseline: ~0.510 F1")
    print(f"Target (combined): 0.800")

    # Save results
    out_path = Path(__file__).parent.parent / "benchmark_openrouter_results.json"
    out_path.write_text(json.dumps(results, indent=2))
    print(f"\nResults saved to {out_path}")

    if ranked:
        winner = ranked[0]
        print(f"\n*** WINNER: {winner[1]['name']} (F1={winner[1]['f1']:.3f}) ***")
        if winner[1]["local"] != "N/A":
            print(f"    Install locally: ollama pull {winner[1]['local']}")


if __name__ == "__main__":
    main()

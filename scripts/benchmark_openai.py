#!/usr/bin/env python3
"""Benchmark OpenAI gpt-4.1-mini and gpt-4.1-nano on relation extraction."""
import json, os, sys, time, requests, re
from pathlib import Path

# Load .env
env_path = Path(__file__).parent.parent / ".env"
for line in env_path.read_text().splitlines():
    line = line.strip()
    if not line or line.startswith("#"): continue
    if "=" in line:
        k, _, v = line.partition("=")
        k, v = k.strip(), v.split("#")[0].strip()
        if v and k not in os.environ: os.environ[k] = v

API_KEY = os.environ.get("CTXGRAPH_API_KEY", "")
if not API_KEY:
    print("ERROR: Need CTXGRAPH_API_KEY in .env")
    sys.exit(1)

RELATION_TYPES = ["chose","rejected","replaced","depends_on","fixed",
                  "introduced","deprecated","caused","constrained_by"]

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


def build_user_prompt(text, entities):
    lines = "\n".join(f"- {e['name']} [{e['entity_type']}]" for e in entities)
    return (f"Entities:\n{lines}\n\nText: {text}\n\n"
            f"Extract relationships using ONLY these types: {', '.join(RELATION_TYPES)}\n"
            f"Output ONLY a JSON array, no other text.")


def call_openai(text, entities, model="gpt-4.1-mini"):
    t0 = time.time()
    resp = requests.post("https://api.openai.com/v1/chat/completions",
        headers={"Authorization": f"Bearer {API_KEY}", "Content-Type": "application/json"},
        json={"model": model, "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": build_user_prompt(text, entities)}
        ], "temperature": 0.0, "max_tokens": 512},
        timeout=30)
    lat = time.time() - t0
    if resp.status_code != 200:
        raise RuntimeError(f"HTTP {resp.status_code}: {resp.text[:200]}")
    return resp.json()["choices"][0]["message"]["content"], lat


def parse_relations(content, entities):
    entity_names = {e["name"].lower(): e["name"] for e in entities}
    def match(name):
        nl = name.lower()
        if nl in entity_names: return entity_names[nl]
        for ek, ev in entity_names.items():
            if ek in nl or nl in ek: return ev
        return None
    rels = set()
    text = content.strip()
    if "```" in text:
        m = re.search(r'```(?:json)?\s*\n?(.*?)\n?```', text, re.DOTALL)
        if m: text = m.group(1).strip()
    try:
        triples = json.loads(text)
        if isinstance(triples, list):
            for t in triples:
                if not isinstance(t, dict): continue
                h, r, tl = match(t.get("head","")), t.get("relation",""), match(t.get("tail",""))
                if h and tl and r in RELATION_TYPES and h != tl:
                    rels.add(f"{h}:{r}:{tl}")
            return rels
    except (json.JSONDecodeError, KeyError):
        pass
    for line in text.splitlines():
        line = line.strip().rstrip(",")
        if not line.startswith("{"): continue
        try:
            t = json.loads(line)
            h, r, tl = match(t.get("head","")), t.get("relation",""), match(t.get("tail",""))
            if h and tl and r in RELATION_TYPES and h != tl:
                rels.add(f"{h}:{r}:{tl}")
        except (json.JSONDecodeError, KeyError):
            pass
    return rels


def compute_f1(pred, exp):
    if not pred and not exp: return 1.0, 1.0, 1.0
    tp = len(pred & exp)
    p = tp/len(pred) if pred else 0.0
    r = tp/len(exp) if exp else 0.0
    f1 = 2*p*r/(p+r) if (p+r) > 0 else 0.0
    return p, r, f1


def main():
    models = ["gpt-4.1-mini", "gpt-4.1-nano"]
    for arg in sys.argv[1:]:
        if arg.startswith("--models="):
            models = arg.split("=")[1].split(",")

    fixture = Path(__file__).parent.parent / "crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json"
    episodes = json.loads(fixture.read_text())

    for model in models:
        print(f"\n{'='*60}")
        print(f"Model: {model}")
        print(f"{'='*60}")
        tot_f1 = tot_p = tot_r = tot_lat = 0.0
        errors = 0
        for i, ep in enumerate(episodes):
            exp = {f"{r['head']}:{r['relation']}:{r['tail']}" for r in ep["expected_relations"]}
            try:
                content, lat = call_openai(ep["text"], ep["expected_entities"], model)
                pred = parse_relations(content, ep["expected_entities"])
                p, r, f1 = compute_f1(pred, exp)
                tot_f1 += f1; tot_p += p; tot_r += r; tot_lat += lat
                tag = "OK" if f1 >= 0.5 else "LOW" if f1 > 0 else "MISS"
                print(f"  Ep{i:2d}: F1={f1:.3f} P={p:.3f} R={r:.3f} ({lat:.1f}s) [{tag}]", end="")
                if f1 < 1.0:
                    missed = exp - pred
                    spur = pred - exp
                    parts = []
                    if missed: parts.append(f"missed={sorted(missed)}")
                    if spur: parts.append(f"spurious={sorted(spur)}")
                    if parts: print(f" {'; '.join(parts)}", end="")
                print()
            except Exception as e:
                errors += 1
                print(f"  Ep{i:2d}: ERROR — {e}")
            time.sleep(0.3)
        n = len(episodes) - errors
        if n > 0:
            avg_f1 = tot_f1/n
            avg_p = tot_p/n
            avg_r = tot_r/n
            print(f"\n>> {model}: F1={avg_f1:.3f} P={avg_p:.3f} R={avg_r:.3f} "
                  f"lat={tot_lat/n:.1f}s errors={errors}")
            print(f"   Combined (0.845 entity + {avg_f1:.3f} rel) / 2 = {(0.845+avg_f1)/2:.3f}")
        else:
            print(f"\n>> {model}: ALL ERRORS")


if __name__ == "__main__":
    main()

# ctxgraph4agent — Project Direction

## What This Project Is

A context graph engine for AI agents that **learns, forgets, and stays within budget** — so agents get smarter over time without getting slower or more expensive.

This is NOT just another knowledge graph. The differentiator is the **memory lifecycle**:

```
Store → TTL → Forget → Decay → Re-verify → Compress → Budget → Learn
```

## Why Not Just Use Hindsight?

[Hindsight](https://github.com/vectorize-io/hindsight) (by Vectorize, 7.1k stars, 91.4% LongMemEval) is the current SOTA for agent memory. It's open source, free to self-host. We researched it thoroughly before committing to this direction.

**What Hindsight does well:**
- 4 retrieval strategies (semantic + keyword + graph + temporal) with RRF fusion
- Entity tracking and multi-hop graph traversal
- Belief revision via LLM-based "reflect" operation
- 91.4% recall accuracy on LongMemEval

**What Hindsight does NOT do (and we do):**

| Problem | Hindsight | ctxgraph4agent |
|---------|-----------|----------------|
| Cost grows with age | Stores everything forever | TTL + forgetting, cost stays flat |
| Everything equal priority | No memory priority system | Per-type retention policies |
| No forgetting | Remember everything, even stale data | Active decay + re-verification |
| Heavy infrastructure | Requires PostgreSQL 15+ | Single SQLite file, zero deps |
| Reflect is slow | 800-2600ms (needs LLM call) | Reflect at write time or not at all |
| No skill accumulation | Stores facts, not behaviors | Skills layer — agent gets better |
| One-size-fits-all | Same policy for all agents | Per-agent memory policies |
| No budget control | Injects as much context as retrieved | Fixed memory budget per query |

**Our thesis:** Hindsight remembers. Our system learns. Memory that accumulates forever is a liability, not an asset.

## The Memory Lifecycle

### 1. Store (exists)
Episodes → NER → entities → relations → edges → SQLite.
Already built. Working. Benchmarked (0.846 F1 vs Graphiti's 0.601).

### 2. TTL (to build)
Every memory node gets a time-to-live based on its type:

```
facts:         90d   (expire and re-verify)
patterns:      never (learned behaviors, keep forever)
experiences:   14d   (conversation details, drop after 2 weeks)
preferences:   30d   (re-verify with user monthly)
decisions:     90d   (archive after 3 months, keep summary)
```

Different agents get different policies. A finance agent remembers longer than a coding agent.

### 3. Forget (to build)
When TTL expires, the node isn't immediately deleted. It enters a **decay** phase:
- Confidence score decreases over time
- Less likely to be injected into context
- Still queryable explicitly, just not auto-retrieved
- Fully deleted only after decay period ends without re-verification

### 4. Decay (to build)
Nodes have a freshness score that affects retrieval ranking:

```
freshness = base_confidence * decay_function(age, type)

decay_function for facts:      exponential, half-life = TTL/2
decay_function for patterns:   constant (never decays)
decay_function for experiences: linear drop to 0 at TTL
```

A 3-day-old experience is more relevant than a 10-day-old one. A pattern from 6 months ago is equally relevant to one from yesterday.

### 5. Re-verify (to build)
Before a node expires, the system can trigger re-verification:

```
Approaches (pick based on agent type and cost tolerance):
- Passive:  wait until the agent encounters contradicting info → update immediately
- Active:   periodically surface stale memories → ask agent "is this still true?"
- Implicit: if a memory is recalled and used → auto-renew TTL
- Budget:   only re-verify memories above a usage threshold (if never recalled, let it die)
```

**Key insight:** If something is important enough to remember forever, it's important enough to re-verify. A "permanent" memory that's wrong is worse than no memory.

### 6. Compress (to build)
Old memories get summarized before decay:

```
14 daily standup episodes (14 nodes)
  → compress after 7 days
  → 1 summary node: "Week of March 24: focused on auth migration, resolved 3 bugs"
  → original episodes decay normally
```

This keeps the graph small while preserving patterns. The summary inherits relationships from the compressed nodes.

### 7. Budget (to build)
Fixed token budget for memory injection per query:

```
Total context window: 128k tokens
├─ System prompt:        5k
├─ Conversation:        40k
├─ Tools/skills:        10k
├─ Memory budget:       20k  ← HARD CAP
└─ Response space:      53k
```

Within the 20k budget, the retrieval engine fills slots by priority:
1. Fresh, high-confidence, frequently-recalled memories (most slots)
2. Stale memories only if query specifically references them
3. Patterns always included (they're small and permanent)

**This is the cost control mechanism.** Without a budget, memory cost grows linearly with age. With a budget, cost is bounded forever.

### 8. Learn (to build)
The skills layer — what makes the agent genuinely better over time:

```
Session 1:  "Fix this Docker networking bug" → takes 5 attempts
Session 10: Similar bug → pattern recognized, takes 1 attempt
Session 50: "Fix Docker bug" → agent already knows the user's Docker setup,
            common pitfalls, preferred fix approach
```

Skills are NOT facts. They're behavioral knowledge:
- **What worked** → do this pattern again
- **What failed** → never do this again
- **What the user preferred** → always do it this way
- **What was efficient** → prefer this approach

Skills have their own lifecycle:
- Created from compressed experience patterns
- Never expire (they're proven behaviors)
- Can be superseded (new better pattern replaces old)
- Shared across sessions and agents (if configured)

## Architecture Impact

### What Changes in Existing Crates

**ctxgraph-core** — Add to existing types:
- `ttl: Option<Duration>` on entities and edges
- `decay_score: f64` — computed freshness (not stored, calculated at query time)
- `usage_count: u32` — how often this node has been recalled
- `last_recalled_at: Option<DateTime>` — for implicit re-verification
- `compression_id: Option<Uuid>` — link compressed nodes to their summary
- `MemoryBudget` config — per-agent token limits

**ctxgraph-core/storage** — Add:
- Background TTL sweep (can be lazy — check at query time, not a daemon)
- Compression pipeline (batch compress old episodes into summaries)
- Budget-aware retrieval (rank by freshness * relevance * budget_remaining)

**ctxgraph-mcp** — New tools:
- `set_policy` — configure memory policies per agent/type
- `forget` — manually expire a memory or type
- `compress` — trigger compression of old episodes
- `stats` — memory health (total nodes, by type, decayed, budget usage)
- `learn` — extract patterns from recent experiences

### What Stays the Same

The extraction pipeline (GLiNER, GLiREL, heuristics, temporal parsing) — this is solid and doesn't need changes. The graph model (entities, edges, bi-temporal) — this already supports everything we need. SQLite storage — single file, zero deps, this is the right choice.

## What We're NOT Building

- **Vector similarity** — FTS5 + graph traversal is enough. Adding pgvector-like embeddings to SQLite adds complexity for marginal gain.
- **LLM-based reflect at query time** — The 800-2600ms Hindsight bottleneck. We reflect at write time or not at all.
- **PostgreSQL dependency** — SQLite is the right choice. Single file, zero config, works everywhere.
- **Disposition traits** — Cool Hindsight feature but not essential for v1.
- **Mental models** — Can add later as a layer on top of patterns.
- **Forever storage** — Nothing is permanent. Everything has a refresh cycle.

## Different Agents, Different Policies

```toml
# ctxgraph.toml — per-agent memory policies

[policies.programming]
facts_ttl = "14d"
experiences_ttl = "7d"
patterns_ttl = "never"
decisions_ttl = "30d"
memory_budget_tokens = 8000
compress_after = "7d"

[policies.finance]
facts_ttl = "365d"
experiences_ttl = "90d"
patterns_ttl = "never"
decisions_ttl = "365d"
memory_budget_tokens = 15000
compress_after = "30d"

[policies.assistant]  # like me (Hermes/Apex)
facts_ttl = "90d"
experiences_ttl = "14d"
patterns_ttl = "never"
decisions_ttl = "never"
preferences_ttl = "30d"
memory_budget_tokens = 10000
compress_after = "14d"
```

## Build Order

```
Phase A: TTL + Decay (foundation)
  - Add ttl field to entities/edges
  - Compute decay_score at query time
  - Budget-aware retrieval ranking
  - Tests for decay behavior

Phase B: Compress (size control)
  - Batch compression pipeline
  - Episode → summary with relationship inheritance
  - Compression triggers (time-based, size-based)

Phase C: Re-verify (quality maintenance)
  - Passive re-verification (contradiction detection)
  - Implicit renewal (usage-based TTL refresh)
  - Optional active re-verification (surface stale memories)

Phase D: Learn (the differentiator)
  - Pattern extraction from compressed experiences
  - Skill creation and evolution
  - Cross-session skill persistence
  - Per-agent policy configuration
```

## Key Principles

1. **Forgetting is a feature, not a bug.** Stale data is worse than missing data.
2. **Cost must stay flat regardless of age.** Budget caps make this possible.
3. **Agents should get better, not just bigger.** Skills compound; facts decay.
4. **Nothing is permanent.** Everything has a refresh cycle.
5. **SQLite first.** Zero external dependencies. Single file. Works everywhere.
6. **Quality over quantity.** A small graph of relevant, fresh memories beats a massive graph of everything.
7. **Per-agent policies.** A finance agent and a coding agent should not have the same memory behavior.

## Reference

- Hindsight paper: https://arxiv.org/abs/2512.12818
- Hindsight repo: https://github.com/vectorize-io/hindsight
- Hindsight vs RAG analysis: https://hindsight.vectorize.io/developer/rag-vs-hindsight

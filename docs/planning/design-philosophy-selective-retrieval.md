# Design Philosophy: Selective Proactive Retrieval

> Status: Established
> Created: 2026-04-04
> Origin: hermes-mindgraph-plugin research + ctxgraph4agent architecture discussion

## Core Principle

Retrieve context when the cost of NOT having it is high, not when it's low.

## Vision Horizon

Target: system must remain useful after 6 months of continuous daily use. Context quality degrades linearly with age unless actively managed.

Two-tier memory lifecycle:

- **3 months: re-verify** -- "is this still true?" Soft check. LLM re-evaluates against current state. Stale memories get flagged, contradicted memories get surfaced, valid memories get renewed.
- **6 months: cleanup** -- "keep or delete?" Hard check. Automated. No LLM needed. If not renewed recently and not frequently used, archive it.

```
Store ──> decay starts (A2)
  │
  ├── 3 months: re-verify triggers (C3)
  │     ├── still valid? → renew, bump usage_count (C2)
  │     ├── contradicted? → flag for review (C1)
  │     └── unknown? → let it keep decaying
  │
  └── 6 months: TTL enforcement (A6)
        ├── renewed recently? → keep
        ├── high usage_count? → keep
        └── neither? → archive
```

Phase A (TTL + Decay) is the foundation. Without automatic forgetting, the system accumulates stale context that pollutes retrieval results and degrades decision quality. The 3-month soft check prevents drift; the 6-month hard check prevents rot.

## The Two Approaches

### Reflexive Retrieval (MindGraph's Approach)
- Retrieve on EVERY turn, inject into every prompt
- "Always retrieve before acting on stored knowledge" as a hard rule
- 15+ API calls per session, every user message triggers a search
- Designed for general-purpose agents that need to "remember everything about everyone"

### Selective Retrieval (Our Approach)
- Retrieve only at decision points where context changes the outcome
- No retrieval for routine operations (ls, cat, grep, simple commands)
- Designed for agents that need to be sharp when it matters, not noisy when they don't

## Why Reflexive is Wrong for Us

1. **Noise pollution**: Injecting graph context during `ls` or `cat` wastes tokens and distracts the model from the actual task
2. **Latency tax**: Every retrieval is a round-trip. Per-turn retrieval means every turn pays the cost, even when nothing relevant exists
3. **Precision over recall**: We chose FTS over vector embeddings for the same reason. Retrieve when needed, not as a reflex
4. **Prompt budget**: Context window is finite. Filling it with irrelevant memory on routine turns crowds out what matters

## When to Retrieve

Retrieve at decision points where missing context leads to bad outcomes:

| Decision Point | Why | Example |
|---------------|-----|---------|
| Skill creation | Without context, skills lose provenance and repeat past mistakes | Creating a skill after a debugging session |
| Skill improvement | Without knowing WHY a skill was written, enhancement is guesswork | Updating a skill made 3 months ago |
| Planning | Plans built without past context repeat failed approaches | Planning a feature that was attempted before |
| Cross-session continuity | User references past work or says "remember when" | "What did we decide about the database?" |
| Debugging | Past incidents and solutions prevent re-solving the same bug | Fixing an error seen in a previous session |

## When NOT to Retrieve

| Situation | Why |
|-----------|-----|
| Running a shell command | The command doesn't need context |
| Reading a file | The file speaks for itself |
| Simple Q&A | The answer is in the question |
| Routine operations | No decision is being made |
| Greetings / acknowledgments | < 15 chars, not worth a search |

## What We Borrowed from MindGraph

Not the reflexive approach, but the result categorization pattern. When we DO retrieve, sort results by behavioral implication:

- **Knowledge**: facts and entities relevant to the task
- **Open questions**: things still unresolved that might affect the decision
- **Weak claims**: low-confidence items that should be flagged, not asserted
- **Decisions**: past decisions that constrain current options
- **Relationships**: connections between entities that provide context

A flat result list is less useful than a categorized one. The categorization helps the agent reason about what to DO with each piece of retrieved context.

## Tiered Retrieval

Retrieval follows the same 3-month / 6-month tiers as the memory lifecycle:

1. **Default: search last 3 months** -- recent context, likely still valid, fast
2. **Ask user: "need older context?"** -- before expanding the search window
3. **Full 6-month search** -- only if user confirms

Rationale: 6-month-old memories are near the cleanup boundary. Most will have low decay_score or fail re-verify. Pulling them into every retrieval pollutes the prompt with likely-stale information, wastes tokens, and adds noise that makes the agent less sharp. The 3-month default gives the sweet spot: old enough to have context, young enough to likely be valid.

User decides whether to dig deeper. The agent never assumes older is better -- it asks.

## Design Implications for ctxgraph4agent

1. Proactive retrieval is NOT a per-turn hook -- it's a decision-point trigger
2. The `prefetch()` hook should gate on message type/intent, not just length
3. Context budget (20k tokens) is shared across all injection -- don't waste it on routine turns
4. Phase D's `learn` command is itself a retrieval decision point (retrieve patterns before creating skills)
5. FTS5 search should be fast enough (< 20ms) that retrieval at decision points feels instant, making the selectivity feel natural rather than limiting

## Related

- Design note: skill-provenance-ttl.md (why provenance matters for skill enhancement)
- hermes-mindgraph-plugin research: proactive_graph_retrieve() and _ProactiveMetrics
- Phase A stories: decay_score, TTL, renewal (C2)
- Phase D stories: skill creation (D2), learn command (D4)

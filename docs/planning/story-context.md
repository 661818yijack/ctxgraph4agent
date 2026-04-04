# ctxgraph4agent Story Writing Context

## Project
ctxgraph4agent — a context graph engine for AI agents that learns, forgets, and stays within budget.

## Current State
- Phase 0 DONE: Episode ingestion → NER (GLiNER) → relations (GLiREL) → SQLite. 0.846 F1.
- Rust workspace, edition 2024, SQLite-first, zero external deps
- Crates: ctxgraph-core, ctxgraph-extract, ctxgraph-embed, ctxgraph-cli, ctxgraph-mcp

## What We're Building (4 Phases, 16-20 stories)

### Phase A: TTL + Decay (foundation) — 5 stories
Every memory node gets a time-to-live. Decay function computes freshness at query time. Budget-aware retrieval.

### Phase B: Compress (size control) — 4 stories
Old episodes batch-compressed into summary nodes. Summary inherits relationships. Triggers: time-based or size-based.

### Phase C: Re-verify (quality maintenance) — 4 stories
Passive re-verification (contradiction detection). Implicit renewal (usage-based TTL refresh). Budget-based (only re-verify used memories).

### Phase D: Learn (the differentiator) — 4 stories
Pattern extraction from compressed experiences. Skill creation/evolution. Cross-session persistence. Per-agent policy config.

## Memory Type TTL Defaults
- facts: 90d (expire and re-verify)
- patterns: never (learned behaviors, keep forever)
- experiences: 14d (conversation details, drop after 2 weeks)
- preferences: 30d (re-verify with user monthly)
- decisions: 90d (archive after 3 months, keep summary)

## Decay Functions
- facts: exponential, half-life = TTL/2
- patterns: constant (never decays)
- experiences: linear drop to 0 at TTL

## Budget
- Total context window: 128k tokens
- Memory budget: 20k HARD CAP
- Fill slots by priority: fresh high-confidence > stale if referenced > patterns (always)

## Architecture Constraints
- SQLite first, single file, zero deps
- No vector similarity (FTS5 + graph traversal)
- No LLM reflect at query time
- No PostgreSQL
- Per-agent memory policies via ctxgraph.toml

## Key Files
- ctxgraph-core/src/lib.rs — core types (Entity, Edge, Episode)
- ctxgraph-core/src/storage/sqlite.rs — SQLite storage
- ctxgraph-extract/src/pipeline.rs — extraction pipeline
- ctxgraph-extract/src/schema.rs — config schema
- ctxgraph-mcp/src/server.rs — MCP tools
- ctxgraph-cli/src/main.rs — CLI

## What We're NOT Building
- Vector similarity
- LLM-based reflect at query time
- PostgreSQL dependency
- Disposition traits
- Mental models
- Forever storage

## Key Principles
1. Forgetting is a feature, not a bug
2. Cost must stay flat regardless of age
3. Agents should get better, not just bigger
4. Nothing is permanent
5. SQLite first
6. Quality over quantity
7. Per-agent policies

## Story Format Requirements
Each story must have:
- Story ID (e.g., A1, A2, B1, B2...)
- Title
- Phase and priority
- Acceptance criteria (3-6 items)
- Technical requirements (specific files, types, functions)
- Dependencies (which stories must be done first)
- Dev effort estimate (S/M/L)
- Test plan (what to test)

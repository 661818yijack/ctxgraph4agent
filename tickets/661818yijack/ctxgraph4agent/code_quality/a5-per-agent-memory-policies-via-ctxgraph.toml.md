---
id: A4a-A5
title: 'A5: Per-agent memory policies via ctxgraph.toml'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: apex-agent
created_at: '2026-04-04T06:05:46.374355Z'
updated_at: '2026-04-04T06:05:46.374355Z'
tags:
- a5
- phase-a
- memory-lifecycle
---

<!-- DESCRIPTION -->
Phase A Story 5 (P1, Medium effort, depends on A1+A4c). Extend ctxgraph.toml with [policies.<agent_name>] section: per-type TTLs, memory_budget_tokens, compress_after, max_episodes, max_patterns_included, stale_threshold, provenance_ttl_days, context_ttl_days. MemoryPolicyConfig struct. MCP set_policy tool. Graph::init loads policy.

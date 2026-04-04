---
id: C2
title: "C2: Implicit TTL renewal (recalled and used -> auto-renew)"
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: open
owner: apex-agent
phase: C
priority: P1
effort: M
depends_on:
- A1
- A3
created_at: '2026-04-04T08:55:00.000000Z'
updated_at: '2026-04-04T08:55:00.000000Z'
tags:
- c2
- phase-c
- ttl
- renewal
---

<!-- DESCRIPTION -->
Phase C Story 2 (P1, Medium effort, depends on A1+A3). Implicit TTL renewal: when a memory is recalled via `retrieve_for_context` and its content is used, its TTL is implicitly renewed. Renewal resets `created_at` to now. Gated by `max_renewals` policy (default 5). Uses `renewal_count` (NOT `usage_count`) — renewal_count only increments on actual renewal; usage_count tracks general recall frequency for scoring. Only Facts and Preferences are eligible; Experiences are not.

### Acceptance Criteria:
1. `Storage::renew_memory(id: &str, memory_type: MemoryType) -> Result<bool>` updates `created_at` to now and increments `renewal_count` if renewal is allowed
2. Renewal only applies to `Fact` and `Preference` memory types (not Experience, Pattern, or Decision)
3. Renewal count tracked via `renewal_count` (separate from `usage_count`) — if `renewal_count >= max_renewals`, renewal is denied
4. `MemoryPolicyConfig` has `max_renewals: usize` field (default 5)
5. Renewal returns false (no-op) if memory is already expired (decay_score = 0.0)
6. `retrieve_for_context` automatically calls `renew_memory` for each returned memory (only Facts and Preferences)
7. Renewal only fires for memories that appear in the final returned results (within budget), not all candidates

### Technical Requirements:
- Files to modify: types.rs (add max_renewals to MemoryPolicyConfig), storage/migrations.rs (migration 009: renewal_count INTEGER), storage/sqlite.rs (renew_memory, integrate into retrieve_for_context)
- Config: `max_renewals = 5` in `[policies.<agent>]`

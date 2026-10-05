---
id: BUG-5
title: Fractional limit is ignored and defaulted
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:08.700848Z'
updated_at: '2026-10-05T02:08:01.664363Z'
tags:
- apex-daily-20261005
- use-case
- req-input
- uc-fractional-limit
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Normalize MCP positive limits. Acceptance: non-u64 limit values fall back to the tool default.

<!-- FIX -->
Non-u64 JSON limit values are ignored by the shared parser and fall back to defaults.

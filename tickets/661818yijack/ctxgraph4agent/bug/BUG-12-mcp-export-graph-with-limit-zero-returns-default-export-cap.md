---
id: BUG-12
title: MCP export_graph with limit zero returns default export cap
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:09.695254Z'
updated_at: '2026-10-05T02:08:02.718480Z'
tags:
- apex-daily-20261005
- use-case
- req-list
- uc-export-zero
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Requirement: Protect list and export limits. Acceptance: export_graph limit=0 uses default export cap.

<!-- FIX -->
Added test proving export_graph limit=0 uses the default export cap.

---
id: BUG-10
title: Protect list and export limits
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:09.414597Z'
updated_at: '2026-10-05T02:08:02.413270Z'
tags:
- apex-daily-20261005
- requirement
- req-list
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Parent: MCP tools reject non-positive pagination limits. Entity listing and graph export should avoid accidental empty outputs from zero limits while preserving offset and type filters.

<!-- FIX -->
Applied shared positive limit parsing to list_entities and export_graph.

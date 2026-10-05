---
id: BUG-2
title: Normalize MCP positive limits
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-05T02:03:08.274252Z'
updated_at: '2026-10-05T02:08:01.228581Z'
tags:
- apex-daily-20261005
- requirement
- req-input
- mcp-limit
version: 2
schema_version: v0.2
due_date: '2026-10-06'
---

<!-- DESCRIPTION -->
Parent: MCP tools reject non-positive pagination limits. Use a shared parser so missing, null, zero, or non-integer limits resolve to safe defaults while valid positive integers are preserved.

<!-- FIX -->
Added shared positive_usize_arg parser for MCP JSON arguments.

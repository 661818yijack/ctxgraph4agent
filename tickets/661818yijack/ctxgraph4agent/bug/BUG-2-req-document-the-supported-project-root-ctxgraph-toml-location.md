---
id: BUG-2
title: 'REQ: Document the supported project-root ctxgraph.toml location'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph.toml.example
created_at: '2026-10-02T02:03:07.618723Z'
updated_at: '2026-10-02T02:06:05.811906Z'
tags:
- config
- docs
- requirement
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Requirement for parent config-path alignment: docs/examples must instruct users to place ctxgraph.toml at the project root because open_graph resolves config beside .ctxgraph, not inside it.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.

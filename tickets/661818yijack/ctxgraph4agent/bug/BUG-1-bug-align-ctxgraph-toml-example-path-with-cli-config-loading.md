---
id: BUG-1
title: 'BUG: Align ctxgraph.toml example path with CLI config loading'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: ctxgraph.toml.example
created_at: '2026-10-02T02:03:07.479702Z'
updated_at: '2026-10-02T02:06:05.671350Z'
tags:
- config
- cli
- docs
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Parent: ctxgraph.toml.example tells users to copy config into .ctxgraph/ctxgraph.toml, but the CLI loads ctxgraph.toml from the project root next to .ctxgraph. Fix the documented path and cover the config path resolution behavior with tests before implementation.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.

---
id: BUG-6
title: 'UC: User avoids unsupported .ctxgraph/ctxgraph.toml path'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: low
status: done
owner: 661818yijack
file: ctxgraph.toml.example
created_at: '2026-10-02T02:03:22.330647Z'
updated_at: '2026-10-02T02:06:06.358137Z'
tags:
- config
- docs
- use-case
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Use case under BUG-2: example comments should not direct users to .ctxgraph/ctxgraph.toml, because the CLI does not read that location.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.

---
id: BUG-4
title: 'REQ: Keep extraction loading behavior unchanged'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/mod.rs
created_at: '2026-10-02T02:03:07.883097Z'
updated_at: '2026-10-02T02:06:06.085883Z'
tags:
- config
- cli
- regression
- requirement
version: 2
schema_version: v0.2
due_date: '2026-10-03'
---

<!-- DESCRIPTION -->
Requirement for parent config-path alignment: refactor only the config path helper and docs; do not change model discovery or fallback loading behavior.

<!-- FIX -->
Fixed by documenting the supported project-root ctxgraph.toml location, extracting/test-covering CLI config path resolution, and preserving existing extraction loading fallback behavior.

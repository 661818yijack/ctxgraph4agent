---
id: BUG-26
title: 'UC: cargo test catches future zero-limit regressions'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-12T02:02:12.421572Z'
updated_at: '2026-10-12T02:05:00.056842Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-13'
---

<!-- DESCRIPTION -->
Use case under BUG-14: cargo test validates CLI limit helper.

<!-- FIX -->
cargo test -p ctxgraph-cli and cargo test --workspace --no-default-features now exercise the validator.

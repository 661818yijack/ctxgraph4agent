---
id: BUG-12
title: 'UC: Namespaced source label remains unchanged'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:12.425639Z'
updated_at: '2026-10-14T02:04:44.322079Z'
tags:
- apex-daily
- use-case
- source-normalization
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Use case: labels like slack:eng remain unchanged after normalization.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

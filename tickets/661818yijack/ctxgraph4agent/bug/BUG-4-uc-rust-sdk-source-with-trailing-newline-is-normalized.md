---
id: BUG-4
title: 'UC: Rust SDK source with trailing newline is normalized'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:11.360303Z'
updated_at: '2026-10-14T02:04:44.302927Z'
tags:
- apex-daily
- use-case
- source-normalization
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Use case: an SDK caller reads a source from a line-based config or env file and should not persist the newline as part of the label.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

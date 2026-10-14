---
id: BUG-1
title: Normalize episode source labels at ingestion
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:10.958127Z'
updated_at: '2026-10-14T02:04:43.885701Z'
tags:
- apex-daily
- source-normalization
- parent
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Parent: source labels provided through the Episode builder should be canonicalized before storage so CLI filters and stats do not split equivalent user input.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

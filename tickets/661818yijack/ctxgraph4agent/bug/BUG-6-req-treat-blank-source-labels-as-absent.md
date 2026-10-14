---
id: BUG-6
title: 'REQ: Treat blank source labels as absent'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:11.623657Z'
updated_at: '2026-10-14T02:04:43.498895Z'
tags:
- apex-daily
- requirement
- source-normalization
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Requirement: labels containing only whitespace should not create meaningless source buckets in stats, lists, or filters.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

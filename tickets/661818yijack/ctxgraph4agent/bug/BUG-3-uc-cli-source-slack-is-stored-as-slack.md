---
id: BUG-3
title: 'UC: CLI source '' slack '' is stored as ''slack'''
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:11.225614Z'
updated_at: '2026-10-14T02:04:44.162629Z'
tags:
- apex-daily
- use-case
- source-normalization
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Use case: a user passes a source label with shell/copy-paste whitespace and later expects --source slack filters to match it.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

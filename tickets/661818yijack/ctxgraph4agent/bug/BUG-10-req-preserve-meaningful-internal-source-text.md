---
id: BUG-10
title: 'REQ: Preserve meaningful internal source text'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:12.156167Z'
updated_at: '2026-10-14T02:04:44.047086Z'
tags:
- apex-daily
- requirement
- source-normalization
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Requirement: normalization must not collapse or rewrite meaningful internal characters inside a source label.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

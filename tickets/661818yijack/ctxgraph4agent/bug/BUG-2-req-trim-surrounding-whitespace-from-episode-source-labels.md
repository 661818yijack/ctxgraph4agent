---
id: BUG-2
title: 'REQ: Trim surrounding whitespace from episode source labels'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-14T02:02:11.093452Z'
updated_at: '2026-10-14T02:04:44.024796Z'
tags:
- apex-daily
- requirement
- source-normalization
version: 2
schema_version: v0.2
due_date: '2026-10-15'
---

<!-- DESCRIPTION -->
Requirement: source labels with accidental leading or trailing whitespace must store the intended canonical label.

<!-- FIX -->
Implemented EpisodeBuilder::source canonicalization: trims surrounding whitespace, omits blank labels, preserves meaningful internal and non-ASCII text; covered by failing-first regression tests and workspace test run.

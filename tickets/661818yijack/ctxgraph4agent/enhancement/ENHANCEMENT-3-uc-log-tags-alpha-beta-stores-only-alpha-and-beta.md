---
id: ENHANCEMENT-3
title: 'UC: log --tags alpha,,beta stores only alpha and beta'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:46.438122Z'
updated_at: '2026-09-28T02:04:42.820103Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Use case: user accidentally includes a double comma in CLI tags; metadata should not contain an empty string tag.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

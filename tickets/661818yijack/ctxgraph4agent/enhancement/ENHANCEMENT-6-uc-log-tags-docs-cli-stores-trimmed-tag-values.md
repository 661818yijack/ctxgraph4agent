---
id: ENHANCEMENT-6
title: 'UC: log --tags " docs , cli " stores trimmed tag values'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:46.844176Z'
updated_at: '2026-09-28T02:04:43.239453Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Use case: comma-separated CLI tags with spaces should persist as docs and cli.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

---
id: ENHANCEMENT-1
title: Normalize CLI episode tags before storage
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:46.168942Z'
updated_at: '2026-09-28T02:04:42.532656Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Parent: ensure ctxgraph stores clean episode tag metadata from CLI and builder input; avoid blank tags from comma-separated user input.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

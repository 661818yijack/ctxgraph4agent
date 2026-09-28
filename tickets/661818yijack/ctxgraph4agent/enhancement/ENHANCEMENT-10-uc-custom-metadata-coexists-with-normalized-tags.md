---
id: ENHANCEMENT-10
title: 'UC: custom metadata coexists with normalized tags'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:47.381173Z'
updated_at: '2026-09-28T02:04:43.790322Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Use case: direct SDK callers combining meta() and tag() should retain custom metadata and clean tags.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

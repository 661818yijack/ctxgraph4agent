---
id: ENHANCEMENT-7
title: 'UC: EpisodeBuilder::tag trims direct builder calls'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:46.979525Z'
updated_at: '2026-09-28T02:04:43.377336Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Use case: SDK callers using Episode::builder(...).tag(" docs ") should get the same clean metadata as CLI callers.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

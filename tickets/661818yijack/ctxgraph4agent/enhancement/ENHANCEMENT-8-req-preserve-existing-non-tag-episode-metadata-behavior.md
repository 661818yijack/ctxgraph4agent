---
id: ENHANCEMENT-8
title: 'REQ: Preserve existing non-tag episode metadata behavior'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:47.113005Z'
updated_at: '2026-09-28T02:04:43.513742Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Requirement for parent tag normalization: source/content/default memory type and ordinary metadata remain unchanged.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

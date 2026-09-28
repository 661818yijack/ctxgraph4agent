---
id: ENHANCEMENT-2
title: 'REQ: Ignore empty tag segments'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:46.304284Z'
updated_at: '2026-09-28T02:04:42.675335Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Requirement for parent tag normalization: blank comma-separated tag segments must not be persisted.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

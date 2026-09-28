---
id: ENHANCEMENT-5
title: 'REQ: Trim tag whitespace consistently'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-09-28T02:02:46.709544Z'
updated_at: '2026-09-28T02:04:43.103640Z'
tags:
- tag-normalization
- daily-cron
version: 2
schema_version: v0.2
due_date: '2026-09-29'
---

<!-- DESCRIPTION -->
Requirement for parent tag normalization: tags with surrounding spaces must be normalized before persistence.

<!-- FIX -->
Implemented EpisodeBuilder tag normalization: trim tag values, skip blank tags, preserve untagged/custom metadata behavior; verified with focused and workspace Rust tests.

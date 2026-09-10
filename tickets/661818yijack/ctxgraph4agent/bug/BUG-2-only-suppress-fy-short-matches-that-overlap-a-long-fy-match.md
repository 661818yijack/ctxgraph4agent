---
id: BUG-2
title: Only suppress FY short matches that overlap a long FY match
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-extract/src/temporal.rs
created_at: '2026-09-10T04:51:38Z'
updated_at: '2026-09-10T04:53:26.112733Z'
tags:
- requirement
- parent:bug-1
- temporal
- fy
- cron-20260910
version: 1
schema_version: v0.2
due_date: '2026-09-11'
---

<!-- DESCRIPTION -->
Requirement under parent BUG-1.

<!-- FIX -->
Replaced global same-year suppression with overlap-aware long-form span checks.

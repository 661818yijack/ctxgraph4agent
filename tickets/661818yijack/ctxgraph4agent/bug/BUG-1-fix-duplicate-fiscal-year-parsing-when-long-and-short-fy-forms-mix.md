---
id: BUG-1
title: Fix duplicate fiscal-year parsing when long and short FY forms mix
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: crates/ctxgraph-extract/src/temporal.rs
created_at: '2026-09-10T04:51:38Z'
updated_at: '2026-09-10T04:53:26.112733Z'
tags:
- parent
- temporal
- fy
- cron-20260910
version: 1
schema_version: v0.2
due_date: '2026-09-11'
---

<!-- DESCRIPTION -->
Parent ticket: temporal parser should capture separate FY2026 and FY26 mentions without suppressing non-overlapping short forms.

<!-- FIX -->
Fixed fiscal-year parsing to suppress only overlapping short FY matches, preserving separate FY26 mentions alongside FY2026.

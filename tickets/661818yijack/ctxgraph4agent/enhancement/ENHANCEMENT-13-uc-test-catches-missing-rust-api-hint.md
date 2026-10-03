---
id: ENHANCEMENT-13
title: 'UC: Test catches missing Rust API hint'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-03T02:03:26.297092Z'
updated_at: '2026-10-03T02:05:43.573002Z'
tags: []
version: 2
schema_version: v0.2
due_date: '2026-10-04'
---

<!-- DESCRIPTION -->
Use case: a regression test fails if the empty-state guidance stops preserving the manual Rust API path.

<!-- FIX -->
Regression test requires Rust API hint in entities empty-state guidance.

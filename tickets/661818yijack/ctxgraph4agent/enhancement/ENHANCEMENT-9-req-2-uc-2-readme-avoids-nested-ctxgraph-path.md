---
id: ENHANCEMENT-9
title: 'REQ-2-UC-2: README avoids nested .ctxgraph path'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-07-22T02:04:44.412961Z'
updated_at: '2026-07-22T02:05:54.034528Z'
tags:
- sdk
- docs
- doctest
version: 2
schema_version: v0.2
due_date: '2026-07-23'
---

<!-- DESCRIPTION -->
Use case for ENHANCEMENT-1 REQ-2: snippet calls Graph::init on the project directory, not on .ctxgraph itself.

<!-- FIX -->
Implemented SDK quick start doctest, corrected README Rust SDK snippet, ran cargo fmt --check and cargo test --workspace successfully.

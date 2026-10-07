---
id: ENHANCEMENT-4
title: '[Stats byte formatter] REQ: add GB and TB output'
repo: 661818yijack/ctxgraph4agent
category: enhancement
severity: low
status: done
owner: 661818yijack
file: crates/ctxgraph-cli/src/commands/stats.rs
created_at: '2026-10-07T02:02:22.625392Z'
updated_at: '2026-10-07T02:05:45.521696Z'
tags:
- daily-20261007
- req
- stats
- large-units
version: 2
schema_version: v0.2
due_date: '2026-10-08'
---

<!-- DESCRIPTION -->
Requirement: ctxgraph stats must scale database sizes to GB and TB instead of showing very large MB values.

<!-- FIX -->
Added GB and TB branches to format_bytes and unit coverage for 1.0 GB, 1.5 GB, and 1.0 TB.

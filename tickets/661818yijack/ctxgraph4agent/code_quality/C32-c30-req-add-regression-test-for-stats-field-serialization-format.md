---
id: C32
title: '[C30] REQ: Add regression test for stats field serialization format'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: in_progress
owner: 661818yijack
file: crates/ctxgraph-mcp/tests/
created_at: '2026-04-19T02:04:54.187611Z'
updated_at: '2026-04-19T02:05:20.954286Z'
tags:
- req
- test
- stats
---

<!-- DESCRIPTION -->
Add a test in crates/ctxgraph-mcp/tests/ that verifies the stats response serializes sources, total_entities_by_type, and decayed_entities_by_type as JSON objects (not arrays). The test should create an entity, run stats, and assert the fields are objects with string keys and integer values.

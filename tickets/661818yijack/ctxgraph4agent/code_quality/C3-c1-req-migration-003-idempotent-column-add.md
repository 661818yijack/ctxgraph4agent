---
id: C3
title: '[C1] REQ: Migration 003 idempotent column add'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/migrations.rs
created_at: '2026-04-04T03:39:00.948511Z'
updated_at: '2026-04-04T03:44:41.920501Z'
tags:
- req
- c1
- migration
---

<!-- DESCRIPTION -->
Migration 003 adds memory_type TEXT NOT NULL DEFAULT 'Fact' and ttl_seconds INTEGER to entities and edges tables. Must use WHERE ttl_seconds IS NULL for idempotent re-runs. Existing rows get default values.

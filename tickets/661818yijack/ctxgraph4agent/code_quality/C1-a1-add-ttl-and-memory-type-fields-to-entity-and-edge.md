---
id: C1
title: 'A1: Add ttl and memory_type fields to Entity and Edge'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs
created_at: '2026-04-04T03:38:54.422964Z'
updated_at: '2026-04-04T03:44:41.593355Z'
tags:
- a1
- memory-type
- ttl
- migration
- phase-a
---

<!-- DESCRIPTION -->
Phase A Story 1 (P0, Medium effort, no dependencies, migration 003). Add MemoryType enum (Fact/Pattern/Experience/Preference/Decision) with default TTLs. Add memory_type and ttl fields to Entity and Edge structs. Migration 003 adds columns to entities/edges tables (idempotent UPDATE WHERE IS NULL). Update all insert/read paths in sqlite.rs.

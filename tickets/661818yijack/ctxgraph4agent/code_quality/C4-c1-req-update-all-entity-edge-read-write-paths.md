---
id: C4
title: '[C1] REQ: Update all Entity/Edge read/write paths'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-04T03:39:01.108707Z'
updated_at: '2026-04-04T03:44:42.085758Z'
tags:
- req
- c1
- sqlite
---

<!-- DESCRIPTION -->
Add memory_type and ttl fields to Entity and Edge structs. Update insert_entity, insert_edge, map_entity_row, map_edge_row, get_entity, get_entity_by_name, get_entity_by_name_and_type, list_entities, get_edges_for_entity, get_current_edges_for_entity, search_entities, traverse to handle new columns.

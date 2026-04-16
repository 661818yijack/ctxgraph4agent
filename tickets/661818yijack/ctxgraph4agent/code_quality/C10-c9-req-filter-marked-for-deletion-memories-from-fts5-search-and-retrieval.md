---
id: C10
title: '[C9] REQ: Filter marked_for_deletion memories from FTS5 search and retrieval'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-16T02:03:40.572756Z'
updated_at: '2026-04-16T02:03:40.572756Z'
tags:
- req
- retrieval
- search
- fts5
- filter
---

<!-- DESCRIPTION -->
Add WHERE clause exclusion for json_extract(metadata, '$.marked_for_deletion') = true in:
- fts_search_entities() — entity FTS5 query
- fts_search_edges() — edge FTS5 query
- retrieve_candidates() — candidate retrieval pipeline
- search_episodes() — episode search
- search_entities() — entity search

Soft-expired memories should be invisible to all search/retrieval paths.

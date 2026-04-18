---
id: C27
title: '[C25] REQ: Add soft-expire filter to retrieval pipeline FTS5 search functions'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:36.606917Z'
updated_at: '2026-04-18T02:03:36.606917Z'
tags:
- req
- retrieval
- fts5
- pipeline
---

<!-- DESCRIPTION -->
The internal fts_search_entities() (line 2541), fts_search_edges() (line 2578), and fts_search_episodes() (line 2623) used by retrieve_for_context() do not filter out marked_for_deletion memories. Add the filter to all three. These functions feed the retrieval pipeline — without filtering, soft-expired data gets injected into agent context.

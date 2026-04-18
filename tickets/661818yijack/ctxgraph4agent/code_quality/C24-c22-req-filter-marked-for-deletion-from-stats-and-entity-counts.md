---
id: C24
title: '[C22] REQ: Filter marked_for_deletion from stats() and entity counts'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-17T02:05:00.595208Z'
updated_at: '2026-04-18T02:03:16.813604Z'
tags:
- req
- c22
- stats
- counts
---

<!-- DESCRIPTION -->
stats() raw entity/edge counts and get_entity_counts_by_type() do not exclude marked_for_deletion memories. The stats tool shows inflated counts. get_decayed_counts_by_type() also doesn't filter. Add the marked_for_deletion exclusion to all counting queries in sqlite.rs.

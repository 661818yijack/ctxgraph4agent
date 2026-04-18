---
id: C28
title: '[C25] REQ: Filter soft-expired items from get_1hop_candidates graph traversal'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: open
owner: 661818yijack
file: crates/ctxgraph-core/src/storage/sqlite.rs
created_at: '2026-04-18T02:03:39.936154Z'
updated_at: '2026-04-18T02:03:39.936154Z'
tags:
- req
- retrieval
- graph-traversal
---

<!-- DESCRIPTION -->
get_1hop_candidates() (line 2658) traverses edges and neighbor entities during retrieve_for_context(). It does not skip marked_for_deletion edges or entities. Add a metadata check to skip soft-expired items before they become RetrievalCandidates. This prevents soft-expired neighbors from entering the retrieval scoring pipeline.

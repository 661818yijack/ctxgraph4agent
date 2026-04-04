---
id: C2
title: '[C1] REQ: MemoryType enum with default TTLs'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: high
status: closed
owner: 661818yijack
file: crates/ctxgraph-core/src/types.rs
created_at: '2026-04-04T03:39:00.785408Z'
updated_at: '2026-04-04T03:44:41.756680Z'
tags:
- req
- c1
- enum
---

<!-- DESCRIPTION -->
Add MemoryType enum with variants Fact/Pattern/Experience/Preference/Decision. Each variant has a default TTL: Fact=90d, Pattern=None(never), Experience=14d, Preference=30d, Decision=90d. Must impl Serialize/Deserialize/Display. Include default_for_entity_type mapping function.

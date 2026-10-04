---
id: BUG-10
title: 'Use case: parser remains format agnostic'
repo: 661818yijack/ctxgraph4agent
category: bug
severity: medium
status: done
owner: 661818yijack
file: null
created_at: '2026-10-04T02:02:52.217661Z'
updated_at: '2026-10-04T02:06:27.359020Z'
tags:
- use-case
- bug-3
- learn
- llm-parser
version: 2
schema_version: v0.2
due_date: '2026-10-05'
---

<!-- DESCRIPTION -->
Use case for BUG-3: parsing supports the documented JSON array regardless of whether id is encoded as a string or integer.

<!-- FIX -->
Parser now accepts both integer and string ids for the same JSON array contract.

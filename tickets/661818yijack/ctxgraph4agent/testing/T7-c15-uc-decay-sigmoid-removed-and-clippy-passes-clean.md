---
id: T7
title: '[C15] UC: decay_sigmoid removed and clippy passes clean'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-13T02:05:34.556110Z'
updated_at: '2026-04-13T02:09:42.501077Z'
tags:
- uc
- clippy
- dead-code
version: 2
---

<!-- DESCRIPTION -->
GIVEN the decay_sigmoid function exists as dead code, WHEN removed from types.rs and cargo clippy -- -D warnings is run, THEN zero warnings are produced.

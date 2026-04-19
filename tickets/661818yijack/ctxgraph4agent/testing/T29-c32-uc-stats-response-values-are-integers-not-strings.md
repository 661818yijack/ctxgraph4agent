---
id: T29
title: '[C32] UC: Stats response values are integers not strings'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: low
status: open
owner: 661818yijack
file: null
created_at: '2026-04-19T02:05:13.110991Z'
updated_at: '2026-04-19T02:05:13.110991Z'
tags:
- uc
- test
- stats
---

<!-- DESCRIPTION -->
Given stats has data, When stats() is called, Then all values in the map objects must be integers (e.g. {"fact": 5}) not strings.

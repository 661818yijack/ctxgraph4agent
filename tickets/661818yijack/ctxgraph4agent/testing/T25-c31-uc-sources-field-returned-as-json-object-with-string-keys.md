---
id: T25
title: '[C31] UC: sources field returned as JSON object with string keys'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-19T02:05:08.059009Z'
updated_at: '2026-04-19T02:07:44.416126Z'
tags:
- uc
- test
- stats
---

<!-- DESCRIPTION -->
Given stats has episode source data, When stats() is called, Then the sources field must be a JSON object {source_name: count} not an array of arrays.

---
id: C21
title: '[C17] REQ: Update existing decay and cleanup tests for 6-month experience
  TTL'
repo: 661818yijack/ctxgraph4agent
category: code_quality
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-15T02:04:49.954988Z'
updated_at: '2026-04-15T02:10:31.755880Z'
tags:
- req
- testing
- ttl
- experience
---

<!-- DESCRIPTION -->
Tests that check experience decay or cleanup timing (e.g., test_decay_experience_at_ttl_scores_zero, cleanup tests) may hardcode 14d expectations. Update them to use 180d (15552000s). Ensure all tests pass after the change.

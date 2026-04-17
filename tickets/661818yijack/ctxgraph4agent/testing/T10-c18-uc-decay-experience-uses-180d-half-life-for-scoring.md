---
id: T10
title: '[C18] UC: decay_experience uses 180d half-life for scoring'
repo: 661818yijack/ctxgraph4agent
category: testing
severity: medium
status: closed
owner: 661818yijack
file: null
created_at: '2026-04-15T02:05:00.031477Z'
updated_at: '2026-04-15T02:10:32.095252Z'
tags:
- uc
- test
- decay
- experience
---

<!-- DESCRIPTION -->
Given an Experience entity created 90 days ago, when decay_score is computed, then the score is above 0.5 (not near zero). At exactly 180 days, the score should be near 0.

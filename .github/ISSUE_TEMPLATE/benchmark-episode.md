---
name: Benchmark Episode Submission
about: Submit new episodes for the extraction benchmark
title: "[benchmark] "
labels: benchmark
---

## Episode Text

```
Paste your 2-6 sentence episode here. Should describe a real architectural decision, incident, or migration.
```

## Expected Entities

| Name | Entity Type |
|------|-------------|
| Example | Service |

Valid types: Person, Component, Service, Language, Database, Infrastructure, Decision, Constraint, Metric, Pattern

## Expected Relations

| Head | Relation | Tail |
|------|----------|------|
| Example | depends_on | OtherEntity |

Valid relations: chose, rejected, replaced, depends_on, fixed, introduced, deprecated, caused, constrained_by

## Why is this episode interesting?

Explain what makes this episode valuable for the benchmark. For example:
- Edge case (ambiguous entity boundaries, overlapping spans)
- Unusual domain terminology
- Multiple valid interpretations
- Stress-tests a specific relation type
- Real-world text pattern not yet covered

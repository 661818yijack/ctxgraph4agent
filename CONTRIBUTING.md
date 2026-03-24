# Contributing to ctxgraph

ctxgraph welcomes contributions. The highest-impact way to help is submitting new benchmark episodes -- but code contributions, bug reports, and documentation improvements are all appreciated.

## Benchmark Episode Submissions

The extraction benchmark is the primary quality gate for ctxgraph. More episodes mean better coverage of real-world text patterns and harder edge cases.

### JSON Schema

Each episode in `crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json` follows this structure:

```json
{
  "text": "2-6 sentences describing an architectural decision, incident, or migration.",
  "expected_entities": [
    { "name": "Postgres", "entity_type": "Database", "span_start": 27, "span_end": 35 }
  ],
  "expected_relations": [
    { "head": "AuthService", "relation": "depends_on", "tail": "Postgres" }
  ]
}
```

- `span_start` / `span_end` are character offsets into `text` (0-indexed, exclusive end).
- `name` must match the exact substring at those offsets.

### Valid Entity Types (10)

| Type | Description |
|------|-------------|
| Person | Person or engineer |
| Component | Software library or framework |
| Service | Cloud service or API |
| Language | Programming language |
| Database | Database or data store |
| Infrastructure | Server or cloud platform |
| Decision | Architectural decision |
| Constraint | Technical constraint |
| Metric | Performance metric |
| Pattern | Design pattern |

### Valid Relation Types (9)

| Relation | Meaning |
|----------|---------|
| chose | Chose or adopted a technology |
| rejected | Rejected an alternative |
| replaced | One thing replaced another |
| depends_on | Dependency relationship |
| fixed | Something fixed an issue |
| introduced | Introduced or added a component |
| deprecated | Deprecation action |
| caused | Causal relationship |
| constrained_by | Decision constrained by something |

### Guidelines

- **Text**: 2-6 sentences about real architectural decisions, incidents, migrations, or ADRs.
- **Entities**: 2-6 per episode. Use canonical names (`"Postgres"`, not `"the primary Postgres cluster"`). Names must be exact substrings of the text.
- **Relations**: 1-4 per episode. Head and tail must reference entity names defined in `expected_entities`.
- **Adversarial episodes are welcome**: edge cases, ambiguous phrasing, unusual domain terminology, overlapping entity spans, entities that look like relations, etc.

### How to Submit

Open a PR that appends your episodes to `crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json`. Run the benchmark to see how the pipeline handles them:

```bash
CTXGRAPH_MODELS_DIR=~/.cache/ctxgraph/models \
  cargo test --package ctxgraph-extract --test benchmark_test -- --ignored
```

## Code Contributions

- Rust edition 2024
- Run `cargo fmt` and `cargo clippy` before submitting a PR
- Run tests: `cargo test --workspace`
- Run the extraction benchmark (see above) if your changes touch the extraction pipeline

## Bug Reports

Use [GitHub Issues](../../issues). Include:

- What you expected vs. what happened
- Steps to reproduce
- ctxgraph version (`cargo metadata --format-version 1 | jq '.packages[] | select(.name == "ctxgraph") | .version'` or check `Cargo.toml`)
- OS and Rust version

## License

Contributions are licensed under MIT, matching the project license.

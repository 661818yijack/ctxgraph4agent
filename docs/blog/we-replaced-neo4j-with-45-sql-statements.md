# We Replaced Neo4j with 45 SQL Statements

Every knowledge graph tool we evaluated required Neo4j (or FalkorDB), Docker, a JVM, and a network hop. We needed a graph for decision traces — who decided what, why, and what alternatives were considered. We didn't need a distributed cluster. We needed a single file.

So we built ctxgraph's storage layer entirely on SQLite. 7 tables, 3 virtual tables, 8 indexes, 9 triggers, and a recursive CTE. No extensions beyond what ships with SQLite. Here's how it works and where it breaks.

## The schema

A knowledge graph has three kinds of things: **events** (something happened), **entities** (a thing that exists), and **edges** (how things relate). We call events "episodes" because they're atomic units of information — a decision, a Slack message, a git commit.

```sql
CREATE TABLE episodes (
    id          TEXT PRIMARY KEY,
    content     TEXT NOT NULL,
    source      TEXT,
    recorded_at TEXT NOT NULL,
    metadata    TEXT,
    embedding   BLOB
);

CREATE TABLE entities (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    summary     TEXT,
    created_at  TEXT NOT NULL,
    metadata    TEXT
);

CREATE TABLE edges (
    id          TEXT PRIMARY KEY,
    source_id   TEXT NOT NULL REFERENCES entities(id),
    target_id   TEXT NOT NULL REFERENCES entities(id),
    relation    TEXT NOT NULL,
    fact        TEXT,
    valid_from  TEXT,
    valid_until TEXT,
    recorded_at TEXT NOT NULL,
    confidence  REAL DEFAULT 1.0,
    episode_id  TEXT REFERENCES episodes(id),
    metadata    TEXT
);
```

Plus a junction table linking episodes to the entities mentioned in them:

```sql
CREATE TABLE episode_entities (
    episode_id TEXT REFERENCES episodes(id),
    entity_id  TEXT REFERENCES entities(id),
    span_start INTEGER,
    span_end   INTEGER,
    PRIMARY KEY (episode_id, entity_id)
);
```

`span_start` and `span_end` track character offsets — where in the episode text was this entity mentioned. Useful for highlighting, provenance, and debugging extraction.

This is deliberately boring. Entities and edges are the standard property graph model. The interesting parts are what we built on top.

## Graph traversal in one SQL statement

This is the query that replaces Cypher's `MATCH (a)-[*1..3]-(b)`:

```sql
WITH RECURSIVE traversal(entity_id, depth) AS (
    -- Anchor: start from one entity
    SELECT ?1, 0

    UNION

    -- Recursive step: follow edges in both directions
    SELECT
        CASE WHEN e.source_id = t.entity_id THEN e.target_id
             ELSE e.source_id END,
        t.depth + 1
    FROM traversal t
    JOIN edges e ON (e.source_id = t.entity_id OR e.target_id = t.entity_id)
    WHERE t.depth < ?2
      AND e.valid_until IS NULL  -- only current edges
)
SELECT DISTINCT ent.id, ent.name, ent.entity_type, ent.summary,
                ent.created_at, ent.metadata, t.depth
FROM traversal t
JOIN entities ent ON ent.id = t.entity_id
ORDER BY t.depth
```

The `CASE` expression handles bidirectional traversal — if the current entity is the source of an edge, follow to the target; otherwise follow to the source. `UNION` (not `UNION ALL`) prevents cycles by deduplicating visited entities.

The `WHERE e.valid_until IS NULL` clause is the temporal filter — more on that in a moment.

This isn't going to beat Cypher on a 10M-node graph. But for the graphs we care about (<100K nodes, typically <10K), it runs in single-digit milliseconds. The depth limit (`?2`, usually 2-3 hops) keeps the recursion bounded.

After traversal, we collect all edges connecting the discovered entities:

```sql
SELECT id, source_id, target_id, relation, fact,
       valid_from, valid_until, recorded_at, confidence, episode_id, metadata
FROM edges
WHERE source_id IN (?1, ?2, ..., ?n) AND target_id IN (?1, ?2, ..., ?n)
  AND valid_until IS NULL
ORDER BY recorded_at DESC
```

The `IN` clause is built dynamically in Rust with the entity IDs from the traversal result. Each ID is bound twice — once for `source_id`, once for `target_id`. This gives us the complete subgraph around any starting entity.

## Bi-temporal edges: facts expire but never die

Most graph databases treat edges as current state — you either have a `works_at` relationship or you don't. But context graphs are about history. "Alice worked at Google from 2020 to 2025, then joined Meta" is two edges, not one overwritten edge.

Every edge in ctxgraph has two time dimensions:

**Valid time** — when was this fact true in the real world?
```
Alice --[works_at]--> Google   valid_from: 2020-01, valid_until: 2025-06
Alice --[works_at]--> Meta     valid_from: 2025-06, valid_until: NULL
```

**Transaction time** — when was this fact recorded in the system?
```
recorded_at: 2025-06-15T09:00:00Z  (when we learned about the job change)
```

Facts are never deleted. They're invalidated:

```sql
UPDATE edges SET valid_until = ?1 WHERE id = ?2 AND valid_until IS NULL
```

This means we can answer three kinds of temporal queries:

**Current state:** `WHERE valid_until IS NULL`

**Time travel:** "What was true in January 2024?"
```sql
WHERE (valid_from IS NULL OR valid_from <= '2024-01-15')
  AND (valid_until IS NULL OR valid_until > '2024-01-15')
```

**Audit trail:** "What was recorded last week?"
```sql
WHERE recorded_at BETWEEN '2026-03-17' AND '2026-03-24'
```

The bi-temporal model adds one `IS NULL` check to every edge query. In exchange, we get full history, time-travel, and an audit trail with zero additional infrastructure. The recursive CTE from the previous section already filters on `valid_until IS NULL` by default — temporal awareness is baked into traversal, not bolted on.

## Three FTS5 indexes, nine triggers, zero extra infrastructure

SQLite's FTS5 extension gives us full-text search without Elasticsearch. We create three virtual tables — one for each searchable surface:

```sql
CREATE VIRTUAL TABLE episodes_fts USING fts5(
    content, source, metadata,
    content=episodes, content_rowid=rowid
);

CREATE VIRTUAL TABLE entities_fts USING fts5(
    name, entity_type, summary,
    content=entities, content_rowid=rowid
);

CREATE VIRTUAL TABLE edges_fts USING fts5(
    fact, relation,
    content=edges, content_rowid=rowid
);
```

The `content=episodes, content_rowid=rowid` syntax makes these "external content" FTS5 tables — they store only the index, not a copy of the data. The source of truth stays in the base tables.

The catch with external content tables is that FTS5 doesn't automatically stay in sync. We use triggers:

```sql
-- Keep episodes_fts in sync with episodes table
CREATE TRIGGER episodes_ai AFTER INSERT ON episodes BEGIN
    INSERT INTO episodes_fts(rowid, content, source, metadata)
    VALUES (new.rowid, new.content, new.source, new.metadata);
END;

CREATE TRIGGER episodes_ad AFTER DELETE ON episodes BEGIN
    INSERT INTO episodes_fts(episodes_fts, rowid, content, source, metadata)
    VALUES ('delete', old.rowid, old.content, old.source, old.metadata);
END;

CREATE TRIGGER episodes_au AFTER UPDATE ON episodes BEGIN
    INSERT INTO episodes_fts(episodes_fts, rowid, content, source, metadata)
    VALUES ('delete', old.rowid, old.content, old.source, old.metadata);
    INSERT INTO episodes_fts(rowid, content, source, metadata)
    VALUES (new.rowid, new.content, new.source, new.metadata);
END;
```

The `VALUES ('delete', ...)` syntax is FTS5's way of removing an entry — you insert into the FTS table with the special first column set to `'delete'`. The update trigger does a delete-then-insert.

We have 9 triggers total: 3 for episodes, 3 for entities, 3 for edges. Each set handles INSERT, DELETE, and UPDATE.

Searching is then a simple join:

```sql
SELECT e.id, e.content, e.source, e.recorded_at, e.metadata, rank
FROM episodes_fts fts
JOIN episodes e ON e.rowid = fts.rowid
WHERE episodes_fts MATCH ?1
ORDER BY rank
LIMIT ?2
```

FTS5's `rank` is negative (lower = better match), so we negate it in application code to get a positive relevance score. We search episodes, entities, and edges separately — a query for "Postgres" might match an episode mentioning it, an entity named "Postgres", and an edge with fact "billing chose Postgres".

## Hybrid search: FTS5 + vectors + graph, fused with RRF

Keyword search misses semantic similarity. Vector search misses exact matches. Graph traversal misses both but finds structural connections. We run all three and fuse the results using Reciprocal Rank Fusion.

**RRF formula:** For each result appearing in any mode, its fused score is:

```
score(d) = Σ 1/(k + rank_i(d))    for each mode i where d appears
```

With `k = 60` (the standard constant from the original RRF paper).

The implementation is ~30 lines:

```rust
const K: f64 = 60.0;
let mut scores: HashMap<String, f64> = HashMap::new();

// Mode 1: FTS5 keyword search
let fts_results = storage.search_episodes(query, limit * 10);
for (rank, (episode, _)) in fts_results.iter().enumerate() {
    *scores.entry(episode.id.clone()).or_insert(0.0) += 1.0 / (K + rank as f64 + 1.0);
}

// Mode 2: Semantic search (cosine similarity over 384-dim embeddings)
let all_embeddings = graph.get_embeddings()?;
let mut semantic: Vec<(String, f32)> = all_embeddings
    .iter()
    .map(|(id, vec)| (id.clone(), cosine_similarity(query_embedding, vec)))
    .collect();
semantic.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

for (rank, (id, _)) in semantic.iter().enumerate() {
    *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (K + rank as f64 + 1.0);
}

// Fuse: sort by combined score, return top-k
let mut fused: Vec<(String, f64)> = scores.into_iter().collect();
fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
```

Why RRF instead of a weighted sum? FTS5 BM25 scores, cosine similarities, and graph depth are on completely different scales. Normalizing them requires assumptions about score distributions. RRF sidesteps this by using ranks, not scores — a result that appears in the top 5 of both FTS5 and semantic search gets a higher fused score than something that's #1 in FTS5 but absent from semantic results.

The third mode (graph traversal) works differently — it's not a ranked list but a subgraph expansion. We search entities by name, then traverse their neighborhoods. Entities and episodes discovered through graph walk get a bonus score in the fusion. A result found via keyword match AND connected to a matched entity via graph edges is ranked highest.

Embeddings are stored as BLOB columns (serialized f32 little-endian) in the episodes table. Cosine similarity is brute-force over all embeddings. This is fine for <100K episodes. Beyond that, you'd want an HNSW index via sqlite-vec or similar.

## Entity deduplication: Jaro-Winkler meets SQL

When extracting entities from text, "PostgreSQL", "Postgres", and "PG" should resolve to the same node. We use a two-level dedup strategy:

**Level 1: Exact alias match** (SQL)
```sql
SELECT canonical_id FROM aliases WHERE alias_name = ?1 COLLATE NOCASE
```

**Level 2: Fuzzy match** (Rust)

If no exact alias exists, we fetch all entities of the same type and compute Jaro-Winkler similarity in Rust:

```rust
let existing = storage.get_entity_names_by_type(&entity.entity_type)?;
for (existing_id, existing_name) in &existing {
    let sim = strsim::jaro_winkler(&name_lower, &existing_name.to_lowercase());
    if sim >= 0.85 {
        // Merge: store alias, return existing entity ID
        storage.add_alias(&existing_id, &entity.name, sim)?;
        return Ok((existing_id.clone(), true));
    }
}
```

The aliases table:

```sql
CREATE TABLE aliases (
    canonical_id TEXT REFERENCES entities(id),
    alias_name   TEXT NOT NULL,
    similarity   REAL,
    UNIQUE(canonical_id, alias_name)
);
```

Why not do fuzzy matching in SQL? SQLite has no built-in string similarity functions. We could load an extension, but `strsim::jaro_winkler` in Rust is fast enough and avoids the dependency. The threshold (0.85) is tuned for software engineering terminology — "PostgreSQL" vs "Postgres" scores 0.92, "Redis" vs "Redisx" scores 0.96, "React" vs "Redux" scores 0.73 (correctly rejected).

## Performance pragmas

Three lines that matter:

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;
```

**WAL** (Write-Ahead Logging) allows concurrent readers while a writer is active. ctxgraph's typical workload is: one writer ingesting episodes, multiple readers searching and traversing. WAL handles this without locks.

**synchronous = NORMAL** balances durability and speed. We can tolerate losing the last few milliseconds of writes in a crash — this isn't a banking system.

**foreign_keys = ON** because referential integrity matters when edges reference entities. SQLite doesn't enforce foreign keys by default.

## Where this breaks

**Scale ceiling: ~100K-1M nodes.** The recursive CTE scans edges on each recursion step. Without a native graph index, traversal becomes expensive at scale. For ctxgraph's target (team decision logs, not social network graphs), this isn't a concern.

**Brute-force vector search.** We load all embeddings into memory and compute cosine similarity against every one. This is O(n) per query. At 100K episodes with 384-dim embeddings, that's ~150MB of vectors and ~50ms per search. Acceptable, but won't scale to millions. The fix is straightforward: add sqlite-vec or a dedicated HNSW index.

**No native graph query language.** Cypher lets you express `MATCH (a:Person)-[:WORKS_AT]->(b:Company) WHERE b.name = 'Google'` in one line. In SQLite, that's a multi-table JOIN with explicit column references. The recursive CTE is powerful but verbose.

**Concurrent writers.** SQLite allows one writer at a time (WAL mode helps readers but not writers). For a single-user CLI or MCP server, this is fine. For a multi-writer web service, you'd need a different database.

**The graduate-out path.** If you outgrow this, export to JSON and import into Neo4j, Memgraph, or DGraph. The schema maps directly to labeled property graphs. We consider this a feature, not a limitation — start simple, graduate when you actually need it.

## The numbers

We benchmarked ctxgraph against Graphiti (by Zep), which uses Neo4j + 6+ GPT-4o API calls per episode:

| | ctxgraph (SQLite) | Graphiti (Neo4j + GPT-4o) |
|---|---|---|
| Entity F1 | **0.837** | 0.570 |
| Relation F1 | **0.763** | 0.104 |
| Per episode | ~40ms | ~10s |
| Infrastructure | Single file | Neo4j (Docker) + OpenAI API |
| Cost | $0 | ~$2-5 for 50 episodes |

The extraction quality difference isn't about SQLite vs Neo4j — it's about local ONNX models with domain heuristics vs generic GPT-4o prompts. But the infrastructure difference is entirely about the storage choice. Zero setup, zero config, zero cost.

## All 45 statements

For the curious, here's every SQL statement in ctxgraph's storage layer:

- **Schema**: 7 CREATE TABLE, 3 CREATE VIRTUAL TABLE, 8 CREATE INDEX, 9 CREATE TRIGGER
- **Episodes**: INSERT, SELECT by id, SELECT paginated, FTS5 MATCH
- **Entities**: INSERT, SELECT by id, SELECT by name, SELECT by name+type, SELECT names by type, SELECT paginated, FTS5 MATCH
- **Edges**: INSERT, SELECT by id, SELECT by entity (both directions), SELECT current by entity, UPDATE valid_until, SELECT between entity set, FTS5 MATCH
- **Aliases**: INSERT OR IGNORE, SELECT by alias
- **Episode-Entity links**: INSERT OR IGNORE
- **Embeddings**: UPDATE episodes SET embedding, UPDATE entities SET embedding, SELECT all embeddings
- **Stats**: COUNT episodes, COUNT entities, COUNT edges, GROUP BY source, page_count * page_size
- **Traversal**: WITH RECURSIVE CTE
- **Migrations**: CREATE _migrations, SELECT EXISTS, INSERT version

No views. No stored procedures. No computed columns. The complexity lives in the application layer where it's testable and debuggable. The SQL is deliberately boring — the system that emerges from it is not.

---

*ctxgraph is open source (MIT). The full storage layer is [~650 lines of Rust + SQL](https://github.com/rohansx/ctxgraph/tree/main/crates/ctxgraph-core/src/storage). The benchmark corpus is [open for inspection](https://github.com/rohansx/ctxgraph/blob/main/crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json).*

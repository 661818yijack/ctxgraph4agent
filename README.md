# ctxgraph

**Local-first context graph engine for AI agents and human teams.**

---

Three commands. Zero API keys. Your team's decisions become searchable.

```bash
brew install rohansx/tap/ctxgraph
ctxgraph init && ctxgraph models download
ctxgraph log "Migrated auth from Redis sessions to JWT. Chose JWT for stateless scaling."
ctxgraph query "Why did we move away from Redis?"
```

---

## What is ctxgraph?

ctxgraph is a local knowledge graph that extracts entities and relations from plain-text decision logs using on-device ONNX models (GLiNER NER + heuristic relation extraction). Everything is stored in a single SQLite file and searchable via FTS5 keyword matching, semantic embeddings, and graph traversal — fused through Reciprocal Rank Fusion. Zero infrastructure: one Rust binary, no API keys, no Docker, no database server.

## Key Features

- **Fully local** — No API calls, no cloud, no internet after initial model download
- **Single binary** — One Rust executable, one SQLite file
- **Fast** — ~40ms per episode extraction
- **Schema-driven** — Entity types and relation labels are user-defined via `ctxgraph.toml`
- **Bi-temporal history** — Facts are invalidated, never deleted; query the graph at any point in time
- **Three search modes** — FTS5 + semantic embeddings + graph walk, fused via Reciprocal Rank Fusion
- **MCP server** — Connect to Claude Desktop, Cursor, or Claude Code so AI agents read/write your graph
- **Embeddable** — Rust SDK for embedding directly in your application
- **Privacy by default** — Nothing leaves your machine

## Installation

### Prerequisites

- **OS**: macOS (Intel/Apple Silicon) or Linux (x86_64/arm64)
- **RAM**: ~150 MB during inference
- **Disk**: ~750 MB for models (one-time download)
- **No API keys, no Docker, no Python, no database server needed**

### Option 1: Homebrew (recommended)

```bash
brew install rohansx/tap/ctxgraph
```

### Option 2: Prebuilt binary

Download from [GitHub Releases](https://github.com/rohansx/ctxgraph/releases) and add to your PATH:

```bash
# Example for Linux x86_64
curl -L https://github.com/rohansx/ctxgraph/releases/latest/download/ctxgraph-v0.6.0-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv ctxgraph /usr/local/bin/
```

### Option 3: Build from source

```bash
# Requires Rust 1.85+ (edition 2024)
cargo install ctxgraph-cli
```

## Getting Started

### Step 1: Download models (one-time, ~700 MB)

```bash
ctxgraph models download
```

This downloads to `~/.cache/ctxgraph/models/`:

| Model | Size | Purpose |
|-------|------|---------|
| GLiNER v2.1 (INT8 ONNX) | ~653 MB | Named entity recognition |
| GLiNER v2.1 tokenizer | ~17 MB | Tokenizer for NER model |
| all-MiniLM-L6-v2 | ~80 MB | Semantic search embeddings (auto-downloaded on first use) |

After this initial download, **no internet connection is ever needed again**.

### Step 2: Initialize in your project

```bash
cd your-project
ctxgraph init
```

Creates `.ctxgraph/graph.db` (a single SQLite file — your entire knowledge graph).

### Step 3: Start logging decisions

```bash
ctxgraph log "Chose Postgres over SQLite for billing. Reason: concurrent writes."
ctxgraph log --source slack "Priya approved the discount for Reliance"
ctxgraph log --tags "architecture,database" "Switched from REST to gRPC"
```

Each `log` command automatically:
1. Extracts entities (people, services, databases, etc.) using GLiNER
2. Extracts relations (chose, rejected, depends_on, etc.) using heuristics
3. Embeds the text for semantic search
4. Stores everything in the local SQLite graph

### Step 4: Query your knowledge graph

```bash
ctxgraph query "why Postgres?"
ctxgraph query "discount precedents" --limit 5
ctxgraph entities list
ctxgraph entities show Postgres
ctxgraph stats
```

**No API key needed for any of this.** Queries use SQLite FTS5 for keyword search and local MiniLM embeddings for semantic search.

## What It Looks Like

```
$ ctxgraph log "Chose Postgres over SQLite for billing. Reason: concurrent writes."
Episode stored: a1b2c3d4
  Extracted 3 entities
  Created 2 edges
```

```
$ ctxgraph query "why Postgres?"
Found 2 result(s) for 'why Postgres?':

  [a1b2c3d4] (cli, 2025-03-23 14:05) score=0.92
    Chose Postgres over SQLite for billing. Reason: concurrent writes.

  [e5f6a7b8] (slack, 2025-03-20 09:12) score=0.71
    Priya confirmed Postgres handles our write volume — benchmarked at 10k TPS.
```

```
$ ctxgraph entities show Postgres
Entity: Postgres (Database)
ID: 9f8e7d6c-...
Created: 2025-03-23 14:05

Relationships:
  --[chose]--> billing
  --[rejected]--> SQLite (invalidated)
  <--[depends_on]-- payment-service

Neighbors:
  billing (Service)
  SQLite (Database)
  payment-service (Component)
```

```
$ ctxgraph stats
ctxgraph stats
------------------------------
Episodes:  127
Entities:  89
Edges:     214
Sources:   cli (45), git (72), slack (10)
DB size:   2.4 MB
```

### Real-World Scenario

Your team has been logging decisions for three months — architecture choices, vendor evaluations, incident responses. A new engineer joins and asks: "Why are we using Postgres instead of MongoDB for the billing service?"

```bash
$ ctxgraph query "why Postgres for billing?"
```

ctxgraph returns the original decision episode, the benchmark data that supported it, and the Slack discussion where the team evaluated MongoDB and rejected it for lack of ACID transactions. The new engineer gets the full context in seconds instead of asking three people and reading old Slack threads.

## MCP Server (for AI Agents)

Connect ctxgraph to Claude Desktop, Cursor, or Claude Code as an MCP server so AI agents can read and write your knowledge graph.

### Setup

```bash
# Install the MCP server binary
cargo install ctxgraph-mcp

# Make sure models are downloaded
ctxgraph models download

# Initialize a project (if not done already)
cd your-project && ctxgraph init
```

Then add to your AI tool's config:

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "ctxgraph": {
      "command": "ctxgraph-mcp"
    }
  }
}
```

**Cursor** (Settings > MCP Servers):
```json
{
  "mcpServers": {
    "ctxgraph": {
      "command": "ctxgraph-mcp"
    }
  }
}
```

**Claude Code** (`~/.claude.json`):
```json
{
  "mcpServers": {
    "ctxgraph": {
      "command": "ctxgraph-mcp"
    }
  }
}
```

### Available Tools

| Tool | Description |
|---|---|
| `ctxgraph_add_episode` | Record a decision or event |
| `ctxgraph_search` | Search with fused FTS5 + semantic ranking |
| `ctxgraph_get_decision` | Get full decision trace by ID |
| `ctxgraph_traverse` | Walk the graph from an entity |
| `ctxgraph_find_precedents` | Find similar past decisions via embeddings |

All tools run **100% locally** — no API calls, no data leaves your machine.

## How It Works

For a full walkthrough in simpler language, see [`docs/EXPLAINED.md`](docs/EXPLAINED.md).

### In plain English

Think of ctxgraph like a **team memory engine**:

1. You write normal notes like _"We chose Postgres for billing because of concurrent writes."_  
2. ctxgraph reads that note locally and pulls out:
   - **things** (entities) like `Postgres`, `billing`
   - **connections** (relations) like `chose(billing, Postgres)`
   - **time context** (when it became true, and when it was recorded)
3. It stores everything in a local SQLite graph so you can ask:
   - "Why did we choose this?"
   - "What did we believe last month?"
   - "Show related decisions around billing and databases."

You can treat it as "searchable memory + decision timeline + relationship map" in one local file.

```
Your App / CLI / AI Agent
         |
    ctxgraph engine
         |
    +---------------------------------+
    |  Extraction                     |
    |  GLiNER v2.1 (ONNX) - local    |
    |  Entities: 0.837 F1             |
    |  Relations: 0.763 F1            |
    |  Temporal: date/time parsing    |
    +---------------------------------+
         |
    +---------------------------------+
    |  Storage                        |
    |  SQLite + FTS5                  |
    |  Bi-temporal timestamps         |
    |  Graph via recursive CTEs       |
    +---------------------------------+
         |
    +---------------------------------+
    |  Search                         |
    |  FTS5 + Semantic + Graph Walk   |
    |  Fused via Reciprocal Rank      |
    +---------------------------------+
```

### Extraction Pipeline

ctxgraph extracts entities and relationships from plain text using local ONNX models. No API calls, no cost, no internet required.

```
Input:  "Chose Postgres over SQLite for billing. Reason: concurrent writes."

Output: Entities  -> Postgres (Database), SQLite (Database), billing (Service)
        Relations -> chose(billing, Postgres), rejected(billing, SQLite)
        Temporal  -> recorded now, valid indefinitely
```

The pipeline:
1. **NER** — GLiNER v2.1 span-based extraction (10 entity types)
2. **Coreference** — Pronoun resolution to preceding entities
3. **Entity supplement** — Dictionary-based detection for names GLiNER missed
4. **Type remapping** — Fix common misclassifications using domain knowledge
5. **Relation extraction** — Keyword + proximity + schema-aware heuristics (9 relation types)
6. **Conflict resolution** — Resolve contradictory relations per entity pair
7. **Temporal parsing** — Date/time extraction with relative date support

Entity types and relation labels are fully configurable via `ctxgraph.toml`.

### Bi-Temporal History

Every relationship tracks two time dimensions:

- **valid_from / valid_until** — when was this true in the real world?
- **recorded_at** — when was this recorded?

Facts are never deleted — they are invalidated. You can query the graph as it existed at any point in time.

```
Alice -[works_at]-> Google   (2020-01 to 2025-06)
Alice -[works_at]-> Meta     (2025-06 to now)
```

### Search

Three search modes fused via Reciprocal Rank Fusion:

- **FTS5** — keyword matching across episodes, entities, edges
- **Semantic** — 384-dim embeddings via all-MiniLM-L6-v2 (local)
- **Graph traversal** — multi-hop walk via recursive CTEs

A result appearing in multiple modes is ranked highest.

### Quick glossary

- **Episode**: one note/event/decision you log.
- **Entity**: an important noun in that episode (person, service, database, decision, etc.).
- **Edge / Relation**: how two entities are connected (for example, `chose`, `blocked_by`).
- **Bi-temporal**: tracks both "when this was true" and "when we recorded it".
- **FTS5**: fast keyword search in SQLite.
- **Semantic search**: meaning-based search using local embeddings.
- **Graph traversal**: walking connected entities across multiple hops.

## Benchmark

The extraction pipeline is evaluated against 50 software-engineering episodes covering all 10 entity types and 9 relation types. Scores are macro-averaged F1.

The full benchmark corpus of 50 episodes and ground-truth annotations is [available in the repository](crates/ctxgraph-extract/tests/fixtures/benchmark_episodes.json) for inspection and reproduction.

**Note on methodology:** The benchmark corpus was authored by us. It is not cherry-picked to favor our heuristics, but it has not been independently validated yet. We invite community submissions of new episodes and ground-truth annotations to make the benchmark more robust — see [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
cargo test --test benchmark_test -- --ignored --nocapture
```

Requires ONNX models (`ctxgraph models download`).

### Results (v0.6.0 — GLiNER v2.1 INT8, fully local)

| Metric | Score |
|---|---|
| Entity F1 | 0.837 |
| Relation F1 | 0.763 |
| **Combined F1** | **0.800** |
| Latency | ~40ms/episode |

### Comparison with Graphiti

Both systems were tested on the same 50 episodes with identical ground truth.

| | ctxgraph | Graphiti (gpt-4o) |
|---|---|---|
| Entity F1 | **0.837** | 0.570 |
| Relation F1 | **0.763** | 0.104\* |
| Combined F1 | **0.800** | 0.337 |
| API calls | 0 | ~200+ |
| Cost | $0 | ~$2-5 |
| Per episode | ~40ms | ~10s |
| Infrastructure | SQLite | Neo4j (Docker) |
| Privacy | 100% local | Data sent to OpenAI |

\*With generous semantic mapping of Graphiti's free-form relations to ctxgraph's taxonomy.

ctxgraph achieves **2.4x higher combined F1** than Graphiti while being **250x faster** and **100% free**.

### Why Graphiti Scores Lower

Graphiti makes 6+ GPT-4o calls per episode (entity extraction, deduplication, relation extraction, contradiction detection, summarization, community detection). Despite this:

- **Entity names are verbose**: Graphiti extracts `"primary Postgres cluster"` instead of `"Postgres"`, `"legacy SOAP endpoint in UserService"` instead of `"SOAP endpoint"`. Semantically correct, but doesn't match canonical names.
- **Relations are free-form**: Produces verbs like `COMMUNICATES_ENCRYPTED_WITH` and `PREVENTS_CASCADING_FAILURES_WHEN_DOWN` that don't map to a typed taxonomy. Even with generous keyword mapping, only 10/50 episodes produce any matching relations.
- **Different decomposition**: "Migrate from Redis to Postgres" becomes `(AuthService, CONNECTS_TO, primary Postgres cluster)` instead of `(Postgres, replaced, Redis)` + `(AuthService, depends_on, Postgres)`.

ctxgraph uses domain-specific heuristics for software engineering patterns — keyword matching, proximity scoring, coreference resolution, and schema-aware type validation — that produce structured, queryable knowledge without any API calls.

### Infrastructure Comparison

| | Graphiti (Zep) | ctxgraph |
|---|---|---|
| Graph database | Neo4j / FalkorDB (Docker) | SQLite (embedded) |
| LLM API key | Required (OpenAI) | Not required |
| Runtime | Python 3.10+ | Single Rust binary |
| Models | Cloud API (gpt-4o) | Local ONNX (~623 MB) |
| RAM usage | Neo4j: 512MB+ | ~150 MB (inference) |
| Cost per episode | ~$0.01-0.05 | $0.00 |
| Setup time | 15-30 min (Neo4j + pip) | `cargo install` |
| Internet required | Always (LLM calls) | Only for initial model download |
| Privacy | Data sent to OpenAI | Nothing leaves your machine |

See [docs/benchmark.md](docs/benchmark.md) for the full comparison methodology and per-episode results.

## Rust SDK

Embed ctxgraph directly in your Rust application:

```rust
use ctxgraph::{Graph, Episode};

let graph = Graph::init(".ctxgraph")?;

// Log a decision
graph.add_episode(
    Episode::builder("Chose Postgres for billing — concurrent writes required")
        .source("architecture-review")
        .tag("database")
        .build()
)?;

// Search
let results = graph.search("why Postgres?", 10)?;

// Traverse from an entity
let neighbors = graph.traverse("Postgres", 2)?;
```

## CLI Reference

```
ctxgraph init [--name <name>]                         Initialize .ctxgraph/ in current directory
ctxgraph log <text> [--source <src>] [--tags <t1,t2>] Log a decision or event
ctxgraph query <text> [--limit <n>]                   Search the context graph
ctxgraph entities list [--type <type>]                List entities
ctxgraph entities show <id>                           Show entity with relationships
ctxgraph decisions list                               List episodes
ctxgraph decisions show <id>                          Show full decision trace
ctxgraph stats                                        Graph statistics
ctxgraph models download                              Download ONNX models
ctxgraph watch --git [--last <n>]                     Auto-capture git commits (planned)
ctxgraph export --format json|csv                     Export graph data (planned)
ctxgraph-mcp                                           Run as MCP server (separate binary)
```

## Configuration

```toml
# ctxgraph.toml

[schema]
name = "default"

[schema.entities]
Person = "A person involved in a decision"
Component = "A software component or technology"
Decision = "An explicit choice that was made"
Reason = "The justification behind a decision"

[schema.relations]
chose = { head = ["Person"], tail = ["Component"], description = "person chose" }
rejected = { head = ["Person"], tail = ["Component"], description = "person rejected" }
depends_on = { head = ["Component"], tail = ["Component"], description = "dependency" }
```

## Environment Variables

All optional — ctxgraph works out of the box with zero configuration.

| Variable | Default | Description |
|---|---|---|
| `CTXGRAPH_MODELS_DIR` | `~/.cache/ctxgraph/models` | Override ONNX model directory |
| `CTXGRAPH_DB` | `.ctxgraph/graph.db` | Override database path |
| `CTXGRAPH_NO_EMBED` | unset | Set to `1` to disable embedding (FTS5-only search) |

## Troubleshooting

**"no .ctxgraph/ found"** — Run `ctxgraph init` in your project directory first.

**"extraction pipeline not loaded"** — Run `ctxgraph models download` to download ONNX models (~700 MB).

**Slow first query** — The embedding model (~80 MB) is auto-downloaded by fastembed on first use. Subsequent queries are instant.

**High memory usage** — Set `CTXGRAPH_NO_EMBED=1` to disable semantic search and reduce RAM to ~50 MB (FTS5 keyword search still works).

## Project Structure

```
crates/
+-- ctxgraph-core/       Core engine: types, storage, query, temporal
+-- ctxgraph-extract/    Extraction pipeline (GLiNER ONNX, heuristics)
+-- ctxgraph-embed/      Local embedding generation
+-- ctxgraph-cli/        CLI binary
+-- ctxgraph-mcp/        MCP server for AI agents
+-- ctxgraph-sdk/        Re-export crate for embedding in Rust apps
```

## Design Principles

1. **Zero infrastructure** — One binary, one SQLite file
2. **Offline-first** — No internet required after model download
3. **Privacy by default** — Nothing leaves your machine
4. **Schema-driven** — Extraction labels are user-defined, not hardcoded
5. **Embeddable** — Rust library first, CLI second
6. **Append-only history** — Facts invalidated, never deleted

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on submitting code, benchmark episodes, and bug reports.

## License

MIT

# ctxgraph Glossary

ctxgraph is a context graph for AI agents. This glossary defines the terms an agent developer sees when using the Rust API, CLI, MCP server, or planning docs.

## Core graph terms

### Episode

An episode is one recorded piece of context: a message, decision, incident note, meeting snippet, or observation. It is the durable source text that ctxgraph stores before extracting graph structure.

In code, `Episode` contains the content, optional source, metadata, recorded timestamp, memory type, and compression link.

### Entity

An entity is a thing mentioned by an episode, such as a person, service, database, project, or decision. Entities become graph nodes and can be deduplicated across episodes when names are similar enough.

An entity has an entity type and a memory type. The entity type describes what the node is; the memory type controls freshness and expiration behavior.

### Edge

An edge is a typed relationship between two entities. It records a source entity, target entity, relation label, confidence, optional fact text, and temporal validity fields.

Edges make the memory graph useful for questions like “what caused this incident?” or “which database did billing choose?”

### Source

A source identifies where an episode came from, such as `slack`, `meeting`, `github`, or an agent name. Source filters let callers narrow search results without changing the stored graph.

Use source when the origin matters for trust, debugging, or agent-specific recall.

### Memory type

A memory type describes how long a memory should remain fresh by default. ctxgraph currently uses fact, pattern, experience, preference, and decision categories.

Memory type controls the default TTL and decay curve: patterns do not expire by default, while facts, preferences, decisions, and experiences have time-based freshness behavior.

## Lifecycle and retrieval terms

### TTL

TTL means “time to live.” It is the duration after which a memory becomes old enough for expiration or review logic to consider it stale.

TTL is not the same as immediate deletion. It is an input to freshness scoring, cleanup, and re-verification decisions.

### Decay score

Decay score is a freshness value between `0.0` and `1.0`. Fresh, high-confidence memories score higher; stale memories score lower according to their memory type and TTL.

Retrieval uses decay score so agents prefer current context while still allowing important long-tail memories to remain available.

### Soft-expired memory

A soft-expired memory is marked as no longer active but has not been physically removed from the database. This preserves auditability and avoids unsafe deletion during graph cleanup.

Search, listing, traversal, and stats paths should exclude soft-expired records unless a caller intentionally asks for cleanup or audit behavior.

### Re-verification

Re-verification is the process of surfacing stale memories so an agent or user can confirm, update, or expire them. It prevents outdated context from silently polluting future decisions.

Facts and preferences usually need verification or updates. Experiences often expire naturally. Patterns are long-lived by default.

### Retrieval candidate

A retrieval candidate is a memory item considered for inclusion in an answer or agent context window. Candidates can come from text search, semantic search, graph traversal, or fused ranking.

Candidates are ranked by relevance, confidence, freshness, and budget constraints before they are returned to the caller.

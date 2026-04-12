use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

use ctxgraph::{Episode, Graph, MockBatchLabelDescriber, SkillScope};
use ctxgraph_embed::EmbedEngine;
use serde_json::{Value, json};

pub struct ToolContext {
    pub graph: Arc<AsyncMutex<Graph>>,
    pub embed: Option<Arc<EmbedEngine>>,
    /// In-memory embedding cache: episode_id → 384-dim vector.
    ///
    /// Populated lazily on the first `find_precedents` call, then kept warm.
    /// Invalidated (new entry appended) when `add_episode` stores a new embedding
    /// so subsequent searches never re-hit SQLite for already-loaded episodes.
    embedding_cache: Mutex<Option<HashMap<String, Vec<f32>>>>,
}

impl ToolContext {
    pub fn new(graph: Graph, embed: Option<EmbedEngine>) -> Self {
        Self {
            graph: Arc::new(AsyncMutex::new(graph)),
            embed: embed.map(Arc::new),
            embedding_cache: Mutex::new(None),
        }
    }

    /// Populate the in-memory embedding cache from SQLite if it hasn't been loaded yet.
    /// Subsequent calls return immediately — the Option acts as a once-flag.
    async fn warm_cache(&self) -> Result<(), String> {
        let needs_load = {
            let cache = self.embedding_cache.lock().map_err(|e| e.to_string())?;
            cache.is_none()
        };
        if needs_load {
            let graph = self.graph.lock().await;
            let rows = graph.get_embeddings().map_err(|e| e.to_string())?;
            let mut cache = self.embedding_cache.lock().map_err(|e| e.to_string())?;
            *cache = Some(rows.into_iter().collect());
        }
        Ok(())
    }

    /// Tool: add_episode
    /// Adds a new episode to the graph, computes and stores its embedding.
    pub async fn add_episode(&self, args: Value) -> Result<Value, String> {
        let text = args["text"]
            .as_str()
            .ok_or("missing required field: text")?
            .to_string();

        let source = args["source"].as_str().map(|s| s.to_string());

        let mut builder = Episode::builder(&text);
        if let Some(ref src) = source {
            builder = builder.source(src);
        }
        if let Some(tags) = args["tags"].as_array() {
            for tag in tags {
                if let Some(t) = tag.as_str() {
                    builder = builder.tag(t);
                }
            }
        }
        let episode = builder.build();
        let episode_id = episode.id.clone();

        // Store episode
        let result = {
            let graph = self.graph.lock().await;
            graph
                .add_episode(episode)
                .await
                .map_err(|e| e.to_string())?
        };

        // Compute embedding and persist to SQLite (skipped if embed engine is unavailable)
        if let Some(ref embed) = self.embed {
            let embedding = embed.embed(&text).map_err(|e| e.to_string())?;
            {
                let graph = self.graph.lock().await;
                graph
                    .store_embedding(&episode_id, &embedding)
                    .map_err(|e| e.to_string())?;
            }

            // Insert into in-memory cache so find_precedents sees it immediately
            // without another SQLite round-trip.
            if let Ok(mut cache) = self.embedding_cache.lock()
                && let Some(ref mut map) = *cache
            {
                map.insert(episode_id.clone(), embedding);
                // If cache is None (never warmed), leave it — it will be populated
                // from SQLite (including this episode) on the first find_precedents call.
            }
        }

        Ok(json!({
            "episode_id": result.episode_id,
            "entities_found": result.entities_extracted,
            "edges_created": result.edges_created,
        }))
    }

    /// Tool: search
    /// Fused FTS5 + semantic search via RRF. Falls back to FTS5-only if embed is unavailable.
    pub async fn search(&self, args: Value) -> Result<Value, String> {
        let query = args["query"]
            .as_str()
            .ok_or("missing required field: query")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        let source = args["source"].as_str();

        let items: Vec<Value> = if let Some(ref embed) = self.embed {
            // Fused FTS5 + semantic search via RRF
            let query_embedding = embed.embed(&query).map_err(|e| e.to_string())?;
            let results = {
                let graph = self.graph.lock().await;
                graph
                    .search_fused(&query, &query_embedding, limit, source)
                    .map_err(|e| e.to_string())?
            };
            results
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.episode.id,
                        "content": r.episode.content,
                        "score": r.score,
                        "source": r.episode.source,
                        "recorded_at": r.episode.recorded_at.to_rfc3339(),
                    })
                })
                .collect()
        } else {
            // FTS5-only search (embedding not available)
            let results = {
                let graph = self.graph.lock().await;
                graph.search(&query, limit, source).map_err(|e| e.to_string())?
            };
            results
                .into_iter()
                .map(|(episode, score)| {
                    json!({
                        "id": episode.id,
                        "content": episode.content,
                        "score": score,
                        "source": episode.source,
                        "recorded_at": episode.recorded_at.to_rfc3339(),
                    })
                })
                .collect()
        };

        Ok(json!(items))
    }

    /// Tool: get_decision
    /// Retrieve a specific episode by ID with full context.
    pub async fn get_decision(&self, args: Value) -> Result<Value, String> {
        let id = args["id"]
            .as_str()
            .ok_or("missing required field: id")?
            .to_string();

        let graph = self.graph.lock().await;

        let episode = graph
            .get_episode(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("episode not found: {id}"))?;

        // Serialize episode to JSON
        let episode_json = serde_json::to_value(&episode).map_err(|e| e.to_string())?;

        Ok(json!({
            "episode": episode_json,
        }))
    }

    /// Tool: traverse
    /// Traverse the knowledge graph from an entity.
    pub async fn traverse(&self, args: Value) -> Result<Value, String> {
        let entity_name = args["entity_name"]
            .as_str()
            .ok_or("missing required field: entity_name")?
            .to_string();
        let max_depth = (args["max_depth"].as_u64().unwrap_or(2) as usize).min(5);

        let graph = self.graph.lock().await;

        let entity = graph
            .get_entity_by_name(&entity_name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("entity not found: {entity_name}"))?;

        let (entities, edges) = graph
            .traverse(&entity.id, max_depth)
            .map_err(|e| e.to_string())?;

        let entities_json: Vec<Value> = entities
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        let edges_json: Vec<Value> = edges
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        Ok(json!({
            "entities": entities_json,
            "edges": edges_json,
        }))
    }

    /// Tool: find_precedents
    /// Find past episodes most semantically similar to a given context.
    ///
    /// Uses an in-memory embedding cache so the full SQLite embedding table is
    /// only read once per process lifetime. Subsequent calls score entirely in
    /// RAM — O(n) dot-products, no I/O.
    pub async fn find_precedents(&self, args: Value) -> Result<Value, String> {
        let context = args["context"]
            .as_str()
            .ok_or("missing required field: context")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

        let embed = match self.embed {
            Some(ref e) => e,
            None => {
                return Ok(
                    json!({"error": "embedding not available, start with CTXGRAPH_NO_EMBED unset"}),
                );
            }
        };

        // Embed the query (always ~20-50ms CPU inference, unavoidable)
        let context_embedding = embed.embed(&context).map_err(|e| e.to_string())?;

        // Ensure the cache is warm (no-op after first call)
        self.warm_cache().await?;

        // Score entirely in RAM — no SQLite I/O
        let mut scored: Vec<(String, f32)> = {
            let cache = self.embedding_cache.lock().map_err(|e| e.to_string())?;
            cache
                .as_ref()
                .expect("cache must be Some after warm_cache")
                .iter()
                .map(|(id, vec)| {
                    let sim = EmbedEngine::cosine_similarity(&context_embedding, vec);
                    (id.clone(), sim)
                })
                .collect()
        };

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top: Vec<(String, f32)> = scored.into_iter().take(limit).collect();

        let mut results = Vec::new();
        let graph = self.graph.lock().await;
        for (id, sim) in top {
            if let Ok(Some(ep)) = graph.get_episode(&id) {
                results.push(json!({
                    "id": ep.id,
                    "content": ep.content,
                    "source": ep.source,
                    "recorded_at": ep.recorded_at.to_rfc3339(),
                    "similarity": sim,
                }));
            }
        }

        Ok(json!(results))
    }

    /// Tool: list_entities
    /// List entities in the graph, optionally filtered by type.
    pub async fn list_entities(&self, args: Value) -> Result<Value, String> {
        let entity_type = args["entity_type"].as_str().map(|s| s.to_string());
        let limit = args["limit"].as_u64().unwrap_or(100) as usize;
        let offset = args["offset"].as_u64().unwrap_or(0) as usize;

        let graph = self.graph.lock().await;

        let entities = graph
            .list_entities(entity_type.as_deref(), limit)
            .map_err(|e| e.to_string())?;

        // Skip `offset` entities (list_entities doesn't support offset natively)
        let entities: Vec<Value> = entities
            .into_iter()
            .skip(offset)
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        Ok(json!({
            "entities": entities,
            "count": entities.len(),
        }))
    }

    /// Tool: export_graph
    /// Export all entities and edges from the graph.
    pub async fn export_graph(&self, args: Value) -> Result<Value, String> {
        let entity_type = args["entity_type"].as_str().map(|s| s.to_string());
        let include_episodes = args["include_episodes"].as_bool().unwrap_or(false);
        let limit = args["limit"].as_u64().unwrap_or(10000) as usize;

        let graph = self.graph.lock().await;

        let entities = graph
            .list_entities(entity_type.as_deref(), limit)
            .map_err(|e| e.to_string())?;

        // Collect all edges for all entities
        let mut all_edges = Vec::new();
        let mut seen_edge_ids = std::collections::HashSet::new();
        for entity in &entities {
            let edges = graph
                .get_edges_for_entity(&entity.id)
                .map_err(|e| e.to_string())?;
            for edge in edges {
                if seen_edge_ids.insert(edge.id.clone()) {
                    all_edges.push(edge);
                }
            }
        }

        let entities_json: Vec<Value> = entities
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        let edges_json: Vec<Value> = all_edges
            .into_iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();

        let mut result = json!({
            "entities": entities_json,
            "edges": edges_json,
            "entity_count": entities_json.len(),
            "edge_count": edges_json.len(),
        });

        if include_episodes {
            let episodes = graph.list_episodes(limit, 0).map_err(|e| e.to_string())?;
            let episodes_json: Vec<Value> = episodes
                .into_iter()
                .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
                .collect();
            result["episodes"] = json!(episodes_json);
            result["episode_count"] = json!(episodes_json.len());
        }

        Ok(result)
    }

    /// Tool: stats
    pub async fn stats(&self, _args: Value) -> Result<Value, String> {
        let graph = self.graph.lock().await;
        let stats = graph.stats().map_err(|e| e.to_string())?;

        // Convert Vec<(String, usize)> to JSON object
        let total_by_type: serde_json::Value =
            serde_json::to_value(&stats.total_entities_by_type).unwrap_or(serde_json::Value::Null);
        let decayed_by_type: serde_json::Value =
            serde_json::to_value(&stats.decayed_entities_by_type)
                .unwrap_or(serde_json::Value::Null);

        let mut result = json!({
            "episodes": stats.episode_count,
            "entities": stats.entity_count,
            "edges": stats.edge_count,
            "decayed_entities": stats.decayed_entities,
            "decayed_edges": stats.decayed_edges,
            "db_size_bytes": stats.db_size_bytes,
            "sources": stats.sources
        });

        // Add cleanup visibility fields
        if let Some(obj) = result.as_object_mut() {
            obj.insert("total_entities_by_type".to_string(), total_by_type);
            obj.insert("decayed_entities_by_type".to_string(), decayed_by_type);
            obj.insert(
                "last_cleanup_at".to_string(),
                serde_json::Value::String(
                    stats.last_cleanup_at.unwrap_or_else(|| "never".to_string()),
                ),
            );
            obj.insert(
                "queries_since_cleanup".to_string(),
                serde_json::Value::Number(stats.queries_since_cleanup.into()),
            );
            obj.insert(
                "cleanup_interval".to_string(),
                serde_json::Value::Number(stats.cleanup_interval.into()),
            );
            obj.insert(
                "cleanup_in_progress".to_string(),
                serde_json::Value::Bool(stats.cleanup_in_progress),
            );
            // Convenience field
            let next_in = stats
                .cleanup_interval
                .saturating_sub(stats.queries_since_cleanup);
            obj.insert(
                "next_cleanup_in".to_string(),
                serde_json::Value::Number(next_in.into()),
            );
        }

        Ok(result)
    }

    /// Tool: learn
    /// Run the learning pipeline: extract patterns from recent experiences and create or update skills.
    pub async fn learn(&self, args: Value) -> Result<Value, String> {
        let agent = args
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("assistant")
            .to_string();
        let scope_str = args
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or("private");
        let scope = if scope_str == "shared" {
            SkillScope::Shared
        } else {
            SkillScope::Private
        };
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        let describer = MockBatchLabelDescriber;

        let graph = self.graph.lock().await;
        let result = graph
            .run_learning_pipeline(&agent, scope, &describer, limit)
            .await
            .map_err(|e| e.to_string())?;

        Ok(json!({
            "patterns_found": result.patterns_found,
            "skills_created": result.skills_created,
            "skills_updated": result.skills_updated
        }))
    }

    /// Tool: forget
    /// Manually expire memories by ID or by type.
    ///
    /// **By ID:**
    /// - `hard: false` (default): marks the memory for deletion. It will be removed in the next cleanup sweep.
    /// - `hard: true`: immediately deletes the memory from the database.
    ///
    /// **By type:**
    /// - `type`: expire all memories of the given type (fact, experience, preference, decision).
    /// - `hard: false` (default): marks all matching memories for deletion.
    /// - `hard: true`: immediately deletes all matching memories.
    /// - Pattern type is rejected (patterns never expire).
    ///
    /// Use this when you know memories are no longer relevant.
    pub async fn forget(&self, args: Value) -> Result<Value, String> {
        let id = args.get("id").and_then(|v| v.as_str());
        let memory_type = args.get("type").and_then(|v| v.as_str());
        let hard = args.get("hard").and_then(|v| v.as_bool()).unwrap_or(false);

        // Validate: exactly one of id or type must be provided
        match (id, memory_type) {
            (Some(id), None) => {
                // Single memory by ID
                let graph = self.graph.lock().await;
                if hard {
                    graph.expire_memory(id).map_err(|e| e.to_string())?;
                    Ok(json!({"ok": true, "id": id, "hard": true}))
                } else {
                    let found = graph
                        .storage
                        .mark_for_deletion(id)
                        .map_err(|e| e.to_string())?;
                    if found {
                        Ok(
                            json!({"ok": true, "id": id, "hard": false, "note": "will be removed in next cleanup"}),
                        )
                    } else {
                        Err(format!("memory not found: {}", id))
                    }
                }
            }
            (None, Some(memory_type)) => {
                // Bulk expire by type
                let graph = self.graph.lock().await;
                let (entities, edges) = graph
                    .storage
                    .expire_memories_by_type(memory_type, hard)
                    .map_err(|e| e.to_string())?;
                let action = if hard {
                    "deleted"
                } else {
                    "marked for cleanup"
                };
                Ok(json!({
                    "ok": true,
                    "type": memory_type,
                    "hard": hard,
                    "entities_affected": entities,
                    "edges_affected": edges,
                    "note": format!("{} {} {} memories", entities + edges, action, memory_type)
                }))
            }
            (Some(_), Some(_)) => {
                Err("cannot specify both 'id' and 'type'. Use one or the other.".to_string())
            }
            (None, None) => {
                Err("missing required field: either 'id' or 'type' must be provided".to_string())
            }
        }
    }

    /// Tool: traverse_batch
    /// Traverse multiple entities in one call, returning a merged result.
    ///
    /// Replaces N sequential `traverse` calls with a single round-trip.
    /// Entities not found in the graph are silently skipped (same behaviour as
    /// individual `traverse` returning an error for unknown entities).
    pub async fn traverse_batch(&self, args: Value) -> Result<Value, String> {
        let entity_names: Vec<String> = args["entity_names"]
            .as_array()
            .ok_or("missing required field: entity_names")?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        if entity_names.is_empty() {
            return Ok(json!({"entities": [], "edges": []}));
        }

        let max_depth = (args["max_depth"].as_u64().unwrap_or(2) as usize).min(5);

        let graph = self.graph.lock().await;

        let mut all_entities: Vec<Value> = Vec::new();
        let mut all_edges: Vec<Value> = Vec::new();
        // Dedup by ID across multiple traversals
        let mut seen_entities = std::collections::HashSet::new();
        let mut seen_edges = std::collections::HashSet::new();

        for name in &entity_names {
            let entity = match graph.get_entity_by_name(name) {
                Ok(Some(e)) => e,
                Ok(None) | Err(_) => continue, // skip unknown entities
            };

            let (entities, edges) = match graph.traverse(&entity.id, max_depth) {
                Ok(r) => r,
                Err(_) => continue,
            };

            for e in entities {
                if seen_entities.insert(e.id.clone()) {
                    all_entities.push(serde_json::to_value(e).unwrap_or(Value::Null));
                }
            }
            for e in edges {
                if seen_edges.insert(e.id.clone()) {
                    all_edges.push(serde_json::to_value(e).unwrap_or(Value::Null));
                }
            }
        }

        Ok(json!({
            "entities": all_entities,
            "edges": all_edges,
        }))
    }
}

/// Wrap a tool result into an MCP content array.
pub fn tool_result(result: Result<Value, String>) -> Value {
    let text = match result {
        Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string()),
        Err(msg) => serde_json::to_string_pretty(&json!({"error": msg}))
            .unwrap_or_else(|_| format!(r#"{{"error": "{msg}"}}"#)),
    };
    json!({
        "content": [{"type": "text", "text": text}]
    })
}

/// Returns the static tools list payload.
pub fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "add_episode",
                "description": "Add a new episode (decision, observation, or event) to the context graph. Extracts entities and relations automatically.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "text": {"type": "string", "description": "The episode content"},
                        "source": {"type": "string", "description": "Source label (e.g. 'slack', 'meeting', 'code-review')"},
                        "tags": {"type": "array", "items": {"type": "string"}, "description": "Optional tags"}
                    },
                    "required": ["text"]
                }
            },
            {
                "name": "search",
                "description": "Search the context graph using semantic + keyword fusion (RRF). Returns ranked episodes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "limit": {"type": "integer", "description": "Max results (default 10)"},
                        "source": {"type": "string", "description": "Filter by source"}
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "get_decision",
                "description": "Retrieve a specific episode by ID with full entity and relation context.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "Episode UUID"}
                    },
                    "required": ["id"]
                }
            },
            {
                "name": "traverse",
                "description": "Traverse the knowledge graph from an entity, returning connected entities and edges.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_name": {"type": "string", "description": "Entity name to start from"},
                        "max_depth": {"type": "integer", "description": "Traversal depth (default 2, max 5)"}
                    },
                    "required": ["entity_name"]
                }
            },
            {
                "name": "traverse_batch",
                "description": "Traverse multiple entities in one call. Equivalent to N sequential traverse calls but with a single round-trip. Deduplicates entities and edges across all traversals.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_names": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "List of entity names to traverse from"
                        },
                        "max_depth": {"type": "integer", "description": "Traversal depth per entity (default 2, max 5)"}
                    },
                    "required": ["entity_names"]
                }
            },
            {
                "name": "find_precedents",
                "description": "Find past decisions or context most semantically similar to a given situation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "context": {"type": "string", "description": "Description of current situation"},
                        "limit": {"type": "integer", "description": "Max results (default 5)"}
                    },
                    "required": ["context"]
                }
            },
            {
                "name": "list_entities",
                "description": "List entities in the graph with optional type filter and pagination. Useful for graph visualization and exploration.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_type": {"type": "string", "description": "Filter by entity type (e.g. 'person', 'technology', 'component')"},
                        "limit": {"type": "integer", "description": "Max results (default 100)"},
                        "offset": {"type": "integer", "description": "Skip first N results for pagination (default 0)"}
                    }
                }
            },
            {
                "name": "export_graph",
                "description": "Export all entities and edges from the graph. Optionally include episodes and filter by entity type.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "entity_type": {"type": "string", "description": "Filter entities by type"},
                        "include_episodes": {"type": "boolean", "description": "Include episodes in export (default false)"},
                        "limit": {"type": "integer", "description": "Max entities to export (default 10000)"}
                    }
                }
            },
            {
                "name": "stats",
                "description": "Returns memory health statistics: episode/entity/edge counts by type, decayed counts, and database size. Use this to monitor memory lifecycle health.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "learn",
                "description": "Run the learning pipeline: extract patterns from recent experiences and create or update skills.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "agent": {"type": "string", "description": "Agent name (default: 'assistant')"},
                        "scope": {"type": "string", "description": "'private' (default) or 'shared'"},
                        "limit": {"type": "integer", "description": "Max patterns to extract (default: 50)"}
                    }
                }
            },
            {
                "name": "forget",
                "description": "Manually expire memories by ID or by type. By ID: soft expire (mark for cleanup, default) or hard delete. By type: bulk soft expire or hard delete of all memories of that type (fact, experience, preference, decision). Pattern type is rejected.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "string", "description": "Memory ID to expire (mutually exclusive with 'type')"},
                        "type": {"type": "string", "description": "Memory type to bulk expire: fact, experience, preference, decision (mutually exclusive with 'id')"},
                        "hard": {"type": "boolean", "description": "If true, immediately delete. If false (default), mark for cleanup.", "default": false}
                    }
                }
            }
        ]
    })
}

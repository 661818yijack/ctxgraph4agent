use std::sync::{Arc, Mutex};

use ctxgraph::{Episode, Graph};
use ctxgraph_embed::EmbedEngine;
use serde_json::{Value, json};

pub struct ToolContext {
    pub graph: Arc<Mutex<Graph>>,
    pub embed: Arc<EmbedEngine>,
}

impl ToolContext {
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
            let graph = self.graph.lock().map_err(|e| e.to_string())?;
            graph.add_episode(episode).map_err(|e| e.to_string())?
        };

        // Compute and store embedding
        let embedding = self.embed.embed(&text).map_err(|e| e.to_string())?;
        {
            let graph = self.graph.lock().map_err(|e| e.to_string())?;
            graph
                .store_embedding(&episode_id, &embedding)
                .map_err(|e| e.to_string())?;
        }

        Ok(json!({
            "episode_id": result.episode_id,
            "entities_found": result.entities_extracted,
            "edges_created": result.edges_created,
        }))
    }

    /// Tool: search
    /// Fused FTS5 + semantic search via RRF.
    pub async fn search(&self, args: Value) -> Result<Value, String> {
        let query = args["query"]
            .as_str()
            .ok_or("missing required field: query")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;

        // Compute query embedding
        let query_embedding = self.embed.embed(&query).map_err(|e| e.to_string())?;

        let results = {
            let graph = self.graph.lock().map_err(|e| e.to_string())?;
            graph
                .search_fused(&query, &query_embedding, limit)
                .map_err(|e| e.to_string())?
        };

        let items: Vec<Value> = results
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
            .collect();

        Ok(json!(items))
    }

    /// Tool: get_decision
    /// Retrieve a specific episode by ID with full context.
    pub async fn get_decision(&self, args: Value) -> Result<Value, String> {
        let id = args["id"]
            .as_str()
            .ok_or("missing required field: id")?
            .to_string();

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

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

        let graph = self.graph.lock().map_err(|e| e.to_string())?;

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
    pub async fn find_precedents(&self, args: Value) -> Result<Value, String> {
        let context = args["context"]
            .as_str()
            .ok_or("missing required field: context")?
            .to_string();
        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

        // Embed the context
        let context_embedding = self.embed.embed(&context).map_err(|e| e.to_string())?;

        // Load all episode embeddings
        let all_embeddings = {
            let graph = self.graph.lock().map_err(|e| e.to_string())?;
            graph.get_embeddings().map_err(|e| e.to_string())?
        };

        // Compute cosine similarity to all
        let mut scored: Vec<(String, f32)> = all_embeddings
            .into_iter()
            .map(|(id, vec)| {
                let sim = EmbedEngine::cosine_similarity(&context_embedding, &vec);
                (id, sim)
            })
            .collect();

        // Sort by similarity descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top: Vec<(String, f32)> = scored.into_iter().take(limit).collect();

        let mut results = Vec::new();
        let graph = self.graph.lock().map_err(|e| e.to_string())?;
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
            }
        ]
    })
}

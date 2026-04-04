use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::error::{CtxGraphError, Result};
use crate::pattern::PatternExtractor;
use crate::storage::migrations::run_migrations;
use crate::types::*;

pub struct Storage {
    pub(crate) conn: Connection,
}

impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    // ── Episodes ──

    pub fn insert_episode(&self, episode: &Episode) -> Result<()> {
        self.conn.execute(
            "INSERT INTO episodes (id, content, source, recorded_at, metadata, compression_id, memory_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                episode.id,
                episode.content,
                episode.source,
                episode.recorded_at.to_rfc3339(),
                episode.metadata.as_ref().map(|m| m.to_string()),
                episode.compression_id,
                episode.memory_type.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn get_episode(&self, id: &str) -> Result<Option<Episode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, source, recorded_at, metadata, compression_id, memory_type FROM episodes WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![id], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source: row.get(2)?,
                    recorded_at: parse_datetime(&row.get::<_, String>(3)?),
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| parse_metadata(&s)),
                    compression_id: row.get(5)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(6)?),
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, source, recorded_at, metadata, compression_id, memory_type
             FROM episodes ORDER BY recorded_at DESC LIMIT ?1 OFFSET ?2",
        )?;

        let episodes = stmt
            .query_map(params![limit as i64, offset as i64], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source: row.get(2)?,
                    recorded_at: parse_datetime(&row.get::<_, String>(3)?),
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| parse_metadata(&s)),
                    compression_id: row.get(5)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(6)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(episodes)
    }

    // ── Entities ──

    pub fn insert_entity(&self, entity: &Entity) -> Result<()> {
        self.conn.execute(
            "INSERT INTO entities (id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                entity.id,
                entity.name,
                entity.entity_type,
                entity.memory_type.to_string(),
                entity.ttl.map(|d| d.as_secs() as i64),
                entity.summary,
                entity.created_at.to_rfc3339(),
                entity.metadata.as_ref().map(|m| m.to_string()),
                entity.usage_count,
                entity.last_recalled_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at
             FROM entities WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![id], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(3)?),
                    ttl: row.get::<_, Option<i64>>(4)?.and_then(parse_ttl_seconds),
                    summary: row.get(5)?,
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                    metadata: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| parse_metadata(&s)),
                    usage_count: row.get(8)?,
                    last_recalled_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn get_entity_by_name(&self, name: &str) -> Result<Option<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at
             FROM entities WHERE name = ?1",
        )?;

        let result = stmt
            .query_row(params![name], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(3)?),
                    ttl: row.get::<_, Option<i64>>(4)?.and_then(parse_ttl_seconds),
                    summary: row.get(5)?,
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                    metadata: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| parse_metadata(&s)),
                    usage_count: row.get(8)?,
                    last_recalled_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                })
            })
            .optional()?;

        Ok(result)
    }

    pub fn list_entities(&self, entity_type: Option<&str>, limit: usize) -> Result<Vec<Entity>> {
        let (sql, type_param);
        if let Some(et) = entity_type {
            sql = "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at
                   FROM entities WHERE entity_type = ?1 ORDER BY created_at DESC LIMIT ?2";
            type_param = Some(et.to_string());
        } else {
            sql = "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at
                   FROM entities ORDER BY created_at DESC LIMIT ?2";
            type_param = None;
        }

        let mut stmt = self.conn.prepare(sql)?;

        let rows = if let Some(ref tp) = type_param {
            stmt.query_map(params![tp, limit as i64], map_entity_row)?
        } else {
            // For the no-filter case, use a placeholder for ?1 that matches nothing
            // Actually, we need different SQL. Let's handle this properly.
            drop(stmt);
            let mut stmt2 = self.conn.prepare(
                "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at
                 FROM entities ORDER BY created_at DESC LIMIT ?1",
            )?;
            let entities = stmt2
                .query_map(params![limit as i64], map_entity_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            return Ok(entities);
        };

        let entities = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(entities)
    }

    // ── Entity Deduplication ──

    /// Find an entity by exact name and type.
    pub fn get_entity_by_name_and_type(
        &self,
        name: &str,
        entity_type: &str,
    ) -> Result<Option<Entity>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata, usage_count, last_recalled_at
             FROM entities WHERE name = ?1 AND entity_type = ?2",
        )?;

        let result = stmt
            .query_row(params![name, entity_type], map_entity_row)
            .optional()?;

        Ok(result)
    }

    /// Get all entity (id, name) pairs for a given entity type (for fuzzy matching).
    pub fn get_entity_names_by_type(&self, entity_type: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM entities WHERE entity_type = ?1")?;

        let rows = stmt
            .query_map(params![entity_type], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Insert an alias mapping alias_name → canonical_id.
    pub fn add_alias(&self, canonical_id: &str, alias_name: &str, similarity: f64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO aliases (canonical_id, alias_name, similarity)
             VALUES (?1, ?2, ?3)",
            params![canonical_id, alias_name, similarity],
        )?;
        Ok(())
    }

    /// Look up the canonical entity ID for an alias name (case-insensitive).
    pub fn find_by_alias(&self, name: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT canonical_id FROM aliases WHERE alias_name = ?1 COLLATE NOCASE")?;

        let result = stmt
            .query_row(params![name], |row| row.get::<_, String>(0))
            .optional()?;

        Ok(result)
    }

    // ── Edges ──

    pub fn insert_edge(&self, edge: &Edge) -> Result<()> {
        self.conn.execute(
            "INSERT INTO edges (id, source_id, target_id, relation, memory_type, ttl_seconds, fact,
             valid_from, valid_until, recorded_at, confidence, episode_id, metadata, usage_count, last_recalled_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                edge.id,
                edge.source_id,
                edge.target_id,
                edge.relation,
                edge.memory_type.to_string(),
                edge.ttl.map(|d| d.as_secs() as i64),
                edge.fact,
                edge.valid_from.map(|d| d.to_rfc3339()),
                edge.valid_until.map(|d| d.to_rfc3339()),
                edge.recorded_at.to_rfc3339(),
                edge.confidence,
                edge.episode_id,
                edge.metadata.as_ref().map(|m| m.to_string()),
                edge.usage_count,
                edge.last_recalled_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    pub fn get_edge(&self, id: &str) -> Result<Option<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, relation, memory_type, ttl_seconds, fact,
                    valid_from, valid_until, recorded_at, confidence, episode_id, metadata,
                    usage_count, last_recalled_at
             FROM edges WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], map_edge_row).optional()?;

        Ok(result)
    }

    pub fn get_edges_for_entity(&self, entity_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, relation, memory_type, ttl_seconds, fact,
                    valid_from, valid_until, recorded_at, confidence, episode_id, metadata,
                    usage_count, last_recalled_at
             FROM edges WHERE source_id = ?1 OR target_id = ?1
             ORDER BY recorded_at DESC",
        )?;

        let edges = stmt
            .query_map(params![entity_id], map_edge_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(edges)
    }

    pub fn get_current_edges_for_entity(&self, entity_id: &str) -> Result<Vec<Edge>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source_id, target_id, relation, memory_type, ttl_seconds, fact,
                    valid_from, valid_until, recorded_at, confidence, episode_id, metadata,
                    usage_count, last_recalled_at
             FROM edges
             WHERE (source_id = ?1 OR target_id = ?1) AND valid_until IS NULL
             ORDER BY recorded_at DESC",
        )?;

        let edges = stmt
            .query_map(params![entity_id], map_edge_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(edges)
    }

    pub fn invalidate_edge(&self, edge_id: &str, until: DateTime<Utc>) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE edges SET valid_until = ?1 WHERE id = ?2 AND valid_until IS NULL",
            params![until.to_rfc3339(), edge_id],
        )?;

        if changed == 0 {
            return Err(CtxGraphError::NotFound(format!(
                "edge {edge_id} not found or already invalidated"
            )));
        }

        Ok(())
    }

    // ── Contradiction Detection (C1) ──────────────────────────────────────────

    /// Check new edges against existing edges for contradictions.
    ///
    /// A contradiction occurs when a new edge has the same source entity
    /// (by entity_id or normalized entity_name) and same relation type,
    /// but a different target entity or fact value.
    ///
    /// Only contradictions where the existing edge has confidence >=
    /// `contradiction_threshold` are returned. Lower-confidence edges
    /// are silently replaced without being flagged.
    ///
    /// Returns a list of detected contradictions with old/new edge details.
    pub fn check_contradictions(
        &self,
        new_edges: &[Edge],
        contradiction_threshold: f64,
    ) -> Result<Vec<Contradiction>> {
        use crate::types::normalize_entity_name;

        let mut contradictions = Vec::new();

        for new_edge in new_edges {
            // Get the source entity name for fallback matching
            let source_entity_name = self
                .get_entity(&new_edge.source_id)?
                .map(|e| normalize_entity_name(&e.name))
                .unwrap_or_default();

            // Find existing current edges for this source entity by source_id
            // (entity_name fallback handled below when source_id doesn't match)
            let existing_edges = self.get_current_edges_for_entity(&new_edge.source_id)?;

            for existing_edge in existing_edges {
                // Only match edges where the existing edge has the same
                // source as the new edge (same subject), not edges where the entity
                // is the target.
                if existing_edge.source_id != new_edge.source_id {
                    continue;
                }

                // Skip if same relation type is not matched
                if existing_edge.relation != new_edge.relation {
                    continue;
                }

                // Skip if it's the same edge (shouldn't happen but safety check)
                if existing_edge.id == new_edge.id {
                    continue;
                }

                // Compare target_id first (entity identity),
                // then fall back to fact string comparison only if target_ids
                // are the same but fact content differs meaningfully.
                //
                // If both edges have the same target_id, they refer to the same
                // entity and are not contradictory even if the fact string wording
                // differs. Only if target_ids differ do we have a true contradiction.
                if existing_edge.target_id != new_edge.target_id {
                    // Different target entities — this is a contradiction
                    // Check confidence threshold
                    if existing_edge.confidence < contradiction_threshold {
                        // Below threshold: silently invalidate without recording contradiction
                        let now = Utc::now();
                        let _ = self.invalidate_edge_internal(&existing_edge.id, now);
                        continue;
                    }

                    // entity_id is the source entity's id (which is new_edge.source_id)
                    let entity_id = Some(new_edge.source_id.clone());

                    contradictions.push(Contradiction {
                        old_edge_id: existing_edge.id,
                        new_edge_id: new_edge.id.clone(),
                        entity_id,
                        entity_name: source_entity_name.clone(),
                        relation: new_edge.relation.clone(),
                        old_value: existing_edge.target_id.clone(),
                        new_value: new_edge.target_id.clone(),
                        existing_confidence: existing_edge.confidence,
                    });
                    continue;
                }

                // Same target_id — check if fact strings differ meaningfully
                let old_fact = existing_edge.fact.as_deref();
                let new_fact = new_edge.fact.as_deref();

                // If both have no fact string, they're identical (no contradiction)
                if old_fact.is_none() && new_fact.is_none() {
                    continue;
                }

                // If one has fact and other doesn't, or they differ, check threshold
                let fact_differs = old_fact != new_fact;
                if !fact_differs {
                    continue;
                }

                // Fact strings differ — treat as potential contradiction if above threshold
                if existing_edge.confidence < contradiction_threshold {
                    let now = Utc::now();
                    let _ = self.invalidate_edge_internal(&existing_edge.id, now);
                    continue;
                }

                let entity_id = Some(new_edge.source_id.clone());
                let old_value = old_fact.unwrap_or(&existing_edge.target_id).to_string();
                let new_value = new_fact.unwrap_or(&new_edge.target_id).to_string();

                contradictions.push(Contradiction {
                    old_edge_id: existing_edge.id,
                    new_edge_id: new_edge.id.clone(),
                    entity_id,
                    entity_name: source_entity_name.clone(),
                    relation: new_edge.relation.clone(),
                    old_value,
                    new_value,
                    existing_confidence: existing_edge.confidence,
                });
            }
        }

        Ok(contradictions)
    }

    /// Invalidate an edge by setting valid_until to now, without checking if already invalidated.
    /// Used internally for silent invalidation (below threshold).
    fn invalidate_edge_internal(&self, edge_id: &str, until: DateTime<Utc>) -> Result<()> {
        self.conn.execute(
            "UPDATE edges SET valid_until = ?1 WHERE id = ?2 AND valid_until IS NULL",
            params![until.to_rfc3339(), edge_id],
        )?;
        Ok(())
    }

    /// Invalidate an edge and update its metadata with the contradicting edge ID.
    ///
    /// Used when a contradiction is detected above the threshold.
    /// Sets `valid_until = now` and adds `contradicted_by` to metadata.
    pub fn invalidate_contradicted(&self, old_edge_id: &str, new_edge_id: &str) -> Result<()> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        // First, get the existing metadata
        let existing_metadata: Option<String> = self
            .conn
            .query_row(
                "SELECT metadata FROM edges WHERE id = ?1",
                params![old_edge_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        // Parse and update metadata
        let mut metadata = existing_metadata
            .and_then(|s| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&s).ok())
            .unwrap_or(serde_json::Map::new());

        metadata.insert(
            "contradicted_by".to_string(),
            serde_json::Value::String(new_edge_id.to_string()),
        );
        metadata.insert(
            "contradicted_at".to_string(),
            serde_json::Value::String(now_str.clone()),
        );

        let metadata_str = serde_json::to_string(&metadata).unwrap_or_default();

        self.conn.execute(
            "UPDATE edges SET valid_until = ?1, metadata = ?2 WHERE id = ?3 AND valid_until IS NULL",
            params![now_str, metadata_str, old_edge_id],
        )?;

        Ok(())
    }

    // ── Episode-Entity links ──

    pub fn link_episode_entity(
        &self,
        episode_id: &str,
        entity_id: &str,
        span_start: Option<usize>,
        span_end: Option<usize>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO episode_entities (episode_id, entity_id, span_start, span_end)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                episode_id,
                entity_id,
                span_start.map(|s| s as i64),
                span_end.map(|s| s as i64),
            ],
        )?;
        Ok(())
    }

    // ── FTS5 Search ──

    pub fn search_episodes(&self, query: &str, limit: usize) -> Result<Vec<(Episode, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.content, e.source, e.recorded_at, e.metadata,
                    e.compression_id, e.memory_type,
                    rank
             FROM episodes_fts fts
             JOIN episodes e ON e.rowid = fts.rowid
             WHERE episodes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                let episode = Episode {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source: row.get(2)?,
                    recorded_at: parse_datetime(&row.get::<_, String>(3)?),
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| parse_metadata(&s)),
                    compression_id: row.get(5)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(6)?),
                };
                let rank: f64 = row.get(7)?;
                Ok((episode, -rank)) // FTS5 rank is negative (lower = better)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    pub fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<(Entity, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.memory_type, e.ttl_seconds, e.summary, e.created_at, e.metadata, e.usage_count, e.last_recalled_at,
                    rank
             FROM entities_fts fts
             JOIN entities e ON e.rowid = fts.rowid
             WHERE entities_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                let entity = Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(3)?),
                    ttl: row.get::<_, Option<i64>>(4)?.and_then(parse_ttl_seconds),
                    summary: row.get(5)?,
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                    metadata: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| parse_metadata(&s)),
                    usage_count: row.get(8)?,
                    last_recalled_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                };
                let rank: f64 = row.get(10)?;
                Ok((entity, -rank))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    // ── Embeddings ──

    /// Store a raw f32 embedding blob for an episode.
    pub fn store_episode_embedding(&self, episode_id: &str, data: &[u8]) -> Result<()> {
        self.conn.execute(
            "UPDATE episodes SET embedding = ?1 WHERE id = ?2",
            params![data, episode_id],
        )?;
        Ok(())
    }

    /// Store a raw f32 embedding blob for an entity.
    pub fn store_entity_embedding(&self, entity_id: &str, data: &[u8]) -> Result<()> {
        self.conn.execute(
            "UPDATE entities SET embedding = ?1 WHERE id = ?2",
            params![data, entity_id],
        )?;
        Ok(())
    }

    /// Load all episode embeddings as (id, raw bytes) pairs.
    pub fn get_all_episode_embeddings(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, embedding FROM episodes WHERE embedding IS NOT NULL")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ── Episode Compression ──

    /// Compress a set of episodes into a single summary episode.
    ///
    /// Creates a new episode with memory_type Fact containing the provided summary,
    /// links all source episodes to the summary via compression_id, and merges
    /// entity links from source episodes to the compressed episode.
    ///
    /// This is a pure persistence method — summary generation is the caller's responsibility
    /// (typically the Graph layer which can call an LLM).
    ///
    /// Returns the ID of the new compressed summary episode.
    pub fn compress_episodes(&self, episode_ids: &[String], summary: &str) -> Result<String> {
        if episode_ids.is_empty() {
            return Err(CtxGraphError::InvalidInput(
                "cannot compress empty episode list".to_string(),
            ));
        }

        // Create the compressed episode with Fact memory_type
        let compressed_episode = Episode {
            id: uuid::Uuid::now_v7().to_string(),
            content: summary.to_string(),
            source: Some("compression".to_string()),
            recorded_at: Utc::now(),
            metadata: Some(serde_json::json!({
                "compressed_count": episode_ids.len(),
            })),
            compression_id: None,
            memory_type: MemoryType::Fact,
        };

        let compressed_id = compressed_episode.id.clone();
        self.insert_episode(&compressed_episode)?;

        // Set compression_id on all source episodes
        for ep_id in episode_ids {
            self.conn.execute(
                "UPDATE episodes SET compression_id = ?1 WHERE id = ?2",
                params![compressed_id, ep_id],
            )?;
        }

        // Merge entity links from source episodes to compressed episode
        // Collect all unique entity_ids linked to source episodes
        let placeholders: Vec<String> = (1..=episode_ids.len()).map(|i| format!("?{i}")).collect();
        let in_clause = placeholders.join(", ");

        let sql = format!(
            "SELECT DISTINCT entity_id FROM episode_entities WHERE episode_id IN ({in_clause})"
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let entity_ids: Vec<String> = stmt
            .query_map(rusqlite::params_from_iter(episode_ids.iter()), |row| {
                row.get::<_, String>(0)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Link compressed episode to all merged entities
        for entity_id in &entity_ids {
            self.conn.execute(
                "INSERT OR IGNORE INTO episode_entities (episode_id, entity_id) VALUES (?1, ?2)",
                params![compressed_id, entity_id],
            )?;
        }

        Ok(compressed_id)
    }

    /// List episodes that have not been compressed, recorded before the given date.
    ///
    /// Used to find candidates for compression — old episodes without a compression_id
    /// that are not themselves compressed summaries (source = 'compression').
    pub fn list_uncompressed_episodes(&self, before: DateTime<Utc>) -> Result<Vec<Episode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, source, recorded_at, metadata, compression_id, memory_type
             FROM episodes
             WHERE compression_id IS NULL AND source IS NOT 'compression' AND recorded_at < ?1
             ORDER BY recorded_at ASC",
        )?;

        let episodes = stmt
            .query_map(params![before.to_rfc3339()], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source: row.get(2)?,
                    recorded_at: parse_datetime(&row.get::<_, String>(3)?),
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| parse_metadata(&s)),
                    compression_id: row.get(5)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(6)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(episodes)
    }

    // ── Stats ──

    pub fn stats(&self) -> Result<GraphStats> {
        let episode_count: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))?;
        let entity_count: usize =
            self.conn
                .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))?;
        let edge_count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;

        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(source, 'unknown'), COUNT(*)
             FROM episodes GROUP BY source ORDER BY COUNT(*) DESC",
        )?;
        let sources = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let db_size_bytes: u64 = self.conn.query_row(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )?;

        Ok(GraphStats {
            episode_count,
            entity_count,
            edge_count,
            sources,
            db_size_bytes,
        })
    }

    // ── Touch (Usage Tracking) ──

    /// Increment usage_count and set last_recalled_at for an entity.
    pub fn touch_entity(&self, id: &str) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE entities SET usage_count = usage_count + 1, last_recalled_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;

        if changed == 0 {
            return Err(CtxGraphError::NotFound(format!("entity {id} not found")));
        }
        Ok(())
    }

    /// Increment usage_count and set last_recalled_at for an edge.
    pub fn touch_edge(&self, id: &str) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE edges SET usage_count = usage_count + 1, last_recalled_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;

        if changed == 0 {
            return Err(CtxGraphError::NotFound(format!("edge {id} not found")));
        }
        Ok(())
    }

    // ── Graph Traversal ──

    pub fn traverse(
        &self,
        start_entity_id: &str,
        max_depth: usize,
        current_only: bool,
    ) -> Result<(Vec<Entity>, Vec<Edge>)> {
        let valid_clause = if current_only {
            "AND e.valid_until IS NULL"
        } else {
            ""
        };

        let sql = format!(
            r#"
            WITH RECURSIVE traversal(entity_id, depth) AS (
                SELECT ?1, 0

                UNION

                SELECT
                    CASE WHEN e.source_id = t.entity_id THEN e.target_id
                         ELSE e.source_id END,
                    t.depth + 1
                FROM traversal t
                JOIN edges e ON (e.source_id = t.entity_id OR e.target_id = t.entity_id)
                WHERE t.depth < ?2
                  {valid_clause}
            )
            SELECT DISTINCT ent.id, ent.name, ent.entity_type, ent.memory_type, ent.ttl_seconds, ent.summary,
                            ent.created_at, ent.metadata, ent.usage_count, ent.last_recalled_at
            FROM traversal t
            JOIN entities ent ON ent.id = t.entity_id
            ORDER BY t.depth
            "#
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let entities = stmt
            .query_map(params![start_entity_id, max_depth as i64], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(3)?),
                    ttl: row.get::<_, Option<i64>>(4)?.and_then(parse_ttl_seconds),
                    summary: row.get(5)?,
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                    metadata: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| parse_metadata(&s)),
                    usage_count: row.get(8)?,
                    last_recalled_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Collect all edges between traversed entities
        let entity_ids: Vec<String> = entities.iter().map(|e| e.id.clone()).collect();
        let edges = self.get_edges_between(&entity_ids, current_only)?;

        Ok((entities, edges))
    }

    fn get_edges_between(&self, entity_ids: &[String], current_only: bool) -> Result<Vec<Edge>> {
        if entity_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = (1..=entity_ids.len()).map(|i| format!("?{i}")).collect();
        let in_clause = placeholders.join(", ");
        let valid_clause = if current_only {
            "AND valid_until IS NULL"
        } else {
            ""
        };

        let sql = format!(
            "SELECT id, source_id, target_id, relation, memory_type, ttl_seconds, fact,
                    valid_from, valid_until, recorded_at, confidence, episode_id, metadata,
                    usage_count, last_recalled_at
             FROM edges
             WHERE source_id IN ({in_clause}) AND target_id IN ({in_clause})
             {valid_clause}
             ORDER BY recorded_at DESC"
        );

        let mut stmt = self.conn.prepare(&sql)?;

        // Bind entity_ids twice (once for source_id IN, once for target_id IN)
        let mut all_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        for id in entity_ids {
            all_params.push(Box::new(id.clone()));
        }
        for id in entity_ids {
            all_params.push(Box::new(id.clone()));
        }

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            all_params.iter().map(|p| p.as_ref()).collect();

        let edges = stmt
            .query_map(&*param_refs, map_edge_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(edges)
    }

    // ── Pattern Extraction (D1a) ─────────────────────────────────────────────

    /// Get all compression groups with their associated entities and edges.
    ///
    /// A compression group consists of:
    /// - One compressed (summary) episode (`compression_id IS NULL`)
    /// - All source episodes that were compressed into it (`compression_id = summary_id`)
    /// - Entities linked to the source episodes
    /// - Edges from the source episodes
    ///
    /// This method is used by D1a (co-occurrence counting) to extract pattern candidates.
    pub fn get_compression_groups(
        &self,
        before: DateTime<Utc>,
    ) -> Result<Vec<CompressionGroupData>> {
        // Find all compressed summary episodes (source = 'compression') recorded before `before`
        let mut stmt = self.conn.prepare(
            "SELECT id, content, source, recorded_at, metadata, compression_id, memory_type
             FROM episodes WHERE source = 'compression' AND recorded_at < ?1",
        )?;

        let comp_episodes: Vec<Episode> = stmt
            .query_map(params![before.to_rfc3339()], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source: row.get(2)?,
                    recorded_at: parse_datetime(&row.get::<_, String>(3)?),
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| parse_metadata(&s)),
                    compression_id: row.get(5)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(6)?),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut groups = Vec::new();
        for comp_ep in comp_episodes {
            // Get source episode IDs for this compression group
            // Source episodes have compression_id = comp_ep.id
            let mut src_stmt = self
                .conn
                .prepare("SELECT id FROM episodes WHERE compression_id = ?1")?;
            let source_ids: Vec<String> = src_stmt
                .query_map(params![comp_ep.id.clone()], |row| row.get(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Get entities linked to source episodes
            let mut entity_stmt = self.conn.prepare(
                "SELECT DISTINCT e.id, e.name, e.entity_type, e.memory_type, e.ttl_seconds,
                        e.summary, e.created_at, e.metadata, e.usage_count, e.last_recalled_at
                 FROM entities e
                 JOIN episode_entities ee ON e.id = ee.entity_id
                 WHERE ee.episode_id IN (SELECT id FROM episodes WHERE compression_id = ?1)",
            )?;
            let entities: Vec<Entity> = entity_stmt
                .query_map(params![comp_ep.id.clone()], map_entity_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            // Get edges from source episodes (episode_id matches source episode IDs)
            let mut edge_stmt = self.conn.prepare(
                "SELECT e.id, e.source_id, e.target_id, e.relation, e.memory_type, e.ttl_seconds,
                        e.fact, e.valid_from, e.valid_until, e.recorded_at, e.confidence,
                        e.episode_id, e.metadata, e.usage_count, e.last_recalled_at
                 FROM edges e
                 WHERE e.episode_id IN (SELECT id FROM episodes WHERE compression_id = ?1)
                 AND e.valid_until IS NULL",
            )?;
            let edges: Vec<Edge> = edge_stmt
                .query_map(params![comp_ep.id.clone()], map_edge_row)?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            groups.push(CompressionGroupData {
                compression_id: comp_ep.id,
                source_episode_ids: source_ids,
                entities,
                edges,
            });
        }

        Ok(groups)
    }

    /// Extract pattern candidates from all compression groups using co-occurrence counting.
    ///
    /// Loads all compression groups via `get_compression_groups` and runs the pure-logic
    /// `PatternExtractor` to produce ranked candidates filtered by the given config.
    pub fn get_pattern_candidates(
        &self,
        config: &PatternExtractorConfig,
    ) -> Result<Vec<PatternCandidate>> {
        let before = Utc::now();
        let groups = self.get_compression_groups(before)?;
        let extractor = PatternExtractor::new();
        Ok(extractor.extract(&groups, config))
    }

    /// Store a pattern candidate as a LearnedPattern entity.
    ///
    /// The pattern is stored with:
    /// - `entity_type = "LearnedPattern"`
    /// - `memory_type = Pattern`
    /// - `ttl = None` (never expires)
    /// - `name` = first 80 chars of description (truncated at word boundary)
    /// - `summary` = the behavioral description from D1b
    ///
    /// Returns the entity ID of the stored pattern.
    pub fn store_pattern(&self, candidate: &PatternCandidate) -> Result<String> {
        let id = candidate.id.clone();

        // Truncate description at word boundary for entity name (max 80 chars)
        let name = candidate
            .description
            .as_ref()
            .map(|d| truncate_at_word_boundary(d, 80))
            .unwrap_or_else(|| format!("Pattern {}", &id[..8]));

        // Create the pattern entity with Pattern memory type (never expires)
        let mut entity = Entity::with_memory(&name, "LearnedPattern", MemoryType::Pattern, None);
        entity.id = id.clone();
        // Store the behavioral description in the summary field
        entity.summary = candidate.description.clone();

        self.insert_entity(&entity)?;
        Ok(id)
    }

    // ── Skills (D2 + D3) ──────────────────────────────────────────────────

    /// Create a new skill in the database.
    ///
    /// Stores the skill with all fields, serializing `entity_types` and
    /// `provenance` as JSON strings.
    pub fn create_skill(&self, skill: &Skill) -> Result<()> {
        let entity_types_json = serde_json::to_string(&skill.entity_types)?;
        let provenance_json = match skill.provenance.as_ref() {
            Some(p) => Some(serde_json::to_string(p)?),
            None => None,
        };

        self.conn.execute(
            "INSERT INTO skills (id, name, description, trigger_condition, action,
             success_count, failure_count, confidence, superseded_by, created_at,
             entity_types, provenance, scope, created_by_agent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                skill.id,
                skill.name,
                skill.description,
                skill.trigger_condition,
                skill.action,
                skill.success_count,
                skill.failure_count,
                skill.confidence,
                skill.superseded_by,
                skill.created_at.to_rfc3339(),
                entity_types_json,
                provenance_json,
                skill.scope.to_string(),
                skill.created_by_agent,
            ],
        )?;
        Ok(())
    }

    /// List all active (non-superseded) skills.
    ///
    /// Returns skills where `superseded_by IS NULL`, ordered by `created_at DESC`.
    pub fn list_skills(&self) -> Result<Vec<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, trigger_condition, action,
                    success_count, failure_count, confidence, superseded_by,
                    created_at, entity_types, provenance, scope, created_by_agent
             FROM skills
             WHERE superseded_by IS NULL
             ORDER BY created_at DESC",
        )?;

        let skills = stmt
            .query_map([], map_skill_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(skills)
    }

    /// List all skills including superseded ones.
    pub fn list_all_skills(&self) -> Result<Vec<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, trigger_condition, action,
                    success_count, failure_count, confidence, superseded_by,
                    created_at, entity_types, provenance, scope, created_by_agent
             FROM skills
             ORDER BY created_at DESC",
        )?;

        let skills = stmt
            .query_map([], map_skill_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(skills)
    }

    /// Get a single skill by ID.
    pub fn get_skill(&self, id: &str) -> Result<Option<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, trigger_condition, action,
                    success_count, failure_count, confidence, superseded_by,
                    created_at, entity_types, provenance, scope, created_by_agent
             FROM skills WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], map_skill_row).optional()?;
        Ok(result)
    }

    /// Supersede a skill by setting its `superseded_by` field.
    ///
    /// The old skill is kept for audit but excluded from retrieval via `list_skills`.
    pub fn supersede_skill(&self, skill_id: &str, superseded_by: &str) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE skills SET superseded_by = ?1 WHERE id = ?2 AND superseded_by IS NULL",
            params![superseded_by, skill_id],
        )?;

        if changed == 0 {
            return Err(CtxGraphError::NotFound(format!(
                "skill {skill_id} not found or already superseded"
            )));
        }

        Ok(())
    }

    /// Change a skill's scope from Private to Shared (D3).
    ///
    /// This is a one-way operation — skills cannot be un-shared.
    pub fn share_skill(&self, skill_id: &str) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE skills SET scope = 'shared' WHERE id = ?1 AND scope = 'private'",
            params![skill_id],
        )?;

        if changed == 0 {
            return Err(CtxGraphError::NotFound(format!(
                "skill {skill_id} not found or already shared"
            )));
        }

        Ok(())
    }

    /// Get skills for a specific agent (D3).
    ///
    /// Returns shared skills (visible to all agents) plus private skills
    /// owned by the specified agent. Superseded skills are excluded.
    pub fn get_skills_for_agent(&self, agent: &str) -> Result<Vec<Skill>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, trigger_condition, action,
                    success_count, failure_count, confidence, superseded_by,
                    created_at, entity_types, provenance, scope, created_by_agent
             FROM skills
             WHERE superseded_by IS NULL
               AND (scope = 'shared' OR created_by_agent = ?1)
             ORDER BY created_at DESC",
        )?;

        let skills = stmt
            .query_map(params![agent], map_skill_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(skills)
    }

    /// Search skills via FTS5 full-text search.
    ///
    /// Searches both name and description fields. Returns skills ordered
    /// by FTS5 relevance (rank). Only active (non-superseded) skills are returned.
    pub fn search_skills(&self, query: &str, limit: usize) -> Result<Vec<(Skill, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.description, s.trigger_condition, s.action,
                    s.success_count, s.failure_count, s.confidence, s.superseded_by,
                    s.created_at, s.entity_types, s.provenance, s.scope, s.created_by_agent,
                    fts.rank
             FROM skills_fts fts
             JOIN skills s ON s.id = fts.skill_id
             WHERE skills_fts MATCH ?1 AND s.superseded_by IS NULL
             ORDER BY fts.rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                let skill = Skill {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    trigger_condition: row.get(3)?,
                    action: row.get(4)?,
                    success_count: row.get(5)?,
                    failure_count: row.get(6)?,
                    confidence: row.get(7)?,
                    superseded_by: row.get(8)?,
                    created_at: parse_datetime(&row.get::<_, String>(9)?),
                    entity_types: parse_json_vec(&row.get::<_, Option<String>>(10)?),
                    provenance: row
                        .get::<_, Option<String>>(11)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    scope: SkillScope::from_db(&row.get::<_, String>(12)?),
                    created_by_agent: row.get(13)?,
                };
                let rank: f64 = row.get(14)?;
                Ok((skill, -rank)) // FTS5 rank is negative (lower = better)
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    // ── Candidate Retrieval (A4a) ─────────────────────────────────────────────

    /// Retrieve deduplicated candidates via FTS5 search + 1-hop graph traversal.
    ///
    /// Combines:
    /// 1. FTS5 BM25 search across entities(name), edges(relation), episodes(content)
    /// 2. 1-hop graph traversal from FTS5-matched entities
    ///
    /// Deduplicates by (entity_id OR edge_id), keeping the higher BM25 score.
    /// Patterns (LearnedPattern entities) are only included if they matched via FTS5,
    /// subject to the `max_patterns_included` cap.
    ///
    /// Returns empty vec (not error) if query yields no results.
    pub fn retrieve_candidates(
        &self,
        query: &str,
        limit: usize,
        max_patterns_included: usize,
    ) -> Result<Vec<RetrievalCandidate>> {
        // Collect all candidates in a map: key = "entity:<id>" or "edge:<id>", value = candidate
        // This enables deduplication by entity_id or edge_id while keeping the higher score.
        use std::collections::HashMap;
        let mut cand_map: HashMap<String, RetrievalCandidate> = HashMap::new();

        // ── FTS5: Entity names ────────────────────────────────────────────────
        let entity_results = self.fts_search_entities(query, 100)?;
        for (entity, score) in entity_results {
            let key = format!("entity:{}", entity.id);
            let candidate = Self::entity_to_candidate(&entity, score);
            cand_map.insert(key, candidate);
        }

        // Collect all FTS5-matched entity IDs for graph traversal
        let fts_entity_ids: Vec<String> = cand_map
            .values()
            .filter(|c| c.entity_id.is_some())
            .map(|c| c.entity_id.clone().unwrap())
            .collect();

        // ── FTS5: Edge relations ─────────────────────────────────────────────
        let edge_results = self.fts_search_edges(query, 100)?;
        for (edge, score) in edge_results {
            let key = format!("edge:{}", edge.id);
            // Only insert if not already present with a higher score
            let entry = cand_map.entry(key.clone());
            match entry {
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(Self::edge_to_candidate(&edge, score));
                }
                std::collections::hash_map::Entry::Occupied(mut e) => {
                    if score > e.get().fts_score {
                        e.insert(Self::edge_to_candidate(&edge, score));
                    }
                }
            }
        }

        // ── FTS5: Episode content ─────────────────────────────────────────────
        let episode_results = self.fts_search_episodes(query, 100)?;
        for (episode, score) in episode_results {
            // Episodes don't have entity_id or edge_id, so they can't be deduplicated
            // against entities/edges. We include them as-is.
            let candidate = Self::episode_to_candidate(&episode, score);
            let key = format!("episode:{}", episode.id);
            cand_map.insert(key, candidate);
        }

        // ── Graph traversal: 1-hop neighbors from FTS5-matched entities ────────
        const DEFAULT_GRAPH_SCORE: f64 = 0.3;
        for entity_id in &fts_entity_ids {
            // Get 1-hop neighbors (entities and edges) for this entity
            let neighbors = self.get_1hop_candidates(entity_id, DEFAULT_GRAPH_SCORE)?;
            for neighbor in neighbors {
                let key = if neighbor.entity_id.is_some() {
                    format!("entity:{}", neighbor.entity_id.as_ref().unwrap())
                } else {
                    format!("edge:{}", neighbor.edge_id.as_ref().unwrap())
                };
                let entry = cand_map.entry(key);
                match entry {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(neighbor);
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        // Keep the one with higher score
                        if neighbor.fts_score > e.get().fts_score {
                            e.insert(neighbor);
                        }
                    }
                }
            }
        }

        // ── Apply max_patterns_included cap ───────────────────────────────────
        let candidates: Vec<RetrievalCandidate> = cand_map.into_values().collect();

        // Separate patterns from other candidates using into_iter so we own the values
        let (patterns, non_patterns): (Vec<RetrievalCandidate>, Vec<RetrievalCandidate>) =
            candidates.into_iter().partition(|c| c.memory_type == MemoryType::Pattern);

        let limited_patterns: Vec<RetrievalCandidate> = if max_patterns_included == 0 {
            Vec::new()
        } else {
            // Take up to max_patterns_included patterns, sorted by score descending
            let mut patterns_sorted = patterns;
            patterns_sorted.sort_by(|a, b| {
                b.fts_score
                    .partial_cmp(&a.fts_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            patterns_sorted
                .into_iter()
                .take(max_patterns_included)
                .collect()
        };

        // Combine: non-patterns + limited patterns
        let mut result: Vec<RetrievalCandidate> = non_patterns;
        result.extend(limited_patterns);

        // Sort by fts_score descending
        result.sort_by(|a, b| {
            b.fts_score
                .partial_cmp(&a.fts_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply overall limit
        result.truncate(limit);
        Ok(result)
    }

    // ── Budget Enforcement (A4c) ─────────────────────────────────────────────

    /// Retrieve memories for context injection, honoring budget constraints.
    ///
    /// Orchestrates the full retrieval pipeline:
    /// 1. A4a: retrieve_candidates — FTS5 + graph traversal
    /// 2. A4b: score and rank candidates (via Graph::rank_candidates logic)
    /// 3. A4c: enforce_budget — greedy selection within token budget
    ///
    /// Uses the provided `budget_tokens` directly rather than looking up
    /// an agent policy (policy lookup is A5).
    ///
    /// Returns `(ranked_memories, tokens_spent)` where:
    /// - `ranked_memories`: selected memories within budget, sorted by score descending
    /// - `tokens_spent`: total token estimate for returned memories
    ///
    /// If `budget_tokens` is 0, returns empty vec.
    pub fn retrieve_for_context(
        &self,
        query: &str,
        agent_name: &str,
        budget_tokens: usize,
    ) -> crate::error::Result<(Vec<crate::types::RankedMemory>, usize)> {
        // A4a: Retrieve candidates (use default limit of 100, max_patterns 50)
        let candidates = self.retrieve_candidates(query, 100, 50)?;

        if candidates.is_empty() {
            return Ok((Vec::new(), 0));
        }

        // A4b: Score and rank candidates
        // Inline the ranking logic since rank_candidates is on Graph,
        // but we don't have a Graph reference here. This is the same logic
        // as Graph::rank_candidates but without the Graph dependency.
        use crate::types::{score_candidate, ScoredCandidate};

        let mut scored: Vec<ScoredCandidate> = candidates
            .into_iter()
            .map(|c| {
                let composite_score = score_candidate(&c);
                ScoredCandidate {
                    candidate: c,
                    composite_score,
                }
            })
            .filter(|sc| sc.composite_score > 0.0)
            .collect();

        scored.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // A4c: Enforce budget
        let (ranked, tokens_spent) = crate::types::enforce_budget(scored, budget_tokens);

        Ok((ranked, tokens_spent))
    }

    /// FTS5 search over entities_fts (name, entity_type, summary).
    fn fts_search_entities(&self, query: &str, limit: usize) -> Result<Vec<(Entity, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.name, e.entity_type, e.memory_type, e.ttl_seconds, e.summary,
                    e.created_at, e.metadata, e.usage_count, e.last_recalled_at,
                    rank
             FROM entities_fts fts
             JOIN entities e ON e.rowid = fts.rowid
             WHERE entities_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                let entity = Entity {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    entity_type: row.get(2)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(3)?),
                    ttl: row.get::<_, Option<i64>>(4)?.and_then(parse_ttl_seconds),
                    summary: row.get(5)?,
                    created_at: parse_datetime(&row.get::<_, String>(6)?),
                    metadata: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| parse_metadata(&s)),
                    usage_count: row.get(8)?,
                    last_recalled_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
                };
                let rank: f64 = row.get(10)?;
                Ok((entity, -rank))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// FTS5 search over edges_fts (fact, relation).
    fn fts_search_edges(&self, query: &str, limit: usize) -> Result<Vec<(Edge, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.source_id, e.target_id, e.relation, e.memory_type, e.ttl_seconds,
                    e.fact, e.valid_from, e.valid_until, e.recorded_at, e.confidence,
                    e.episode_id, e.metadata, e.usage_count, e.last_recalled_at,
                    rank
             FROM edges_fts fts
             JOIN edges e ON e.rowid = fts.rowid
             WHERE edges_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                let edge = Edge {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    target_id: row.get(2)?,
                    relation: row.get(3)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(4)?),
                    ttl: row.get::<_, Option<i64>>(5)?.and_then(parse_ttl_seconds),
                    fact: row.get(6)?,
                    valid_from: row.get::<_, Option<String>>(7)?.map(|s| parse_datetime(&s)),
                    valid_until: row.get::<_, Option<String>>(8)?.map(|s| parse_datetime(&s)),
                    recorded_at: parse_datetime(&row.get::<_, String>(9)?),
                    confidence: row.get(10)?,
                    episode_id: row.get(11)?,
                    metadata: row
                        .get::<_, Option<String>>(12)?
                        .and_then(|s| parse_metadata(&s)),
                    usage_count: row.get(13)?,
                    last_recalled_at: row.get::<_, Option<String>>(14)?.map(|s| parse_datetime(&s)),
                };
                let rank: f64 = row.get(15)?;
                Ok((edge, -rank))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// FTS5 search over episodes_fts (content, source, metadata).
    fn fts_search_episodes(&self, query: &str, limit: usize) -> Result<Vec<(Episode, f64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT e.id, e.content, e.source, e.recorded_at, e.metadata,
                    e.compression_id, e.memory_type,
                    rank
             FROM episodes_fts fts
             JOIN episodes e ON e.rowid = fts.rowid
             WHERE episodes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit as i64], |row| {
                let episode = Episode {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    source: row.get(2)?,
                    recorded_at: parse_datetime(&row.get::<_, String>(3)?),
                    metadata: row
                        .get::<_, Option<String>>(4)?
                        .and_then(|s| parse_metadata(&s)),
                    compression_id: row.get(5)?,
                    memory_type: MemoryType::from_db(&row.get::<_, String>(6)?),
                };
                let rank: f64 = row.get(7)?;
                Ok((episode, -rank))
            })?
            .collect::<std::result::Result<Vec<_>, _> >()?;

        Ok(results)
    }

    /// Get 1-hop neighbors (entities and edges) for a given entity.
    /// Used by retrieve_candidates for graph traversal.
    fn get_1hop_candidates(
        &self,
        entity_id: &str,
        default_score: f64,
    ) -> Result<Vec<RetrievalCandidate>> {
        let mut candidates = Vec::new();

        // Get 1-hop edges
        let edges = self.get_current_edges_for_entity(entity_id)?;

        // Collect neighbor entity IDs while building edge candidates
        let mut neighbor_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        for edge in &edges {
            candidates.push(Self::edge_to_candidate(edge, default_score));
            if edge.source_id == entity_id {
                neighbor_ids.insert(edge.target_id.clone());
            } else {
                neighbor_ids.insert(edge.source_id.clone());
            }
        }

        for nid in neighbor_ids {
            if let Some(entity) = self.get_entity(&nid)? {
                candidates.push(Self::entity_to_candidate(&entity, default_score));
            }
        }

        Ok(candidates)
    }

    /// Convert an Entity to a RetrievalCandidate.
    fn entity_to_candidate(entity: &Entity, fts_score: f64) -> RetrievalCandidate {
        RetrievalCandidate {
            entity_id: Some(entity.id.clone()),
            edge_id: None,
            content: entity.summary.clone().unwrap_or_else(|| entity.name.clone()),
            fts_score,
            memory_type: entity.memory_type,
            created_at: entity.created_at,
            ttl: entity.ttl,
            base_confidence: 1.0,
            usage_count: entity.usage_count,
        }
    }

    /// Convert an Edge to a RetrievalCandidate.
    fn edge_to_candidate(edge: &Edge, fts_score: f64) -> RetrievalCandidate {
        RetrievalCandidate {
            entity_id: None,
            edge_id: Some(edge.id.clone()),
            content: edge.fact.clone().unwrap_or_else(|| edge.relation.clone()),
            fts_score,
            memory_type: edge.memory_type,
            created_at: edge.recorded_at,
            ttl: edge.ttl,
            base_confidence: edge.confidence,
            usage_count: edge.usage_count,
        }
    }

    /// Convert an Episode to a RetrievalCandidate.
    fn episode_to_candidate(episode: &Episode, fts_score: f64) -> RetrievalCandidate {
        RetrievalCandidate {
            entity_id: None,
            edge_id: None,
            content: episode.content.clone(),
            fts_score,
            memory_type: episode.memory_type,
            created_at: episode.recorded_at,
            ttl: episode.memory_type.default_ttl(),
            base_confidence: 1.0,
            usage_count: 0,
        }
    }

    /// Get all stored patterns (LearnedPattern entities).
    ///
    /// Returns `PatternCandidate` objects with descriptions populated from the
    /// entity `summary` field. Other fields (occurrence_count, source_groups, etc.)
    /// are not tracked on stored patterns and will be zero/empty.
    pub fn get_patterns(&self) -> Result<Vec<PatternCandidate>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, entity_type, memory_type, ttl_seconds, summary, created_at, metadata,
                    usage_count, last_recalled_at
             FROM entities WHERE entity_type = 'LearnedPattern'",
        )?;

        let candidates: Vec<PatternCandidate> = stmt
            .query_map([], |row| {
                let entity = map_entity_row(row)?;
                let summary = row.get::<_, Option<String>>(5)?;
                Ok(PatternCandidate {
                    id: entity.id,
                    entity_types: vec![],
                    entity_pair: None,
                    relation_triplet: None,
                    occurrence_count: 0, // not tracked on stored pattern
                    source_groups: vec![],
                    confidence: 1.0,
                    description: summary,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(candidates)
    }
}

// ── Helper functions ──

fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|e| {
            eprintln!(
                "ctxgraph: warning: failed to parse datetime '{s}': {e}, using now as fallback"
            );
            Utc::now()
        })
}

/// Safely convert a ttl_seconds value from DB (i64) to Duration.
/// Negative values (corrupted data) are treated as None to avoid wrapping to huge durations.
fn parse_ttl_seconds(secs: i64) -> Option<Duration> {
    if secs >= 0 {
        Some(Duration::from_secs(secs as u64))
    } else {
        None
    }
}

/// Parse a JSON metadata string, logging a warning on failure instead of silently dropping data.
fn parse_metadata(s: &str) -> Option<serde_json::Value> {
    match serde_json::from_str(s) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("ctxgraph: warning: failed to parse metadata JSON: {e}");
            None
        }
    }
}

/// Truncate a string at a word boundary to max_len characters.
///
/// If the string is longer than max_len, finds the last space within
/// the first max_len characters and truncates there.
fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let truncated = &s[..max_len];
    // Find the last space in the truncated portion
    if let Some(last_space) = truncated.rfind(' ') {
        s[..last_space].trim().to_string()
    } else {
        truncated.to_string()
    }
}

/// Parse a JSON string into a Vec<String>. Returns empty vec on parse failure.
fn parse_json_vec(s: &Option<String>) -> Vec<String> {
    match s {
        None => Vec::new(),
        Some(json_str) => serde_json::from_str(json_str).unwrap_or_default(),
    }
}

fn map_skill_row(row: &rusqlite::Row) -> rusqlite::Result<Skill> {
    Ok(Skill {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        trigger_condition: row.get(3)?,
        action: row.get(4)?,
        success_count: row.get(5)?,
        failure_count: row.get(6)?,
        confidence: row.get(7)?,
        superseded_by: row.get(8)?,
        created_at: parse_datetime(&row.get::<_, String>(9)?),
        entity_types: parse_json_vec(&row.get::<_, Option<String>>(10)?),
        provenance: row
            .get::<_, Option<String>>(11)?
            .and_then(|s| serde_json::from_str(&s).ok()),
        scope: SkillScope::from_db(&row.get::<_, String>(12)?),
        created_by_agent: row.get(13)?,
    })
}

fn map_entity_row(row: &rusqlite::Row) -> rusqlite::Result<Entity> {
    Ok(Entity {
        id: row.get(0)?,
        name: row.get(1)?,
        entity_type: row.get(2)?,
        memory_type: MemoryType::from_db(&row.get::<_, String>(3)?),
        ttl: row.get::<_, Option<i64>>(4)?.and_then(parse_ttl_seconds),
        summary: row.get(5)?,
        created_at: parse_datetime(&row.get::<_, String>(6)?),
        metadata: row
            .get::<_, Option<String>>(7)?
            .and_then(|s| parse_metadata(&s)),
        usage_count: row.get(8)?,
        last_recalled_at: row.get::<_, Option<String>>(9)?.map(|s| parse_datetime(&s)),
    })
}

fn map_edge_row(row: &rusqlite::Row) -> rusqlite::Result<Edge> {
    Ok(Edge {
        id: row.get(0)?,
        source_id: row.get(1)?,
        target_id: row.get(2)?,
        relation: row.get(3)?,
        memory_type: MemoryType::from_db(&row.get::<_, String>(4)?),
        ttl: row.get::<_, Option<i64>>(5)?.and_then(parse_ttl_seconds),
        fact: row.get(6)?,
        valid_from: row.get::<_, Option<String>>(7)?.map(|s| parse_datetime(&s)),
        valid_until: row.get::<_, Option<String>>(8)?.map(|s| parse_datetime(&s)),
        recorded_at: parse_datetime(&row.get::<_, String>(9)?),
        confidence: row.get(10)?,
        episode_id: row.get(11)?,
        metadata: row
            .get::<_, Option<String>>(12)?
            .and_then(|s| parse_metadata(&s)),
        usage_count: row.get(13)?,
        last_recalled_at: row
            .get::<_, Option<String>>(14)?
            .map(|s| parse_datetime(&s)),
    })
}

/// rusqlite optional helper
trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::error::{CtxGraphError, Result};
use crate::pattern::PatternExtractor;
use crate::storage::migrations::run_migrations;
use crate::types::*;

pub struct Storage {
    conn: Connection,
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

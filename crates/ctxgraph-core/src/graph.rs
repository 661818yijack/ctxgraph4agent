use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::error::{CtxGraphError, Result};
use crate::pattern::{PatternDescriber, PatternExtractor};
use crate::skill::{SkillCreator, SkillSynthesizer};
use crate::storage::Storage;
use crate::types::*;

#[cfg(feature = "extract")]
use ctxgraph_extract::pipeline::ExtractionPipeline;
#[cfg(feature = "extract")]
use ctxgraph_extract::schema::ExtractionSchema;

pub struct Graph {
    pub storage: Storage,
    #[allow(dead_code)]
    db_path: PathBuf,
    #[cfg(feature = "extract")]
    pipeline: Option<ExtractionPipeline>,
}

impl Graph {
    /// Open an existing ctxgraph database.
    pub fn open(db_path: &Path) -> Result<Self> {
        if !db_path.exists() {
            return Err(CtxGraphError::NotFound(format!(
                "database not found at {}. Run `ctxgraph init` first.",
                db_path.display()
            )));
        }
        let storage = Storage::open(db_path)?;
        Ok(Self {
            storage,
            db_path: db_path.to_path_buf(),
            #[cfg(feature = "extract")]
            pipeline: None,
        })
    }

    /// Initialize a new ctxgraph project in the given directory.
    /// Creates `.ctxgraph/` directory with a fresh database.
    pub fn init(dir: &Path) -> Result<Self> {
        let ctxgraph_dir = dir.join(".ctxgraph");
        let db_path = ctxgraph_dir.join("graph.db");

        if db_path.exists() {
            return Err(CtxGraphError::AlreadyExists(format!(
                "ctxgraph already initialized at {}",
                ctxgraph_dir.display()
            )));
        }

        fs::create_dir_all(&ctxgraph_dir)?;

        let storage = Storage::open(&db_path)?;
        Ok(Self {
            storage,
            db_path,
            #[cfg(feature = "extract")]
            pipeline: None,
        })
    }

    /// Open an existing database or create a new one at the given path.
    ///
    /// Unlike `open()`, this method creates the parent directory and database file
    /// if they do not exist, making it suitable for explicit `--db <path>` usage
    /// where the caller controls the full path.
    pub fn open_or_create(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }
        let storage = Storage::open(db_path)?;
        Ok(Self {
            storage,
            db_path: db_path.to_path_buf(),
            #[cfg(feature = "extract")]
            pipeline: None,
        })
    }

    /// Open in-memory database (for testing).
    pub fn in_memory() -> Result<Self> {
        let storage = Storage::open_in_memory()?;
        Ok(Self {
            storage,
            db_path: PathBuf::from(":memory:"),
            #[cfg(feature = "extract")]
            pipeline: None,
        })
    }

    /// Load the extraction pipeline from models in the given directory.
    ///
    /// Once loaded, `add_episode()` will automatically extract entities and relations.
    /// Call this after `open()` or `init()` to enable extraction.
    #[cfg(feature = "extract")]
    pub fn load_extraction_pipeline(&mut self, models_dir: &Path) -> Result<()> {
        let pipeline = ExtractionPipeline::with_defaults(models_dir)
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;
        self.pipeline = Some(pipeline);
        Ok(())
    }

    /// Load the extraction pipeline with a custom schema.
    #[cfg(feature = "extract")]
    pub fn load_extraction_pipeline_with_schema(
        &mut self,
        models_dir: &Path,
        schema: ExtractionSchema,
        confidence_threshold: f64,
    ) -> Result<()> {
        let pipeline = ExtractionPipeline::new(schema, models_dir, confidence_threshold)
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;
        self.pipeline = Some(pipeline);
        Ok(())
    }

    /// Load the extraction pipeline from a ctxgraph.toml config file.
    ///
    /// Reads `[schema]` for entity/relation types and `[llm]` for LLM fallback config.
    #[cfg(feature = "extract")]
    pub fn load_extraction_pipeline_from_config(
        &mut self,
        models_dir: &Path,
        config_path: &Path,
    ) -> Result<()> {
        let config_content = std::fs::read_to_string(config_path).map_err(|e| {
            CtxGraphError::Extraction(format!("failed to read {}: {e}", config_path.display()))
        })?;

        // Parse the full TOML
        let toml_value: toml::Value = toml::from_str(&config_content)
            .map_err(|e| CtxGraphError::Extraction(format!("TOML parse error: {e}")))?;

        // Load schema section (or use defaults)
        let schema = if toml_value.get("schema").is_some() {
            ExtractionSchema::from_toml(&config_content)
                .map_err(|e| CtxGraphError::Extraction(e.to_string()))?
        } else {
            ExtractionSchema::default()
        };

        // Load LLM config section
        let llm_config: ctxgraph_extract::llm_extract::LlmConfig =
            if let Some(llm_table) = toml_value.get("llm") {
                llm_table.clone().try_into().unwrap_or_default()
            } else {
                Default::default()
            };

        let confidence = toml_value
            .get("extraction")
            .and_then(|e| e.get("confidence_threshold"))
            .and_then(|v| v.as_float())
            .unwrap_or(0.5);

        let pipeline =
            ExtractionPipeline::with_llm_config(schema, models_dir, confidence, &llm_config)
                .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        self.pipeline = Some(pipeline);
        Ok(())
    }

    /// Check if the extraction pipeline is loaded.
    #[cfg(feature = "extract")]
    pub fn has_extraction_pipeline(&self) -> bool {
        self.pipeline.is_some()
    }

    // ── Core Operations ──

    /// Add an episode to the graph. Returns the episode ID and extraction results.
    ///
    /// If an extraction pipeline is loaded, entities and relations are automatically
    /// extracted from the episode content and stored in the graph.
    pub fn add_episode(&self, episode: Episode) -> Result<EpisodeResult> {
        self.storage.insert_episode(&episode)?;

        #[cfg(feature = "extract")]
        if let Some(ref pipeline) = self.pipeline {
            return self.add_episode_with_extraction(&episode, pipeline);
        }

        Ok(EpisodeResult {
            episode_id: episode.id,
            entities_extracted: 0,
            edges_created: 0,
            contradictions_found: 0,
        })
    }

    /// Internal: extract entities/relations and store them.
    #[cfg(feature = "extract")]
    fn add_episode_with_extraction(
        &self,
        episode: &Episode,
        pipeline: &ExtractionPipeline,
    ) -> Result<EpisodeResult> {
        let result = pipeline
            .extract(&episode.content, episode.recorded_at)
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let mut entities_extracted = 0;
        let mut edges_created = 0;
        let mut contradictions_found = 0;

        // Map extracted entity text → entity ID for edge creation
        let mut entity_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        // Step 1: Create or reuse entities (with fuzzy dedup at 0.85 threshold)
        for extracted in &result.entities {
            let entity = Entity::new(&extracted.text, &extracted.entity_type);
            let entity_id = match self.add_entity_deduped(entity, 0.85)? {
                (id, false) => {
                    entities_extracted += 1;
                    id
                }
                (id, true) => id, // merged into existing entity
            };

            entity_id_map.insert(extracted.text.clone(), entity_id.clone());

            // Link episode ↔ entity
            let _ = self.storage.link_episode_entity(
                &episode.id,
                &entity_id,
                Some(extracted.span_start),
                Some(extracted.span_end),
            );
        }

        // Step 2: Create edges from relations
        let mut new_edges: Vec<Edge> = Vec::new();
        for rel in &result.relations {
            let source_id = match entity_id_map.get(&rel.head) {
                Some(id) => id,
                None => continue, // head entity not found
            };
            let target_id = match entity_id_map.get(&rel.tail) {
                Some(id) => id,
                None => continue, // tail entity not found
            };

            let mut edge = Edge::new(source_id, target_id, &rel.relation);
            edge.confidence = rel.confidence;
            edge.episode_id = Some(episode.id.clone());
            edge.fact = Some(format!("{} {} {}", rel.head, rel.relation, rel.tail));

            self.storage.insert_edge(&edge)?;
            edges_created += 1;
            new_edges.push(edge);
        }

        // Step 3: Check for contradictions with existing edges (C1)
        // Use default contradiction threshold of 0.2
        let contradiction_threshold = 0.2;
        let contradictions =
            self.storage
                .check_contradictions(&new_edges, contradiction_threshold)?;

        // Invalidate contradicted edges and count
        for contradiction in &contradictions {
            if let Err(e) = self
                .storage
                .invalidate_contradicted(&contradiction.old_edge_id, &contradiction.new_edge_id)
            {
                eprintln!(
                    "ctxgraph: warning: failed to invalidate contradicted edge {}: {}",
                    contradiction.old_edge_id, e
                );
            }
        }
        contradictions_found = contradictions.len();

        Ok(EpisodeResult {
            episode_id: episode.id.clone(),
            entities_extracted,
            edges_created,
            contradictions_found,
        })
    }

    /// Get an episode by ID.
    pub fn get_episode(&self, id: &str) -> Result<Option<Episode>> {
        self.storage.get_episode(id)
    }

    /// List episodes with pagination.
    pub fn list_episodes(&self, limit: usize, offset: usize) -> Result<Vec<Episode>> {
        self.storage.list_episodes(limit, offset)
    }

    /// Add an entity to the graph.
    pub fn add_entity(&self, entity: Entity) -> Result<()> {
        self.storage.insert_entity(&entity)
    }

    /// Add an entity with fuzzy deduplication against existing entities of the same type.
    ///
    /// If an existing entity with Jaro-Winkler similarity >= threshold exists,
    /// returns that entity's ID and stores the new name as an alias.
    /// Otherwise creates a new entity.
    ///
    /// Returns (entity_id, was_merged: bool).
    pub fn add_entity_deduped(&self, entity: Entity, threshold: f64) -> Result<(String, bool)> {
        // 1. Check alias table first (exact alias match)
        if let Some(canonical_id) = self.storage.find_by_alias(&entity.name)? {
            return Ok((canonical_id, true));
        }

        // 2. Get all existing entities of same type
        let existing = self.storage.get_entity_names_by_type(&entity.entity_type)?;

        // 3. Compute Jaro-Winkler similarity to each
        let name_lower = entity.name.to_lowercase();
        let mut best: Option<(String, f64)> = None;
        for (existing_id, existing_name) in &existing {
            let sim = strsim::jaro_winkler(&name_lower, &existing_name.to_lowercase());
            if sim >= threshold && best.as_ref().is_none_or(|(_, best_sim)| sim > *best_sim) {
                best = Some((existing_id.clone(), sim));
            }
        }

        // 4. If match found: add alias and return existing id
        if let Some((canonical_id, sim)) = best {
            self.storage.add_alias(&canonical_id, &entity.name, sim)?;
            return Ok((canonical_id, true));
        }

        // 5. Otherwise: insert new entity
        let id = entity.id.clone();
        self.storage.insert_entity(&entity)?;
        Ok((id, false))
    }

    /// Get an entity by ID.
    pub fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        self.storage.get_entity(id)
    }

    /// Get an entity by name.
    pub fn get_entity_by_name(&self, name: &str) -> Result<Option<Entity>> {
        self.storage.get_entity_by_name(name)
    }

    /// List entities, optionally filtered by type.
    pub fn list_entities(&self, entity_type: Option<&str>, limit: usize) -> Result<Vec<Entity>> {
        self.storage.list_entities(entity_type, limit)
    }

    /// Add an edge between two entities.
    pub fn add_edge(&self, edge: Edge) -> Result<()> {
        self.storage.insert_edge(&edge)
    }

    /// Get all edges for an entity (both as source and target).
    pub fn get_edges_for_entity(&self, entity_id: &str) -> Result<Vec<Edge>> {
        self.storage.get_edges_for_entity(entity_id)
    }

    /// Invalidate an edge (set valid_until to now).
    pub fn invalidate_edge(&self, edge_id: &str) -> Result<()> {
        self.storage.invalidate_edge(edge_id, chrono::Utc::now())
    }

    /// Increment usage_count and set last_recalled_at for an entity.
    ///
    /// Called automatically when an entity is consumed in retrieval results.
    /// Used by A4b (scoring bonus) and A6 (cleanup "keep" signal).
    pub fn touch_entity(&self, id: &str) -> Result<()> {
        self.storage.touch_entity(id)
    }

    /// Increment usage_count and set last_recalled_at for an edge.
    pub fn touch_edge(&self, id: &str) -> Result<()> {
        self.storage.touch_edge(id)
    }

    /// Link an episode to an entity.
    pub fn link_episode_entity(
        &self,
        episode_id: &str,
        entity_id: &str,
        span_start: Option<usize>,
        span_end: Option<usize>,
    ) -> Result<()> {
        self.storage
            .link_episode_entity(episode_id, entity_id, span_start, span_end)
    }

    // ── Embeddings ──

    /// Store an embedding for an episode. The embedding is serialized as
    /// little-endian f32 bytes.
    pub fn store_embedding(&self, episode_id: &str, embedding: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        self.storage.store_episode_embedding(episode_id, &bytes)
    }

    /// Store an embedding for an entity.
    pub fn store_entity_embedding(&self, entity_id: &str, embedding: &[f32]) -> Result<()> {
        let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
        self.storage.store_entity_embedding(entity_id, &bytes)
    }

    /// Load all episode embeddings as (episode_id, Vec<f32>) pairs.
    pub fn get_embeddings(&self) -> Result<Vec<(String, Vec<f32>)>> {
        let raw = self.storage.get_all_episode_embeddings()?;
        let result = raw
            .into_iter()
            .map(|(id, bytes)| {
                let floats: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                    .collect();
                (id, floats)
            })
            .collect();
        Ok(result)
    }

    /// Fused search using Reciprocal Rank Fusion (RRF) over FTS5 + semantic results.
    ///
    /// `query_embedding` should be the pre-computed embedding for `query`.
    /// Returns episodes ranked by combined RRF score.
    pub fn search_fused(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<FusedEpisodeResult>> {
        const K: f64 = 60.0;

        // Accumulate RRF scores per episode id
        let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut episodes_map: std::collections::HashMap<String, Episode> =
            std::collections::HashMap::new();

        // --- FTS5 ranked list ---
        // Fetch a generous pool for RRF (up to 10x limit or 200)
        let fts_pool = (limit * 10).max(200);
        let fts_results = self.storage.search_episodes(query, fts_pool);
        if let Ok(fts) = fts_results {
            for (rank, (episode, _)) in fts.into_iter().enumerate() {
                let rrf = 1.0 / (K + rank as f64 + 1.0);
                *scores.entry(episode.id.clone()).or_insert(0.0) += rrf;
                episodes_map.insert(episode.id.clone(), episode);
            }
        }

        // --- Semantic (cosine similarity) ranked list ---
        let all_embeddings = self.get_embeddings()?;
        if !all_embeddings.is_empty() && !query_embedding.is_empty() {
            // Compute cosine similarities
            let mut semantic: Vec<(String, f32)> = all_embeddings
                .into_iter()
                .map(|(id, vec)| {
                    let sim = cosine_similarity(query_embedding, &vec);
                    (id, sim)
                })
                .collect();
            // Sort descending by similarity
            semantic.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (rank, (ep_id, _sim)) in semantic.into_iter().enumerate() {
                let rrf = 1.0 / (K + rank as f64 + 1.0);
                *scores.entry(ep_id.clone()).or_insert(0.0) += rrf;
                // Fetch episode if not already cached
                if let std::collections::hash_map::Entry::Vacant(e) = episodes_map.entry(ep_id)
                    && let Ok(Some(ep)) = self.storage.get_episode(e.key())
                {
                    e.insert(ep);
                }
            }
        }

        // Sort by total RRF score descending
        let mut fused: Vec<(String, f64)> = scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let results = fused
            .into_iter()
            .take(limit)
            .filter_map(|(id, score)| {
                episodes_map
                    .remove(&id)
                    .map(|episode| FusedEpisodeResult { episode, score })
            })
            .collect();

        Ok(results)
    }

    // ── Search ──

    /// Search episodes via FTS5 full-text search.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(Episode, f64)>> {
        self.storage.search_episodes(query, limit)
    }

    /// Search entities via FTS5.
    pub fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<(Entity, f64)>> {
        self.storage.search_entities(query, limit)
    }

    // ── Candidate Retrieval (A4a) ───────────────────────────────────────────

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
        self.storage
            .retrieve_candidates(query, limit, max_patterns_included)
    }

    // ── Scoring and Ranking (A4b) ───────────────────────────────────────────

    /// Score and rank retrieval candidates by composite score.
    ///
    /// Computes composite scores for each candidate using `score_candidate`,
    /// filters out expired memories (decay_score = 0.0), and returns
    /// candidates sorted by composite_score descending.
    ///
    /// Composite formula: `decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))`
    ///
    /// - Expired memories (decay_score = 0.0) are filtered out
    /// - Patterns get a floor of 0.5 on their composite score
    pub fn rank_candidates(&self, candidates: Vec<RetrievalCandidate>) -> Vec<ScoredCandidate> {
        // Score each candidate and filter out expired ones (score = 0.0)
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

        // Sort by composite_score descending
        scored.sort_by(|a, b| {
            b.composite_score
                .partial_cmp(&a.composite_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored
    }

    // ── Budget Enforcement (A4c) ───────────────────────────────────────────

    /// Retrieve memories for context injection, honoring budget constraints.
    ///
    /// Orchestrates the full retrieval pipeline:
    /// 1. A4a: retrieve_candidates — FTS5 + graph traversal
    /// 2. A4b: rank_candidates — score and sort by composite score
    /// 3. A4c: enforce_budget — greedy selection within token budget
    ///
    /// This is a convenience passthrough to Storage::retrieve_for_context.
    ///
    /// Returns `(ranked_memories, tokens_spent)` where:
    /// - `ranked_memories`: selected memories within budget, sorted by score descending
    /// - `tokens_spent`: total token estimate for returned memories
    ///
    /// Uses the provided `budget_tokens` directly rather than looking up
    /// an agent policy (policy lookup is A5).
    ///
    /// Also triggers lazy cleanup if query_count threshold is reached.
    pub fn retrieve_for_context(
        &self,
        query: &str,
        agent_name: &str,
        budget_tokens: usize,
    ) -> Result<(Vec<RankedMemory>, usize)> {
        // Trigger lazy cleanup check before retrieval
        self.maybe_trigger_cleanup()?;
        self.storage
            .retrieve_for_context(query, agent_name, budget_tokens)
    }

    // ── Traversal ──

    /// Get context around an entity — its neighbors and connecting edges.
    pub fn get_entity_context(&self, entity_id: &str) -> Result<EntityContext> {
        let entity = self
            .storage
            .get_entity(entity_id)?
            .ok_or_else(|| CtxGraphError::NotFound(format!("entity {entity_id}")))?;

        let edges = self.storage.get_current_edges_for_entity(entity_id)?;

        // Collect neighbor IDs
        let mut neighbor_ids: Vec<String> = Vec::new();
        for edge in &edges {
            if edge.source_id == entity_id {
                neighbor_ids.push(edge.target_id.clone());
            } else {
                neighbor_ids.push(edge.source_id.clone());
            }
        }

        let mut neighbors = Vec::new();
        for nid in &neighbor_ids {
            if let Some(n) = self.storage.get_entity(nid)? {
                neighbors.push(n);
            }
        }

        Ok(EntityContext {
            entity,
            edges,
            neighbors,
        })
    }

    /// Multi-hop graph traversal from a starting entity.
    pub fn traverse(
        &self,
        start_entity_id: &str,
        max_depth: usize,
    ) -> Result<(Vec<Entity>, Vec<Edge>)> {
        self.storage.traverse(start_entity_id, max_depth, true)
    }

    // ── Stats ──

    /// Get graph-wide statistics.
    pub fn stats(&self) -> Result<GraphStats> {
        self.storage.stats()
    }

    // ── Cleanup (A6) ─────────────────────────────────────────────────────────

    /// Clean up expired memories based on the agent policy's grace_period.
    ///
    /// Runs the cleanup logic defined in `Storage::cleanup_expired` using
    /// the default grace_period (7 days = 604800 seconds) from `AgentPolicy`.
    ///
    /// This is a direct cleanup call. The lazy trigger in `retrieve_for_context`
    /// automatically runs cleanup every 100 queries if last_cleanup_at > 24h ago.
    pub fn cleanup(&self) -> Result<CleanupResult> {
        let grace_period = AgentPolicy::default().grace_period_secs;
        self.storage.cleanup_expired(grace_period)
    }

    /// Clean up expired memories with a custom grace_period.
    ///
    /// Use this to override the default grace period from AgentPolicy.
    pub fn cleanup_with_grace_period(&self, grace_period_secs: u64) -> Result<CleanupResult> {
        self.storage.cleanup_expired(grace_period_secs)
    }

    /// Get stale memories with decay_score below the given threshold.
    ///
    /// Used by the reverify CLI to list memories needing attention.
    /// - decay_score > 0.7 → Keep
    /// - decay_score 0.3-0.7 → Update
    /// - decay_score < 0.3 → Expire
    pub fn get_stale_memories(
        &self,
        threshold: f64,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<StaleMemory>> {
        self.storage
            .get_stale_memories(threshold, limit, offset)
    }

    /// Renew a memory by resetting its TTL to the default for its memory_type.
    ///
    /// Used by the reverify CLI to extend a memory's life without re-verification.
    /// Returns true if found and updated, false if not found.
    pub fn renew_memory(&self, id: &str, memory_type: MemoryType) -> Result<bool> {
        self.storage.renew_memory_bypass(id, memory_type)
    }

    /// Update a memory's content and/or memory_type.
    ///
    /// If memory_type is changed, the TTL is reset to the new type's default.
    /// Returns Ok(true) if found and updated, Ok(false) if not found.
    pub fn update_memory(
        &self,
        id: &str,
        content: Option<&str>,
        memory_type: Option<MemoryType>,
    ) -> Result<bool> {
        self.storage.update_memory(id, content, memory_type)
    }

    /// Immediately expire (delete) a memory by ID.
    ///
    /// Handles "not found" gracefully (no error returned).
    pub fn expire_memory(&self, id: &str) -> Result<()> {
        self.storage.expire_memory(id)
    }

    /// Lazy cleanup trigger — runs cleanup if:
    /// - query_count is a multiple of 100 (every 100 queries)
    /// - last_cleanup_at is more than 24 hours ago
    ///
    /// Uses the agent policy's grace_period (default 7 days).
    fn maybe_trigger_cleanup(&self) -> Result<()> {
        let count = self.storage.query_count();
        if count % 100 == 0 {
            // Check last_cleanup_at
            if let Some(last_cleanup) = self.storage.get_system_metadata("last_cleanup_at")? {
                if let Ok(last_dt) = DateTime::parse_from_rfc3339(&last_cleanup) {
                    let elapsed = Utc::now()
                        .signed_duration_since(last_dt.with_timezone(&Utc));
                    if elapsed.num_hours() > 24 {
                        let grace = AgentPolicy::default().grace_period_secs;
                        let _ = self.storage.cleanup_expired(grace);
                    }
                }
            } else {
                // Never cleaned — run now
                let grace = AgentPolicy::default().grace_period_secs;
                let _ = self.storage.cleanup_expired(grace);
            }
        }
        Ok(())
    }

    // ── Episode Compression ──

    /// Generate a compression summary from source episodes.
    ///
    /// Concatenates episode contents (truncated to budget) and returns a placeholder summary.
    /// TODO: Replace with actual LLM call when LLM client is available in Graph layer.
    pub fn generate_compression_summary(&self, episodes: &[Episode]) -> Result<String> {
        if episodes.is_empty() {
            return Err(CtxGraphError::InvalidInput(
                "cannot compress empty episode list".to_string(),
            ));
        }

        // Truncate each content to ~200 chars for the budget (~2000 chars total for 10 episodes)
        let budget_per_episode = 200;
        let truncated: Vec<&str> = episodes
            .iter()
            .map(|ep| {
                if ep.content.len() > budget_per_episode {
                    &ep.content[..budget_per_episode]
                } else {
                    ep.content.as_str()
                }
            })
            .collect();

        // TODO: Replace with actual LLM call when LLM client is available in Graph layer
        Ok(format!(
            "Summary of {} episodes: {}",
            episodes.len(),
            truncated.join(" | ")
        ))
    }

    /// Compress a set of episodes into a single summary episode with Fact memory_type.
    ///
    /// Orchestrates: load episodes -> generate summary -> persist via Storage.
    /// Source episodes get their compression_id set to the new summary episode's ID.
    /// Entity links from all source episodes are merged into the compressed episode.
    /// Returns the ID of the new compressed summary episode.
    pub fn compress_episodes(&self, episode_ids: &[String]) -> Result<String> {
        if episode_ids.is_empty() {
            return Err(CtxGraphError::InvalidInput(
                "cannot compress empty episode list".to_string(),
            ));
        }

        // Load all source episodes
        let mut episodes: Vec<Episode> = Vec::new();
        for id in episode_ids {
            let episode = self
                .storage
                .get_episode(id)?
                .ok_or_else(|| CtxGraphError::NotFound(format!("episode {id} not found")))?;
            episodes.push(episode);
        }

        // Generate summary (placeholder — will be LLM call in future)
        let summary = self.generate_compression_summary(&episodes)?;

        // Persist via Storage (pure SQLite)
        self.storage.compress_episodes(episode_ids, &summary)
    }

    /// List episodes that have not been compressed, recorded before the given date.
    pub fn list_uncompressed_episodes(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Episode>> {
        self.storage.list_uncompressed_episodes(before)
    }

    /// Run time-based batch compression (B3).
    ///
    /// Finds all uncompressed episodes older than `config.max_age_days`,
    /// groups them by calendar day, and compresses them in batches.
    ///
    /// Idempotent: already-compressed episodes are skipped.
    pub fn run_batch_compression(&self, config: &CompressionConfig) -> Result<CompressionResult> {
        self.storage.run_batch_compression(config)
    }

    /// Run size-based batch compression if the number of uncompressed episodes
    /// exceeds the threshold (B3).
    ///
    /// If `uncompressed_count < threshold`, returns a zeroed result.
    /// Otherwise compresses all episodes older than 7 days.
    pub fn run_compression_if_needed(
        &self,
        threshold: usize,
        batch_size: usize,
    ) -> Result<CompressionResult> {
        self.storage.run_compression_if_needed(threshold, batch_size)
    }

    // ── Pattern Extraction (D1a + D1b) ────────────────────────────────────────

    /// Extract pattern candidates from compression groups using co-occurrence counting (D1a).
    ///
    /// Loads compression groups from the last 7 days and runs the `PatternExtractor`
    /// to find entity types, entity pairs, and relation triplets that appear
    /// repeatedly across groups.
    ///
    /// Returns ranked candidates sorted by `occurrence_count` descending,
    /// capped at `max_patterns_per_extraction`.
    pub fn extract_pattern_candidates(
        &self,
        config: &PatternExtractorConfig,
    ) -> Result<Vec<PatternCandidate>> {
        let before = Utc::now();
        let groups = self.storage.get_compression_groups(before)?;
        let extractor = PatternExtractor::new();
        Ok(extractor.extract(&groups, config))
    }

    /// Generate a behavioral description for a pattern candidate using the LLM (D1b).
    ///
    /// Uses the provided `PatternDescriber` implementation to generate a 1-2 sentence
    /// description that captures the behavioral insight, NOT co-occurrence metadata.
    ///
    /// This is a pure delegation — the actual LLM call happens in the `PatternDescriber`.
    pub fn generate_pattern_description(
        &self,
        candidate: &PatternCandidate,
        source_summaries: &[String],
        describer: &dyn PatternDescriber,
    ) -> Result<String> {
        describer.generate(candidate, source_summaries)
    }

    /// Full D1a + D1b pipeline: extract candidates and generate descriptions.
    ///
    /// Orchestrates:
    /// 1. Extract pattern candidates from compression groups (D1a)
    /// 2. For each candidate, generate a behavioral description using LLM (D1b)
    /// 3. Store each described pattern as a LearnedPattern entity
    ///
    /// If LLM description generation fails for a candidate, that candidate is skipped
    /// but others continue. Returns partial results on partial failure.
    ///
    /// Returns the list of successfully stored pattern candidates with descriptions.
    pub fn extract_and_describe_patterns(
        &self,
        config: &PatternExtractorConfig,
        describer: &dyn PatternDescriber,
    ) -> Result<Vec<PatternCandidate>> {
        // D1a: extract candidates
        let candidates = self.extract_pattern_candidates(config)?;

        // Empty candidate list is not an error — return early
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let mut described = Vec::new();
        for mut candidate in candidates {
            // Get source episode contents for context
            let source_summaries: Vec<String> = candidate
                .source_groups
                .iter()
                .filter_map(|gid| {
                    self.storage
                        .get_episode(gid)
                        .ok()
                        .flatten()
                        .map(|ep| ep.content)
                })
                .collect();

            // D1b: generate description
            match self.generate_pattern_description(&candidate, &source_summaries, describer) {
                Ok(description) => {
                    candidate.description = Some(description.clone());
                    // Store the pattern
                    if let Err(e) = self.storage.store_pattern(&candidate) {
                        eprintln!(
                            "ctxgraph: warning: failed to store pattern {}: {}",
                            candidate.id, e
                        );
                        // Skip this candidate but continue with others
                    } else {
                        described.push(candidate);
                    }
                }
                Err(e) => {
                    // LLM failure: skip this candidate but continue
                    // Per acceptance criteria: "extraction can be retried"
                    eprintln!(
                        "ctxgraph: warning: failed to generate description for candidate {}: {}",
                        candidate.id, e
                    );
                }
            }
        }

        Ok(described)
    }

    /// List all stored patterns (LearnedPattern entities).
    ///
    /// Returns patterns with descriptions populated from the entity `summary` field.
    pub fn get_patterns(&self) -> Result<Vec<PatternCandidate>> {
        self.storage.get_patterns()
    }

    /// Store a pattern candidate as a LearnedPattern entity.
    ///
    /// This is a low-level storage operation used by the D1b pipeline.
    /// The pattern is stored with `entity_type = "LearnedPattern"`,
    /// `memory_type = Pattern`, `ttl = None`, and entity `name` truncated
    /// to 80 chars at word boundary from the description.
    pub fn store_pattern(&self, candidate: &PatternCandidate) -> Result<String> {
        self.storage.store_pattern(candidate)
    }

    /// Get all compression groups before the given timestamp.
    ///
    /// Used by D1a for pattern candidate extraction.
    pub fn get_compression_groups(
        &self,
        before: DateTime<Utc>,
    ) -> Result<Vec<CompressionGroupData>> {
        self.storage.get_compression_groups(before)
    }

    // ── Skill Creation and Evolution (D2) ─────────────────────────────────────

    /// Create skills from pattern candidates (D2 orchestration).
    ///
    /// Orchestrates the full skill creation pipeline:
    /// 1. Load edges for the patterns' source groups
    /// 2. Use SkillCreator to produce DraftSkill vec from patterns + edges
    /// 3. For each draft, use SkillSynthesizer to produce behavioral fields
    /// 4. Build Skill struct with provenance and store via Storage
    ///
    /// If the synthesizer fails for a draft, the error propagates (no partial
    /// skills stored, per D2 AC: "If LLM fails, skill creation returns error").
    ///
    /// Returns the list of successfully created skills.
    pub fn create_skills_from_patterns(
        &self,
        patterns: &[PatternCandidate],
        synthesizer: &dyn SkillSynthesizer,
        agent: &str,
        config: &SkillCreatorConfig,
        scope: SkillScope,
        limit: usize,
    ) -> Result<Vec<Skill>> {
        if patterns.is_empty() {
            return Ok(Vec::new());
        }

        // Load edges associated with pattern source groups
        let all_group_ids: Vec<String> = patterns
            .iter()
            .flat_map(|p| p.source_groups.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut all_edges: Vec<Edge> = Vec::new();
        for gid in &all_group_ids {
            // Load edges from source episodes in this compression group
            let mut stmt = self
                .storage
                .conn
                .prepare(
                    "SELECT e.id, e.source_id, e.target_id, e.relation, e.memory_type,
                            e.ttl_seconds, e.fact, e.valid_from, e.valid_until,
                            e.recorded_at, e.confidence, e.episode_id, e.metadata,
                            e.usage_count, e.last_recalled_at
                     FROM edges e
                     WHERE e.episode_id IN (SELECT id FROM episodes WHERE compression_id = ?1)
                     AND e.valid_until IS NULL",
                )
                .map_err(CtxGraphError::Storage)?;

            let edges: Vec<Edge> = stmt
                .query_map(rusqlite::params![gid], |row| {
                    Ok(Edge {
                        id: row.get(0)?,
                        source_id: row.get(1)?,
                        target_id: row.get(2)?,
                        relation: row.get(3)?,
                        memory_type: MemoryType::from_db(&row.get::<_, String>(4)?),
                        ttl: row.get::<_, Option<i64>>(5)?.and_then(|s| {
                            if s >= 0 {
                                Some(Duration::from_secs(s as u64))
                            } else {
                                None
                            }
                        }),
                        fact: row.get(6)?,
                        valid_from: row.get::<_, Option<String>>(7)?.map(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now())
                        }),
                        valid_until: row.get::<_, Option<String>>(8)?.map(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now())
                        }),
                        recorded_at: {
                            let s: String = row.get(9)?;
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now())
                        },
                        confidence: row.get(10)?,
                        episode_id: row.get(11)?,
                        metadata: row
                            .get::<_, Option<String>>(12)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                        usage_count: row.get(13)?,
                        last_recalled_at: row.get::<_, Option<String>>(14)?.map(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc))
                                .unwrap_or_else(|_| Utc::now())
                        }),
                    })
                })
                .map_err(CtxGraphError::Storage)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(CtxGraphError::Storage)?;

            all_edges.extend(edges);
        }

        // Build source summaries map: compression_group_id -> [summary contents]
        let mut source_summaries: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for gid in &all_group_ids {
            if let Ok(Some(ep)) = self.storage.get_episode(gid) {
                // The compression group ID is the summary episode ID
                source_summaries
                    .entry(gid.clone())
                    .or_default()
                    .push(ep.content);
            }
        }

        // Step 2: Draft skills from patterns + edges
        let drafts = SkillCreator::draft_skills(patterns, &all_edges, &source_summaries, config);

        if drafts.is_empty() {
            return Ok(Vec::new());
        }

        // Step 3+4: Synthesize and store each skill (up to limit)
        let mut created_skills = Vec::new();
        let drafts: Vec<_> = drafts.into_iter().take(limit).collect();
        for draft in drafts {
            let (name, description, trigger_condition, action) = synthesizer.synthesize(&draft)?;

            let confidence = Skill::compute_confidence(draft.success_count, draft.failure_count);
            let provenance = Some(Skill::generate_provenance(
                format!(
                    "Derived from {} pattern(s): {}",
                    draft.source_pattern_ids.len(),
                    draft.source_pattern_ids.join(", ")
                ),
                &draft.source_summaries,
                config.reasoning_ttl_days,
                config.context_facts_ttl_days,
            ));

            let skill = Skill {
                id: uuid::Uuid::now_v7().to_string(),
                name,
                description,
                trigger_condition,
                action,
                success_count: draft.success_count,
                failure_count: draft.failure_count,
                confidence,
                superseded_by: None,
                created_at: Utc::now(),
                entity_types: draft.entity_types,
                provenance,
                scope,
                created_by_agent: agent.to_string(),
            };

            self.storage.create_skill(&skill)?;
            created_skills.push(skill);
        }

        Ok(created_skills)
    }

    /// List all active (non-superseded) skills.
    pub fn list_skills(&self) -> Result<Vec<Skill>> {
        self.storage.list_skills()
    }

    /// Supersede a skill — marks the old skill as replaced by a new one (D2).
    ///
    /// The old skill is kept for audit but excluded from `list_skills`.
    pub fn supersede_skill(&self, skill_id: &str, new_skill_id: &str) -> Result<()> {
        self.storage.supersede_skill(skill_id, new_skill_id)
    }

    /// Search skills via FTS5 full-text search (D2).
    pub fn search_skills(&self, query: &str, limit: usize) -> Result<Vec<(Skill, f64)>> {
        self.storage.search_skills(query, limit)
    }

    // ── Cross-session Skill Persistence and Sharing (D3) ──────────────────────

    /// Share a skill — changes scope from Private to Shared (D3).
    ///
    /// One-way operation: skills cannot be un-shared.
    pub fn share_skill(&self, skill_id: &str) -> Result<()> {
        self.storage.share_skill(skill_id)
    }

    /// Get skills visible to a specific agent (D3).
    ///
    /// Returns shared skills (visible to all agents) plus private skills
    /// owned by the specified agent. Superseded skills are excluded.
    pub fn get_skills_for_agent(&self, agent: &str) -> Result<Vec<Skill>> {
        self.storage.get_skills_for_agent(agent)
    }

    /// Retrieve skills relevant to a context query for a specific agent (D3).
    ///
    /// Searches skills via FTS5 and returns results scored with a floor of 0.8.
    ///
    /// TODO(Budget): Skills retrieved here should enter the candidate set with
    /// floor score 0.8 and go through enforce_budget. The Budget pipeline
    /// (Phase A4) is not yet implemented, so this currently returns all matching
    /// skills with their FTS5 relevance scores. When Budget is implemented,
    /// wrap results in ScoredCandidate with max(score, 0.8) and pass through
    /// enforce_budget.
    pub fn retrieve_skills_for_context(
        &self,
        agent: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<(Skill, f64)>> {
        // Get all skills visible to this agent
        let agent_skills = self.get_skills_for_agent(agent)?;

        if query.is_empty() || agent_skills.is_empty() {
            // Return all agent skills with floor score 0.8 if no query
            return Ok(agent_skills
                .into_iter()
                .take(limit)
                .map(|s| (s, 0.8))
                .collect());
        }

        // Search via FTS5 — this already filters by superseded_by IS NULL
        let search_results = self.storage.search_skills(query, limit * 2)?;

        // Filter to only include skills visible to this agent
        let agent_skill_ids: std::collections::HashSet<String> =
            agent_skills.iter().map(|s| s.id.clone()).collect();

        let mut results: Vec<(Skill, f64)> = search_results
            .into_iter()
            .filter(|(skill, _)| agent_skill_ids.contains(&skill.id))
            .map(|(skill, score)| (skill, score.max(0.8)))
            .collect();

        // Sort by score descending and cap at limit
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    // ── Learning Pipeline (D4) ────────────────────────────────────────────────

    /// Run the full learning pipeline: D1a → D1b → D2 → D3 supersession.
    ///
    /// This is the main entry point for the `learn` CLI command and MCP tool.
    /// It orchestrates:
    /// 1. Extract pattern candidates from compression groups (D1a)
    /// 2. Generate behavioral descriptions using LLM (D1b)
    /// 3. Store described patterns (dedup against existing)
    /// 4. Create skills from new patterns (D2)
    /// 5. Supersede existing skills with overlapping entity types but different actions (D3)
    ///
    /// Returns a `LearningOutcome` summarizing what was found/created/updated.
    pub fn run_learning_pipeline(
        &self,
        agent: &str,
        scope: SkillScope,
        describer: &dyn PatternDescriber,
        synthesizer: &dyn SkillSynthesizer,
        limit: usize,
    ) -> Result<LearningOutcome> {
        // D1a: Extract pattern candidates from recent compression groups
        let config = PatternExtractorConfig::default();
        let candidates = self.extract_pattern_candidates(&config)?;

        if candidates.is_empty() {
            return Ok(LearningOutcome {
                patterns_found: 0,
                patterns_new: 0,
                skills_created: 0,
                skills_updated: 0,
                skill_ids: Vec::new(),
            });
        }

        // D1b: Generate descriptions for candidates
        let mut described_candidates: Vec<PatternCandidate> = Vec::new();
        for mut candidate in candidates {
            let source_summaries: Vec<String> = candidate
                .source_groups
                .iter()
                .filter_map(|gid| self.storage.get_episode(gid).ok().flatten())
                .map(|ep| ep.content)
                .collect();

            match self.generate_pattern_description(&candidate, &source_summaries, describer) {
                Ok(description) => {
                    candidate.description = Some(description);
                    described_candidates.push(candidate);
                }
                Err(e) => {
                    eprintln!(
                        "ctxgraph: warning: Failed to generate description for candidate: {}",
                        e
                    );
                }
            }
        }

        // Store described patterns and deduplicate against existing
        let existing_patterns = self.storage.get_patterns()?;
        let existing_keys: std::collections::HashSet<String> =
            existing_patterns.iter().map(|p| pattern_key(p)).collect();

        let mut new_patterns: Vec<PatternCandidate> = Vec::new();
        for p in &described_candidates {
            let key = pattern_key(p);
            if !existing_keys.contains(&key) {
                // Store the new pattern
                if let Err(e) = self.storage.store_pattern(p) {
                    eprintln!("ctxgraph: warning: Failed to store pattern {}: {}", p.id, e);
                } else {
                    new_patterns.push(p.clone());
                }
            }
        }

        // D2: Create skills from new patterns
        let skill_config = SkillCreatorConfig::default();
        let new_skills = self.create_skills_from_patterns(
            &new_patterns,
            synthesizer,
            agent,
            &skill_config,
            scope,
            limit,
        )?;

        // D3: Supersession — check new skills against existing
        let existing_skills = self.get_skills_for_agent(agent)?;
        let mut skills_updated = 0;

        for new_skill in &new_skills {
            for old_skill in &existing_skills {
                // Supersede if entity_types overlap AND actions differ
                if new_skill
                    .entity_types
                    .iter()
                    .any(|et| old_skill.entity_types.contains(et))
                    && new_skill.action != old_skill.action
                {
                    if let Err(e) = self.supersede_skill(&old_skill.id, &new_skill.id) {
                        eprintln!(
                            "ctxgraph: warning: Failed to supersede skill {} with {}: {}",
                            old_skill.id, new_skill.id, e
                        );
                    } else {
                        skills_updated += 1;
                    }
                    break; // Only supersede each old skill once
                }
            }
        }

        let skill_ids: Vec<String> = new_skills.iter().map(|s| s.id.clone()).collect();

        Ok(LearningOutcome {
            patterns_found: described_candidates.len(),
            patterns_new: new_patterns.len(),
            skills_created: new_skills.len(),
            skills_updated,
            skill_ids,
        })
    }
}

/// Generate a unique key for a pattern candidate for deduplication.
///
/// Uses entity_pair or relation_triplet (not description) to ensure
/// the same pattern is not stored twice even if described differently.
fn pattern_key(p: &PatternCandidate) -> String {
    if let Some(ref triplet) = p.relation_triplet {
        format!("triplet:{}:{}:{}", triplet.0, triplet.1, triplet.2)
    } else if let Some(ref pair) = p.entity_pair {
        format!("pair:{}:{}", pair.0, pair.1)
    } else {
        format!("types:{}", p.entity_types.join(","))
    }
}

/// Compute cosine similarity between two f32 vectors.
/// Returns 0.0 if either vector has zero magnitude.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}

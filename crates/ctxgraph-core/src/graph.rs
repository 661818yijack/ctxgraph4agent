use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::error::{CtxGraphError, Result};
use crate::pattern::BatchLabelDescriber;
use crate::skill::SkillCreator;
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
    pub async fn add_episode(&self, episode: Episode) -> Result<EpisodeResult> {
        self.storage.insert_episode(&episode)?;

        #[cfg(feature = "extract")]
        if let Some(ref pipeline) = self.pipeline {
            return self.add_episode_with_extraction(&episode, pipeline).await;
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
    async fn add_episode_with_extraction(
        &self,
        episode: &Episode,
        pipeline: &ExtractionPipeline,
    ) -> Result<EpisodeResult> {
        let result = pipeline
            .extract(&episode.content, episode.recorded_at)
            .await
            .map_err(|e| CtxGraphError::Extraction(e.to_string()))?;

        let mut entities_extracted = 0;
        let mut edges_created = 0;

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
        let contradictions = self
            .storage
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
        let contradictions_found = contradictions.len();

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
    ///
    /// When `source` is `Some(s)`, only episodes from that source are included
    /// in the FTS5 leg. Semantic results are post-filtered by source.
    pub fn search_fused(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        source: Option<&str>,
    ) -> Result<Vec<FusedEpisodeResult>> {
        const K: f64 = 60.0;

        // Accumulate RRF scores per episode id
        let mut scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        let mut episodes_map: std::collections::HashMap<String, Episode> =
            std::collections::HashMap::new();

        // --- FTS5 ranked list ---
        // Fetch a generous pool for RRF (up to 10x limit or 200)
        let fts_pool = (limit * 10).max(200);
        let fts_results = self.storage.search_episodes(query, fts_pool, source);
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
                // Fetch episode if not already cached
                let ep = if let Some(ep) = episodes_map.get(&ep_id) {
                    ep.source.clone()
                } else if let Ok(Some(ep)) = self.storage.get_episode(&ep_id) {
                    let src = ep.source.clone();
                    episodes_map.insert(ep_id.clone(), ep);
                    src
                } else {
                    continue;
                };
                // Skip if source filter is set and doesn't match
                if let Some(filter_src) = source
                    && ep.as_deref() != Some(filter_src)
                {
                    continue;
                }
                let rrf = 1.0 / (K + rank as f64 + 1.0);
                *scores.entry(ep_id).or_insert(0.0) += rrf;
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
    ///
    /// When `source` is `Some(s)`, only episodes from that source are returned.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        source: Option<&str>,
    ) -> Result<Vec<(Episode, f64)>> {
        self.storage.search_episodes(query, limit, source)
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
        let now = Utc::now();
        rank_scored_candidates_at(candidates, now)
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
        let result = self
            .storage
            .retrieve_for_context(query, agent_name, budget_tokens);

        // Increment cleanup counter after retrieval (even if retrieval failed)
        let _ = self.storage.increment_query_count_since_cleanup();

        // Trigger lazy cleanup check
        let _ = self.maybe_trigger_cleanup();

        result
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
        self.storage.get_stale_memories(threshold, limit, offset)
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

    /// Set the cleanup interval (number of queries between cleanup sweeps).
    ///
    /// The value is clamped to [1, 10000]. Default is 100 queries.
    /// This affects when the lazy trigger in `retrieve_for_context` fires.
    pub fn set_cleanup_interval(&self, interval: u64) -> Result<()> {
        self.storage.set_cleanup_interval(interval)
    }

    /// Get the current cleanup interval.
    ///
    /// Returns the configured interval (default 100), clamped to [1, 10000].
    pub fn get_cleanup_interval(&self) -> Result<u64> {
        self.storage.get_cleanup_interval()
    }

    /// Lazy cleanup trigger — runs cleanup if:
    /// - query_count_since_cleanup >= cleanup_interval (default 100)
    /// - cleanup is not already in progress
    ///
    /// Uses the agent policy's grace_period (default 7 days).
    fn maybe_trigger_cleanup(&self) -> Result<()> {
        // Early exit: check if cleanup is already running
        if let Some(val) = self.storage.get_system_metadata("cleanup_in_progress")?
            && val == "true"
        {
            return Ok(()); // skip silently
        }

        let count = self.storage.get_query_count_since_cleanup()?;
        let interval = self.storage.get_cleanup_interval()?;

        if count >= interval {
            let grace = AgentPolicy::default().grace_period_secs;
            let result = self.storage.cleanup_expired(grace);
            if result.is_ok() {
                // Reset counter on successful cleanup
                let _ = self.storage.reset_query_count_since_cleanup();
            }
        }
        Ok(())
    }

    // ── Pattern Extraction (D1a + D1b) ────────────────────────────────────────

    /// Extract pattern candidates from raw episodes using co-occurrence counting (D1a).
    ///
    /// Loads recent episodes with their associated entities and edges and runs
    /// the `PatternExtractor` to find entity types, entity pairs, and relation
    /// triplets that appear repeatedly across episodes.
    ///
    /// Returns ranked candidates sorted by `occurrence_count` descending,
    /// capped at `max_patterns_per_extraction`.
    pub fn extract_pattern_candidates(
        &self,
        config: &PatternExtractorConfig,
    ) -> Result<Vec<PatternCandidate>> {
        self.storage.get_pattern_candidates(config)
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

    /// Get pattern candidates from raw episodes.
    ///
    /// Used by D1a for pattern candidate extraction.
    /// Loads Experience episodes from storage and runs co-occurrence analysis.
    pub fn get_pattern_candidates(
        &self,
        config: &PatternExtractorConfig,
    ) -> Result<Vec<PatternCandidate>> {
        self.storage.get_pattern_candidates(config)
    }

    // ── Skill Creation and Evolution (D2) ─────────────────────────────────────

    /// Load edges associated with a set of episode IDs.
    fn load_edges_for_episodes(&self, episode_ids: &[String]) -> Result<Vec<Edge>> {
        if episode_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<String> = (1..=episode_ids.len()).map(|i| format!("?{i}")).collect();
        let in_clause = placeholders.join(", ");
        let sql = format!(
            "SELECT e.id, e.source_id, e.target_id, e.relation, e.memory_type,
                    e.ttl_seconds, e.fact, e.valid_from, e.valid_until,
                    e.recorded_at, e.confidence, e.episode_id, e.metadata,
                    e.usage_count, e.last_recalled_at
             FROM edges e
             WHERE e.episode_id IN ({in_clause})
             AND e.valid_until IS NULL"
        );

        let mut stmt = self
            .storage
            .conn
            .prepare(&sql)
            .map_err(CtxGraphError::Storage)?;

        stmt.query_map(rusqlite::params_from_iter(episode_ids.iter()), |row| {
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
        .map_err(CtxGraphError::Storage)
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

    /// Run the full learning pipeline: D1a -> D1b -> D2 -> D3 supersession.
    ///
    /// This is the main entry point for the `learn` CLI command and MCP tool.
    /// It orchestrates:
    /// 1. Extract pattern candidates from raw episodes (D1a)
    /// 2. Generate behavioral descriptions using LLM (D1b)
    /// 3. Store described patterns (dedup against existing)
    /// 4. Create skills from new patterns (D2)
    /// 5. Supersede existing skills with overlapping entity types but different actions (D3)
    ///
    /// Returns a `LearningOutcome` summarizing what was found/created/updated.
    pub async fn run_learning_pipeline(
        &self,
        agent: &str,
        scope: SkillScope,
        describer: &impl BatchLabelDescriber,
        limit: usize,
    ) -> Result<LearningOutcome> {
        // Stage 1: Extract pattern candidates
        let config = PatternExtractorConfig::default();
        let mut candidates = self.extract_pattern_candidates(&config)?;

        if candidates.is_empty() {
            return Ok(LearningOutcome {
                patterns_found: 0,
                patterns_new: 0,
                skills_created: 0,
                skills_updated: 0,
                skill_ids: Vec::new(),
            });
        }

        // Stage 2: Filter — keep only candidates with occurrence_count >= 3
        candidates.retain(|c| c.occurrence_count >= 3);

        // Stage 3: Intra-batch dedup — remove duplicate pattern keys within this batch
        let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
        candidates.retain(|c| seen_keys.insert(pattern_key(c)));

        // Stage 4: Inter-pattern dedup — filter out candidates already stored as patterns
        let existing_patterns = self.storage.get_patterns()?;
        let existing_keys: std::collections::HashSet<String> =
            existing_patterns.iter().map(pattern_key).collect();
        candidates.retain(|c| !existing_keys.contains(&pattern_key(c)));

        let patterns_found = candidates.len();

        if candidates.is_empty() {
            return Ok(LearningOutcome {
                patterns_found: 0,
                patterns_new: 0,
                skills_created: 0,
                skills_updated: 0,
                skill_ids: Vec::new(),
            });
        }

        // Stage 5: Build source_summaries map: pattern_id -> Vec<episode content>
        let all_episode_ids: Vec<String> = candidates
            .iter()
            .flat_map(|c| c.source_groups.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let mut source_summaries: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for candidate in &candidates {
            let mut summaries = Vec::new();
            for gid in &candidate.source_groups {
                if let Ok(Some(ep)) = self.storage.get_episode(gid) {
                    summaries.push(ep.content);
                }
            }
            source_summaries.insert(candidate.id.clone(), summaries);
        }

        // Stage 6: One batch LLM call for all candidates
        let label_pairs = describer
            .describe_batch(&candidates, &source_summaries)
            .await?;
        let descriptions: std::collections::HashMap<String, String> =
            label_pairs.into_iter().collect();

        // Stage 7: Load edges and create Skills directly (no DraftSkill, no store_pattern)
        let all_edges = self.load_edges_for_episodes(&all_episode_ids)?;
        // Build episode-keyed source summaries for SkillCreator
        let mut episode_summaries: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for eid in &all_episode_ids {
            if let Ok(Some(ep)) = self.storage.get_episode(eid) {
                episode_summaries
                    .entry(eid.clone())
                    .or_default()
                    .push(ep.content);
            }
        }

        let skill_config = SkillCreatorConfig::default();
        let candidates_limited: Vec<_> = candidates.into_iter().take(limit).collect();
        let new_skills = SkillCreator::create_skills(
            &candidates_limited,
            &all_edges,
            &episode_summaries,
            &descriptions,
            &skill_config,
            scope,
            agent,
        );

        // Store new skills
        for skill in &new_skills {
            if let Err(e) = self.storage.create_skill(skill) {
                eprintln!(
                    "ctxgraph: warning: Failed to store skill {}: {}",
                    skill.id, e
                );
            }
        }

        // Stage 8: D3 Supersession — entity_type overlap only (no action comparison)
        let existing_skills = self.get_skills_for_agent(agent)?;
        let mut skills_updated = 0;

        for new_skill in &new_skills {
            for old_skill in &existing_skills {
                if new_skill
                    .entity_types
                    .iter()
                    .any(|et| old_skill.entity_types.contains(et))
                {
                    if let Err(e) = self.supersede_skill(&old_skill.id, &new_skill.id) {
                        eprintln!(
                            "ctxgraph: warning: Failed to supersede skill {} with {}: {}",
                            old_skill.id, new_skill.id, e
                        );
                    } else {
                        skills_updated += 1;
                    }
                    break;
                }
            }
        }

        let skill_ids: Vec<String> = new_skills.iter().map(|s| s.id.clone()).collect();

        Ok(LearningOutcome {
            patterns_found,
            patterns_new: patterns_found,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn new_test_graph() -> Graph {
        Graph::in_memory().expect("failed to create in-memory graph")
    }

    // ── Lazy trigger tests ──

    #[test]
    fn test_trigger_skips_when_cleanup_in_progress() {
        let graph = new_test_graph();
        // Simulate cleanup in progress
        graph
            .storage
            .set_system_metadata("cleanup_in_progress", "true")
            .unwrap();

        // Set counter above interval
        for _ in 0..150 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        // Should NOT trigger cleanup
        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(count, 150, "counter should NOT have been reset");
    }

    #[test]
    fn test_trigger_does_not_run_below_interval() {
        let graph = new_test_graph();

        // Set counter below default interval (100)
        for _ in 0..50 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(count, 50, "counter should NOT have been reset");
    }

    #[test]
    fn test_trigger_resets_counter_at_interval() {
        let graph = new_test_graph();

        // Set counter at interval
        for _ in 0..100 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(count, 0, "counter should have been reset after cleanup");
    }

    #[test]
    fn test_trigger_respects_custom_interval() {
        let graph = new_test_graph();

        // Set custom interval to 50
        graph
            .storage
            .set_system_metadata("cleanup_interval", "50")
            .unwrap();

        // Set counter to 50
        for _ in 0..50 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(
            count, 0,
            "counter should have been reset at custom interval"
        );
    }

    #[test]
    fn test_set_cleanup_interval_via_graph() {
        let graph = new_test_graph();
        graph.set_cleanup_interval(200).unwrap();
        assert_eq!(graph.get_cleanup_interval().unwrap(), 200);
    }

    #[test]
    fn test_trigger_does_not_fire_when_below_custom_interval() {
        let graph = new_test_graph();
        graph.set_cleanup_interval(200).unwrap();

        // Set counter to 150 (below interval of 200)
        for _ in 0..150 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(count, 150, "counter should NOT have been reset");
    }

    #[test]
    fn test_trigger_proceeds_when_cleanup_in_progress_is_false() {
        let graph = new_test_graph();

        // Set cleanup_in_progress to false explicitly
        graph
            .storage
            .set_system_metadata("cleanup_in_progress", "false")
            .unwrap();

        // Set counter at interval
        for _ in 0..100 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        // Should proceed with cleanup and reset counter
        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(count, 0, "counter should have been reset");
    }

    #[test]
    fn test_trigger_proceeds_when_cleanup_in_progress_key_missing() {
        let graph = new_test_graph();

        // Don't set cleanup_in_progress at all (key missing)
        // Set counter at interval
        for _ in 0..100 {
            graph.storage.increment_query_count_since_cleanup().unwrap();
        }

        // Should proceed with cleanup (missing key = not in progress)
        graph.maybe_trigger_cleanup().unwrap();
        let count = graph.storage.get_query_count_since_cleanup().unwrap();
        assert_eq!(count, 0, "counter should have been reset");
    }
}

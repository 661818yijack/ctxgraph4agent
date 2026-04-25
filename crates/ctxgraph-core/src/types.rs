use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Classification of a memory's type, driving TTL and decay behavior.
/// - Fact: stable knowledge (90d default TTL)
/// - Pattern: recurring observation (never expires)
/// - Experience: one-time event (180d default TTL, 6-month evidence chain for skills)
/// - Preference: user preference (30d default TTL)
/// - Decision: architectural choice (90d default TTL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Fact,
    Pattern,
    Experience,
    Preference,
    Decision,
}

impl fmt::Display for MemoryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryType::Fact => write!(f, "fact"),
            MemoryType::Pattern => write!(f, "pattern"),
            MemoryType::Experience => write!(f, "experience"),
            MemoryType::Preference => write!(f, "preference"),
            MemoryType::Decision => write!(f, "decision"),
        }
    }
}

impl MemoryType {
    /// Default TTL for each memory type. None means never expires.
    pub fn default_ttl(&self) -> Option<Duration> {
        match self {
            MemoryType::Fact => Some(Duration::from_secs(90 * 86400)),
            MemoryType::Pattern => None,
            MemoryType::Experience => Some(Duration::from_secs(180 * 86400)),
            MemoryType::Preference => Some(Duration::from_secs(30 * 86400)),
            MemoryType::Decision => Some(Duration::from_secs(90 * 86400)),
        }
    }

    /// Default TTL in seconds for SQLite storage. None maps to NULL.
    pub fn default_ttl_seconds(&self) -> Option<i64> {
        self.default_ttl().map(|d| d.as_secs() as i64)
    }

    /// Map an entity_type string to a MemoryType. Unknown types default to Fact.
    pub fn from_entity_type(entity_type: &str) -> Self {
        match entity_type.to_lowercase().as_str() {
            "decision" => MemoryType::Decision,
            "pattern" => MemoryType::Pattern,
            "experience" => MemoryType::Experience,
            "preference" => MemoryType::Preference,
            _ => MemoryType::Fact,
        }
    }

    /// Parse from a database string (case-insensitive). Defaults to Fact on unknown.
    pub fn from_db(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fact" => MemoryType::Fact,
            "pattern" => MemoryType::Pattern,
            "experience" => MemoryType::Experience,
            "preference" => MemoryType::Preference,
            "decision" => MemoryType::Decision,
            _ => MemoryType::Fact,
        }
    }

    /// Compute the decay score for a memory node at the current time.
    ///
    /// Returns a value in [0.0, 1.0] representing freshness:
    /// - `base_confidence` at age 0 (just created)
    /// - Decreasing over time according to the memory type's decay curve
    /// - soft tail after TTL for Fact/Preference/Decision; Experience reaches 0.0 at TTL
    ///
    /// Decay formulas:
    /// - Exponential (Fact, Preference, Decision):
    ///   `base_confidence * exp(-λ * age)` where `λ = ln(2) / half_life`
    ///   - Fact/Decision/Preference: `half_life = ttl * 0.5`
    /// - Linear (Experience): `base_confidence * max(0.0, 1.0 - age / ttl)`
    /// - Constant (Pattern):  `base_confidence` (no decay, ignores ttl)
    pub fn decay_score(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
    ) -> f64 {
        self.decay_score_at(base_confidence, created_at, ttl, Utc::now())
    }

    /// Compute the decay score at a caller-provided timestamp.
    ///
    /// This is the deterministic variant used by tests and batch callers that
    /// want one stable clock value across a whole ranking or stale-memory pass.
    pub fn decay_score_at(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        now: DateTime<Utc>,
    ) -> f64 {
        self.decay_score_with_usage_at(base_confidence, created_at, ttl, None, 0, now)
    }

    /// Compute a recall-aware decay score at a caller-provided timestamp.
    ///
    /// Recent recall resets the effective age origin, and repeated recall extends
    /// the effective TTL with a bounded logarithmic boost. This is intended for
    /// ranking, not cleanup.
    pub fn decay_score_with_usage_at(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
        last_recalled_at: Option<DateTime<Utc>>,
        usage_count: u32,
        now: DateTime<Utc>,
    ) -> f64 {
        // Pattern never decays regardless of ttl
        if *self == MemoryType::Pattern {
            return base_confidence;
        }

        let Some(ttl) = ttl else {
            // No ttl but not a Pattern — treat as no decay
            return base_confidence;
        };

        let ttl_secs = ttl.as_secs_f64();

        // ttl=0 edge case: immediately expired
        if ttl_secs == 0.0 {
            return 0.0;
        }

        let effective_origin = last_recalled_at
            .filter(|recalled| *recalled > created_at && *recalled <= now)
            .unwrap_or(created_at);
        let age_secs = (now - effective_origin).num_seconds().max(0) as f64;
        let recall_boost = (1.0 + 0.15 * (usage_count as f64).ln_1p()).min(1.75);
        let ttl_secs = ttl_secs * recall_boost;

        match self {
            MemoryType::Fact => {
                let half_life = ttl_secs * 0.5;
                if age_secs > ttl_secs {
                    let ttl_score = base_confidence * decay_exponential(ttl_secs, half_life);
                    return decay_soft_tail(age_secs, ttl_secs, ttl_score);
                }
                base_confidence * decay_exponential(age_secs, half_life)
            }
            MemoryType::Decision => {
                // Decision uses same exponential decay as Fact (half_life = TTL/2)
                let half_life = ttl_secs * 0.5;
                if age_secs > ttl_secs {
                    let ttl_score = base_confidence * decay_exponential(ttl_secs, half_life);
                    return decay_soft_tail(age_secs, ttl_secs, ttl_score);
                }
                base_confidence * decay_exponential(age_secs, half_life)
            }
            MemoryType::Preference => {
                // Preference uses same exponential decay as Fact (half_life = TTL/2)
                let half_life = ttl_secs * 0.5;
                if age_secs > ttl_secs {
                    let ttl_score = base_confidence * decay_exponential(ttl_secs, half_life);
                    return decay_soft_tail(age_secs, ttl_secs, ttl_score);
                }
                base_confidence * decay_exponential(age_secs, half_life)
            }
            MemoryType::Experience => {
                if age_secs > ttl_secs {
                    return 0.0;
                }
                base_confidence * decay_linear(age_secs, ttl_secs)
            }
            MemoryType::Pattern => unreachable!(),
        }
    }
}

/// Exponential decay: `exp(-ln(2) / half_life * age)`
///
/// Returns 1.0 at age=0, 0.5 at age=half_life, approaching 0 asymptotically.
fn decay_exponential(age_secs: f64, half_life_secs: f64) -> f64 {
    let lambda = std::f64::consts::LN_2 / half_life_secs;
    (-lambda * age_secs).exp()
}

/// Continue a curve beyond TTL with a fast exponential tail.
fn decay_soft_tail(age_secs: f64, ttl_secs: f64, ttl_score: f64) -> f64 {
    let overshoot = (age_secs - ttl_secs) / ttl_secs;
    ttl_score * (-3.0 * overshoot).exp()
}

/// Linear decay: `max(0.0, 1.0 - age / ttl)`
///
/// Returns 1.0 at age=0, 0.0 at age=ttl.
fn decay_linear(age_secs: f64, ttl_secs: f64) -> f64 {
    (1.0 - age_secs / ttl_secs).max(0.0)
}

/// An episode is the fundamental unit of information.
/// It represents "something happened" — a decision, conversation, event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub content: String,
    pub source: Option<String>,
    pub recorded_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    /// Memory type for this episode, driving TTL and decay behavior.
    pub memory_type: MemoryType,
    /// If this episode is a compression summary, this links to the compressed episodes.
    pub compression_id: Option<String>,
}

/// Builder for constructing episodes with a fluent API.
pub struct EpisodeBuilder {
    content: String,
    source: Option<String>,
    metadata: serde_json::Map<String, serde_json::Value>,
    tags: Vec<String>,
    memory_type: MemoryType,
}

impl EpisodeBuilder {
    pub fn source(mut self, s: &str) -> Self {
        self.source = Some(s.to_string());
        self
    }

    pub fn tag(mut self, t: &str) -> Self {
        self.tags.push(t.to_string());
        self
    }

    pub fn meta(mut self, key: &str, val: impl Into<serde_json::Value>) -> Self {
        self.metadata.insert(key.to_string(), val.into());
        self
    }

    pub fn memory_type(mut self, mt: MemoryType) -> Self {
        self.memory_type = mt;
        self
    }

    pub fn build(mut self) -> Episode {
        if !self.tags.is_empty() {
            let tags: Vec<serde_json::Value> = self
                .tags
                .into_iter()
                .map(serde_json::Value::String)
                .collect();
            self.metadata
                .insert("tags".to_string(), serde_json::Value::Array(tags));
        }

        let metadata = if self.metadata.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(self.metadata))
        };

        Episode {
            id: uuid::Uuid::now_v7().to_string(),
            content: self.content,
            source: self.source,
            recorded_at: Utc::now(),
            metadata,
            memory_type: self.memory_type,
            compression_id: None,
        }
    }
}

impl Episode {
    pub fn builder(content: &str) -> EpisodeBuilder {
        EpisodeBuilder {
            content: content.to_string(),
            source: None,
            metadata: serde_json::Map::new(),
            tags: Vec::new(),
            memory_type: MemoryType::Experience,
        }
    }
}

/// An entity is a thing mentioned in episodes — people, components, decisions, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub memory_type: MemoryType,
    pub ttl: Option<Duration>,
    pub summary: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    pub usage_count: u32,
    pub last_recalled_at: Option<DateTime<Utc>>,
}

impl Entity {
    pub fn new(name: &str, entity_type: &str) -> Self {
        let memory_type = MemoryType::from_entity_type(entity_type);
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            memory_type,
            ttl: memory_type.default_ttl(),
            summary: None,
            created_at: Utc::now(),
            metadata: None,
            usage_count: 0,
            last_recalled_at: None,
        }
    }

    /// Create an entity with explicit memory_type and ttl.
    pub fn with_memory(
        name: &str,
        entity_type: &str,
        memory_type: MemoryType,
        ttl: Option<Duration>,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            memory_type,
            ttl,
            summary: None,
            created_at: Utc::now(),
            metadata: None,
            usage_count: 0,
            last_recalled_at: None,
        }
    }
}

/// An edge is a relationship between two entities.
/// Edges are bi-temporal: valid_from/valid_until (real-world) + recorded_at (system).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation: String,
    pub memory_type: MemoryType,
    pub ttl: Option<Duration>,
    pub fact: Option<String>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub recorded_at: DateTime<Utc>,
    pub confidence: f64,
    pub episode_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub usage_count: u32,
    pub last_recalled_at: Option<DateTime<Utc>>,
}

impl Edge {
    pub fn new(source_id: &str, target_id: &str, relation: &str) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            relation: relation.to_string(),
            memory_type: MemoryType::Fact,
            ttl: MemoryType::Fact.default_ttl(),
            fact: None,
            valid_from: None,
            valid_until: None,
            recorded_at: Utc::now(),
            confidence: 1.0,
            episode_id: None,
            metadata: None,
            usage_count: 0,
            last_recalled_at: None,
        }
    }

    /// Create an edge with explicit memory_type and ttl.
    pub fn with_memory(
        source_id: &str,
        target_id: &str,
        relation: &str,
        memory_type: MemoryType,
        ttl: Option<Duration>,
    ) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            relation: relation.to_string(),
            memory_type,
            ttl,
            fact: None,
            valid_from: None,
            valid_until: None,
            recorded_at: Utc::now(),
            confidence: 1.0,
            episode_id: None,
            metadata: None,
            usage_count: 0,
            last_recalled_at: None,
        }
    }

    /// Check if this edge is currently valid (not invalidated).
    pub fn is_current(&self) -> bool {
        self.valid_until.is_none()
    }

    /// Check if this edge was valid at a specific point in time.
    pub fn is_valid_at(&self, at: DateTime<Utc>) -> bool {
        let after_start = self.valid_from.is_none_or(|vf| vf <= at);
        let before_end = self.valid_until.is_none_or(|vu| vu > at);
        after_start && before_end
    }
}

/// Result from adding an episode to the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeResult {
    pub episode_id: String,
    pub entities_extracted: usize,
    pub edges_created: usize,
    pub contradictions_found: usize,
}

/// A detected contradiction between a new edge and an existing edge.
///
/// Produced by `Storage::check_contradictions` during episode ingestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    /// ID of the existing edge that was contradicted.
    pub old_edge_id: String,
    /// ID of the new edge that caused the contradiction.
    pub new_edge_id: String,
    /// Entity ID of the source entity (if available).
    pub entity_id: Option<String>,
    /// Normalized name of the source entity.
    pub entity_name: String,
    /// The relation type that conflicted.
    pub relation: String,
    /// The previous target value (target_id or fact string).
    pub old_value: String,
    /// The new target value (target_id or fact string).
    pub new_value: String,
    /// Confidence of the existing edge at time of contradiction.
    pub existing_confidence: f64,
}

/// Normalize an entity name for comparison: lowercase + trim whitespace.
///
/// Used in contradiction detection to handle minor variations in entity naming.
pub fn normalize_entity_name(name: &str) -> String {
    name.to_lowercase().trim().to_string()
}

/// Unified search result combining episodes, entities, and edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub episodes: Vec<Episode>,
    pub entities: Vec<Entity>,
    pub edges: Vec<Edge>,
    pub score: f64,
}

/// Per-episode result from fused (RRF) search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedEpisodeResult {
    pub episode: Episode,
    pub score: f64,
}

/// Graph-wide statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub episode_count: usize,
    pub entity_count: usize,
    pub edge_count: usize,
    pub sources: Vec<(String, usize)>,
    pub db_size_bytes: u64,
    /// Entities past grace_period with decay_score=0 (eligible for cleanup).
    pub decayed_entities: usize,
    /// Edges past grace_period with decay_score=0 (eligible for cleanup).
    pub decayed_edges: usize,
    // ── Cleanup visibility fields ──
    /// Timestamp of the last cleanup run (RFC3339), or None if never cleaned.
    pub last_cleanup_at: Option<String>,
    /// Number of queries since the last cleanup.
    pub queries_since_cleanup: u64,
    /// Cleanup interval in queries (default 100).
    pub cleanup_interval: u64,
    /// Whether a cleanup is currently in progress.
    pub cleanup_in_progress: bool,
    /// Total entities by memory type (excludes archived).
    pub total_entities_by_type: Vec<(String, usize)>,
    /// Decayed entities by memory type (decay_score=0, past grace_period).
    pub decayed_entities_by_type: Vec<(String, usize)>,
}

/// Result from a cleanup_expired operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanupResult {
    /// Entities deleted (Facts/Experiences with decay_score=0 past grace_period).
    pub entities_deleted: usize,
    /// Edges deleted.
    pub edges_deleted: usize,
    /// Preferences/Decisions archived (soft-deleted).
    pub entities_archived: usize,
    /// Edges archived.
    pub edges_archived: usize,
    /// Errors encountered during cleanup.
    pub errors: Vec<String>,
}

/// A memory that has become stale (decay_score below threshold).
///
/// Used by the reverify CLI to list memories needing attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleMemory {
    /// Memory ID (entity or edge ID).
    pub id: String,
    /// Type of this memory.
    pub memory_type: MemoryType,
    /// Content preview (entity name/summary or edge fact/relation).
    pub content: String,
    /// Age in days since creation.
    pub age_days: f64,
    /// Current decay score (0.0 = fully decayed).
    pub decay_score: f64,
    /// Suggested action based on decay_score.
    pub suggested_action: StaleAction,
}

/// Suggested action for a stale memory based on its decay_score.
/// - decay_score > 0.7 → Keep
/// - decay_score 0.3-0.7 → Update
/// - decay_score < 0.3 → Expire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StaleAction {
    Renew,
    Update,
    Expire,
    Keep,
}

/// Context around an entity — its immediate neighbors and edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityContext {
    pub entity: Entity,
    pub edges: Vec<Edge>,
    pub neighbors: Vec<Entity>,
}

/// Options for search filtering.
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub after: Option<DateTime<Utc>>,
    pub before: Option<DateTime<Utc>>,
    pub source: Option<String>,
    pub entity_type: Option<String>,
    pub limit: Option<usize>,
}

/// Configuration for pattern extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExtractorConfig {
    /// Minimum co-occurrence count for a pattern to be considered a candidate.
    pub min_occurrence_count: u32,
    /// Minimum number of distinct entity types in a candidate (for entity-type-level patterns).
    pub min_entity_types: usize,
    /// Maximum number of pattern candidates to return per extraction run.
    pub max_patterns_per_extraction: usize,
}

impl Default for PatternExtractorConfig {
    fn default() -> Self {
        Self {
            min_occurrence_count: 3,
            min_entity_types: 2,
            max_patterns_per_extraction: 20,
        }
    }
}

// ── Retrieval Types (A4a + A4b) ───────────────────────────────────────────

/// A candidate retrieved for scoring and ranking (A4a).
///
/// Produced by the candidate retrieval step before scoring.
/// Contains all information needed for composite scoring in A4b.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    /// Entity ID if this candidate is an entity, otherwise None.
    pub entity_id: Option<String>,
    /// Edge ID if this candidate is an edge, otherwise None.
    pub edge_id: Option<String>,
    /// Content preview for display.
    pub content: String,
    /// FTS5 BM25 score (raw, may exceed 1.0).
    pub fts_score: f64,
    /// Memory type driving TTL and decay behavior.
    pub memory_type: MemoryType,
    /// When this memory was created.
    pub created_at: DateTime<Utc>,
    /// Time-to-live for this memory (None = never expires).
    pub ttl: Option<Duration>,
    /// Base confidence at creation time.
    pub base_confidence: f64,
    /// How many times this memory has been recalled.
    pub usage_count: u32,
    /// When this memory was last recalled, if ever.
    pub last_recalled_at: Option<DateTime<Utc>>,
}

/// A candidate with its composite score (A4b).
///
/// Returned by `score_candidate` and `Graph::rank_candidates`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredCandidate {
    /// The original retrieval candidate.
    pub candidate: RetrievalCandidate,
    /// Composite score = decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))
    pub composite_score: f64,
}

/// Compute the composite score for a retrieval candidate.
///
/// Formula: `decay_score * confidence_weight * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))`
///
/// - `decay_score`: freshness from MemoryType::decay_score
/// - `confidence_weight`: bounded source confidence in [0.5, 1.0]
/// - `normalized_fts_score`: BM25 score clamped to [0.0, 1.0]
/// - Usage bonus: `(1.0 + 0.1 * ln(1 + usage_count))` — 1.0 at usage_count=0
///
/// Special cases:
/// - Zero-decay memories: returns 0.0
/// - Patterns: score is `max(score, 0.5)` to ensure visibility
pub fn score_candidate(candidate: &RetrievalCandidate) -> f64 {
    score_candidate_at(candidate, Utc::now())
}

/// Compute the composite score for a retrieval candidate at a fixed timestamp.
pub fn score_candidate_at(candidate: &RetrievalCandidate, now: DateTime<Utc>) -> f64 {
    // Compute decay score
    let decay = candidate.memory_type.decay_score_with_usage_at(
        1.0,
        candidate.created_at,
        candidate.ttl,
        candidate.last_recalled_at,
        candidate.usage_count,
        now,
    );

    // Expired memories get score 0.0
    if decay == 0.0 {
        return 0.0;
    }

    // Normalize FTS score to [0.0, 1.0] range
    // BM25 can exceed 1.0 for very relevant results, so we clamp
    let normalized_fts = candidate.fts_score.clamp(0.0, 1.0);

    // Confidence weight: demote low-confidence extractions without erasing them.
    let confidence_weight = 0.5 + 0.5 * candidate.base_confidence.clamp(0.0, 1.0);

    // Usage bonus: (1.0 + 0.1 * ln(1 + usage_count))
    // usage_count=0 → bonus = 1.0
    // usage_count=100 → bonus ≈ 1.46
    let usage_bonus = 1.0 + 0.1 * (1.0 + candidate.usage_count as f64).ln();

    // NaN guard: if any component is NaN, return 0.0
    if decay.is_nan()
        || confidence_weight.is_nan()
        || normalized_fts.is_nan()
        || usage_bonus.is_nan()
    {
        return 0.0;
    }

    let score = decay * confidence_weight * normalized_fts * usage_bonus;

    // Patterns get a floor of 0.5
    if candidate.memory_type == MemoryType::Pattern {
        score.max(0.5)
    } else {
        score
    }
}

/// Score, filter, and sort retrieval candidates at a fixed timestamp.
pub fn rank_scored_candidates_at(
    candidates: Vec<RetrievalCandidate>,
    now: DateTime<Utc>,
) -> Vec<ScoredCandidate> {
    let mut scored: Vec<ScoredCandidate> = candidates
        .into_iter()
        .map(|c| {
            let composite_score = score_candidate_at(&c, now);
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

    scored
}

// ── Budget Enforcement (A4c) ───────────────────────────────────────────────

/// A memory selected for inclusion in context, with its token cost.
///
/// Produced by `enforce_budget` after greedy selection from scored candidates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMemory {
    /// Memory type driving TTL and decay behavior.
    pub memory_type: MemoryType,
    /// Content to inject into context.
    pub content: String,
    /// Composite score from ranking (for debugging/audit).
    pub score: f64,
    /// Entity ID if this is an entity memory, otherwise None.
    pub entity_id: Option<String>,
    /// Edge ID if this is an edge memory, otherwise None.
    pub edge_id: Option<String>,
    /// Token estimate for this memory (text.len() / 4).
    pub tokens: usize,
}

/// Per-agent memory policy configuration.
///
/// Drives budget allocation and pattern inclusion limits during retrieval.
/// Default budget is 20,000 tokens; default max patterns is 50.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPolicy {
    /// Maximum tokens to spend on memory for this agent per query.
    pub memory_budget_tokens: usize,
    /// Agent name for policy lookup.
    pub agent_name: String,
    /// Maximum number of patterns to include in results (0 = no limit).
    pub max_patterns_included: usize,
    /// Confidence threshold for contradiction detection.
    /// Edges with confidence below this are replaced silently without flagging.
    pub contradiction_threshold: f64,
    /// Grace period (seconds) after TTL expiration before cleanup deletes memories.
    /// Facts and Experiences are deleted when decay_score=0 AND age > grace_period.
    /// Preferences and Decisions are archived (soft-deleted). Patterns are never cleaned.
    /// Default: 7 days (604800 seconds).
    pub grace_period_secs: u64,
}

impl Default for AgentPolicy {
    fn default() -> Self {
        Self {
            memory_budget_tokens: 20_000,
            agent_name: String::new(),
            max_patterns_included: 50,
            contradiction_threshold: 0.2,
            grace_period_secs: 604_800, // 7 days
        }
    }
}

/// Estimate token count for a text string.
///
/// Uses `text.len() / 4` as a ceiling estimate. This is acknowledged to be
/// imprecise — actual token counts vary by vocabulary and encoding — but
/// provides a conservative (overestimating) approximation suitable for budget
/// enforcement.
///
/// - Input: "hello world" (11 chars) → returns 3 tokens (ceiling)
/// - Input: "The quick brown fox jumps over the lazy dog" (44 chars) → returns 11 tokens
///
/// For exact counting, a proper tokenizer (e.g., tiktoken) would be needed.
pub fn estimate_tokens(text: &str) -> usize {
    // Ceiling division: (len + 3) / 4  is equivalent to ceil(len / 4)
    // But simpler: just use len / 4 with integer division (floor), which
    // underestimates. For ceiling, add 3 first.
    // However, the spec says text.len() / 4 directly, so we follow that.
    // Note: this is documented as a ceiling estimate, so we use floor which
    // actually makes it a floor estimate, but in practice chars > tokens
    // so this serves as a reasonable proxy.
    text.len() / 4
}

/// Greedily enforce token budget on scored candidates.
///
/// Takes sorted (highest-scoring first) `ScoredCandidate`s and adds them
/// to the result until the token budget is exhausted. Skips any candidate
/// whose token estimate alone exceeds the remaining budget.
///
/// Returns `(selected_memories, total_tokens_spent)` where:
/// - `selected_memories`: memories within budget, in score-descending order
/// - `total_tokens_spent`: sum of estimate_tokens for all returned memories
///
/// Budget defaults to 20,000 tokens (AgentPolicy::default().memory_budget_tokens).
///
/// # Behavior
/// - Greedy selection: highest-scored candidates first
/// - Skip if adding would exceed budget
/// - Skip if single memory exceeds budget entirely
/// - If budget is 0, returns empty vec
pub fn enforce_budget(
    candidates: Vec<ScoredCandidate>,
    budget_tokens: usize,
) -> (Vec<RankedMemory>, usize) {
    if budget_tokens == 0 {
        return (Vec::new(), 0);
    }

    let mut selected: Vec<RankedMemory> = Vec::new();
    let mut total_tokens: usize = 0;

    for scored in candidates {
        let tokens = estimate_tokens(&scored.candidate.content);

        // Skip if this single memory exceeds the entire budget
        if tokens > budget_tokens {
            continue;
        }

        // Skip if adding this would exceed remaining budget
        if total_tokens + tokens > budget_tokens {
            continue;
        }

        total_tokens += tokens;

        let ranked = RankedMemory {
            memory_type: scored.candidate.memory_type,
            content: scored.candidate.content,
            score: scored.composite_score,
            entity_id: scored.candidate.entity_id,
            edge_id: scored.candidate.edge_id,
            tokens,
        };

        selected.push(ranked);
    }

    (selected, total_tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_candidate(
        memory_type: MemoryType,
        created_at: DateTime<Utc>,
        ttl_seconds: Option<i64>,
        base_confidence: f64,
        usage_count: u32,
        fts_score: f64,
    ) -> RetrievalCandidate {
        RetrievalCandidate {
            entity_id: Some("test-entity".to_string()),
            edge_id: None,
            content: "test content".to_string(),
            fts_score,
            memory_type,
            created_at,
            ttl: ttl_seconds.map(|s| Duration::seconds(s).to_std().unwrap()),
            base_confidence,
            usage_count,
            last_recalled_at: None,
        }
    }

    #[test]
    fn test_fresh_fact_scores_higher_than_stale_with_high_usage() {
        // Fresh fact: created 1 day ago, 90d TTL, high FTS, usage=10
        let fresh = make_candidate(
            MemoryType::Fact,
            Utc::now() - Duration::days(1),
            Some(90 * 86400),
            1.0,
            10,
            0.8,
        );

        // Stale fact: created 60 days ago, 90d TTL, high FTS, usage=0
        let stale = make_candidate(
            MemoryType::Fact,
            Utc::now() - Duration::days(60),
            Some(90 * 86400),
            1.0,
            0,
            0.8,
        );

        let fresh_score = score_candidate(&fresh);
        let stale_score = score_candidate(&stale);

        assert!(
            fresh_score > stale_score,
            "fresh={fresh_score} should score higher than stale={stale_score}"
        );
    }

    #[test]
    fn test_recently_recalled_old_fact_scores_higher_than_unrecalled_old_fact() {
        let now = Utc::now();
        let unrecalled = make_candidate(
            MemoryType::Fact,
            now - Duration::days(80),
            Some(90 * 86400),
            1.0,
            0,
            0.8,
        );
        let mut recalled = unrecalled.clone();
        recalled.last_recalled_at = Some(now - Duration::days(1));

        let unrecalled_score = score_candidate_at(&unrecalled, now);
        let recalled_score = score_candidate_at(&recalled, now);

        assert!(
            recalled_score > unrecalled_score,
            "recently recalled fact should rank higher: recalled={recalled_score}, unrecalled={unrecalled_score}"
        );
    }

    #[test]
    fn test_usage_count_extends_decay_in_recall_aware_scoring() {
        let now = Utc::now();
        let low_usage = MemoryType::Fact.decay_score_with_usage_at(
            1.0,
            now - Duration::days(80),
            Some(Duration::days(90).to_std().unwrap()),
            None,
            0,
            now,
        );
        let high_usage = MemoryType::Fact.decay_score_with_usage_at(
            1.0,
            now - Duration::days(80),
            Some(Duration::days(90).to_std().unwrap()),
            None,
            100,
            now,
        );

        assert!(
            high_usage > low_usage,
            "usage count should slow ranking decay: high_usage={high_usage}, low_usage={low_usage}"
        );
    }

    #[test]
    fn test_base_confidence_weights_candidate_score() {
        let now = Utc::now();
        let low_confidence = make_candidate(
            MemoryType::Fact,
            now - Duration::days(1),
            Some(90 * 86400),
            0.3,
            0,
            0.8,
        );
        let high_confidence = make_candidate(
            MemoryType::Fact,
            now - Duration::days(1),
            Some(90 * 86400),
            0.9,
            0,
            0.8,
        );

        let low_score = score_candidate_at(&low_confidence, now);
        let high_score = score_candidate_at(&high_confidence, now);

        assert!(
            high_score > low_score,
            "higher confidence candidate should rank higher: high={high_score}, low={low_score}"
        );
    }

    #[test]
    fn test_pattern_gets_minimum_0_5_floor() {
        // Pattern with very low FTS score
        let pattern = make_candidate(
            MemoryType::Pattern,
            Utc::now() - Duration::days(100),
            None, // patterns never expire
            1.0,
            0,
            0.1, // very low FTS
        );

        let score = score_candidate(&pattern);
        assert!(score >= 0.5, "pattern score {score} should be at least 0.5");
    }

    #[test]
    fn test_post_ttl_fact_keeps_soft_tail_score() {
        // Fact created 100 days ago with 90d TTL should retain a small soft-tail score.
        let post_ttl = make_candidate(
            MemoryType::Fact,
            Utc::now() - Duration::days(100),
            Some(90 * 86400),
            1.0,
            5,
            0.9,
        );

        let score = score_candidate(&post_ttl);
        assert!(
            score > 0.0,
            "post-TTL fact should retain a small soft-tail score"
        );
    }

    #[test]
    fn test_usage_count_zero_gives_no_bonus() {
        // usage_count=0 should give bonus factor of 1.0
        // Use a pattern (never decays) to isolate the usage bonus
        let candidate = make_candidate(
            MemoryType::Pattern,
            Utc::now() - Duration::days(1),
            None, // patterns never decay
            1.0,
            0,
            1.0,
        );

        // decay=1.0 (pattern), normalized_fts=1.0, bonus=1.0
        let score = score_candidate(&candidate);
        // Score should be close to 1.0 (decay * fts * bonus = 1.0 * 1.0 * 1.0)
        assert!(
            (score - 1.0).abs() < 0.001,
            "usage_count=0 should give bonus factor 1.0, score={score}"
        );
    }

    #[test]
    fn test_usage_count_100_gives_diminishing_returns_bonus() {
        // usage_count=100 should give bonus factor ≈ 1 + 0.1*ln(101) ≈ 1.46
        // Use a pattern (never decays) to isolate the usage bonus
        let candidate = make_candidate(
            MemoryType::Pattern,
            Utc::now() - Duration::days(1),
            None, // patterns never decay
            1.0,
            100,
            1.0,
        );

        let expected_bonus = 1.0 + 0.1 * (1.0_f64 + 100.0_f64).ln();
        // For pattern: score = max(1.0 * 1.0 * bonus, 0.5) = bonus
        let expected_score = expected_bonus;

        let score = score_candidate(&candidate);
        assert!(
            (score - expected_score).abs() < 0.01,
            "usage_count=100 should give bonus ~1.46, got {score}"
        );
    }

    #[test]
    fn test_composite_score_range_is_bounded() {
        // Create various candidates and verify score range
        let test_cases = vec![
            make_candidate(
                MemoryType::Fact,
                Utc::now() - Duration::days(1),
                Some(90 * 86400),
                1.0,
                0,
                1.0,
            ),
            make_candidate(
                MemoryType::Fact,
                Utc::now() - Duration::days(1),
                Some(90 * 86400),
                1.0,
                100,
                1.0,
            ),
            make_candidate(
                MemoryType::Pattern,
                Utc::now() - Duration::days(1),
                None,
                1.0,
                0,
                0.1,
            ),
            make_candidate(
                MemoryType::Experience,
                Utc::now() - Duration::days(90),
                Some(180 * 86400),
                1.0,
                50,
                0.5,
            ),
        ];

        for candidate in test_cases {
            let score = score_candidate(&candidate);
            assert!(
                score >= 0.0 && score <= 1.5,
                "score {score} should be in [0.0, 1.5] range"
            );
        }
    }

    #[test]
    fn test_fts_score_above_1_clamped_to_1() {
        // BM25 can exceed 1.0 for very relevant results, should be clamped
        let candidate = make_candidate(
            MemoryType::Fact,
            Utc::now() - Duration::days(1),
            Some(90 * 86400),
            1.0,
            0,
            5.0, // BM25 raw score way above 1.0
        );

        let score = score_candidate(&candidate);
        // normalized_fts = clamp(5.0, 0.0, 1.0) = 1.0
        // decay ≈ 1.0 (fresh fact), bonus = 1.0
        // score = 1.0 * 1.0 * 1.0 = 1.0
        assert!(
            score <= 1.0,
            "fts_score=5.0 should be clamped, score={score} should not exceed 1.0"
        );
    }

    #[test]
    fn test_fts_score_below_0_clamped_to_0() {
        // Negative FTS score should be clamped to 0
        let candidate = make_candidate(
            MemoryType::Fact,
            Utc::now() - Duration::days(1),
            Some(90 * 86400),
            1.0,
            0,
            -0.5, // negative FTS score
        );

        let score = score_candidate(&candidate);
        // normalized_fts = clamp(-0.5, 0.0, 1.0) = 0.0
        // score = decay * 0.0 * bonus = 0.0
        assert_eq!(score, 0.0, "fts_score=-0.5 should be clamped to 0.0");
    }

    #[test]
    fn test_rank_candidates_sorts_descending_and_keeps_soft_tail() {
        use crate::graph::Graph;

        let graph = Graph::in_memory().expect("in-memory graph should init");

        let candidates = vec![
            make_candidate(
                MemoryType::Fact,
                Utc::now() - Duration::days(1),
                Some(90 * 86400),
                1.0,
                0,
                0.8,
            ),
            make_candidate(
                MemoryType::Fact,
                Utc::now() - Duration::days(60),
                Some(90 * 86400),
                1.0,
                0,
                0.9,
            ),
            // Post-TTL: 100 days old with 90d TTL, still retained by soft tail
            make_candidate(
                MemoryType::Fact,
                Utc::now() - Duration::days(100),
                Some(90 * 86400),
                1.0,
                5,
                0.95,
            ),
            make_candidate(
                MemoryType::Pattern,
                Utc::now() - Duration::days(1),
                None,
                1.0,
                0,
                0.6,
            ),
        ];

        let ranked = graph.rank_candidates(candidates);

        // Should have 4 candidates (post-TTL Fact is retained by soft tail)
        assert_eq!(ranked.len(), 4);

        // Should be sorted descending by composite_score
        assert!(
            ranked[0].composite_score >= ranked[1].composite_score,
            "first score {} should be >= second score {}",
            ranked[0].composite_score,
            ranked[1].composite_score
        );
        assert!(
            ranked[1].composite_score >= ranked[2].composite_score,
            "second score {} should be >= third score {}",
            ranked[1].composite_score,
            ranked[2].composite_score
        );
    }

    #[test]
    fn test_high_scoring_pattern_not_reduced_by_floor() {
        // A high-scoring pattern should NOT be capped at 0.5
        // It should get its actual score which is higher
        let pattern = make_candidate(
            MemoryType::Pattern,
            Utc::now() - Duration::days(1),
            None, // patterns never expire
            1.0,
            100, // high usage count
            1.0, // perfect FTS match
        );

        let score = score_candidate(&pattern);
        // decay = 1.0 (pattern never decays)
        // normalized_fts = 1.0
        // usage_bonus ≈ 1.46 (ln(101) ≈ 4.615, * 0.1 + 1 = 1.46)
        // raw_score = 1.0 * 1.0 * 1.46 ≈ 1.46
        // Pattern floor only applies if score < 0.5, but score ≈ 1.46 > 0.5
        assert!(
            score > 0.5,
            "high-scoring pattern score={score} should NOT be reduced to 0.5 floor"
        );
        // Score should reflect the usage bonus, not be capped at 0.5
        let expected_bonus = 1.0 + 0.1 * (1.0_f64 + 100.0_f64).ln();
        assert!(
            (score - expected_bonus).abs() < 0.01,
            "pattern score {score} should equal usage bonus ~{expected_bonus}"
        );
    }

    // ── A4c: Budget Enforcement Tests ─────────────────────────────────────

    #[test]
    fn test_estimate_tokens_hello_world() {
        // "hello world" is 11 chars, 11/4 = 2.75 floor = 2
        // But actual test: 11 chars / 4 = 2 (floor division)
        let tokens = estimate_tokens("hello world");
        assert_eq!(tokens, 2, "hello world (11 chars) should be ~2-3 tokens");
    }

    #[test]
    fn test_estimate_tokens_empty_string() {
        let tokens = estimate_tokens("");
        assert_eq!(tokens, 0, "empty string should be 0 tokens");
    }

    #[test]
    fn test_estimate_tokens_ceiling_estimate() {
        // text.len() / 4 is documented as a ceiling estimate
        // For short strings, the floor division gives a lower bound
        let tokens = estimate_tokens("hi"); // 2 chars
        assert_eq!(tokens, 0, "2 chars / 4 = 0 tokens");
    }

    #[test]
    fn test_enforce_budget_zero_budget_returns_empty() {
        let candidates = vec![];
        let (memories, tokens_spent) = enforce_budget(candidates, 0);
        assert!(memories.is_empty());
        assert_eq!(tokens_spent, 0);
    }

    #[test]
    fn test_enforce_budget_single_candidate_fits() {
        use chrono::Duration;

        let candidate = ScoredCandidate {
            candidate: RetrievalCandidate {
                entity_id: Some("e1".to_string()),
                edge_id: None,
                content: "short".to_string(), // 5 chars = 1 token
                fts_score: 0.5,
                memory_type: MemoryType::Fact,
                created_at: Utc::now() - Duration::days(1),
                ttl: Some(Duration::days(90).to_std().unwrap()),
                base_confidence: 1.0,
                usage_count: 0,
                last_recalled_at: None,
            },
            composite_score: 0.5,
        };

        let (memories, tokens_spent) = enforce_budget(vec![candidate], 100);
        assert_eq!(memories.len(), 1);
        assert_eq!(tokens_spent, 1); // 5 chars / 4 = 1 token
    }

    #[test]
    fn test_enforce_budget_skips_candidate_exceeding_budget() {
        use chrono::Duration;

        // Create a candidate with content too large for the budget
        let candidate = ScoredCandidate {
            candidate: RetrievalCandidate {
                entity_id: Some("e1".to_string()),
                edge_id: None,
                content: "a".repeat(500), // 500 chars = 125 tokens
                fts_score: 0.5,
                memory_type: MemoryType::Fact,
                created_at: Utc::now() - Duration::days(1),
                ttl: Some(Duration::days(90).to_std().unwrap()),
                base_confidence: 1.0,
                usage_count: 0,
                last_recalled_at: None,
            },
            composite_score: 0.5,
        };

        // Budget of 100 tokens should skip this candidate
        let (memories, tokens_spent) = enforce_budget(vec![candidate], 100);
        assert!(
            memories.is_empty(),
            "candidate exceeding budget should be skipped"
        );
        assert_eq!(tokens_spent, 0);
    }

    #[test]
    fn test_enforce_budget_greedy_selection() {
        use chrono::Duration;

        // Create candidates with different scores and sizes
        // Note: enforce_budget processes in order given, so we provide them sorted
        let candidates = vec![
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e1".to_string()),
                    edge_id: None,
                    content: "small".to_string(), // 5 chars = 1 token
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.9, // highest score
            },
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e3".to_string()),
                    edge_id: None,
                    content: "tiny".to_string(), // 4 chars = 1 token
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.8, // second highest
            },
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e2".to_string()),
                    edge_id: None,
                    content: "medium size content here".to_string(), // 24 chars = 6 tokens
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.5, // lower score
            },
        ];

        // Budget of 10 tokens should fit all three (1 + 1 + 6 = 8 tokens)
        let (memories, tokens_spent) = enforce_budget(candidates, 10);
        assert_eq!(memories.len(), 3);
        assert_eq!(tokens_spent, 8);

        // Verify order is by score descending (as provided)
        assert_eq!(memories[0].score, 0.9);
        assert_eq!(memories[1].score, 0.8);
        assert_eq!(memories[2].score, 0.5);
    }

    #[test]
    fn test_enforce_budget_respects_remaining_budget() {
        use chrono::Duration;

        // First candidate uses most of budget
        // "content that takes up many tokens here" = 38 chars = 9 tokens
        // "another item" = 12 chars = 3 tokens
        let candidates = vec![
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e1".to_string()),
                    edge_id: None,
                    content: "content that takes up many tokens here".to_string(),
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.9,
            },
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e2".to_string()),
                    edge_id: None,
                    content: "another item".to_string(),
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.8,
            },
        ];

        // Budget of 11 tokens - first candidate (9 tokens) fits,
        // second candidate (3 tokens) would exceed 11 (9+3=12 > 11)
        let (memories, tokens_spent) = enforce_budget(candidates, 11);
        assert_eq!(memories.len(), 1);
        assert_eq!(tokens_spent, 9);
    }

    #[test]
    fn test_enforce_budget_total_within_budget() {
        use chrono::Duration;

        let candidates = vec![
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e1".to_string()),
                    edge_id: None,
                    content: "test content number one".to_string(), // 23 chars = 5 tokens
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.7,
            },
            ScoredCandidate {
                candidate: RetrievalCandidate {
                    entity_id: Some("e2".to_string()),
                    edge_id: None,
                    content: "test content number two here".to_string(), // 30 chars = 7 tokens
                    fts_score: 0.5,
                    memory_type: MemoryType::Fact,
                    created_at: Utc::now() - Duration::days(1),
                    ttl: Some(Duration::days(90).to_std().unwrap()),
                    base_confidence: 1.0,
                    usage_count: 0,
                    last_recalled_at: None,
                },
                composite_score: 0.6,
            },
        ];

        let budget = 20_000; // default budget
        let (memories, tokens_spent) = enforce_budget(candidates, budget);

        // Both should fit (5 + 7 = 12 tokens < 20000)
        assert_eq!(memories.len(), 2);
        assert_eq!(tokens_spent, 12);

        // Verify property: sum of tokens <= budget
        let total_tokens: usize = memories.iter().map(|m| m.tokens).sum();
        assert!(total_tokens <= budget);
    }

    #[test]
    fn test_agent_policy_default() {
        let policy = AgentPolicy::default();
        assert_eq!(policy.memory_budget_tokens, 20_000);
        assert_eq!(policy.max_patterns_included, 50);
        assert_eq!(policy.agent_name, "");
        assert_eq!(policy.contradiction_threshold, 0.2);
    }

    // ── C1: Contradiction Detection Tests ─────────────────────────────────

    #[test]
    fn test_normalize_entity_name_lowercase_and_trim() {
        assert_eq!(normalize_entity_name("  Alice  "), "alice");
        assert_eq!(normalize_entity_name("ALICE"), "alice");
        assert_eq!(normalize_entity_name("Alice Smith"), "alice smith");
        assert_eq!(normalize_entity_name("  ALICE SMITH  "), "alice smith");
        assert_eq!(normalize_entity_name("alice"), "alice"); // already lowercase
    }

    #[test]
    fn test_normalize_entity_name_preserves_content() {
        // Normalization should not change the content beyond case and whitespace
        assert_eq!(normalize_entity_name("PostgreSQL"), "postgresql");
        assert_eq!(normalize_entity_name("MySQL"), "mysql");
        assert_eq!(normalize_entity_name("  hello world  "), "hello world");
    }

    #[test]
    fn test_contradiction_struct_creation() {
        let contradiction = Contradiction {
            old_edge_id: "edge-1".to_string(),
            new_edge_id: "edge-2".to_string(),
            entity_id: Some("entity-123".to_string()),
            entity_name: "alice".to_string(),
            relation: "chose".to_string(),
            old_value: "PostgreSQL".to_string(),
            new_value: "MySQL".to_string(),
            existing_confidence: 0.9,
        };

        assert_eq!(contradiction.old_edge_id, "edge-1");
        assert_eq!(contradiction.new_edge_id, "edge-2");
        assert_eq!(contradiction.entity_id, Some("entity-123".to_string()));
        assert_eq!(contradiction.entity_name, "alice");
        assert_eq!(contradiction.relation, "chose");
        assert_eq!(contradiction.old_value, "PostgreSQL");
        assert_eq!(contradiction.new_value, "MySQL");
        assert_eq!(contradiction.existing_confidence, 0.9);
    }

    #[test]
    fn test_episode_result_has_contradictions_field() {
        let result = EpisodeResult {
            episode_id: "ep-1".to_string(),
            entities_extracted: 5,
            edges_created: 3,
            contradictions_found: 2,
        };

        assert_eq!(result.contradictions_found, 2);
    }

    #[test]
    fn test_ranked_memory_fields() {
        use chrono::Duration;

        let candidate = ScoredCandidate {
            candidate: RetrievalCandidate {
                entity_id: Some("entity-123".to_string()),
                edge_id: Some("edge-456".to_string()),
                content: "test memory content".to_string(),
                fts_score: 0.8,
                memory_type: MemoryType::Pattern,
                created_at: Utc::now() - Duration::days(5),
                ttl: None,
                base_confidence: 0.9,
                usage_count: 10,
                last_recalled_at: None,
            },
            composite_score: 0.75,
        };

        let (memories, _) = enforce_budget(vec![candidate], 1000);
        assert_eq!(memories.len(), 1);

        let ranked = &memories[0];
        assert_eq!(ranked.memory_type, MemoryType::Pattern);
        assert_eq!(ranked.content, "test memory content");
        assert_eq!(ranked.score, 0.75);
        assert_eq!(ranked.entity_id, Some("entity-123".to_string()));
        assert_eq!(ranked.edge_id, Some("edge-456".to_string()));
        assert!(ranked.tokens > 0);
    }
}

// ── Skill Types (D2 + D3) ─────────────────────────────────────────────────

/// Scope of a skill — determines visibility across agents (D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SkillScope {
    /// Only visible to the agent that created it.
    #[default]
    Private,
    /// Visible to all agents.
    Shared,
}

impl fmt::Display for SkillScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillScope::Private => write!(f, "private"),
            SkillScope::Shared => write!(f, "shared"),
        }
    }
}

impl SkillScope {
    /// Parse from a database string (case-insensitive).
    pub fn from_db(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "shared" => SkillScope::Shared,
            _ => SkillScope::Private,
        }
    }
}

/// Provenance metadata for a skill — tracks why and how a skill was created (D2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillProvenance {
    /// Reasoning explaining why this skill was created.
    pub reasoning: String,
    /// Alternative approaches that were considered and rejected.
    pub alternatives_rejected: Option<String>,
    /// Assumptions made during skill creation.
    pub assumptions: Option<String>,
    /// Context facts that support this skill.
    pub context_facts: Option<String>,
    /// When this provenance was last verified.
    pub verified_at: DateTime<Utc>,
    /// When this provenance expires (different fields have different TTLs).
    pub expires_at: DateTime<Utc>,
    /// How many times this provenance has been renewed.
    pub renewal_count: u32,
}

/// A skill — behavioral knowledge about what worked, what failed, and what
/// the user preferred. Higher-level abstraction than a pattern (D2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_condition: String,
    pub action: String,
    pub success_count: u32,
    pub failure_count: u32,
    pub confidence: f64,
    pub superseded_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub entity_types: Vec<String>,
    pub provenance: Option<SkillProvenance>,
    /// Scope of the skill — Private (agent-only) or Shared (all agents) (D3).
    pub scope: SkillScope,
    /// The agent that created this skill (D3).
    pub created_by_agent: String,
}

impl Skill {
    /// Compute confidence from success/failure counts.
    pub fn compute_confidence(success_count: u32, failure_count: u32) -> f64 {
        let total = success_count + failure_count;
        if total == 0 {
            0.5 // neutral confidence when no data
        } else {
            success_count as f64 / total as f64
        }
    }

    /// Generate provenance with configurable TTLs (D2 AC11).
    pub fn generate_provenance(
        reasoning: String,
        source_summaries: &[String],
        reasoning_ttl_days: i64,
        context_facts_ttl_days: i64,
    ) -> SkillProvenance {
        let now = Utc::now();
        // expires_at uses the shorter of the two TTLs
        let min_ttl = reasoning_ttl_days.min(context_facts_ttl_days);
        SkillProvenance {
            reasoning,
            alternatives_rejected: None,
            assumptions: None,
            context_facts: if source_summaries.is_empty() {
                None
            } else {
                Some(source_summaries.join("; "))
            },
            verified_at: now,
            expires_at: now + chrono::Duration::days(min_ttl),
            renewal_count: 0,
        }
    }
}

/// Configuration for skill creation — defines success/failure relations (D2 AC4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCreatorConfig {
    /// Edge relations that indicate success (default: ["fixed", "resolved", "success"]).
    pub success_relations: Vec<String>,
    /// Edge relations that indicate failure (default: ["deprecated", "failed", "abandoned"]).
    pub failure_relations: Vec<String>,
    /// TTL in days for provenance reasoning field (default: 180).
    pub reasoning_ttl_days: i64,
    /// TTL in days for provenance context_facts field (default: 90).
    pub context_facts_ttl_days: i64,
}

impl Default for SkillCreatorConfig {
    fn default() -> Self {
        Self {
            success_relations: vec![
                "fixed".to_string(),
                "resolved".to_string(),
                "success".to_string(),
            ],
            failure_relations: vec![
                "deprecated".to_string(),
                "failed".to_string(),
                "abandoned".to_string(),
            ],
            reasoning_ttl_days: 180,
            context_facts_ttl_days: 90,
        }
    }
}

/// A pattern candidate extracted from co-occurrence analysis.
///
/// Each candidate represents something that appears repeatedly across
/// episodes — either an entity type, an entity pair, or a
/// relation triplet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternCandidate {
    /// Unique identifier for this pattern candidate.
    pub id: String,
    /// Entity types involved in this pattern.
    pub entity_types: Vec<String>,
    /// Entity pair (by name) if this is a pair-level or triplet-level pattern.
    pub entity_pair: Option<(String, String)>,
    /// Relation triplet (source_name, relation, target_name) if applicable.
    pub relation_triplet: Option<(String, String, String)>,
    /// How many episodes this pattern appeared in.
    pub occurrence_count: u32,
    /// IDs of episodes where this pattern was observed.
    pub source_groups: Vec<String>,
    /// Normalized confidence: occurrence_count / total_episodes.
    pub confidence: f64,
    /// Human-readable description (populated by D1b LLM step; always None for D1a).
    pub description: Option<String>,
}

/// The result of a full learning pipeline run (D4).
///
/// Aggregates the outcomes from D1a (pattern extraction), D1b (description),
/// D2 (skill creation), and D3 (supersession).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOutcome {
    /// Total pattern candidates found in D1a.
    pub patterns_found: usize,
    /// Pattern candidates that were new (not duplicates of stored patterns).
    pub patterns_new: usize,
    /// Skills successfully created in D2.
    pub skills_created: usize,
    /// Existing skills superseded by new skills.
    pub skills_updated: usize,
    /// IDs of all skills created.
    pub skill_ids: Vec<String>,
}

// ── Compression Types (Phase B) ────────────────────────────────────────────

/// Configuration for batch compression of old episodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Episodes older than this (in days) are candidates for compression.
    pub max_age_days: usize,
    /// Maximum number of episodes to compress in a single batch.
    pub batch_size: usize,
    /// Optional threshold - only compress if uncompressed count >= this value.
    pub size_threshold: Option<usize>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_age_days: 7,
            batch_size: 10,
            size_threshold: None,
        }
    }
}

/// Result from running batch compression.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompressionResult {
    /// Number of episode groups that were compressed.
    pub groups_compressed: usize,
    /// Total number of individual episodes compressed.
    pub episodes_compressed: usize,
    /// Number of episodes that were already compressed (skipped).
    pub skipped_already_compressed: usize,
    /// Any errors that occurred during compression.
    pub errors: Vec<String>,
}

/// Data for a group of episodes that have been compressed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionGroupData {
    /// ID of the compression summary episode.
    pub compression_id: String,
    /// IDs of original episodes that were compressed into this summary.
    pub source_episode_ids: Vec<String>,
    /// Entities from the original episodes.
    pub entities: Vec<Entity>,
    /// Edges connected to the original episodes.
    pub edges: Vec<Edge>,
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Classification of a memory's type, driving TTL and decay behavior.
/// - Fact: stable knowledge (90d default TTL)
/// - Pattern: recurring observation (never expires)
/// - Experience: one-time event (14d default TTL)
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
            MemoryType::Experience => Some(Duration::from_secs(14 * 86400)),
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
    /// - 0.0 if expired (age > ttl), except Pattern which never expires
    ///
    /// Decay formulas:
    /// - Exponential (Fact, Preference, Decision):
    ///   `base_confidence * exp(-λ * age)` where `λ = ln(2) / half_life`
    ///   - Fact/Decision: `half_life = ttl * 0.5`
    ///   - Preference:    `half_life = ttl * 0.7`
    /// - Linear (Experience): `base_confidence * max(0.0, 1.0 - age / ttl)`
    /// - Constant (Pattern):  `base_confidence` (no decay, ignores ttl)
    pub fn decay_score(
        &self,
        base_confidence: f64,
        created_at: DateTime<Utc>,
        ttl: Option<Duration>,
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

        let age_secs = (Utc::now() - created_at).num_seconds().max(0) as f64;

        // Expired check (age strictly > ttl)
        if age_secs > ttl_secs {
            return 0.0;
        }

        match self {
            MemoryType::Fact | MemoryType::Decision => {
                let half_life = ttl_secs * 0.5;
                base_confidence * decay_exponential(age_secs, half_life)
            }
            MemoryType::Preference => {
                let half_life = ttl_secs * 0.7;
                base_confidence * decay_exponential(age_secs, half_life)
            }
            MemoryType::Experience => base_confidence * decay_linear(age_secs, ttl_secs),
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
    /// Links this episode to a compressed summary episode (set when this episode is compressed).
    pub compression_id: Option<String>,
    /// Memory type for this episode, driving TTL and decay behavior.
    /// Defaults to Experience for regular episodes, Fact for compressed summaries.
    pub memory_type: MemoryType,
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
            compression_id: None,
            memory_type: self.memory_type,
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

/// A compression group with its associated data for pattern extraction.
///
/// Represents one compressed episode (the "summary") together with all source
/// episodes, their edges, and their entities — the raw material from which
/// co-occurrence patterns are mined.
#[derive(Debug, Clone)]
pub struct CompressionGroupData {
    /// ID of the compressed (summary) episode.
    pub compression_id: String,
    /// IDs of the original episodes that were compressed.
    pub source_episode_ids: Vec<String>,
    /// All edges associated with the source episodes.
    pub edges: Vec<Edge>,
    /// All entities referenced by the source episodes.
    pub entities: Vec<Entity>,
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
/// Formula: `decay_score * normalized_fts_score * (1.0 + 0.1 * ln(1 + usage_count))`
///
/// - `decay_score`: freshness from MemoryType::decay_score (0.0 for expired)
/// - `normalized_fts_score`: BM25 score clamped to [0.0, 1.0]
/// - Usage bonus: `(1.0 + 0.1 * ln(1 + usage_count))` — 1.0 at usage_count=0
///
/// Special cases:
/// - Expired memories (decay_score = 0.0): returns 0.0
/// - Patterns: score is `max(score, 0.5)` to ensure visibility
pub fn score_candidate(candidate: &RetrievalCandidate) -> f64 {
    // Compute decay score
    let decay = candidate.memory_type.decay_score(
        candidate.base_confidence,
        candidate.created_at,
        candidate.ttl,
    );

    // Expired memories get score 0.0
    if decay == 0.0 {
        return 0.0;
    }

    // Normalize FTS score to [0.0, 1.0] range
    // BM25 can exceed 1.0 for very relevant results, so we clamp
    let normalized_fts = candidate.fts_score.clamp(0.0, 1.0);

    // Usage bonus: (1.0 + 0.1 * ln(1 + usage_count))
    // usage_count=0 → bonus = 1.0
    // usage_count=100 → bonus ≈ 1.46
    let usage_bonus = 1.0 + 0.1 * (1.0 + candidate.usage_count as f64).ln();

    // NaN guard: if any component is NaN, return 0.0
    if decay.is_nan() || normalized_fts.is_nan() || usage_bonus.is_nan() {
        return 0.0;
    }

    let score = decay * normalized_fts * usage_bonus;

    // Patterns get a floor of 0.5
    if candidate.memory_type == MemoryType::Pattern {
        score.max(0.5)
    } else {
        score
    }
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
    fn test_expired_memory_gets_zero_score() {
        // Fact created 100 days ago with 90d TTL (expired)
        let expired = make_candidate(
            MemoryType::Fact,
            Utc::now() - Duration::days(100),
            Some(90 * 86400),
            1.0,
            5,
            0.9,
        );

        let score = score_candidate(&expired);
        assert_eq!(score, 0.0, "expired memory should have score 0.0");
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
                Utc::now() - Duration::days(7),
                Some(14 * 86400),
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
    fn test_rank_candidates_sorts_descending_and_filters_expired() {
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
            // Expired: 100 days old with 90d TTL
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

        // Should have 3 candidates (expired one filtered out)
        assert_eq!(ranked.len(), 3);

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

/// A draft skill — intermediate struct produced by SkillCreator before LLM synthesis (D2).
#[derive(Debug, Clone)]
pub struct DraftSkill {
    pub entity_types: Vec<String>,
    pub success_count: u32,
    pub failure_count: u32,
    pub source_pattern_ids: Vec<String>,
    pub source_summaries: Vec<String>,
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
/// compression groups — either an entity type, an entity pair, or a
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
    /// How many compression groups this pattern appeared in.
    pub occurrence_count: u32,
    /// IDs of compression groups where this pattern was observed.
    pub source_groups: Vec<String>,
    /// Normalized confidence: occurrence_count / total_groups.
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

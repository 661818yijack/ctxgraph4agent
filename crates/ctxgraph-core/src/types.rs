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

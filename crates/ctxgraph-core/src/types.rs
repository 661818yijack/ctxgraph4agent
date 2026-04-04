use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::fmt;

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
}

/// Builder for constructing episodes with a fluent API.
pub struct EpisodeBuilder {
    content: String,
    source: Option<String>,
    metadata: serde_json::Map<String, serde_json::Value>,
    tags: Vec<String>,
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
        }
    }

    /// Create an entity with explicit memory_type and ttl.
    pub fn with_memory(name: &str, entity_type: &str, memory_type: MemoryType, ttl: Option<Duration>) -> Self {
        Self {
            id: uuid::Uuid::now_v7().to_string(),
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            memory_type,
            ttl,
            summary: None,
            created_at: Utc::now(),
            metadata: None,
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

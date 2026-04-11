//! Pattern extraction via co-occurrence counting.
//!
//! The `PatternExtractor` analyzes raw episodes to discover recurring
//! patterns — entity types, entity pairs, and relation triplets that appear
//! frequently across experiences. This is the "Learn" step in the
//! memory lifecycle: agents get smarter by recognizing what works, what fails,
//! and what the user prefers.
//!
//! No LLM calls are required — this is pure statistical analysis.

use std::collections::{HashMap, HashSet};

use crate::types::{Edge, Entity, Episode, PatternCandidate, PatternExtractorConfig};

/// Pure-logic pattern extractor.
///
/// Analyzes raw episodes with their entities and edges and returns ranked
/// pattern candidates based on co-occurrence counts. Stateless — all data
/// is passed in via the `extract` method.
pub struct PatternExtractor;

/// Trait for generating behavioral labels for a batch of pattern candidates.
///
/// Implementations call an LLM once with all candidates, returning a label per
/// candidate. The label is a 1-2 sentence behavioral description answering
/// "what should the agent do/avoid when this pattern recurs?"
pub trait BatchLabelDescriber: Send + Sync {
    /// Generate labels for all candidates in a single batch call.
    ///
    /// `source_summaries` maps pattern_id → episode content strings for context.
    ///
    /// Returns a Vec of `(candidate_id, label)` pairs. May return fewer results
    /// than input if some candidates are skipped.
    fn describe_batch(
        &self,
        candidates: &[PatternCandidate],
        source_summaries: &HashMap<String, Vec<String>>,
    ) -> impl std::future::Future<Output = crate::error::Result<Vec<(String, String)>>> + Send;
}

// ── Test implementations ──────────────────────────────────────────────────────

/// A `BatchLabelDescriber` that returns a deterministic label per candidate.
/// Useful for testing the pipeline without a real LLM.
pub struct MockBatchLabelDescriber;

impl BatchLabelDescriber for MockBatchLabelDescriber {
    async fn describe_batch(
        &self,
        candidates: &[PatternCandidate],
        _source_summaries: &HashMap<String, Vec<String>>,
    ) -> crate::error::Result<Vec<(String, String)>> {
        Ok(candidates
            .iter()
            .map(|c| {
                let label = if let Some(ref triplet) = c.relation_triplet {
                    let (src, rel, tgt) = triplet;
                    format!(
                        "When {} is involved, {} tends to trigger {} — anticipate this dependency.",
                        src, rel, tgt
                    )
                } else if let Some(ref pair) = c.entity_pair {
                    let (a, b) = pair;
                    format!(
                        "{} and {} frequently co-occur — consider their interdependence.",
                        a, b
                    )
                } else {
                    format!(
                        "Entity type(s) {} appear across multiple contexts — a recurring theme.",
                        c.entity_types.join(", ")
                    )
                };
                (c.id.clone(), label)
            })
            .collect())
    }
}

/// A `BatchLabelDescriber` that always returns an error.
/// Useful for testing LLM failure handling.
pub struct FailingBatchLabelDescriber {
    message: String,
}

impl FailingBatchLabelDescriber {
    /// Create a `FailingBatchLabelDescriber` with a custom error message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl BatchLabelDescriber for FailingBatchLabelDescriber {
    async fn describe_batch(
        &self,
        _candidates: &[PatternCandidate],
        _source_summaries: &HashMap<String, Vec<String>>,
    ) -> crate::error::Result<Vec<(String, String)>> {
        Err(crate::error::CtxGraphError::Extraction(
            self.message.clone(),
        ))
    }
}

/// Internal accumulator for a single co-occurrence entry.
struct OccurrenceEntry {
    count: u32,
    source_groups: HashSet<String>,
}

impl OccurrenceEntry {
    fn new(group_id: &str) -> Self {
        let mut sg = HashSet::new();
        sg.insert(group_id.to_string());
        Self {
            count: 1,
            source_groups: sg,
        }
    }

    fn increment(&mut self, group_id: &str) {
        self.count += 1;
        self.source_groups.insert(group_id.to_string());
    }
}

impl Default for PatternExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternExtractor {
    /// Create a new `PatternExtractor` instance.
    ///
    /// The extractor is stateless — this constructor exists for API consistency
    /// and potential future configuration.
    pub fn new() -> Self {
        Self
    }

    /// Extract pattern candidates from raw episodes, entities, and edges.
    ///
    /// Algorithm:
    /// 1. Build entity-id → name/type lookups from the entities list.
    /// 2. Iterate each edge, extracting entity types, entity pairs, and
    ///    relation triplets. Deduplicate per episode (using edge episode_id).
    /// 3. Count co-occurrences across episodes in HashMaps.
    /// 4. Filter by `min_occurrence_count`.
    /// 5. Build `PatternCandidate` for each qualifying entry.
    /// 6. Sort by `occurrence_count` descending, cap at `max_patterns_per_extraction`.
    ///
    /// Entity pair and triplet keys use entity IDs internally for accurate
    /// matching; the resulting candidates store entity *names* for readability.
    pub fn extract(
        &self,
        episodes: &[Episode],
        entities: &[Entity],
        edges: &[Edge],
        config: &PatternExtractorConfig,
    ) -> Vec<PatternCandidate> {
        if episodes.is_empty() || edges.is_empty() {
            return Vec::new();
        }

        let total_episodes = episodes.len();

        // Build entity-id → name/type lookup
        let entity_name_map: HashMap<String, String> = entities
            .iter()
            .map(|e| (e.id.clone(), e.name.clone()))
            .collect();

        let entity_type_map: HashMap<String, String> = entities
            .iter()
            .map(|e| (e.id.clone(), e.entity_type.clone()))
            .collect();

        // Accumulators
        let mut type_counts: HashMap<String, OccurrenceEntry> = HashMap::new();
        let mut pair_counts: HashMap<(String, String), OccurrenceEntry> = HashMap::new();
        let mut triplet_counts: HashMap<(String, String, String), OccurrenceEntry> = HashMap::new();

        // Track what we've already counted per episode to avoid overcounting
        let mut counted_entities_per_episode: HashMap<String, HashSet<String>> = HashMap::new();
        let mut counted_types_per_episode: HashMap<String, HashSet<String>> = HashMap::new();
        let mut counted_pairs_per_episode: HashMap<String, HashSet<(String, String)>> =
            HashMap::new();
        let mut counted_triplets_per_episode: HashMap<String, HashSet<(String, String, String)>> =
            HashMap::new();

        for edge in edges {
            let eid = match &edge.episode_id {
                Some(id) => id.clone(),
                None => continue, // skip edges not linked to an episode
            };

            let entities_entry = counted_entities_per_episode.entry(eid.clone()).or_default();
            let types_entry = counted_types_per_episode.entry(eid.clone()).or_default();
            let pairs_entry = counted_pairs_per_episode.entry(eid.clone()).or_default();
            let triplets_entry = counted_triplets_per_episode.entry(eid.clone()).or_default();

            // Resolve source entity name/type
            let source_name = entity_name_map
                .get(&edge.source_id)
                .cloned()
                .unwrap_or_else(|| edge.source_id.clone());
            let target_name = entity_name_map
                .get(&edge.target_id)
                .cloned()
                .unwrap_or_else(|| edge.target_id.clone());

            // Count entity types for source
            if let Some(src_type) = entity_type_map.get(&edge.source_id)
                && entities_entry.insert(edge.source_id.clone())
                    && types_entry.insert(src_type.clone())
                {
                    type_counts
                        .entry(src_type.clone())
                        .and_modify(|e| e.increment(&eid))
                        .or_insert_with(|| OccurrenceEntry::new(&eid));
                }

            // Count entity types for target
            if let Some(tgt_type) = entity_type_map.get(&edge.target_id)
                && entities_entry.insert(edge.target_id.clone())
                    && types_entry.insert(tgt_type.clone())
                {
                    type_counts
                        .entry(tgt_type.clone())
                        .and_modify(|e| e.increment(&eid))
                        .or_insert_with(|| OccurrenceEntry::new(&eid));
                }

            // Count the entity pair (sorted by name for canonical ordering)
            let pair_key = if source_name <= target_name {
                (source_name.clone(), target_name.clone())
            } else {
                (target_name.clone(), source_name.clone())
            };

            if pairs_entry.insert(pair_key.clone()) {
                pair_counts
                    .entry(pair_key)
                    .and_modify(|e| e.increment(&eid))
                    .or_insert_with(|| OccurrenceEntry::new(&eid));
            }

            // Count the relation triplet (directional — order matters)
            let triplet_key = (
                source_name.clone(),
                edge.relation.clone(),
                target_name.clone(),
            );
            if triplets_entry.insert(triplet_key.clone()) {
                triplet_counts
                    .entry(triplet_key)
                    .and_modify(|e| e.increment(&eid))
                    .or_insert_with(|| OccurrenceEntry::new(&eid));
            }
        }

        let min_count = config.min_occurrence_count;
        let mut candidates: Vec<PatternCandidate> = Vec::new();

        // Entity type candidates
        for (entity_type, entry) in &type_counts {
            if entry.count >= min_count {
                candidates.push(PatternCandidate {
                    id: uuid::Uuid::now_v7().to_string(),
                    entity_types: vec![entity_type.clone()],
                    entity_pair: None,
                    relation_triplet: None,
                    occurrence_count: entry.count,
                    source_groups: entry.source_groups.iter().cloned().collect(),
                    confidence: entry.count as f64 / total_episodes as f64,
                    description: None,
                });
            }
        }

        // Entity pair candidates
        for ((a, b), entry) in &pair_counts {
            if entry.count >= min_count {
                let mut pair_entity_types: Vec<String> = Vec::new();
                for (eid, ename) in &entity_name_map {
                    if (ename == a || ename == b)
                        && let Some(et) = entity_type_map.get(eid)
                            && !pair_entity_types.contains(et) {
                                pair_entity_types.push(et.clone());
                            }
                }

                candidates.push(PatternCandidate {
                    id: uuid::Uuid::now_v7().to_string(),
                    entity_types: pair_entity_types,
                    entity_pair: Some((a.clone(), b.clone())),
                    relation_triplet: None,
                    occurrence_count: entry.count,
                    source_groups: entry.source_groups.iter().cloned().collect(),
                    confidence: entry.count as f64 / total_episodes as f64,
                    description: None,
                });
            }
        }

        // Relation triplet candidates
        for ((a, rel, b), entry) in &triplet_counts {
            if entry.count >= min_count {
                let mut triplet_entity_types: Vec<String> = Vec::new();
                for (eid, ename) in &entity_name_map {
                    if (ename == a || ename == b)
                        && let Some(et) = entity_type_map.get(eid)
                            && !triplet_entity_types.contains(et) {
                                triplet_entity_types.push(et.clone());
                            }
                }

                candidates.push(PatternCandidate {
                    id: uuid::Uuid::now_v7().to_string(),
                    entity_types: triplet_entity_types,
                    entity_pair: Some((a.clone(), b.clone())),
                    relation_triplet: Some((a.clone(), rel.clone(), b.clone())),
                    occurrence_count: entry.count,
                    source_groups: entry.source_groups.iter().cloned().collect(),
                    confidence: entry.count as f64 / total_episodes as f64,
                    description: None,
                });
            }
        }

        // Sort by occurrence_count descending
        candidates.sort_by(|a, b| {
            b.occurrence_count.cmp(&a.occurrence_count).then_with(|| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        // Filter by min_entity_types: only applies to entity-type-only candidates.
        let min_types = config.min_entity_types;
        candidates.retain(|c| {
            if c.entity_pair.is_some() || c.relation_triplet.is_some() {
                return true;
            }
            c.entity_types.len() >= min_types
        });

        // Cap at max_patterns_per_extraction
        candidates.truncate(config.max_patterns_per_extraction);

        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Edge, Entity, MemoryType};
    use chrono::Utc;
    use std::time::Duration;

    /// Helper to create a test entity with a fixed (deterministic) ID.
    fn make_entity(id: &str, name: &str, entity_type: &str) -> Entity {
        Entity {
            id: id.to_string(),
            name: name.to_string(),
            entity_type: entity_type.to_string(),
            memory_type: MemoryType::Fact,
            ttl: Some(Duration::from_secs(90 * 86400)),
            summary: None,
            created_at: Utc::now(),
            metadata: None,
            usage_count: 0,
            last_recalled_at: None,
        }
    }

    /// Helper to create a test edge with a fixed ID.
    fn make_edge(
        id: &str,
        source_id: &str,
        target_id: &str,
        relation: &str,
        episode_id: &str,
    ) -> Edge {
        Edge {
            id: id.to_string(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            relation: relation.to_string(),
            memory_type: MemoryType::Fact,
            ttl: Some(Duration::from_secs(90 * 86400)),
            fact: None,
            valid_from: None,
            valid_until: None,
            recorded_at: Utc::now(),
            confidence: 1.0,
            episode_id: Some(episode_id.to_string()),
            metadata: None,
            usage_count: 0,
            last_recalled_at: None,
        }
    }

    /// Helper to create a test episode with a fixed (deterministic) ID.
    fn make_episode(id: &str) -> Episode {
        Episode {
            id: id.to_string(),
            content: format!("Test episode {id}"),
            source: Some("test".to_string()),
            recorded_at: Utc::now(),
            metadata: None,
            memory_type: MemoryType::Experience,
            compression_id: None,
        }
    }

    // ── Unit tests ──

    #[tokio::test]
    async fn test_empty_episodes_returns_empty_candidates() {
        let episodes: Vec<Episode> = Vec::new();
        let entities: Vec<Entity> = Vec::new();
        let edges: Vec<Edge> = Vec::new();
        let config = PatternExtractorConfig::default();
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);
        assert!(candidates.is_empty());
    }

    #[tokio::test]
    async fn test_two_episodes_below_threshold() {
        // Two episodes sharing the same entity pair but threshold is 3 → no candidates
        let episodes = vec![make_episode("ep1"), make_episode("ep2")];
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges = vec![
            make_edge("edge1", "e1", "e2", "depends_on", "ep1"),
            make_edge("edge2", "e1", "e2", "depends_on", "ep2"),
        ];

        let config = PatternExtractorConfig::default(); // min_occurrence_count = 3
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);
        assert!(
            candidates.is_empty(),
            "expected no candidates with only 2 episodes and threshold 3"
        );
    }

    #[tokio::test]
    async fn test_four_episodes_sharing_entity_pair_finds_candidate() {
        let episodes: Vec<Episode> = (1..=4).map(|i| make_episode(&format!("ep{i}"))).collect();
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges: Vec<Edge> = (1..=4)
            .map(|i| {
                make_edge(
                    &format!("edge{i}"),
                    "e1",
                    "e2",
                    "depends_on",
                    &format!("ep{i}"),
                )
            })
            .collect();

        let config = PatternExtractorConfig::default(); // min_occurrence_count = 3
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        // Should find at least the entity pair and triplet candidates
        assert!(
            !candidates.is_empty(),
            "expected candidates with 4 episodes sharing same pair"
        );

        // The triplet (Docker, depends_on, Network) should have count 4
        let triplet = candidates.iter().find(|c| c.relation_triplet.is_some());
        assert!(triplet.is_some(), "expected a triplet candidate");
        let t = triplet.unwrap();
        assert_eq!(t.occurrence_count, 4);
        assert_eq!(
            t.relation_triplet.as_ref().unwrap(),
            &(
                String::from("Docker"),
                String::from("depends_on"),
                String::from("Network")
            )
        );
    }

    #[tokio::test]
    async fn test_min_occurrence_count_one_returns_more_candidates() {
        let episodes = vec![make_episode("ep1"), make_episode("ep2")];
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges = vec![
            make_edge("edge1", "e1", "e2", "depends_on", "ep1"),
            make_edge("edge2", "e1", "e2", "depends_on", "ep2"),
        ];

        // With threshold 3 → no candidates
        let config3 = PatternExtractorConfig {
            min_occurrence_count: 3,
            ..Default::default()
        };
        let candidates3 = PatternExtractor::new().extract(&episodes, &entities, &edges, &config3);
        assert!(candidates3.is_empty());

        // With threshold 1 → should find candidates
        let config1 = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates1 = PatternExtractor::new().extract(&episodes, &entities, &edges, &config1);
        assert!(
            !candidates1.is_empty(),
            "expected candidates with threshold 1"
        );
        assert!(candidates1.len() > candidates3.len());
    }

    #[tokio::test]
    async fn test_results_capped_at_max_patterns() {
        // Create 5 episodes each with unique entity types to generate many candidates
        let episodes: Vec<Episode> = (1..=5).map(|i| make_episode(&format!("ep{i}"))).collect();
        let mut entities = Vec::new();
        let mut edges = Vec::new();
        for i in 1..=5 {
            entities.push(make_entity(
                &format!("e{i}a"),
                &format!("Entity{i}A"),
                "TypeA",
            ));
            entities.push(make_entity(
                &format!("e{i}b"),
                &format!("Entity{i}B"),
                "TypeB",
            ));
            edges.push(make_edge(
                &format!("edge{i}"),
                &format!("e{i}a"),
                &format!("e{i}b"),
                &format!("rel{i}"),
                &format!("ep{i}"),
            ));
        }

        // With threshold 1 and max 3, should get at most 3 candidates
        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            max_patterns_per_extraction: 3,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);
        assert!(
            candidates.len() <= 3,
            "expected at most 3 candidates, got {}",
            candidates.len()
        );
    }

    #[tokio::test]
    async fn test_confidence_is_normalized() {
        // 4 episodes, all sharing same pair → confidence should be 4/4 = 1.0
        let episodes: Vec<Episode> = (1..=4).map(|i| make_episode(&format!("ep{i}"))).collect();
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges: Vec<Edge> = (1..=4)
            .map(|i| {
                make_edge(
                    &format!("edge{i}"),
                    "e1",
                    "e2",
                    "depends_on",
                    &format!("ep{i}"),
                )
            })
            .collect();

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        // Every candidate should have confidence = count / 4
        for c in &candidates {
            let expected = c.occurrence_count as f64 / 4.0;
            assert!(
                (c.confidence - expected).abs() < f64::EPSILON,
                "expected confidence {expected}, got {}",
                c.confidence
            );
        }
    }

    #[tokio::test]
    async fn test_source_groups_correctly_populated() {
        let episodes: Vec<Episode> = (1..=3).map(|i| make_episode(&format!("ep{i}"))).collect();
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges: Vec<Edge> = (1..=3)
            .map(|i| {
                make_edge(
                    &format!("edge{i}"),
                    "e1",
                    "e2",
                    "depends_on",
                    &format!("ep{i}"),
                )
            })
            .collect();

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        let triplet = candidates
            .iter()
            .find(|c| c.relation_triplet.is_some())
            .expect("expected triplet candidate");

        assert_eq!(triplet.source_groups.len(), 3);
        assert!(triplet.source_groups.contains(&"ep1".to_string()));
        assert!(triplet.source_groups.contains(&"ep2".to_string()));
        assert!(triplet.source_groups.contains(&"ep3".to_string()));
    }

    #[tokio::test]
    async fn test_results_ranked_by_occurrence_count_descending() {
        // Create a scenario where one entity type appears in all episodes,
        // but another pair only appears in some.
        let episodes: Vec<Episode> = (1..=5).map(|i| make_episode(&format!("ep{i}"))).collect();
        let mut entities = vec![make_entity("e1", "Docker", "Component")];
        let mut edges: Vec<Edge> = Vec::new();

        // Only episodes 1-4 have the Docker-Network pair
        entities.push(make_entity("e2", "Network", "Component"));
        for i in 1..=4 {
            edges.push(make_edge(
                &format!("edge{i}"),
                "e1",
                "e2",
                "depends_on",
                &format!("ep{i}"),
            ));
        }

        // Only episodes 1-2 have Docker-Volume pair
        entities.push(make_entity("e3", "Volume", "Component"));
        for i in 1..=2 {
            edges.push(make_edge(
                &format!("edgevol{i}"),
                "e1",
                "e3",
                "mounts",
                &format!("ep{i}"),
            ));
        }

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        // First candidate should have highest count (Component type appears in all 5)
        if candidates.len() >= 2 {
            assert!(
                candidates[0].occurrence_count >= candidates[1].occurrence_count,
                "candidates should be sorted by occurrence_count descending"
            );
        }
    }

    #[tokio::test]
    async fn test_entity_pair_canonical_ordering() {
        // Entity pair should be stored in sorted (canonical) order
        let episodes = vec![make_episode("ep1")];
        let entities = vec![
            make_entity("e1", "Zebra", "Animal"),
            make_entity("e2", "Apple", "Fruit"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "eats", "ep1")];

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        let pair = candidates
            .iter()
            .find(|c| c.entity_pair.is_some())
            .expect("expected a pair candidate");

        // Should be ("Apple", "Zebra") — alphabetical, not ("Zebra", "Apple")
        let (a, b) = pair.entity_pair.as_ref().unwrap();
        assert!(
            a <= b,
            "entity pair should be in canonical order: got ({a}, {b})"
        );
    }

    #[tokio::test]
    async fn test_batch_describer_returns_all() {
        let episodes = vec![
            make_episode("ep1"),
            make_episode("ep2"),
            make_episode("ep3"),
        ];
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Infrastructure"),
        ];
        let edges: Vec<Edge> = (1..=3)
            .map(|i| {
                make_edge(
                    &format!("edge{i}"),
                    "e1",
                    "e2",
                    "depends_on",
                    &format!("ep{i}"),
                )
            })
            .collect();

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);
        assert!(!candidates.is_empty());

        let describer = MockBatchLabelDescriber;
        let summaries = HashMap::new();
        let results = describer
            .describe_batch(&candidates, &summaries)
            .await
            .unwrap();

        assert_eq!(
            results.len(),
            candidates.len(),
            "should return one label per candidate"
        );
        for (id, label) in &results {
            assert!(!id.is_empty());
            assert!(!label.is_empty());
        }
        // Every returned id must correspond to a real candidate id
        let candidate_ids: std::collections::HashSet<_> =
            candidates.iter().map(|c| &c.id).collect();
        for (id, _) in &results {
            assert!(
                candidate_ids.contains(id),
                "returned id {id} not in candidates"
            );
        }
    }

    #[tokio::test]
    async fn test_batch_describer_empty() {
        let describer = MockBatchLabelDescriber;
        let results = describer
            .describe_batch(&[], &HashMap::new())
            .await
            .unwrap();
        assert!(
            results.is_empty(),
            "empty input should produce empty output"
        );
    }

    #[tokio::test]
    async fn test_failing_batch_describer_returns_error() {
        let describer = FailingBatchLabelDescriber::new("LLM unavailable");
        let result = describer.describe_batch(&[], &HashMap::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_description_always_none() {
        let episodes = vec![make_episode("ep1")];
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "depends_on", "ep1")];

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        for c in &candidates {
            assert!(
                c.description.is_none(),
                "D1a should never produce descriptions"
            );
        }
    }

    #[tokio::test]
    async fn test_entity_types_populated_on_pair_candidate() {
        let episodes = vec![make_episode("ep1")];
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Infrastructure"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "depends_on", "ep1")];

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&episodes, &entities, &edges, &config);

        let pair = candidates
            .iter()
            .find(|c| c.entity_pair.is_some())
            .expect("expected a pair candidate");

        assert!(
            !pair.entity_types.is_empty(),
            "entity_types should be populated on pair candidate"
        );
        assert!(
            pair.entity_types.contains(&"Component".to_string()),
            "expected 'Component' in entity_types"
        );
        assert!(
            pair.entity_types.contains(&"Infrastructure".to_string()),
            "expected 'Infrastructure' in entity_types"
        );
    }
}

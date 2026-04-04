//! Pattern extraction via co-occurrence counting.
//!
//! The `PatternExtractor` analyzes compression groups to discover recurring
//! patterns — entity types, entity pairs, and relation triplets that appear
//! frequently across compressed experiences. This is the "Learn" step in the
//! memory lifecycle: agents get smarter by recognizing what works, what fails,
//! and what the user prefers.
//!
//! No LLM calls are required — this is pure statistical analysis.

use std::collections::{HashMap, HashSet};

use crate::types::{CompressionGroupData, PatternCandidate, PatternExtractorConfig};

/// Pure-logic pattern extractor.
///
/// Analyzes compression groups and returns ranked pattern candidates based
/// on co-occurrence counts. Stateless — all data is passed in via the
/// `extract` method.
pub struct PatternExtractor;

/// Trait for generating behavioral descriptions of pattern candidates (D1b).
///
/// Implementations can use an LLM or any other mechanism to produce a 1-2 sentence
/// description capturing the behavioral insight represented by a pattern.
///
/// The description should answer "what does this pattern mean for the agent?"
/// — e.g., "When Docker networking issues occur, the agent should check DNS
/// configuration first" — NOT co-occurrence metadata like "Docker and Network
/// appeared together 5 times."
pub trait PatternDescriber {
    /// Generate a behavioral description for the given pattern candidate.
    ///
    /// `source_summaries` contains the content of compression group summary episodes
    /// that this pattern was extracted from, providing context for the LLM.
    fn generate(
        &self,
        candidate: &PatternCandidate,
        source_summaries: &[String],
    ) -> crate::error::Result<String>;
}

// ── Test implementations ──────────────────────────────────────────────────────

/// A PatternDescriber implementation that returns hardcoded behavioral descriptions.
/// Useful for testing the D1b pipeline without a real LLM.
pub struct MockPatternDescriber {
    /// Fixed description to return for all candidates.
    description: String,
}

impl MockPatternDescriber {
    /// Create a MockPatternDescriber with a fixed description.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
        }
    }

    /// Create a MockPatternDescriber that generates a description based on candidate data.
    pub fn for_candidate(candidate: &PatternCandidate) -> Self {
        // Generate a realistic behavioral description based on candidate type
        let description = if let Some(ref triplet) = candidate.relation_triplet {
            let (src, rel, tgt) = triplet;
            format!(
                "When {} is involved, {} tends to trigger {} — this pattern has been observed repeatedly and the agent should anticipate this dependency.",
                src, rel, tgt
            )
        } else if let Some(ref pair) = candidate.entity_pair {
            let (a, b) = pair;
            format!(
                "The {} and {} frequently co-occur in problem scenarios — the agent should consider their interdependence when troubleshooting.",
                a, b
            )
        } else {
            let types = candidate.entity_types.join(", ");
            format!(
                "Entity type(s) {} appear across multiple contexts — this suggests a recurring theme that warrants attention.",
                types
            )
        };
        Self { description }
    }
}

impl PatternDescriber for MockPatternDescriber {
    fn generate(
        &self,
        _candidate: &PatternCandidate,
        _source_summaries: &[String],
    ) -> crate::error::Result<String> {
        Ok(self.description.clone())
    }
}

/// A PatternDescriber implementation that always returns an error.
/// Useful for testing LLM failure handling in the D1b pipeline.
pub struct FailingPatternDescriber {
    message: String,
}

impl FailingPatternDescriber {
    /// Create a FailingPatternDescriber with a custom error message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl PatternDescriber for FailingPatternDescriber {
    fn generate(
        &self,
        _candidate: &PatternCandidate,
        _source_summaries: &[String],
    ) -> crate::error::Result<String> {
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

impl PatternExtractor {
    /// Create a new `PatternExtractor` instance.
    ///
    /// The extractor is stateless — this constructor exists for API consistency
    /// and potential future configuration.
    pub fn new() -> Self {
        Self
    }

    /// Extract pattern candidates from compression group data.
    ///
    /// Algorithm:
    /// 1. Iterate each compression group, extracting entity types, entity
    ///    pairs, and relation triplets from its edges and entities.
    /// 2. Count co-occurrences across groups in HashMaps.
    /// 3. Filter by `min_occurrence_count`.
    /// 4. Build `PatternCandidate` for each qualifying entry.
    /// 5. Sort by `occurrence_count` descending, cap at `max_patterns_per_extraction`.
    ///
    /// Entity pair and triplet keys use entity IDs internally for accurate
    /// matching; the resulting candidates store entity *names* for readability.
    pub fn extract(
        &self,
        groups: &[CompressionGroupData],
        config: &PatternExtractorConfig,
    ) -> Vec<PatternCandidate> {
        if groups.is_empty() {
            return Vec::new();
        }

        let total_groups = groups.len();

        // Build entity-id → name lookup per group (and globally for resolution)
        let entity_name_map: HashMap<String, String> = groups
            .iter()
            .flat_map(|g| g.entities.iter())
            .map(|e| (e.id.clone(), e.name.clone()))
            .collect();

        let entity_type_map: HashMap<String, String> = groups
            .iter()
            .flat_map(|g| g.entities.iter())
            .map(|e| (e.id.clone(), e.entity_type.clone()))
            .collect();

        // Accumulators
        let mut type_counts: HashMap<String, OccurrenceEntry> = HashMap::new();
        let mut pair_counts: HashMap<(String, String), OccurrenceEntry> = HashMap::new();
        let mut triplet_counts: HashMap<(String, String, String), OccurrenceEntry> = HashMap::new();

        for group in groups {
            let gid = &group.compression_id;
            let mut counted_this_group: HashSet<String> = HashSet::new();
            let mut counted_types_this_group: HashSet<String> = HashSet::new();

            // Count entity types from all entities in the group (dedup by type)
            for entity in &group.entities {
                if counted_this_group.insert(entity.id.clone()) {
                    if counted_types_this_group.insert(entity.entity_type.clone()) {
                        type_counts
                            .entry(entity.entity_type.clone())
                            .and_modify(|e| e.increment(gid))
                            .or_insert_with(|| OccurrenceEntry::new(gid));
                    }
                }
            }

            // Track deduplicated pair/triplet keys per group to avoid overcounting
            let mut counted_pairs_this_group: HashSet<(String, String)> = HashSet::new();
            let mut counted_triplets_this_group: HashSet<(String, String, String)> = HashSet::new();

            // Count entity pairs and relation triplets from edges
            for edge in &group.edges {
                // Resolve source entity name/type
                let source_name = entity_name_map
                    .get(&edge.source_id)
                    .cloned()
                    .unwrap_or_else(|| edge.source_id.clone());
                let target_name = entity_name_map
                    .get(&edge.target_id)
                    .cloned()
                    .unwrap_or_else(|| edge.target_id.clone());

                // Count the entity pair (sorted by name for canonical ordering)
                // Dedup per group: only count each unique pair once per group
                let pair_key = if source_name <= target_name {
                    (source_name.clone(), target_name.clone())
                } else {
                    (target_name.clone(), source_name.clone())
                };

                if counted_pairs_this_group.insert(pair_key.clone()) {
                    pair_counts
                        .entry(pair_key)
                        .and_modify(|e| e.increment(gid))
                        .or_insert_with(|| OccurrenceEntry::new(gid));
                }

                // Count the relation triplet (directional — order matters)
                // Dedup per group: only count each unique triplet once per group
                let triplet_key = (
                    source_name.clone(),
                    edge.relation.clone(),
                    target_name.clone(),
                );
                if counted_triplets_this_group.insert(triplet_key.clone()) {
                    triplet_counts
                        .entry(triplet_key)
                        .and_modify(|e| e.increment(gid))
                        .or_insert_with(|| OccurrenceEntry::new(gid));
                }

                // Also count entity types for source and target if not already counted
                if let Some(src_type) = entity_type_map.get(&edge.source_id) {
                    if counted_types_this_group.insert(src_type.clone()) {
                        type_counts
                            .entry(src_type.clone())
                            .and_modify(|e| e.increment(gid))
                            .or_insert_with(|| OccurrenceEntry::new(gid));
                    }
                }
                if let Some(tgt_type) = entity_type_map.get(&edge.target_id) {
                    if counted_types_this_group.insert(tgt_type.clone()) {
                        type_counts
                            .entry(tgt_type.clone())
                            .and_modify(|e| e.increment(gid))
                            .or_insert_with(|| OccurrenceEntry::new(gid));
                    }
                }
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
                    confidence: entry.count as f64 / total_groups as f64,
                    description: None,
                });
            }
        }

        // Entity pair candidates
        for ((a, b), entry) in &pair_counts {
            if entry.count >= min_count {
                // Collect entity types from entities with matching names
                let mut pair_entity_types: Vec<String> = Vec::new();
                for (eid, ename) in &entity_name_map {
                    if ename == a || ename == b {
                        if let Some(et) = entity_type_map.get(eid) {
                            if !pair_entity_types.contains(et) {
                                pair_entity_types.push(et.clone());
                            }
                        }
                    }
                }

                candidates.push(PatternCandidate {
                    id: uuid::Uuid::now_v7().to_string(),
                    entity_types: pair_entity_types,
                    entity_pair: Some((a.clone(), b.clone())),
                    relation_triplet: None,
                    occurrence_count: entry.count,
                    source_groups: entry.source_groups.iter().cloned().collect(),
                    confidence: entry.count as f64 / total_groups as f64,
                    description: None,
                });
            }
        }

        // Relation triplet candidates
        for ((a, rel, b), entry) in &triplet_counts {
            if entry.count >= min_count {
                let mut triplet_entity_types: Vec<String> = Vec::new();
                for (eid, ename) in &entity_name_map {
                    if ename == a || ename == b {
                        if let Some(et) = entity_type_map.get(eid) {
                            if !triplet_entity_types.contains(et) {
                                triplet_entity_types.push(et.clone());
                            }
                        }
                    }
                }

                candidates.push(PatternCandidate {
                    id: uuid::Uuid::now_v7().to_string(),
                    entity_types: triplet_entity_types,
                    entity_pair: Some((a.clone(), b.clone())),
                    relation_triplet: Some((a.clone(), rel.clone(), b.clone())),
                    occurrence_count: entry.count,
                    source_groups: entry.source_groups.iter().cloned().collect(),
                    confidence: entry.count as f64 / total_groups as f64,
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
        // Pair/triplet candidates inherently represent multi-entity patterns and
        // are always kept regardless of entity type dedup.
        let min_types = config.min_entity_types;
        candidates.retain(|c| {
            // Always keep pair/triplet candidates
            if c.entity_pair.is_some() || c.relation_triplet.is_some() {
                return true;
            }
            // Entity-type-only candidates: filter by min_entity_types
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

    /// Helper to build a compression group.
    fn make_group(
        compression_id: &str,
        source_episode_ids: &[&str],
        entities: Vec<Entity>,
        edges: Vec<Edge>,
    ) -> CompressionGroupData {
        CompressionGroupData {
            compression_id: compression_id.to_string(),
            source_episode_ids: source_episode_ids.iter().map(|s| s.to_string()).collect(),
            edges,
            entities,
        }
    }

    // ── Unit tests ──

    #[test]
    fn test_empty_groups_returns_empty_candidates() {
        let groups: Vec<CompressionGroupData> = Vec::new();
        let config = PatternExtractorConfig::default();
        let candidates = PatternExtractor::new().extract(&groups, &config);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_two_groups_below_threshold() {
        // Two groups sharing the same entity pair but threshold is 3 → no candidates
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "depends_on", "ep1")];

        let group1 = make_group("comp1", &["ep1"], entities.clone(), edges.clone());
        let group2 = make_group("comp2", &["ep2"], entities, edges);

        let groups = vec![group1, group2];
        let config = PatternExtractorConfig::default(); // min_occurrence_count = 3
        let candidates = PatternExtractor::new().extract(&groups, &config);
        assert!(
            candidates.is_empty(),
            "expected no candidates with only 2 groups and threshold 3"
        );
    }

    #[test]
    fn test_four_groups_sharing_entity_pair_finds_candidate() {
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let _edge = make_edge("edge1", "e1", "e2", "depends_on", "ep1");

        let mut groups = Vec::new();
        for i in 1..=4 {
            let edges = vec![make_edge(
                &format!("edge{i}"),
                "e1",
                "e2",
                "depends_on",
                &format!("ep{i}"),
            )];
            groups.push(make_group(
                &format!("comp{i}"),
                &[&format!("ep{i}")],
                entities.clone(),
                edges,
            ));
        }

        let config = PatternExtractorConfig::default(); // min_occurrence_count = 3
        let candidates = PatternExtractor::new().extract(&groups, &config);

        // Should find at least the entity pair and triplet candidates
        assert!(
            !candidates.is_empty(),
            "expected candidates with 4 groups sharing same pair"
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

    #[test]
    fn test_min_occurrence_count_one_returns_more_candidates() {
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edge = make_edge("edge1", "e1", "e2", "depends_on", "ep1");

        let group1 = make_group("comp1", &["ep1"], entities.clone(), vec![edge.clone()]);
        let group2 = make_group("comp2", &["ep2"], entities, vec![edge]);

        let groups = vec![group1, group2];

        // With threshold 3 → no candidates
        let config3 = PatternExtractorConfig {
            min_occurrence_count: 3,
            ..Default::default()
        };
        let candidates3 = PatternExtractor::new().extract(&groups, &config3);
        assert!(candidates3.is_empty());

        // With threshold 1 → should find candidates
        let config1 = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates1 = PatternExtractor::new().extract(&groups, &config1);
        assert!(
            !candidates1.is_empty(),
            "expected candidates with threshold 1"
        );
        assert!(candidates1.len() > candidates3.len());
    }

    #[test]
    fn test_results_capped_at_max_patterns() {
        // Create 5 groups each with unique entity types to generate many candidates
        let mut groups = Vec::new();
        for i in 1..=5 {
            let entities = vec![
                make_entity(&format!("e{i}a"), &format!("Entity{i}A"), "TypeA"),
                make_entity(&format!("e{i}b"), &format!("Entity{i}B"), "TypeB"),
            ];
            let edges = vec![make_edge(
                &format!("edge{i}"),
                &format!("e{i}a"),
                &format!("e{i}b"),
                &format!("rel{i}"),
                &format!("ep{i}"),
            )];
            groups.push(make_group(
                &format!("comp{i}"),
                &[&format!("ep{i}")],
                entities,
                edges,
            ));
        }

        // With threshold 1 and max 3, should get at most 3 candidates
        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            max_patterns_per_extraction: 3,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);
        assert!(
            candidates.len() <= 3,
            "expected at most 3 candidates, got {}",
            candidates.len()
        );
    }

    #[test]
    fn test_confidence_is_normalized() {
        // 4 groups, all sharing same pair → confidence should be 4/4 = 1.0
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];

        let mut groups = Vec::new();
        for i in 1..=4 {
            let edges = vec![make_edge(
                &format!("edge{i}"),
                "e1",
                "e2",
                "depends_on",
                &format!("ep{i}"),
            )];
            groups.push(make_group(
                &format!("comp{i}"),
                &[&format!("ep{i}")],
                entities.clone(),
                edges,
            ));
        }

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);

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

    #[test]
    fn test_source_groups_correctly_populated() {
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];

        let mut groups = Vec::new();
        for i in 1..=3 {
            let edges = vec![make_edge(
                &format!("edge{i}"),
                "e1",
                "e2",
                "depends_on",
                &format!("ep{i}"),
            )];
            groups.push(make_group(
                &format!("comp{i}"),
                &[&format!("ep{i}")],
                entities.clone(),
                edges,
            ));
        }

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);

        let triplet = candidates
            .iter()
            .find(|c| c.relation_triplet.is_some())
            .expect("expected triplet candidate");

        assert_eq!(triplet.source_groups.len(), 3);
        assert!(triplet.source_groups.contains(&"comp1".to_string()));
        assert!(triplet.source_groups.contains(&"comp2".to_string()));
        assert!(triplet.source_groups.contains(&"comp3".to_string()));
    }

    #[test]
    fn test_results_ranked_by_occurrence_count_descending() {
        // Create a scenario where one entity type appears in all groups,
        // but another pair only appears in some.
        let mut groups = Vec::new();
        for i in 1..=5 {
            let mut entities = vec![make_entity("e1", "Docker", "Component")];
            let mut edges = Vec::new();

            // Only groups 1-4 have the Docker-Network pair
            if i <= 4 {
                entities.push(make_entity("e2", "Network", "Component"));
                edges.push(make_edge(
                    &format!("edge{i}"),
                    "e1",
                    "e2",
                    "depends_on",
                    &format!("ep{i}"),
                ));
            }

            // Only groups 1-2 have Docker-Volume pair
            if i <= 2 {
                entities.push(make_entity("e3", "Volume", "Component"));
                edges.push(make_edge(
                    &format!("edgevol{i}"),
                    "e1",
                    "e3",
                    "mounts",
                    &format!("ep{i}"),
                ));
            }

            groups.push(make_group(
                &format!("comp{i}"),
                &[&format!("ep{i}")],
                entities,
                edges,
            ));
        }

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);

        // First candidate should have highest count (Component type appears in all 5)
        if candidates.len() >= 2 {
            assert!(
                candidates[0].occurrence_count >= candidates[1].occurrence_count,
                "candidates should be sorted by occurrence_count descending"
            );
        }
    }

    #[test]
    fn test_entity_pair_canonical_ordering() {
        // Entity pair should be stored in sorted (canonical) order
        let entities = vec![
            make_entity("e1", "Zebra", "Animal"),
            make_entity("e2", "Apple", "Fruit"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "eats", "ep1")];

        let group = make_group("comp1", &["ep1"], entities, edges);
        let groups = vec![group];

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);

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

    #[test]
    fn test_description_always_none() {
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "depends_on", "ep1")];

        let group = make_group("comp1", &["ep1"], entities, edges);
        let groups = vec![group];

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);

        for c in &candidates {
            assert!(
                c.description.is_none(),
                "D1a should never produce descriptions"
            );
        }
    }

    #[test]
    fn test_entity_types_populated_on_pair_candidate() {
        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Infrastructure"),
        ];
        let edges = vec![make_edge("edge1", "e1", "e2", "depends_on", "ep1")];

        let group = make_group("comp1", &["ep1"], entities, edges);
        let groups = vec![group];

        let config = PatternExtractorConfig {
            min_occurrence_count: 1,
            ..Default::default()
        };
        let candidates = PatternExtractor::new().extract(&groups, &config);

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

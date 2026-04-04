//! Skill creation and evolution (D2) — pure logic module.
//!
//! The `SkillCreator` takes pattern candidates and associated edge data
//! to produce `DraftSkill` structs. It filters patterns by success/failure
//! relations and aggregates counts. No I/O — all data is passed in.

use crate::types::{DraftSkill, Edge, PatternCandidate, SkillCreatorConfig};
use std::collections::{HashMap, HashSet};

/// Pure-logic skill creator.
///
/// Analyzes pattern candidates alongside their associated edges to produce
/// draft skills — intermediate structs ready for LLM synthesis of
/// name, trigger_condition, action, and description.
pub struct SkillCreator;

/// Trait for synthesizing behavioral fields on a draft skill via LLM (D2).
///
/// Implementations take a `DraftSkill` and produce the behavioral
/// fields: name, description, trigger_condition, and action.
pub trait SkillSynthesizer: Send + Sync {
    /// Synthesize behavioral fields for a draft skill.
    ///
    /// Returns (name, description, trigger_condition, action).
    fn synthesize(
        &self,
        draft: &DraftSkill,
    ) -> crate::error::Result<(String, String, String, String)>;
}

// ── Test implementations ──────────────────────────────────────────────────────

/// A SkillSynthesizer that generates deterministic behavioral fields from draft data.
/// Useful for testing without a real LLM.
pub struct MockSkillSynthesizer;

impl SkillSynthesizer for MockSkillSynthesizer {
    fn synthesize(
        &self,
        draft: &DraftSkill,
    ) -> crate::error::Result<(String, String, String, String)> {
        let types = if draft.entity_types.is_empty() {
            "general".to_string()
        } else {
            draft.entity_types.join(", ")
        };

        let is_positive = draft.success_count > draft.failure_count;
        let name = if is_positive {
            format!("Successful {} pattern", types)
        } else if draft.failure_count > 0 {
            format!("Risky {} anti-pattern", types)
        } else {
            format!("Observed {} pattern", types)
        };

        let description = format!(
            "A behavioral skill derived from {} patterns involving {}. \
             {} successes and {} failures observed.",
            draft.source_pattern_ids.len(),
            types,
            draft.success_count,
            draft.failure_count,
        );

        let trigger_condition = if draft.source_summaries.is_empty() {
            format!("When working with {}", types)
        } else {
            format!(
                "When patterns related to {} are detected in the current context",
                types
            )
        };

        let action = if is_positive {
            format!(
                "Apply the proven approach for {} — it has succeeded {} times",
                types, draft.success_count
            )
        } else if draft.failure_count > 0 {
            format!(
                "Avoid the failing approach for {} — it has failed {} times",
                types, draft.failure_count
            )
        } else {
            format!(
                "Consider the pattern involving {} when making decisions",
                types
            )
        };

        Ok((name, description, trigger_condition, action))
    }
}

/// A SkillSynthesizer that always returns an error.
/// Useful for testing LLM failure handling.
pub struct FailingSkillSynthesizer {
    message: String,
}

impl FailingSkillSynthesizer {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl SkillSynthesizer for FailingSkillSynthesizer {
    fn synthesize(
        &self,
        _draft: &DraftSkill,
    ) -> crate::error::Result<(String, String, String, String)> {
        Err(crate::error::CtxGraphError::Extraction(
            self.message.clone(),
        ))
    }
}

impl SkillCreator {
    /// Create draft skills from pattern candidates and their associated edges.
    ///
    /// Algorithm:
    /// 1. Build a map of edge relations per pattern (from associated edges).
    /// 2. For each pattern, count success/failure edges based on config relations.
    /// 3. Filter out patterns with zero success AND zero failure signals.
    /// 4. Produce DraftSkill for each qualifying pattern.
    ///
    /// Returns an empty vec if no patterns have success/failure signals.
    pub fn draft_skills(
        patterns: &[PatternCandidate],
        edges: &[Edge],
        source_summaries: &HashMap<String, Vec<String>>,
        config: &SkillCreatorConfig,
    ) -> Vec<DraftSkill> {
        if patterns.is_empty() {
            return Vec::new();
        }

        // Build success/failure relation sets for fast lookup (case-insensitive)
        let success_set: HashSet<String> = config
            .success_relations
            .iter()
            .map(|r| r.to_lowercase())
            .collect();
        let failure_set: HashSet<String> = config
            .failure_relations
            .iter()
            .map(|r| r.to_lowercase())
            .collect();

        // Build a map: episode_id -> edges in that episode
        let mut episode_edges: HashMap<String, Vec<&Edge>> = HashMap::new();
        for edge in edges {
            if let Some(ref ep_id) = edge.episode_id {
                episode_edges.entry(ep_id.clone()).or_default().push(edge);
            }
        }

        let mut drafts = Vec::new();

        for pattern in patterns {
            let mut success_count: u32 = 0;
            let mut failure_count: u32 = 0;
            let mut all_summaries: Vec<String> = Vec::new();

            // Count success/failure from edges associated with this pattern's source groups.
            // Source groups are compression group IDs. We need to find edges whose episode_id
            // was compressed into that group. Since we can't easily do that from here,
            // we also match edges directly by episode_id == source_group_id.
            for group_id in &pattern.source_groups {
                // Try matching edges by episode_id directly
                if let Some(edge_list) = episode_edges.get(group_id) {
                    for edge in edge_list {
                        let relation_lower = edge.relation.to_lowercase();
                        if success_set.contains(&relation_lower) {
                            success_count += 1;
                        } else if failure_set.contains(&relation_lower) {
                            failure_count += 1;
                        }
                    }
                }

                // Collect summaries for this source group
                if let Some(summaries) = source_summaries.get(group_id) {
                    all_summaries.extend(summaries.iter().cloned());
                }
            }

            // Only create a draft if there's at least one success or failure signal
            if success_count == 0 && failure_count == 0 {
                continue;
            }

            // Collect unique entity types from the pattern
            let mut entity_types: Vec<String> = pattern.entity_types.clone();
            entity_types.sort();
            entity_types.dedup();

            drafts.push(DraftSkill {
                entity_types,
                success_count,
                failure_count,
                source_pattern_ids: vec![pattern.id.clone()],
                source_summaries: all_summaries,
            });
        }

        drafts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryType, Skill};
    use chrono::Utc;
    use std::time::Duration;

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

    fn make_pattern(id: &str, source_groups: &[&str], entity_types: &[&str]) -> PatternCandidate {
        PatternCandidate {
            id: id.to_string(),
            entity_types: entity_types.iter().map(|s| s.to_string()).collect(),
            entity_pair: None,
            relation_triplet: None,
            occurrence_count: 3,
            source_groups: source_groups.iter().map(|s| s.to_string()).collect(),
            confidence: 0.75,
            description: None,
        }
    }

    #[test]
    fn test_draft_skills_empty_patterns_returns_empty() {
        let config = SkillCreatorConfig::default();
        let drafts = SkillCreator::draft_skills(&[], &[], &HashMap::new(), &config);
        assert!(drafts.is_empty());
    }

    #[test]
    fn test_draft_skills_no_signals_returns_empty() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Component"])];
        // Edge with unrelated relation
        let edges = vec![make_edge("e1", "a", "b", "unrelated", "ep1")];
        let config = SkillCreatorConfig::default();

        let drafts = SkillCreator::draft_skills(&patterns, &edges, &HashMap::new(), &config);
        assert!(
            drafts.is_empty(),
            "no success/failure signals should produce no drafts"
        );
    }

    #[test]
    fn test_draft_skills_success_relation_creates_draft() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Docker"])];
        let edges = vec![
            make_edge("e1", "a", "b", "resolved", "ep1"),
            make_edge("e2", "a", "b", "fixed", "ep1"),
        ];
        let config = SkillCreatorConfig::default();

        let drafts = SkillCreator::draft_skills(&patterns, &edges, &HashMap::new(), &config);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].success_count, 2);
        assert_eq!(drafts[0].failure_count, 0);
        assert_eq!(drafts[0].entity_types, vec!["Docker"]);
    }

    #[test]
    fn test_draft_skills_failure_relation_creates_draft() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Service"])];
        let edges = vec![make_edge("e1", "a", "b", "failed", "ep1")];
        let config = SkillCreatorConfig::default();

        let drafts = SkillCreator::draft_skills(&patterns, &edges, &HashMap::new(), &config);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].failure_count, 1);
    }

    #[test]
    fn test_draft_skills_mixed_signals() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Component"])];
        let edges = vec![
            make_edge("e1", "a", "b", "resolved", "ep1"),
            make_edge("e2", "a", "b", "failed", "ep1"),
            make_edge("e3", "a", "b", "abandoned", "ep1"),
        ];
        let config = SkillCreatorConfig::default();

        let drafts = SkillCreator::draft_skills(&patterns, &edges, &HashMap::new(), &config);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].success_count, 1);
        assert_eq!(drafts[0].failure_count, 2);
    }

    #[test]
    fn test_draft_skills_relation_case_insensitive() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Component"])];
        let edges = vec![make_edge("e1", "a", "b", "RESOLVED", "ep1")];
        let config = SkillCreatorConfig::default();

        let drafts = SkillCreator::draft_skills(&patterns, &edges, &HashMap::new(), &config);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].success_count, 1);
    }

    #[test]
    fn test_draft_skills_collects_summaries() {
        let patterns = vec![make_pattern("p1", &["comp1"], &["Docker"])];
        let edges = vec![make_edge("e1", "a", "b", "fixed", "comp1")];

        let mut summaries = HashMap::new();
        summaries.insert(
            "comp1".to_string(),
            vec![
                "Fixed DNS issue".to_string(),
                "Restarted container".to_string(),
            ],
        );

        let config = SkillCreatorConfig::default();
        let drafts = SkillCreator::draft_skills(&patterns, &edges, &summaries, &config);

        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].source_summaries.len(), 2);
    }

    #[test]
    fn test_skill_confidence_all_success() {
        assert_eq!(Skill::compute_confidence(5, 0), 1.0);
    }

    #[test]
    fn test_skill_confidence_all_failure() {
        assert_eq!(Skill::compute_confidence(0, 5), 0.0);
    }

    #[test]
    fn test_skill_confidence_mixed() {
        let conf = Skill::compute_confidence(3, 1);
        assert!((conf - 0.75).abs() < 1e-6);
    }

    #[test]
    fn test_skill_confidence_zero_total() {
        assert_eq!(Skill::compute_confidence(0, 0), 0.5);
    }

    #[test]
    fn test_skill_provenance_expires_at_uses_min_ttl() {
        let provenance = Skill::generate_provenance(
            "test reasoning".to_string(),
            &["fact1".to_string()],
            180,
            90,
        );
        let expected = Utc::now() + chrono::Duration::days(90);
        let diff = (provenance.expires_at - expected).num_seconds().abs();
        assert!(diff < 2, "expires_at should be ~90 days from now");
        assert!(provenance.context_facts.is_some());
        assert_eq!(provenance.renewal_count, 0);
    }

    #[test]
    fn test_skill_provenance_empty_summaries_no_context_facts() {
        let provenance = Skill::generate_provenance("test reasoning".to_string(), &[], 180, 90);
        assert!(provenance.context_facts.is_none());
    }

    #[test]
    fn test_mock_skill_synthesizer_produces_fields() {
        let draft = DraftSkill {
            entity_types: vec!["Docker".to_string(), "Network".to_string()],
            success_count: 3,
            failure_count: 1,
            source_pattern_ids: vec!["p1".to_string()],
            source_summaries: vec!["Fixed DNS".to_string()],
        };

        let synthesizer = MockSkillSynthesizer;
        let (name, desc, trigger, action) = synthesizer.synthesize(&draft).unwrap();

        assert!(!name.is_empty());
        assert!(!desc.is_empty());
        assert!(!trigger.is_empty());
        assert!(!action.is_empty());
        // Since success > failure, should be "Successful"
        assert!(
            name.contains("Successful"),
            "name should contain 'Successful': {}",
            name
        );
    }

    #[test]
    fn test_mock_skill_synthesizer_failure_pattern() {
        let draft = DraftSkill {
            entity_types: vec!["Component".to_string()],
            success_count: 0,
            failure_count: 3,
            source_pattern_ids: vec!["p1".to_string()],
            source_summaries: vec![],
        };

        let synthesizer = MockSkillSynthesizer;
        let (name, _, _, action) = synthesizer.synthesize(&draft).unwrap();

        assert!(
            name.contains("Risky"),
            "name should contain 'Risky': {}",
            name
        );
        assert!(
            action.contains("Avoid"),
            "action should contain 'Avoid': {}",
            action
        );
    }

    #[test]
    fn test_failing_skill_synthesizer_returns_error() {
        let draft = DraftSkill {
            entity_types: vec![],
            success_count: 1,
            failure_count: 0,
            source_pattern_ids: vec![],
            source_summaries: vec![],
        };

        let synthesizer = FailingSkillSynthesizer::new("LLM unavailable");
        let result = synthesizer.synthesize(&draft);
        assert!(result.is_err());
    }
}

//! Skill creation and evolution (D2) — pure logic module.
//!
//! The `SkillCreator` takes pattern candidates, associated edge data, and
//! LLM-generated labels to produce `Skill` structs directly. No intermediate
//! DraftSkill layer — candidates go straight to skills.

use crate::types::{Edge, PatternCandidate, Skill, SkillCreatorConfig, SkillScope};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

/// Pure-logic skill creator.
///
/// Analyzes pattern candidates alongside their associated edges and
/// LLM-generated labels to produce Skills directly. Stateless — all data
/// is passed in via the `create_skills` method.
pub struct SkillCreator;

impl SkillCreator {
    /// Create skills directly from pattern candidates, edges, and LLM-generated labels.
    ///
    /// Algorithm:
    /// 1. Count success/failure edges for each pattern from associated episodes.
    /// 2. Filter out patterns with zero success AND zero failure signals.
    /// 3. Build template-based name, trigger_condition, action, and provenance.
    /// 4. Set description from the LLM descriptions map (or empty string if missing).
    ///
    /// Returns an empty vec if no patterns have success/failure signals.
    pub fn create_skills(
        patterns: &[PatternCandidate],
        edges: &[Edge],
        source_summaries: &HashMap<String, Vec<String>>,
        descriptions: &HashMap<String, String>,
        config: &SkillCreatorConfig,
        scope: SkillScope,
        agent: &str,
    ) -> Vec<Skill> {
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

        let mut skills = Vec::new();

        for pattern in patterns {
            let mut success_count: u32 = 0;
            let mut failure_count: u32 = 0;
            let mut all_summaries: Vec<String> = Vec::new();
            let mut success_relations: Vec<String> = Vec::new();

            for group_id in &pattern.source_groups {
                if let Some(edge_list) = episode_edges.get(group_id) {
                    for edge in edge_list {
                        let relation_lower = edge.relation.to_lowercase();
                        if success_set.contains(&relation_lower) {
                            success_count += 1;
                            success_relations.push(edge.relation.clone());
                        } else if failure_set.contains(&relation_lower) {
                            failure_count += 1;
                        }
                    }
                }

                if let Some(summaries) = source_summaries.get(group_id) {
                    all_summaries.extend(summaries.iter().cloned());
                }
            }

            // Only create a skill if there's at least one success or failure signal
            if success_count == 0 && failure_count == 0 {
                continue;
            }

            let mut entity_types: Vec<String> = pattern.entity_types.clone();
            entity_types.sort();
            entity_types.dedup();

            let types_str = if entity_types.is_empty() {
                "general".to_string()
            } else {
                entity_types.join("+")
            };

            // Most common success relation for action template
            let most_common_relation = {
                let mut rel_counts: HashMap<&str, usize> = HashMap::new();
                for r in &success_relations {
                    *rel_counts.entry(r.as_str()).or_insert(0) += 1;
                }
                rel_counts
                    .into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(r, _)| r.to_string())
                    .unwrap_or_else(|| "interact".to_string())
            };

            let name = format!(
                "{} pattern ({} successes, {} failures)",
                types_str, success_count, failure_count
            );
            let description = descriptions
                .get(&pattern.id)
                .cloned()
                .unwrap_or_default();
            let trigger_condition = format!("When {} entities co-occur", types_str);
            let action = format!(
                "When {} entities co-occur, apply {} approach",
                types_str, most_common_relation
            );
            let provenance_reasoning = format!(
                "Pattern observed across {} episodes with {} successes and {} failures",
                pattern.source_groups.len(),
                success_count,
                failure_count
            );
            let context_facts: Vec<String> = all_summaries;

            let provenance = Skill::generate_provenance(
                provenance_reasoning,
                &context_facts,
                180,
                90,
            );

            let confidence = Skill::compute_confidence(success_count, failure_count);

            skills.push(Skill {
                id: uuid::Uuid::now_v7().to_string(),
                name,
                description,
                trigger_condition,
                action,
                success_count,
                failure_count,
                confidence,
                superseded_by: None,
                created_at: Utc::now(),
                entity_types,
                provenance: Some(provenance),
                scope,
                created_by_agent: agent.to_string(),
            });
        }

        skills
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryType, SkillScope};
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

    fn default_args<'a>(
        patterns: &'a [PatternCandidate],
        edges: &'a [Edge],
    ) -> (
        &'a [PatternCandidate],
        &'a [Edge],
        HashMap<String, Vec<String>>,
        HashMap<String, String>,
        SkillCreatorConfig,
        SkillScope,
        &'static str,
    ) {
        (
            patterns,
            edges,
            HashMap::new(),
            HashMap::new(),
            SkillCreatorConfig::default(),
            SkillScope::Private,
            "test-agent",
        )
    }

    #[test]
    fn test_create_skills_from_candidates() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Docker"])];
        let edges = vec![make_edge("e1", "a", "b", "resolved", "ep1")];
        let (p, e, ss, desc, cfg, scope, agent) = default_args(&patterns, &edges);
        let skills = SkillCreator::create_skills(p, e, &ss, &desc, &cfg, scope, agent);
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].created_by_agent, "test-agent");
    }

    #[test]
    fn test_create_skills_no_success_no_failure() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Component"])];
        let edges = vec![make_edge("e1", "a", "b", "unrelated", "ep1")];
        let (p, e, ss, desc, cfg, scope, agent) = default_args(&patterns, &edges);
        let skills = SkillCreator::create_skills(p, e, &ss, &desc, &cfg, scope, agent);
        assert!(skills.is_empty(), "no signal should produce no skills");
    }

    #[test]
    fn test_create_skills_uses_description_from_map() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Docker"])];
        let edges = vec![make_edge("e1", "a", "b", "resolved", "ep1")];
        let mut descriptions = HashMap::new();
        descriptions.insert("p1".to_string(), "Use DNS-first approach.".to_string());
        let skills = SkillCreator::create_skills(
            &patterns,
            &edges,
            &HashMap::new(),
            &descriptions,
            &SkillCreatorConfig::default(),
            SkillScope::Private,
            "test-agent",
        );
        assert_eq!(skills[0].description, "Use DNS-first approach.");
    }

    #[test]
    fn test_create_skills_name_template() {
        let patterns = vec![make_pattern("p1", &["ep1"], &["Docker", "Network"])];
        let edges = vec![make_edge("e1", "a", "b", "resolved", "ep1")];
        let (p, e, ss, desc, cfg, scope, agent) = default_args(&patterns, &edges);
        let skills = SkillCreator::create_skills(p, e, &ss, &desc, &cfg, scope, agent);
        assert!(
            skills[0].name.contains("successes"),
            "name should be template-based: {}",
            skills[0].name
        );
        assert!(
            skills[0].name.contains("failures"),
            "name should contain failures: {}",
            skills[0].name
        );
    }

    #[test]
    fn test_create_skills_provenance_template() {
        let patterns = vec![make_pattern("p1", &["ep1", "ep2"], &["Docker"])];
        let edges = vec![
            make_edge("e1", "a", "b", "resolved", "ep1"),
            make_edge("e2", "a", "b", "failed", "ep2"),
        ];
        let (p, e, ss, desc, cfg, scope, agent) = default_args(&patterns, &edges);
        let skills = SkillCreator::create_skills(p, e, &ss, &desc, &cfg, scope, agent);
        assert!(
            skills[0].provenance.as_ref().unwrap().reasoning.contains("episodes"),
            "provenance should be template-based: {}",
            skills[0].provenance.as_ref().unwrap().reasoning
        );
    }

    // ── Skill helper tests (no changes) ──────────────────────────────────────

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
}

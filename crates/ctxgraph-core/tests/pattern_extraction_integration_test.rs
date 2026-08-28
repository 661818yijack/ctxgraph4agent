//! Integration tests for pattern extraction from raw episodes.
//!
//! These tests verify that `get_pattern_candidates` loads raw episodes,
//! entities, and edges from the database and produces valid pattern
//! candidates — fixing the bug where it always returned empty results
//! because it was still using the removed compression group system.

use chrono::Utc;
use ctxgraph::Graph;
use ctxgraph::pattern::MockBatchLabelDescriber;
use ctxgraph::types::{
    Edge, Entity, Episode, MemoryType, PatternExtractorConfig, Skill, SkillScope,
};
use std::time::Duration;

fn make_episode(id: &str, content: &str) -> Episode {
    Episode {
        id: id.to_string(),
        content: content.to_string(),
        source: Some("test".to_string()),
        recorded_at: Utc::now(),
        metadata: None,
        memory_type: MemoryType::Experience,
        compression_id: None,
    }
}

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

fn make_skill(id: &str, agent: &str, entity_types: Vec<&str>) -> Skill {
    Skill {
        id: id.to_string(),
        name: format!("Prior skill {id}"),
        description: "Prior learned behavior".to_string(),
        trigger_condition: "When the same entities recur".to_string(),
        action: "Reuse the prior behavior".to_string(),
        success_count: 3,
        failure_count: 1,
        confidence: 0.75,
        superseded_by: None,
        created_at: Utc::now(),
        entity_types: entity_types.into_iter().map(str::to_string).collect(),
        provenance: None,
        scope: SkillScope::Private,
        created_by_agent: agent.to_string(),
    }
}

fn make_edge(id: &str, source_id: &str, target_id: &str, relation: &str, episode_id: &str) -> Edge {
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

#[test]
fn test_pattern_extraction_finds_candidates_from_raw_episodes() {
    let graph = Graph::in_memory().expect("failed to create graph");

    // Insert 4 episodes about Docker networking
    let episodes = vec![
        make_episode("ep1", "Docker container failed to connect to network"),
        make_episode(
            "ep2",
            "Docker networking issue resolved by restarting bridge",
        ),
        make_episode("ep3", "Docker network configuration updated for prod"),
        make_episode("ep4", "Docker containers communicate via overlay network"),
    ];
    for ep in &episodes {
        graph.storage.insert_episode(ep).unwrap();
    }

    // Insert entities
    let entities = vec![
        make_entity("e1", "Docker", "Component"),
        make_entity("e2", "Network", "Component"),
    ];
    for e in &entities {
        graph.storage.insert_entity(e).unwrap();
    }

    // Link episodes to entities via episode_entities
    for ep in &episodes {
        graph
            .storage
            .link_episode_entity(&ep.id, "e1", None, None)
            .unwrap();
        graph
            .storage
            .link_episode_entity(&ep.id, "e2", None, None)
            .unwrap();
    }

    // Insert edges linking episodes to entities
    let edges = vec![
        make_edge("edge1", "e1", "e2", "depends_on", "ep1"),
        make_edge("edge2", "e1", "e2", "depends_on", "ep2"),
        make_edge("edge3", "e1", "e2", "depends_on", "ep3"),
        make_edge("edge4", "e1", "e2", "depends_on", "ep4"),
    ];
    for edge in &edges {
        graph.storage.insert_edge(edge).unwrap();
    }

    let config = PatternExtractorConfig::default(); // min_occurrence_count = 3
    let candidates = graph.storage.get_pattern_candidates(&config).unwrap();

    assert!(
        !candidates.is_empty(),
        "expected candidates with 4 episodes sharing same pair, got none"
    );

    // At least one candidate should have occurrence_count >= 3
    let high_count = candidates.iter().any(|c| c.occurrence_count >= 3);
    assert!(
        high_count,
        "expected at least one candidate with occurrence_count >= 3, got counts: {:?}",
        candidates
            .iter()
            .map(|c| c.occurrence_count)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_pattern_extraction_respects_threshold() {
    let graph = Graph::in_memory().expect("failed to create graph");

    // Only 2 episodes — below default threshold of 3
    let episodes = vec![
        make_episode("ep1", "Docker container failed to connect to network"),
        make_episode(
            "ep2",
            "Docker networking issue resolved by restarting bridge",
        ),
    ];
    for ep in &episodes {
        graph.storage.insert_episode(ep).unwrap();
    }

    let entities = vec![
        make_entity("e1", "Docker", "Component"),
        make_entity("e2", "Network", "Component"),
    ];
    for e in &entities {
        graph.storage.insert_entity(e).unwrap();
    }

    for ep in &episodes {
        graph
            .storage
            .link_episode_entity(&ep.id, "e1", None, None)
            .unwrap();
        graph
            .storage
            .link_episode_entity(&ep.id, "e2", None, None)
            .unwrap();
    }

    let edges = vec![
        make_edge("edge1", "e1", "e2", "depends_on", "ep1"),
        make_edge("edge2", "e1", "e2", "depends_on", "ep2"),
    ];
    for edge in &edges {
        graph.storage.insert_edge(edge).unwrap();
    }

    // Default threshold = 3 → should return empty
    let config3 = PatternExtractorConfig::default();
    let candidates3 = graph.storage.get_pattern_candidates(&config3).unwrap();
    assert!(
        candidates3.is_empty(),
        "expected no candidates with threshold 3 and only 2 episodes"
    );

    // Lower threshold to 1 → should find candidates
    let config1 = PatternExtractorConfig {
        min_occurrence_count: 1,
        ..Default::default()
    };
    let candidates1 = graph.storage.get_pattern_candidates(&config1).unwrap();
    assert!(
        !candidates1.is_empty(),
        "expected candidates with threshold 1"
    );
}

#[test]
fn test_learning_pipeline_end_to_end() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let graph = Graph::in_memory().expect("failed to create graph");

        // Insert 4 episodes
        let episodes = vec![
            make_episode("ep1", "Docker container failed to connect to network"),
            make_episode(
                "ep2",
                "Docker networking issue resolved by restarting bridge",
            ),
            make_episode("ep3", "Docker network configuration updated for prod"),
            make_episode("ep4", "Docker containers communicate via overlay network"),
        ];
        for ep in &episodes {
            graph.storage.insert_episode(ep).unwrap();
        }

        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        for e in &entities {
            graph.storage.insert_entity(e).unwrap();
        }

        for ep in &episodes {
            graph
                .storage
                .link_episode_entity(&ep.id, "e1", None, None)
                .unwrap();
            graph
                .storage
                .link_episode_entity(&ep.id, "e2", None, None)
                .unwrap();
        }

        let edges = vec![
            make_edge("edge1", "e1", "e2", "resolved", "ep1"),
            make_edge("edge2", "e1", "e2", "resolved", "ep2"),
            make_edge("edge3", "e1", "e2", "resolved", "ep3"),
            make_edge("edge4", "e1", "e2", "resolved", "ep4"),
        ];
        for edge in &edges {
            graph.storage.insert_edge(edge).unwrap();
        }

        let describer = MockBatchLabelDescriber;
        let outcome = graph
            .run_learning_pipeline("test-agent", ctxgraph::SkillScope::Private, &describer, 50)
            .await
            .unwrap();

        assert!(
            outcome.patterns_found > 0,
            "expected patterns_found > 0, got {}",
            outcome.patterns_found
        );
        assert!(
            outcome.skills_created >= 1,
            "expected at least 1 skill created, got {}",
            outcome.skills_created
        );
    });
}

#[test]
fn test_learning_pipeline_supersedes_only_preexisting_matching_skills() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let graph = Graph::in_memory().expect("failed to create graph");
        let agent = "test-agent";

        let prior_overlap = make_skill("prior-overlap", agent, vec!["Component"]);
        let prior_unrelated = make_skill("prior-unrelated", agent, vec!["Unrelated"]);
        graph.storage.create_skill(&prior_overlap).unwrap();
        graph.storage.create_skill(&prior_unrelated).unwrap();

        let episodes = vec![
            make_episode("ep1", "Docker container failed to connect to network"),
            make_episode(
                "ep2",
                "Docker networking issue resolved by restarting bridge",
            ),
            make_episode("ep3", "Docker network configuration updated for prod"),
            make_episode("ep4", "Docker containers communicate via overlay network"),
        ];
        for ep in &episodes {
            graph.storage.insert_episode(ep).unwrap();
        }

        let entities = vec![
            make_entity("e1", "Docker", "Component"),
            make_entity("e2", "Network", "Component"),
        ];
        for e in &entities {
            graph.storage.insert_entity(e).unwrap();
        }

        for ep in &episodes {
            graph
                .storage
                .link_episode_entity(&ep.id, "e1", None, None)
                .unwrap();
            graph
                .storage
                .link_episode_entity(&ep.id, "e2", None, None)
                .unwrap();
        }

        let edges = vec![
            make_edge("edge1", "e1", "e2", "resolved", "ep1"),
            make_edge("edge2", "e1", "e2", "resolved", "ep2"),
            make_edge("edge3", "e1", "e2", "resolved", "ep3"),
            make_edge("edge4", "e1", "e2", "resolved", "ep4"),
        ];
        for edge in &edges {
            graph.storage.insert_edge(edge).unwrap();
        }

        let describer = MockBatchLabelDescriber;
        let outcome = graph
            .run_learning_pipeline(agent, SkillScope::Private, &describer, 50)
            .await
            .unwrap();

        assert!(outcome.skills_created >= 1, "expected new learned skills");
        assert_eq!(
            outcome.skills_updated, 1,
            "only the pre-existing overlapping skill should be superseded"
        );

        let updated_overlap = graph.storage.get_skill("prior-overlap").unwrap().unwrap();
        let updated_unrelated = graph.storage.get_skill("prior-unrelated").unwrap().unwrap();
        assert!(
            updated_overlap
                .superseded_by
                .as_ref()
                .is_some_and(|id| outcome.skill_ids.contains(id)),
            "prior overlapping skill should point to one of the newly-created skills"
        );
        assert_eq!(updated_unrelated.superseded_by, None);

        for new_skill_id in &outcome.skill_ids {
            let new_skill = graph.storage.get_skill(new_skill_id).unwrap().unwrap();
            assert_eq!(
                new_skill.superseded_by, None,
                "newly-created skill {new_skill_id} must not be superseded by the same run"
            );
        }
    });
}

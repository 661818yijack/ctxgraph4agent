use chrono::Utc;
use ctxgraph::{Edge, Entity, Episode, Graph, MemoryType};
use std::time::Duration;

const EXAMPLE_SOURCE: &str = include_str!("../examples/auto_learn.rs");

fn make_entity(id: &str, name: &str) -> Entity {
    Entity {
        id: id.to_string(),
        name: name.to_string(),
        entity_type: "Component".to_string(),
        memory_type: MemoryType::Fact,
        ttl: Some(Duration::from_secs(90 * 86400)),
        summary: None,
        created_at: Utc::now(),
        metadata: None,
        usage_count: 0,
        last_recalled_at: None,
    }
}

fn make_edge(id: &str, source_id: &str, target_id: &str, episode_id: &str) -> Edge {
    Edge {
        id: id.to_string(),
        source_id: source_id.to_string(),
        target_id: target_id.to_string(),
        relation: "resolved".to_string(),
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

fn seed_three_linked_episodes(graph: &Graph) {
    graph
        .storage
        .insert_entity(&make_entity("e1", "ctxgraph"))
        .unwrap();
    graph
        .storage
        .insert_entity(&make_entity("e2", "Hermes"))
        .unwrap();

    for index in 0..3 {
        let episode_id = format!("ep{index}");
        let episode = Episode {
            id: episode_id.clone(),
            content: format!(
                "Apex reused ctxgraph with Hermes session memory successfully during run {index}"
            ),
            source: Some("auto-learn-example-test".to_string()),
            recorded_at: Utc::now(),
            metadata: None,
            memory_type: MemoryType::Experience,
            compression_id: None,
        };
        graph.storage.insert_episode(&episode).unwrap();
        graph
            .storage
            .link_episode_entity(&episode_id, "e1", None, None)
            .unwrap();
        graph
            .storage
            .link_episode_entity(&episode_id, "e2", None, None)
            .unwrap();
        graph
            .storage
            .insert_edge(&make_edge(&format!("edge{index}"), "e1", "e2", &episode_id))
            .unwrap();
    }
}

#[test]
fn auto_learn_example_documents_no_key_mock_path() {
    assert!(
        EXAMPLE_SOURCE.contains("set_learn_interval(3)"),
        "example should keep the trigger interval small for local exploration"
    );
    assert!(
        EXAMPLE_SOURCE.contains("No API key required"),
        "example should tell users it does not require LLM credentials"
    );
    assert!(
        EXAMPLE_SOURCE.contains("learned_skills"),
        "example should report how many skills were learned"
    );
}

#[tokio::test]
async fn auto_learn_counter_resets_and_creates_skills_after_short_interval() {
    let graph = Graph::in_memory().expect("in-memory graph should open");
    seed_three_linked_episodes(&graph);

    graph
        .storage
        .set_learn_interval(3)
        .expect("short interval should be persisted");
    graph
        .storage
        .set_system_metadata("episodes_since_last_learn", "2")
        .expect("test should be able to stage the counter just below threshold");

    assert_eq!(
        graph.storage.get_episodes_since_last_learn().unwrap(),
        2,
        "counter should be staged below the trigger threshold"
    );

    graph
        .add_episode(
            Episode::builder(
                "Apex added one more ctxgraph note, crossing the short auto-learn threshold",
            )
            .build(),
        )
        .await
        .expect("threshold episode should store and trigger mock auto-learn");

    assert_eq!(
        graph.storage.get_episodes_since_last_learn().unwrap(),
        0,
        "counter should reset after auto-learn fires"
    );
    assert!(
        !graph.storage.list_all_skills().unwrap().is_empty(),
        "pre-seeded repeated entity pairs should produce learned skills through the mock describer"
    );
}

use chrono::Utc;
use ctxgraph::{Edge, Entity, Episode, Graph, MemoryType};
use std::time::Duration;

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

fn seed_repeated_pattern(graph: &Graph) -> ctxgraph::Result<()> {
    graph
        .storage
        .insert_entity(&make_entity("ctxgraph", "ctxgraph"))?;
    graph
        .storage
        .insert_entity(&make_entity("hermes", "Hermes"))?;

    for index in 0..3 {
        let episode_id = format!("seed-{index}");
        let episode = Episode {
            id: episode_id.clone(),
            content: format!(
                "Apex reused ctxgraph with Hermes session memory successfully during run {index}"
            ),
            source: Some("auto-learn-example".to_string()),
            recorded_at: Utc::now(),
            metadata: None,
            memory_type: MemoryType::Experience,
            compression_id: None,
        };

        graph.storage.insert_episode(&episode)?;
        graph
            .storage
            .link_episode_entity(&episode_id, "ctxgraph", None, None)?;
        graph
            .storage
            .link_episode_entity(&episode_id, "hermes", None, None)?;
        graph.storage.insert_edge(&make_edge(
            &format!("edge-{index}"),
            "ctxgraph",
            "hermes",
            &episode_id,
        ))?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> ctxgraph::Result<()> {
    // No API key required: the auto-learn trigger uses the deterministic
    // MockBatchLabelDescriber path, so this example runs fully offline.
    let graph = Graph::in_memory()?;
    seed_repeated_pattern(&graph)?;

    // Keep the demo fast: stage the counter one episode below a short interval.
    graph.storage.set_learn_interval(3)?;
    graph
        .storage
        .set_system_metadata("episodes_since_last_learn", "2")?;

    println!(
        "before_trigger_counter={}",
        graph.storage.get_episodes_since_last_learn()?
    );

    graph
        .add_episode(
            Episode::builder(
                "Apex added one more ctxgraph note, crossing the short auto-learn threshold",
            )
            .source("auto-learn-example")
            .build(),
        )
        .await?;

    let learned_skills = graph.storage.list_all_skills()?;
    println!(
        "after_trigger_counter={}",
        graph.storage.get_episodes_since_last_learn()?
    );
    println!("learned_skills={}", learned_skills.len());
    for skill in learned_skills {
        println!("skill={} confidence={:.2}", skill.name, skill.confidence);
    }

    Ok(())
}

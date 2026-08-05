use ctxgraph::{Edge, Entity, Episode, Graph, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let graph = Graph::in_memory()?;

    let episode = Episode::builder(
        "Apex chose SQLite for local agent memory because it keeps setup offline.",
    )
    .source("agent-journal")
    .tag("architecture")
    .tag("offline")
    .build();
    let episode_id = episode.id.clone();
    graph.add_episode(episode).await?;

    let agent = Entity::new("Apex", "Agent");
    let storage = Entity::new("SQLite", "Database");
    let agent_id = agent.id.clone();
    let storage_id = storage.id.clone();
    graph.add_entity(agent)?;
    graph.add_entity(storage)?;

    graph.link_episode_entity(&episode_id, &agent_id, Some(0), Some(4))?;
    graph.link_episode_entity(&episode_id, &storage_id, Some(12), Some(18))?;

    let mut edge = Edge::new(&agent_id, &storage_id, "chose_storage");
    edge.episode_id = Some(episode_id.clone());
    edge.fact = Some("Apex chose SQLite for local agent memory".to_string());
    graph.add_edge(edge)?;

    let matching_episodes = graph.search("SQLite offline", 5, Some("agent-journal"))?;
    let (related_entities, related_edges) = graph.traverse(&agent_id, 1)?;

    println!("agent memory example");
    println!("episodes matched: {}", matching_episodes.len());
    println!("entities in context: {}", related_entities.len());
    println!("edges in context: {}", related_edges.len());
    println!("run without API keys or model downloads");

    Ok(())
}

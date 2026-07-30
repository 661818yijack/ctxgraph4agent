use ctxgraph::{Edge, Entity, Episode, Graph, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let graph = Graph::in_memory()?;

    // 1. Record memory episodes
    let decision = Episode::builder(
        "We chose SQLite for local agent memory because it is embedded and easy to ship.",
    )
    .source("architecture-log")
    .tag("decision")
    .tag("storage")
    .build();
    graph.add_episode(decision).await?;

    let follow_up = Episode::builder(
        "Agents should search memory before asking humans to repeat project context.",
    )
    .source("operating-principles")
    .tag("workflow")
    .build();
    graph.add_episode(follow_up).await?;

    // 2. List recent memory
    let recent = graph.list_episodes(5, 0, None, None)?;
    println!("recent episodes: {}", recent.len());
    for episode in &recent {
        println!(
            "- [{}] {}",
            episode.source.as_deref().unwrap_or("unknown"),
            episode.content
        );
    }

    // 3. Search prior decisions
    let matches = graph.search("SQLite", 5, None)?;
    println!("search matches for SQLite: {}", matches.len());
    for (episode, score) in &matches {
        println!(
            "- score={score:.3} source={} content={}",
            episode.source.as_deref().unwrap_or("unknown"),
            episode.content
        );
    }

    // 4. Traverse graph context
    let ctxgraph = Entity::new("ctxgraph", "project");
    let sqlite = Entity::new("SQLite", "database");
    let ctxgraph_id = ctxgraph.id.clone();
    let sqlite_id = sqlite.id.clone();

    graph.add_entity(ctxgraph)?;
    graph.add_entity(sqlite)?;
    graph.add_edge(Edge::new(&ctxgraph_id, &sqlite_id, "uses_storage"))?;

    let (entities, edges) = graph.traverse(&ctxgraph_id, 2)?;
    println!(
        "graph context from ctxgraph: {} entities, {} edges",
        entities.len(),
        edges.len()
    );

    Ok(())
}

use ctxgraph::{Episode, Graph, MemoryType, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("ctxgraph-basic-usage.db");

    let graph = Graph::init(&db_path)?;

    let billing_episode = Episode::builder(
        "We chose PostgreSQL for billing because it handles transactional writes well.",
    )
    .source("architecture-note")
    .tag("database")
    .tag("billing")
    .memory_type(MemoryType::Decision)
    .build();

    let cache_episode = Episode::builder("We kept Redis for short-lived session cache data.")
        .source("architecture-note")
        .tag("cache")
        .memory_type(MemoryType::Decision)
        .build();

    let billing_id = graph.add_episode(billing_episode).await?.episode_id;
    let cache_id = graph.add_episode(cache_episode).await?.episode_id;

    let results = graph.search("PostgreSQL billing", 10, Some("architecture-note"))?;
    let recent = graph.list_episodes(10, 0, None, Some("architecture-note"))?;
    let stats = graph.stats()?;

    println!("stored_episode_ids={billing_id},{cache_id}");
    println!("search_results={}", results.len());
    println!("recent_episodes={}", recent.len());
    println!("total_episodes={}", stats.episode_count);

    Ok(())
}

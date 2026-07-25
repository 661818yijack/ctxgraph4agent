use ctxgraph::{Graph, MemoryType, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let graph = Graph::in_memory()?;

    let observation = graph
        .add_text(
            "Tester reported that login retries were flaky after the gateway restart",
            Some("telegram"),
        )
        .await?;

    let decision = graph
        .add_text_with_type(
            "Decided to keep SQLite as the embedded memory store for agent contexts",
            Some("architecture"),
            MemoryType::Decision,
        )
        .await?;

    let telegram_memories = graph.list_episodes(10, 0, None, Some("telegram"))?;
    let architecture_memories = graph.list_episodes(10, 0, None, Some("architecture"))?;

    assert_eq!(telegram_memories.len(), 1);
    assert_eq!(telegram_memories[0].id, observation.episode_id);
    assert_eq!(architecture_memories.len(), 1);
    assert_eq!(architecture_memories[0].id, decision.episode_id);
    assert_eq!(architecture_memories[0].memory_type, MemoryType::Decision);

    println!(
        "recorded {} telegram memory and {} architecture decision",
        telegram_memories.len(),
        architecture_memories.len()
    );

    Ok(())
}

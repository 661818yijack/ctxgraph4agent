//! Source-filtered recall for multi-agent memory.
//!
//! This example stays fully local: it uses an in-memory graph and the FTS5
//! search path, so it does not need downloaded models or API keys.
//!
//! Run:
//! ```bash
//! cargo run -p ctxgraph --example source_filtered_recall --no-default-features
//! ```

use ctxgraph::{Episode, Graph};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let graph = Graph::in_memory()?;

    seed_team_memory(&graph).await?;

    let apex_only = graph.search("Postgres", 10, Some("apex"))?;
    println!("Apex recall:");
    for (episode, score) in apex_only {
        println!("- [{score:.3}] {}", episode.content);
    }

    let all_sources = graph.search("Postgres", 10, None)?;
    println!("\nAll-source recall count: {}", all_sources.len());

    Ok(())
}

async fn seed_team_memory(graph: &Graph) -> ctxgraph::Result<()> {
    graph
        .add_episode(
            Episode::builder("Apex chose Postgres for billing audit logs")
                .source("apex")
                .tag("billing")
                .build(),
        )
        .await?;

    graph
        .add_episode(
            Episode::builder("Lux reviewed Postgres migration risk for analytics")
                .source("lux")
                .tag("analytics")
                .build(),
        )
        .await?;

    graph
        .add_episode(
            Episode::builder("Cclow validated Redis session expiry behavior")
                .source("cclow")
                .tag("qa")
                .build(),
        )
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn filters_recall_to_one_agent_source() {
        let graph = Graph::in_memory().unwrap();
        seed_team_memory(&graph).await.unwrap();

        let apex_only = graph.search("Postgres", 10, Some("apex")).unwrap();
        assert_eq!(apex_only.len(), 1);
        assert_eq!(apex_only[0].0.source.as_deref(), Some("apex"));
        assert!(apex_only[0].0.content.contains("billing audit logs"));
    }

    #[tokio::test]
    async fn nonmatching_source_excludes_otherwise_matching_content() {
        let graph = Graph::in_memory().unwrap();
        seed_team_memory(&graph).await.unwrap();

        let cclow_postgres = graph.search("Postgres", 10, Some("cclow")).unwrap();
        assert!(cclow_postgres.is_empty());
    }

    #[tokio::test]
    async fn no_source_filter_recalls_all_matching_sources() {
        let graph = Graph::in_memory().unwrap();
        seed_team_memory(&graph).await.unwrap();

        let all_sources = graph.search("Postgres", 10, None).unwrap();
        let sources: std::collections::BTreeSet<_> = all_sources
            .iter()
            .filter_map(|(episode, _)| episode.source.as_deref())
            .collect();

        assert_eq!(all_sources.len(), 2);
        assert_eq!(sources, ["apex", "lux"].into_iter().collect());
    }
}

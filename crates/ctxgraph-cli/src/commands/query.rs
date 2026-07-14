use chrono::{DateTime, Utc};

use crate::display;

use super::open_graph;

pub fn run(
    text: String,
    limit: usize,
    after: Option<String>,
    source: Option<String>,
) -> ctxgraph::Result<()> {
    let graph = open_graph()?;
    let after_dt = after
        .as_deref()
        .map(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| ctxgraph::CtxGraphError::InvalidInput(format!(
                    "invalid --after timestamp '{s}'; expected RFC3339 like 2026-01-01T00:00:00Z"
                )))
        })
        .transpose()?;
    let results = graph.search(&text, limit, source.as_deref(), after_dt)?;

    if results.is_empty() {
        println!("No results found for '{text}'");
        return Ok(());
    }

    println!("Found {} result(s) for '{text}':\n", results.len());

    for (episode, score) in &results {
        display::print_episode(episode, Some(*score));
    }

    Ok(())
}

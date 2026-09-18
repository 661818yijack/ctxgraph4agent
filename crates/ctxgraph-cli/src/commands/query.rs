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
                .map_err(|_| {
                    ctxgraph::CtxGraphError::InvalidInput(format!(
                        "invalid --after value '{s}'; expected RFC3339 timestamp like 2026-05-01T00:00:00Z"
                    ))
                })
        })
        .transpose()?;
    let results = graph.search_filtered(&text, limit, after_dt, source.as_deref())?;

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

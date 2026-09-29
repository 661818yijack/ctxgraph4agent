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
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|err| ctxgraph::CtxGraphError::InvalidInput(format!("invalid --after: {err}")))?
        .map(|dt| dt.with_timezone::<Utc>(&Utc));
    let results = graph.search_after(&text, limit, after_dt, source.as_deref())?;

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

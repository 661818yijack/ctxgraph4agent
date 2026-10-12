use crate::display;
use crate::validate_positive_limit;

use super::open_graph;

pub fn run(
    text: String,
    limit: usize,
    _after: Option<String>,
    source: Option<String>,
) -> ctxgraph::Result<()> {
    validate_positive_limit("--limit", limit)?;

    let graph = open_graph()?;
    let results = graph.search(&text, limit, source.as_deref())?;

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

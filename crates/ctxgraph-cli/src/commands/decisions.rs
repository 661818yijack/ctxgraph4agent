use chrono::{DateTime, Utc};

use crate::display;

use super::open_graph;

pub fn list(after: Option<String>, source: Option<String>, limit: usize) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    let after_dt = after
        .as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone::<Utc>(&Utc));

    let episodes = graph.list_episodes(limit, 0, after_dt, source.as_deref())?;

    if episodes.is_empty() {
        println!("No episodes found. Log some decisions first:");
        println!("  ctxgraph log \"Chose Postgres for billing service\"");
        return Ok(());
    }

    println!("{:<12} {:<12} CONTENT", "ID", "SOURCE");
    println!("{}", "-".repeat(70));

    for episode in &episodes {
        display::print_episode_row(episode);
    }

    println!("\n{} episodes total", episodes.len());

    Ok(())
}

pub fn show(id: String) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    let episode = graph.get_episode(&id)?;
    let Some(episode) = episode else {
        return Err(ctxgraph::CtxGraphError::NotFound(format!("episode '{id}'")));
    };

    display::print_episode(&episode, None);

    Ok(())
}

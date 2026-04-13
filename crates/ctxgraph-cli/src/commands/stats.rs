use super::open_graph;

pub fn run() -> ctxgraph::Result<()> {
    let graph = open_graph()?;
    let stats = graph.stats()?;

    println!("ctxgraph stats");
    println!("{}", "─".repeat(40));

    // ── Core Counts ──
    println!("Episodes:  {}", stats.episode_count);
    println!("Entities:  {}", stats.entity_count);
    println!("Edges:     {}", stats.edge_count);

    if !stats.sources.is_empty() {
        let sources: Vec<String> = stats
            .sources
            .iter()
            .map(|(name, count)| format!("{name} ({count})"))
            .collect();
        println!("Sources:   {}", sources.join(", "));
    }
    println!("DB size:   {}", format_bytes(stats.db_size_bytes));

    // ── Entities by Type ──
    if !stats.total_entities_by_type.is_empty() {
        println!();
        println!("Entities by type:");
        for (mem_type, count) in &stats.total_entities_by_type {
            println!("  {:<12} {}", format!("{mem_type}:"), count);
        }
    }

    // ── Memory Health ──
    println!();
    println!("Memory health:");
    println!("  Decayed entities: {}", stats.decayed_entities);
    println!("  Decayed edges:    {}", stats.decayed_edges);

    if !stats.decayed_entities_by_type.is_empty() {
        for (mem_type, count) in &stats.decayed_entities_by_type {
            println!("    {:<10} {}", format!("{mem_type}:"), count);
        }
    }

    // ── Cleanup Status ──
    println!();
    println!("Cleanup status:");
    println!(
        "  Last cleanup:    {}",
        stats.last_cleanup_at.as_deref().unwrap_or("never")
    );
    println!("  Cleanup interval: every {} queries", stats.cleanup_interval);
    println!("  Queries since:   {}", stats.queries_since_cleanup);
    let next_in = stats
        .cleanup_interval
        .saturating_sub(stats.queries_since_cleanup);
    println!("  Next cleanup in: {} queries", next_in);

    if stats.cleanup_in_progress {
        println!("  ⚠  Cleanup is currently in progress");
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

use ctxgraph::GraphStats;

use super::open_graph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsFormat {
    Text,
    Json,
}

pub fn run(format: StatsFormat) -> ctxgraph::Result<()> {
    let graph = open_graph()?;
    let stats = graph.stats()?;

    match format {
        StatsFormat::Text => println!("{}", render_stats_text(&stats)),
        StatsFormat::Json => println!("{}", render_stats_json(&stats)?),
    }

    Ok(())
}

fn render_stats_json(stats: &GraphStats) -> ctxgraph::Result<String> {
    serde_json::to_string(stats).map_err(ctxgraph::CtxGraphError::from)
}

fn render_stats_text(stats: &GraphStats) -> String {
    let mut lines = vec!["ctxgraph stats".to_string(), "─".repeat(40)];

    // ── Core Counts ──
    lines.push(format!("Episodes:  {}", stats.episode_count));
    lines.push(format!("Entities:  {}", stats.entity_count));
    lines.push(format!("Edges:     {}", stats.edge_count));

    if !stats.sources.is_empty() {
        let sources: Vec<String> = stats
            .sources
            .iter()
            .map(|(name, count)| format!("{name} ({count})"))
            .collect();
        lines.push(format!("Sources:   {}", sources.join(", ")));
    }
    lines.push(format!("DB size:   {}", format_bytes(stats.db_size_bytes)));

    // ── Entities by Type ──
    if !stats.total_entities_by_type.is_empty() {
        lines.push(String::new());
        lines.push("Entities by type:".to_string());
        for (mem_type, count) in &stats.total_entities_by_type {
            lines.push(format!("  {:<12} {}", format!("{mem_type}:"), count));
        }
    }

    // ── Memory Health ──
    lines.push(String::new());
    lines.push("Memory health:".to_string());
    lines.push(format!("  Decayed entities: {}", stats.decayed_entities));
    lines.push(format!("  Decayed edges:    {}", stats.decayed_edges));

    if !stats.decayed_entities_by_type.is_empty() {
        for (mem_type, count) in &stats.decayed_entities_by_type {
            lines.push(format!("    {:<10} {}", format!("{mem_type}:"), count));
        }
    }

    // ── Cleanup Status ──
    lines.push(String::new());
    lines.push("Cleanup status:".to_string());
    lines.push(format!(
        "  Last cleanup:    {}",
        stats.last_cleanup_at.as_deref().unwrap_or("never")
    ));
    lines.push(format!(
        "  Cleanup interval: every {} queries",
        stats.cleanup_interval
    ));
    lines.push(format!(
        "  Queries since:   {}",
        stats.queries_since_cleanup
    ));
    let next_in = stats
        .cleanup_interval
        .saturating_sub(stats.queries_since_cleanup);
    lines.push(format!("  Next cleanup in: {} queries", next_in));

    if stats.cleanup_in_progress {
        lines.push("  ⚠  Cleanup is currently in progress".to_string());
    }

    lines.join("\n")
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

#[cfg(test)]
mod tests {
    use super::{render_stats_json, render_stats_text};
    use ctxgraph::GraphStats;

    #[test]
    fn stats_json_output_contains_complete_graph_stats_payload() {
        let stats = GraphStats {
            episode_count: 2,
            entity_count: 3,
            edge_count: 4,
            sources: vec![("slack".to_string(), 2)],
            db_size_bytes: 4096,
            decayed_entities: 1,
            decayed_edges: 0,
            last_cleanup_at: Some("2026-09-25T05:00:00Z".to_string()),
            queries_since_cleanup: 7,
            cleanup_interval: 100,
            cleanup_in_progress: false,
            total_entities_by_type: vec![("fact".to_string(), 3)],
            decayed_entities_by_type: vec![("experience".to_string(), 1)],
        };

        let json = render_stats_json(&stats).expect("stats should serialize to JSON");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(value["episode_count"], 2);
        assert_eq!(value["entity_count"], 3);
        assert_eq!(value["edge_count"], 4);
        assert_eq!(value["sources"][0][0], "slack");
        assert_eq!(value["sources"][0][1], 2);
        assert_eq!(value["db_size_bytes"], 4096);
        assert_eq!(value["decayed_entities"], 1);
        assert_eq!(value["decayed_edges"], 0);
        assert_eq!(value["last_cleanup_at"], "2026-09-25T05:00:00Z");
        assert_eq!(value["queries_since_cleanup"], 7);
        assert_eq!(value["cleanup_interval"], 100);
        assert_eq!(value["cleanup_in_progress"], false);
        assert_eq!(value["total_entities_by_type"][0][0], "fact");
        assert_eq!(value["total_entities_by_type"][0][1], 3);
        assert_eq!(value["decayed_entities_by_type"][0][0], "experience");
        assert_eq!(value["decayed_entities_by_type"][0][1], 1);
    }

    #[test]
    fn stats_text_output_stays_human_readable_by_default() {
        let stats = GraphStats {
            episode_count: 0,
            entity_count: 0,
            edge_count: 0,
            sources: Vec::new(),
            db_size_bytes: 0,
            decayed_entities: 0,
            decayed_edges: 0,
            last_cleanup_at: None,
            queries_since_cleanup: 0,
            cleanup_interval: 100,
            cleanup_in_progress: false,
            total_entities_by_type: Vec::new(),
            decayed_entities_by_type: Vec::new(),
        };

        let text = render_stats_text(&stats);

        assert!(text.contains("ctxgraph stats"));
        assert!(text.contains("Episodes:  0"));
        assert!(text.contains("Memory health:"));
        assert!(text.contains("Cleanup status:"));
    }
}

use ctxgraph::{Entity, EntityContext, Episode};

pub fn print_episode(episode: &Episode, score: Option<f64>) {
    let id_short = &episode.id[..8.min(episode.id.len())];
    let source = episode.source.as_deref().unwrap_or("unknown");
    let date = episode.recorded_at.format("%Y-%m-%d %H:%M");

    if let Some(s) = score {
        println!("  [{id_short}] ({source}, {date}) score={s:.2}");
    } else {
        println!("  [{id_short}] ({source}, {date})");
    }

    let content = display_content_preview(&episode.content, 200);
    println!("    {content}");

    if let Some(meta) = &episode.metadata
        && let Some(tags) = meta.get("tags")
    {
        println!("    tags: {tags}");
    }

    println!();
}

pub fn print_episode_row(episode: &Episode) {
    let id_short = &episode.id[..8.min(episode.id.len())];
    let source = episode.source.as_deref().unwrap_or("unknown");
    let content = display_content_preview(&episode.content, 50);
    println!("{:<12} {:<12} {}", id_short, source, content);
}

fn display_content_preview(content: &str, max_chars: usize) -> String {
    let char_count = content.chars().count();
    if char_count <= max_chars {
        return content.to_string();
    }

    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let prefix: String = content.chars().take(max_chars - 3).collect();
    format!("{prefix}...")
}

pub fn print_entity_row(entity: &Entity) {
    let id_short = &entity.id[..8.min(entity.id.len())];
    println!(
        "{:<12} {:<14} {}",
        id_short, entity.entity_type, entity.name
    );
}

pub fn print_entity_context(context: &EntityContext) {
    let e = &context.entity;
    println!("Entity: {} ({})", e.name, e.entity_type);
    println!("ID: {}", e.id);
    println!("Created: {}", e.created_at.format("%Y-%m-%d %H:%M"));

    if let Some(summary) = &e.summary {
        println!("Summary: {summary}");
    }

    if !context.edges.is_empty() {
        println!("\nRelationships:");
        for edge in &context.edges {
            let direction = if edge.source_id == e.id {
                format!(
                    "--[{}]--> {}",
                    edge.relation,
                    find_name(&context.neighbors, &edge.target_id)
                )
            } else {
                format!(
                    "<--[{}]-- {}",
                    edge.relation,
                    find_name(&context.neighbors, &edge.source_id)
                )
            };
            let status = if edge.is_current() {
                ""
            } else {
                " (invalidated)"
            };
            println!("  {direction}{status}");
        }
    }

    if !context.neighbors.is_empty() {
        println!("\nNeighbors:");
        for n in &context.neighbors {
            println!("  {} ({})", n.name, n.entity_type);
        }
    }
}

fn find_name(entities: &[Entity], id: &str) -> String {
    entities
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.name.clone())
        .unwrap_or_else(|| id[..8.min(id.len())].to_string())
}

#[cfg(test)]
mod tests {
    use super::display_content_preview;

    #[test]
    fn content_preview_leaves_short_ascii_unchanged() {
        assert_eq!(display_content_preview("short ASCII", 200), "short ASCII");
    }

    #[test]
    fn content_preview_truncates_long_emoji_without_panicking() {
        let content = "🦀".repeat(80);

        let preview = display_content_preview(&content, 50);

        assert!(preview.ends_with("..."));
        assert!(preview.chars().count() <= 50);
    }

    #[test]
    fn content_preview_truncates_long_cjk_without_panicking() {
        let content = "数据库".repeat(80);

        let preview = display_content_preview(&content, 50);

        assert!(preview.ends_with("..."));
        assert!(preview.chars().count() <= 50);
    }

    #[test]
    fn content_preview_handles_tiny_limits_safely() {
        assert_eq!(display_content_preview("abcdef", 3), "...");
        assert_eq!(display_content_preview("abcdef", 2), "..");
        assert_eq!(display_content_preview("abcdef", 0), "");
    }
}

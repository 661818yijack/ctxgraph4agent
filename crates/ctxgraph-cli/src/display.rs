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

    let content = shorten_for_display(&episode.content, 200);
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
    let content = shorten_for_display(&episode.content, 50);
    println!("{:<12} {:<12} {}", id_short, source, content);
}

fn shorten_for_display(content: &str, max_chars: usize) -> String {
    let content_chars = content.chars().count();
    if content_chars <= max_chars {
        return content.to_string();
    }

    let prefix_chars = max_chars.saturating_sub(3);
    let mut shortened: String = content.chars().take(prefix_chars).collect();
    shortened.push_str("...");
    shortened
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
    use super::shorten_for_display;

    #[test]
    fn short_ascii_content_is_unchanged() {
        assert_eq!(shorten_for_display("short note", 50), "short note");
    }

    #[test]
    fn exact_width_ascii_content_is_unchanged() {
        let content = "a".repeat(50);
        assert_eq!(shorten_for_display(&content, 50), content);
    }

    #[test]
    fn long_ascii_content_uses_visible_prefix_and_ellipsis() {
        let content = "a".repeat(51);
        assert_eq!(
            shorten_for_display(&content, 50),
            format!("{}...", "a".repeat(47))
        );
    }

    #[test]
    fn long_emoji_content_does_not_slice_inside_character() {
        let content = "😀".repeat(60);
        let shortened = shorten_for_display(&content, 50);
        assert_eq!(shortened.chars().count(), 50);
        assert!(shortened.ends_with("..."));
    }

    #[test]
    fn long_cjk_content_does_not_slice_inside_character() {
        let content = "決策".repeat(40);
        let shortened = shorten_for_display(&content, 50);
        assert_eq!(shortened.chars().count(), 50);
        assert!(shortened.ends_with("..."));
    }
}

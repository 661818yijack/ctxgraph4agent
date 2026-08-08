//! `ctxgraph reverify` — list and manage stale memories for re-verification.

use ctxgraph::{MemoryType, StaleMemory};

use super::open_graph;

pub struct ReverifyListOptions {
    pub threshold: f64,
    pub limit: usize,
    pub offset: usize,
    pub format: String,
}

pub struct ReverifyRenewOptions {
    pub id: String,
    pub memory_type: String,
}

pub struct ReverifyUpdateOptions {
    pub id: String,
    pub content: Option<String>,
    pub memory_type: Option<String>,
}

pub struct ReverifyExpireOptions {
    pub id: String,
}

/// List stale memories for re-verification.
pub fn list(options: ReverifyListOptions) -> ctxgraph::Result<()> {
    let graph = open_graph()?;
    let stale_memories =
        graph
            .storage
            .get_stale_memories(options.threshold, options.limit, options.offset)?;

    if stale_memories.is_empty() {
        println!(
            "No stale memories found (threshold: {:.2})",
            options.threshold
        );
        return Ok(());
    }

    match options.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&stale_memories)?);
        }
        _ => {
            print_stale_table(&stale_memories);
        }
    }

    Ok(())
}

/// Renew a specific memory, bypassing max_renewals.
pub fn renew(options: ReverifyRenewOptions) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    let memory_type = MemoryType::from_db(&options.memory_type);
    let renewed = graph
        .storage
        .renew_memory_bypass(&options.id, memory_type)?;

    if renewed {
        println!("Successfully renewed memory: {}", options.id);
    } else {
        println!("Memory not found or could not be renewed: {}", options.id);
    }

    Ok(())
}

/// Update a memory's content and/or memory_type.
pub fn update(options: ReverifyUpdateOptions) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    if options.content.is_none() && options.memory_type.is_none() {
        return Err(ctxgraph::CtxGraphError::InvalidInput(
            "at least one of --content or --memory-type must be provided".to_string(),
        ));
    }

    let memory_type = options.memory_type.map(|mt| MemoryType::from_db(&mt));
    graph
        .storage
        .update_memory(&options.id, options.content.as_deref(), memory_type)?;

    println!("Successfully updated memory: {}", options.id);
    Ok(())
}

/// Immediately expire a memory.
pub fn expire(options: ReverifyExpireOptions) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    graph.storage.expire_memory(&options.id)?;

    println!("Successfully expired memory: {}", options.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::truncate_for_table;

    #[test]
    fn truncate_for_table_keeps_short_unicode_content_unchanged() {
        assert_eq!(truncate_for_table("修复完成✅", 40), "修复完成✅");
    }

    #[test]
    fn truncate_for_table_keeps_exact_width_ascii_unchanged() {
        assert_eq!(truncate_for_table("abcdefgh", 8), "abcdefgh");
    }

    #[test]
    fn truncate_for_table_shortens_long_ascii_with_ellipsis() {
        assert_eq!(
            truncate_for_table("abcdefghijklmnopqrstuvwxyz", 10),
            "abcdefg..."
        );
    }

    #[test]
    fn truncate_for_table_shortens_long_unicode_without_panicking() {
        assert_eq!(truncate_for_table("🚀🚀🚀🚀🚀🚀", 5), "🚀🚀...");
    }

    #[test]
    fn truncate_for_table_keeps_short_ids_unchanged() {
        assert_eq!(truncate_for_table("BUG-104", 8), "BUG-104");
    }
}

fn truncate_for_table(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    let prefix_len = max_chars.saturating_sub(3);
    let mut truncated = value.chars().take(prefix_len).collect::<String>();
    truncated.push_str("...");
    truncated
}

/// Print a human-readable table of stale memories.
fn print_stale_table(memories: &[StaleMemory]) {
    // Header
    println!(
        "{:<12} {:<12} {:<8} {:<10} {:<12} CONTENT",
        "ID", "TYPE", "AGE", "DECAY", "ACTION"
    );
    println!("{}", "-".repeat(100));

    for memory in memories {
        let id_short = truncate_for_table(&memory.id, 8);
        let content = truncate_for_table(&memory.content, 40);

        let action_str = match memory.suggested_action {
            ctxgraph::StaleAction::Renew => "renew",
            ctxgraph::StaleAction::Update => "update",
            ctxgraph::StaleAction::Expire => "expire",
            ctxgraph::StaleAction::Keep => "keep",
        };

        println!(
            "{:<12} {:<12} {:<8} {:<10.3} {:<12} {}",
            id_short,
            memory.memory_type,
            format!("{}d", memory.age_days),
            memory.decay_score,
            action_str,
            content
        );
    }
}

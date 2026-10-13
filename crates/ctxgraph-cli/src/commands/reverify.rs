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

/// Return a character-boundary-safe preview, appending an ASCII ellipsis when shortened.
fn shorten_with_ellipsis(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let preview: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{preview}...")
    } else {
        value.to_string()
    }
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
        let id_short = shorten_with_ellipsis(&memory.id, 8);
        let content = shorten_with_ellipsis(&memory.content, 37);

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

#[cfg(test)]
mod tests {
    use super::*;
    use ctxgraph::{MemoryType, StaleAction};

    fn stale_memory(id: &str, content: &str, suggested_action: StaleAction) -> StaleMemory {
        StaleMemory {
            id: id.to_string(),
            memory_type: MemoryType::Fact,
            content: content.to_string(),
            age_days: 42.0,
            decay_score: 0.25,
            suggested_action,
        }
    }

    #[test]
    fn shorten_with_ellipsis_preserves_ascii_compatibility() {
        assert_eq!(shorten_with_ellipsis("abcdefghijk", 8), "abcdefgh...");
        assert_eq!(
            shorten_with_ellipsis("short ascii content", 37),
            "short ascii content"
        );
    }

    #[test]
    fn shorten_with_ellipsis_preserves_short_non_ascii_values() {
        assert_eq!(shorten_with_ellipsis("短內容", 37), "短內容");
        assert_eq!(shorten_with_ellipsis("短id", 8), "短id");
    }

    #[test]
    fn print_stale_table_handles_long_chinese_content() {
        let memories = vec![stale_memory(
            "ascii-id",
            "這是一段很長的中文記憶內容，用來確認表格預覽不會在多位元組字元中間切開造成恐慌",
            StaleAction::Update,
        )];

        let result = std::panic::catch_unwind(|| print_stale_table(&memories));

        assert!(result.is_ok());
    }

    #[test]
    fn print_stale_table_handles_long_emoji_content() {
        let memories = vec![stale_memory(
            "ascii-id",
            "🧠🚀✨🔥🌊🌙⭐️🍀🎯📚🛠️🧪💡🧭🧩🪄🎨🎵🌈🐉",
            StaleAction::Renew,
        )];

        let result = std::panic::catch_unwind(|| print_stale_table(&memories));

        assert!(result.is_ok());
    }

    #[test]
    fn print_stale_table_handles_long_non_ascii_id() {
        let memories = vec![stale_memory(
            "記憶識別碼非常長超過八個字元",
            "short ascii content",
            StaleAction::Expire,
        )];

        let result = std::panic::catch_unwind(|| print_stale_table(&memories));

        assert!(result.is_ok());
    }

    #[test]
    fn print_stale_table_keeps_short_values_working() {
        let memories = vec![
            stale_memory("短id", "短內容", StaleAction::Keep),
            stale_memory("abcdefghijk", "short ascii content", StaleAction::Update),
        ];

        let result = std::panic::catch_unwind(|| print_stale_table(&memories));

        assert!(result.is_ok());
    }
}

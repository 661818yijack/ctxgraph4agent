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

    let memory_type = parse_reverify_memory_type(&options.memory_type)?;
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

fn parse_reverify_memory_type(value: &str) -> ctxgraph::Result<MemoryType> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "fact" => Ok(MemoryType::Fact),
        "pattern" => Ok(MemoryType::Pattern),
        "experience" => Ok(MemoryType::Experience),
        "preference" => Ok(MemoryType::Preference),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(ctxgraph::CtxGraphError::InvalidInput(format!(
            "invalid memory type '{value}'. Expected one of: fact, pattern, experience, preference, decision"
        ))),
    }
}

fn parse_optional_reverify_memory_type(
    value: Option<String>,
) -> ctxgraph::Result<Option<MemoryType>> {
    value.as_deref().map(parse_reverify_memory_type).transpose()
}

/// Update a memory's content and/or memory_type.
pub fn update(options: ReverifyUpdateOptions) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    if options.content.is_none() && options.memory_type.is_none() {
        return Err(ctxgraph::CtxGraphError::InvalidInput(
            "at least one of --content or --memory-type must be provided".to_string(),
        ));
    }

    let memory_type = parse_optional_reverify_memory_type(options.memory_type)?;
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
    use super::{parse_optional_reverify_memory_type, parse_reverify_memory_type};
    use ctxgraph::{CtxGraphError, MemoryType};

    #[test]
    fn parse_memory_type_accepts_valid_canonical_values() {
        assert_eq!(
            parse_reverify_memory_type("fact").unwrap(),
            MemoryType::Fact
        );
        assert_eq!(
            parse_reverify_memory_type("decision").unwrap(),
            MemoryType::Decision
        );
        assert_eq!(
            parse_reverify_memory_type("experience").unwrap(),
            MemoryType::Experience
        );
        assert_eq!(
            parse_reverify_memory_type("preference").unwrap(),
            MemoryType::Preference
        );
        assert_eq!(
            parse_reverify_memory_type("pattern").unwrap(),
            MemoryType::Pattern
        );
    }

    #[test]
    fn parse_memory_type_is_case_insensitive_and_trims() {
        assert_eq!(
            parse_reverify_memory_type(" Preference ").unwrap(),
            MemoryType::Preference
        );
    }

    #[test]
    fn parse_memory_type_rejects_typo_instead_of_defaulting_to_fact() {
        let err = parse_reverify_memory_type("prefernce").unwrap_err();
        assert!(matches!(err, CtxGraphError::InvalidInput(_)));
        assert!(err.to_string().contains("invalid memory type"));
        assert!(err.to_string().contains("prefernce"));
    }

    #[test]
    fn parse_memory_type_rejects_empty_value() {
        let err = parse_reverify_memory_type(" ").unwrap_err();
        assert!(matches!(err, CtxGraphError::InvalidInput(_)));
        assert!(err.to_string().contains("invalid memory type"));
    }

    #[test]
    fn optional_parser_allows_absent_memory_type_for_content_only_update() {
        assert_eq!(parse_optional_reverify_memory_type(None).unwrap(), None);
    }

    #[test]
    fn optional_parser_rejects_invalid_update_memory_type() {
        let err = parse_optional_reverify_memory_type(Some("unknown".to_string())).unwrap_err();
        assert!(matches!(err, CtxGraphError::InvalidInput(_)));
        assert!(err.to_string().contains("unknown"));
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
        let id_short = if memory.id.len() > 8 {
            format!("{}...", &memory.id[..8])
        } else {
            memory.id.clone()
        };

        let content = if memory.content.len() > 40 {
            format!("{}...", &memory.content[..37])
        } else {
            memory.content.clone()
        };

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

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
    let after = parse_after(after.as_deref())?;
    let results = graph.search(&text, limit, source.as_deref(), after)?;

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

fn parse_after(after: Option<&str>) -> ctxgraph::Result<Option<DateTime<Utc>>> {
    after
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|err| {
                    ctxgraph::CtxGraphError::InvalidInput(format!(
                        "invalid --after value '{value}': expected RFC3339 timestamp ({err})"
                    ))
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::parse_after;

    #[test]
    fn parse_after_accepts_rfc3339_timestamp() {
        let parsed = parse_after(Some("2026-08-01T10:00:00Z")).unwrap();

        assert!(parsed.is_some());
    }

    #[test]
    fn parse_after_rejects_invalid_timestamp() {
        let err = parse_after(Some("not-a-date")).unwrap_err();

        assert!(err.to_string().contains("invalid --after value"));
    }

    #[test]
    fn parse_after_preserves_absent_filter() {
        let parsed = parse_after(None).unwrap();

        assert!(parsed.is_none());
    }
}

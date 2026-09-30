use chrono::{DateTime, Utc};

use crate::display;

use super::open_graph;

pub fn list(after: Option<String>, source: Option<String>, limit: usize) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    let after_dt = parse_after_filter(after.as_deref())?;

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

fn parse_after_filter(after: Option<&str>) -> ctxgraph::Result<Option<DateTime<Utc>>> {
    let Some(value) = after else {
        return Ok(None);
    };

    DateTime::parse_from_rfc3339(value)
        .map(|dt| Some(dt.with_timezone::<Utc>(&Utc)))
        .map_err(|err| {
            ctxgraph::CtxGraphError::InvalidInput(format!(
                "invalid --after timestamp '{value}': expected RFC3339 date-time, e.g. 2026-09-30T02:05:00Z ({err})"
            ))
        })
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

#[cfg(test)]
mod tests {
    use super::parse_after_filter;

    #[test]
    fn parse_after_filter_rejects_date_only_input() {
        let err = parse_after_filter(Some("2026-09-30")).unwrap_err();
        assert!(err.to_string().contains("invalid --after timestamp"));
        assert!(err.to_string().contains("2026-09-30"));
    }

    #[test]
    fn parse_after_filter_accepts_rfc3339_utc_input() {
        let parsed = parse_after_filter(Some("2026-09-30T02:05:00Z"))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-09-30T02:05:00+00:00");
    }

    #[test]
    fn parse_after_filter_accepts_rfc3339_offset_input() {
        let parsed = parse_after_filter(Some("2026-09-30T10:05:00+08:00"))
            .unwrap()
            .unwrap();
        assert_eq!(parsed.to_rfc3339(), "2026-09-30T02:05:00+00:00");
    }

    #[test]
    fn parse_after_filter_accepts_missing_input() {
        assert!(parse_after_filter(None).unwrap().is_none());
    }
}

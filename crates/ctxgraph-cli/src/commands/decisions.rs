use chrono::{DateTime, Utc};

use crate::display;

use super::open_graph;

pub fn list(after: Option<String>, source: Option<String>, limit: usize) -> ctxgraph::Result<()> {
    let graph = open_graph()?;

    let after_dt = parse_after_filter(after)?;

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

fn parse_after_filter(after: Option<String>) -> ctxgraph::Result<Option<DateTime<Utc>>> {
    let Some(raw_after) = after else {
        return Ok(None);
    };

    DateTime::parse_from_rfc3339(&raw_after)
        .map(|dt| Some(dt.with_timezone(&Utc)))
        .map_err(|err| {
            ctxgraph::CtxGraphError::InvalidInput(format!(
                "invalid --after value '{raw_after}': expected RFC3339 timestamp ({err})"
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
    fn parse_after_filter_none_stays_none() {
        assert!(parse_after_filter(None).unwrap().is_none());
    }

    #[test]
    fn parse_after_filter_accepts_rfc3339_and_normalizes_to_utc() {
        let parsed = parse_after_filter(Some("2026-07-21T10:30:00+02:00".to_string()))
            .unwrap()
            .unwrap();

        assert_eq!(parsed.to_rfc3339(), "2026-07-21T08:30:00+00:00");
    }

    #[test]
    fn parse_after_filter_rejects_malformed_timestamp() {
        let err = parse_after_filter(Some("not-a-date".to_string())).unwrap_err();

        assert!(err
            .to_string()
            .contains("invalid --after value 'not-a-date'"));
    }

    #[test]
    fn parse_after_filter_rejects_date_only_timestamp() {
        let err = parse_after_filter(Some("2026-07-21".to_string())).unwrap_err();

        assert!(err
            .to_string()
            .contains("invalid --after value '2026-07-21'"));
    }
}

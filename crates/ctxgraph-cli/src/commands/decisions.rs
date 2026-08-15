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
    let Some(after) = after else {
        return Ok(None);
    };

    DateTime::parse_from_rfc3339(after)
        .map(|dt| Some(dt.with_timezone::<Utc>(&Utc)))
        .map_err(|err| {
            ctxgraph::CtxGraphError::InvalidInput(format!(
                "--after must be an RFC3339 timestamp, got '{after}': {err}"
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
    use chrono::{TimeZone, Utc};
    use ctxgraph::CtxGraphError;

    #[test]
    fn parse_after_filter_returns_none_when_absent() {
        assert!(parse_after_filter(None).unwrap().is_none());
    }

    #[test]
    fn parse_after_filter_accepts_zulu_timestamp() {
        let parsed = parse_after_filter(Some("2026-08-15T10:00:00Z"))
            .unwrap()
            .expect("valid timestamp should return Some");

        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 8, 15, 10, 0, 0).unwrap());
    }

    #[test]
    fn parse_after_filter_normalizes_offset_timestamp_to_utc() {
        let parsed = parse_after_filter(Some("2026-08-15T18:30:00+08:00"))
            .unwrap()
            .expect("valid offset timestamp should return Some");

        assert_eq!(
            parsed,
            Utc.with_ymd_and_hms(2026, 8, 15, 10, 30, 0).unwrap()
        );
    }

    #[test]
    fn parse_after_filter_rejects_malformed_timestamp() {
        let err = parse_after_filter(Some("not-a-date")).unwrap_err();

        assert!(matches!(err, CtxGraphError::InvalidInput(_)));
        assert!(err.to_string().contains("--after"));
        assert!(err.to_string().contains("RFC3339"));
    }

    #[test]
    fn parse_after_filter_rejects_date_only_value() {
        let err = parse_after_filter(Some("2026-08-15")).unwrap_err();

        assert!(matches!(err, CtxGraphError::InvalidInput(_)));
    }
}

use chrono::{TimeZone, Utc};
use ctxgraph::{Episode, Graph, MemoryType};

#[test]
fn default_recorded_at_is_assigned_at_build_time() {
    let before = Utc::now();
    let episode = Episode::builder("live decision log entry")
        .source("daily-cron")
        .tag("live")
        .meta("author", "apex")
        .build();
    let after = Utc::now();

    assert!(episode.recorded_at >= before);
    assert!(episode.recorded_at <= after);
    assert_eq!(episode.source.as_deref(), Some("daily-cron"));
    assert_eq!(episode.metadata.as_ref().unwrap()["author"], "apex");
}

#[test]
fn explicit_recorded_at_preserves_historical_timestamp() {
    let recorded_at = Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap();

    let episode = Episode::builder("Imported historical ADR")
        .recorded_at(recorded_at)
        .source("adr-import")
        .tag("architecture")
        .meta("ticket", "ADR-42")
        .memory_type(MemoryType::Fact)
        .build();

    assert_eq!(episode.recorded_at, recorded_at);
    assert_eq!(episode.source.as_deref(), Some("adr-import"));
    assert_eq!(episode.memory_type, MemoryType::Fact);
    let metadata = episode.metadata.as_ref().expect("metadata is preserved");
    assert_eq!(metadata["ticket"], "ADR-42");
    assert_eq!(metadata["tags"][0], "architecture");
}

#[tokio::test]
async fn builder_recorded_at_enables_deterministic_temporal_listing() {
    let graph = Graph::in_memory().unwrap();
    let old_time = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let new_time = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();

    graph
        .add_episode(
            Episode::builder("Old imported context")
                .recorded_at(old_time)
                .source("import")
                .build(),
        )
        .await
        .unwrap();
    graph
        .add_episode(
            Episode::builder("New imported context")
                .recorded_at(new_time)
                .source("import")
                .build(),
        )
        .await
        .unwrap();

    let all = graph.list_episodes(10, 0, None, Some("import")).unwrap();
    assert_eq!(
        all.iter().map(|e| e.content.as_str()).collect::<Vec<_>>(),
        vec!["New imported context", "Old imported context"]
    );

    let filtered = graph
        .list_episodes(10, 0, Some(new_time), Some("import"))
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].content, "New imported context");
}

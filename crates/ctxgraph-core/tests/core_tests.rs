use chrono::Utc;
use ctxgraph::*;

fn test_graph() -> Graph {
    Graph::in_memory().expect("failed to create in-memory graph")
}

// ── Episode CRUD ──

#[test]
fn test_episode_insert_and_retrieve() {
    let graph = test_graph();
    let episode = Episode::builder("Chose Postgres over SQLite for billing").build();
    let id = episode.id.clone();

    let result = graph.add_episode(episode).unwrap();
    assert_eq!(result.episode_id, id);

    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    assert_eq!(retrieved.content, "Chose Postgres over SQLite for billing");
}

#[test]
fn test_episode_with_source_and_tags() {
    let graph = test_graph();
    let episode = Episode::builder("Priya approved the discount")
        .source("slack")
        .tag("finance")
        .tag("approval")
        .build();
    let id = episode.id.clone();

    graph.add_episode(episode).unwrap();

    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    assert_eq!(retrieved.source.as_deref(), Some("slack"));
    assert!(retrieved.metadata.is_some());

    let meta = retrieved.metadata.unwrap();
    let tags = meta.get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].as_str().unwrap(), "finance");
}

#[test]
fn test_episode_with_metadata() {
    let graph = test_graph();
    let episode = Episode::builder("Budget approved for Q3")
        .meta("author", "rohan")
        .meta("confidence", serde_json::json!(0.95))
        .build();
    let id = episode.id.clone();

    graph.add_episode(episode).unwrap();

    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    let meta = retrieved.metadata.unwrap();
    assert_eq!(meta.get("author").unwrap().as_str().unwrap(), "rohan");
}

#[test]
fn test_list_episodes() {
    let graph = test_graph();

    for i in 0..5 {
        let ep = Episode::builder(&format!("Decision {i}")).build();
        graph.add_episode(ep).unwrap();
    }

    let episodes = graph.list_episodes(3, 0).unwrap();
    assert_eq!(episodes.len(), 3);

    let all = graph.list_episodes(100, 0).unwrap();
    assert_eq!(all.len(), 5);

    let offset = graph.list_episodes(100, 3).unwrap();
    assert_eq!(offset.len(), 2);
}

#[test]
fn test_episode_not_found() {
    let graph = test_graph();
    let result = graph.get_episode("nonexistent-id").unwrap();
    assert!(result.is_none());
}

// ── Entity CRUD ──

#[test]
fn test_entity_insert_and_retrieve() {
    let graph = test_graph();
    let entity = Entity::new("Postgres", "Component");
    let id = entity.id.clone();

    graph.add_entity(entity).unwrap();

    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.name, "Postgres");
    assert_eq!(retrieved.entity_type, "Component");
}

#[test]
fn test_entity_by_name() {
    let graph = test_graph();
    let entity = Entity::new("Priya Sharma", "Person");
    graph.add_entity(entity).unwrap();

    let found = graph.get_entity_by_name("Priya Sharma").unwrap().unwrap();
    assert_eq!(found.entity_type, "Person");

    let not_found = graph.get_entity_by_name("Nonexistent").unwrap();
    assert!(not_found.is_none());
}

#[test]
fn test_list_entities_with_type_filter() {
    let graph = test_graph();

    graph
        .add_entity(Entity::new("Postgres", "Component"))
        .unwrap();
    graph
        .add_entity(Entity::new("SQLite", "Component"))
        .unwrap();
    graph.add_entity(Entity::new("Priya", "Person")).unwrap();
    graph.add_entity(Entity::new("billing", "Service")).unwrap();

    let all = graph.list_entities(None, 100).unwrap();
    assert_eq!(all.len(), 4);

    let components = graph.list_entities(Some("Component"), 100).unwrap();
    assert_eq!(components.len(), 2);

    let people = graph.list_entities(Some("Person"), 100).unwrap();
    assert_eq!(people.len(), 1);
    assert_eq!(people[0].name, "Priya");
}

// ── Edge CRUD + Bi-temporal ──

#[test]
fn test_edge_insert_and_retrieve() {
    let graph = test_graph();

    let pg = Entity::new("Postgres", "Component");
    let billing = Entity::new("billing", "Service");
    let pg_id = pg.id.clone();
    let billing_id = billing.id.clone();
    graph.add_entity(pg).unwrap();
    graph.add_entity(billing).unwrap();

    let edge = Edge::new(&pg_id, &billing_id, "chosen_for");
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    let edges = graph.get_edges_for_entity(&pg_id).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].id, edge_id);
    assert_eq!(edges[0].relation, "chosen_for");
}

#[test]
fn test_edge_is_current() {
    let edge = Edge::new("a", "b", "test");
    assert!(edge.is_current());
}

#[test]
fn test_edge_invalidation() {
    let graph = test_graph();

    let alice = Entity::new("Alice", "Person");
    let google = Entity::new("Google", "Organization");
    let alice_id = alice.id.clone();
    let google_id = google.id.clone();
    graph.add_entity(alice).unwrap();
    graph.add_entity(google).unwrap();

    let mut edge = Edge::new(&alice_id, &google_id, "works_at");
    edge.valid_from = Some(Utc::now());
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    // Edge should be current
    let edges = graph.get_edges_for_entity(&alice_id).unwrap();
    assert_eq!(edges.len(), 1);
    assert!(edges[0].is_current());

    // Invalidate
    graph.invalidate_edge(&edge_id).unwrap();

    // Should still appear in all-edges query
    let all_edges = graph.get_edges_for_entity(&alice_id).unwrap();
    assert_eq!(all_edges.len(), 1);
    assert!(!all_edges[0].is_current());
}

#[test]
fn test_edge_valid_at() {
    let mut edge = Edge::new("a", "b", "test");
    let now = Utc::now();
    edge.valid_from = Some(now - chrono::Duration::days(30));
    edge.valid_until = Some(now - chrono::Duration::days(10));

    // 20 days ago: should be valid
    assert!(edge.is_valid_at(now - chrono::Duration::days(20)));

    // 5 days ago: should not be valid (after valid_until)
    assert!(!edge.is_valid_at(now - chrono::Duration::days(5)));

    // 40 days ago: should not be valid (before valid_from)
    assert!(!edge.is_valid_at(now - chrono::Duration::days(40)));
}

#[test]
fn test_invalidate_nonexistent_edge() {
    let graph = test_graph();
    let result = graph.invalidate_edge("nonexistent");
    assert!(result.is_err());
}

// ── Episode-Entity Links ──

#[test]
fn test_episode_entity_link() {
    let graph = test_graph();

    let episode = Episode::builder("Chose Postgres for billing").build();
    let ep_id = episode.id.clone();
    graph.add_episode(episode).unwrap();

    let entity = Entity::new("Postgres", "Component");
    let ent_id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    graph
        .link_episode_entity(&ep_id, &ent_id, Some(6), Some(14))
        .unwrap();

    // Link should be idempotent (INSERT OR IGNORE)
    graph
        .link_episode_entity(&ep_id, &ent_id, Some(6), Some(14))
        .unwrap();
}

// ── FTS5 Search ──

#[test]
fn test_fts5_search_episodes() {
    let graph = test_graph();

    graph
        .add_episode(Episode::builder("Chose Postgres over SQLite for billing").build())
        .unwrap();
    graph
        .add_episode(Episode::builder("Switched from REST to gRPC for internal services").build())
        .unwrap();
    graph
        .add_episode(Episode::builder("Priya approved the discount for Reliance").build())
        .unwrap();

    let results = graph.search("Postgres", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].0.content.contains("Postgres"));

    let results = graph.search("billing OR discount", 10).unwrap();
    assert_eq!(results.len(), 2);
}

#[test]
fn test_fts5_search_empty_results() {
    let graph = test_graph();
    graph
        .add_episode(Episode::builder("Chose Postgres").build())
        .unwrap();

    let results = graph.search("nonexistent_term_xyz", 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_fts5_search_entities() {
    let graph = test_graph();

    graph
        .add_entity(Entity::new("Postgres", "Component"))
        .unwrap();
    graph
        .add_entity(Entity::new("SQLite", "Component"))
        .unwrap();
    graph.add_entity(Entity::new("Priya", "Person")).unwrap();

    let results = graph.search_entities("Postgres", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].0.name, "Postgres");

    let results = graph.search_entities("Component", 10).unwrap();
    assert_eq!(results.len(), 2);
}

// ── Entity Context ──

#[test]
fn test_entity_context() {
    let graph = test_graph();

    let pg = Entity::new("Postgres", "Component");
    let billing = Entity::new("billing", "Service");
    let rohan = Entity::new("rohan", "Person");
    let pg_id = pg.id.clone();
    let billing_id = billing.id.clone();
    let rohan_id = rohan.id.clone();

    graph.add_entity(pg).unwrap();
    graph.add_entity(billing).unwrap();
    graph.add_entity(rohan).unwrap();

    graph
        .add_edge(Edge::new(&pg_id, &billing_id, "chosen_for"))
        .unwrap();
    graph
        .add_edge(Edge::new(&rohan_id, &pg_id, "chose"))
        .unwrap();

    let context = graph.get_entity_context(&pg_id).unwrap();
    assert_eq!(context.entity.name, "Postgres");
    assert_eq!(context.edges.len(), 2);
    assert_eq!(context.neighbors.len(), 2);
}

// ── Stats ──

#[test]
fn test_stats() {
    let graph = test_graph();

    graph
        .add_episode(Episode::builder("Decision 1").source("manual").build())
        .unwrap();
    graph
        .add_episode(Episode::builder("Decision 2").source("manual").build())
        .unwrap();
    graph
        .add_episode(Episode::builder("Slack message").source("slack").build())
        .unwrap();

    let pg = Entity::new("Postgres", "Component");
    let pg_id = pg.id.clone();
    graph.add_entity(pg).unwrap();
    let billing = Entity::new("billing", "Service");
    let billing_id = billing.id.clone();
    graph.add_entity(billing).unwrap();

    graph
        .add_edge(Edge::new(&pg_id, &billing_id, "chosen_for"))
        .unwrap();

    let stats = graph.stats().unwrap();
    assert_eq!(stats.episode_count, 3);
    assert_eq!(stats.entity_count, 2);
    assert_eq!(stats.edge_count, 1);
    assert_eq!(stats.sources.len(), 2);
}

// ── Graph Init ──

#[test]
fn test_graph_init_and_open() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    // Init should succeed
    let _graph = Graph::init(dir).unwrap();

    // Init again should fail (already exists)
    let result = Graph::init(dir);
    assert!(result.is_err());

    // Open should succeed
    let db_path = dir.join(".ctxgraph").join("graph.db");
    let _graph = Graph::open(&db_path).unwrap();
}

#[test]
fn test_graph_open_nonexistent() {
    let result = Graph::open(std::path::Path::new("/tmp/nonexistent/graph.db"));
    assert!(result.is_err());
}

// ── Embedding Storage ──

#[test]
fn test_store_and_retrieve_embedding() {
    let graph = test_graph();
    let episode = Episode::builder("Embedding test episode").build();
    let ep_id = episode.id.clone();
    graph.add_episode(episode).unwrap();

    // Store a fake 384-dim embedding
    let embedding: Vec<f32> = (0..384).map(|i| i as f32 / 384.0).collect();
    graph.store_embedding(&ep_id, &embedding).unwrap();

    // Retrieve all embeddings — should include ours
    let all = graph.get_embeddings().unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, ep_id);
    assert_eq!(all[0].1.len(), 384);
    // Check round-trip fidelity for a few values
    for (i, &v) in all[0].1.iter().enumerate() {
        let expected = i as f32 / 384.0;
        assert!(
            (v - expected).abs() < 1e-6,
            "mismatch at index {i}: {v} vs {expected}"
        );
    }
}

#[test]
fn test_get_embeddings_empty() {
    let graph = test_graph();
    let embeddings = graph.get_embeddings().unwrap();
    assert!(embeddings.is_empty());
}

#[test]
fn test_search_fused_no_embeddings() {
    let graph = test_graph();

    graph
        .add_episode(Episode::builder("Chose Postgres for billing").build())
        .unwrap();
    graph
        .add_episode(Episode::builder("Switched from REST to gRPC").build())
        .unwrap();

    // Fused search with a dummy query embedding — FTS5 results only
    let query_embedding = vec![0.0f32; 384];
    let results = graph
        .search_fused("Postgres", &query_embedding, 10)
        .unwrap();

    // Should still return FTS5 hits even with zero-magnitude query embedding
    assert!(!results.is_empty());
    assert!(results[0].episode.content.contains("Postgres"));
}

#[test]
fn test_search_fused_with_embeddings() {
    let graph = test_graph();

    let ep1 = Episode::builder("Chose Postgres for billing").build();
    let ep2 = Episode::builder("Switched from REST to gRPC").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();

    // Synthetic embeddings: ep1 in direction [1, 0, ...], ep2 in direction [0, 1, ...]
    let mut emb1 = vec![0.0f32; 384];
    emb1[0] = 1.0;
    let mut emb2 = vec![0.0f32; 384];
    emb2[1] = 1.0;

    graph.store_embedding(&id1, &emb1).unwrap();
    graph.store_embedding(&id2, &emb2).unwrap();

    // Query in direction of ep1
    let query_embedding = emb1.clone();
    let results = graph
        .search_fused("Postgres", &query_embedding, 10)
        .unwrap();

    // ep1 should rank first (matches both FTS5 and semantic)
    assert!(!results.is_empty());
    assert_eq!(results[0].episode.id, id1);
}

// ── UUID v7 Ordering ──

#[test]
fn test_uuid_v7_is_time_sortable() {
    let id1 = uuid::Uuid::now_v7().to_string();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let id2 = uuid::Uuid::now_v7().to_string();

    assert!(
        id1 < id2,
        "UUID v7 should be lexicographically time-sortable"
    );
}

// ── Migrations Idempotent ──

#[test]
fn test_migrations_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // Open twice — migrations should not fail on second open
    let _storage = ctxgraph::storage::Storage::open(&db_path).unwrap();
    drop(_storage);
    let _storage = ctxgraph::storage::Storage::open(&db_path).unwrap();
}

// ── Entity Deduplication ──

#[test]
fn test_entity_dedup_merges_similar() {
    let graph = test_graph();

    // Add "PostgreSQL" entity
    let pg = Entity::new("PostgreSQL", "Component");
    let (pg_id, merged) = graph.add_entity_deduped(pg, 0.85).unwrap();
    assert!(!merged, "First insert should not be merged");

    // Add "Postgres" entity with dedup threshold 0.85 — should merge
    let postgres = Entity::new("Postgres", "Component");
    let (deduped_id, was_merged) = graph.add_entity_deduped(postgres, 0.85).unwrap();
    assert!(was_merged, "Postgres should be merged into PostgreSQL");
    assert_eq!(
        deduped_id, pg_id,
        "Should return canonical PostgreSQL entity id"
    );

    // Only one entity should exist
    let all = graph.list_entities(Some("Component"), 100).unwrap();
    assert_eq!(
        all.len(),
        1,
        "Only one Component entity should exist after merge"
    );
    assert_eq!(all[0].name, "PostgreSQL");
}

#[test]
fn test_entity_dedup_preserves_different() {
    let graph = test_graph();

    let pg = Entity::new("PostgreSQL", "Component");
    graph.add_entity_deduped(pg, 0.85).unwrap();

    // "Redis" has very low similarity to "PostgreSQL"
    let redis = Entity::new("Redis", "Component");
    let (_, was_merged) = graph.add_entity_deduped(redis, 0.85).unwrap();
    assert!(!was_merged, "Redis should not be merged with PostgreSQL");

    let all = graph.list_entities(Some("Component"), 100).unwrap();
    assert_eq!(
        all.len(),
        2,
        "Both PostgreSQL and Redis should exist as separate entities"
    );
}

#[test]
fn test_entity_dedup_alias_lookup() {
    let graph = test_graph();

    // Add canonical entity
    let pg = Entity::new("PostgreSQL", "Component");
    let (pg_id, _) = graph.add_entity_deduped(pg, 0.85).unwrap();

    // Add alias variant
    let postgres = Entity::new("Postgres", "Component");
    let (merged_id, was_merged) = graph.add_entity_deduped(postgres, 0.85).unwrap();
    assert!(was_merged);
    assert_eq!(merged_id, pg_id);

    // Adding "Postgres" again should hit alias table (exact alias match)
    let postgres2 = Entity::new("Postgres", "Component");
    let (alias_id, alias_merged) = graph.add_entity_deduped(postgres2, 0.85).unwrap();
    assert!(alias_merged, "Second 'Postgres' should hit alias table");
    assert_eq!(alias_id, pg_id, "Alias lookup should return canonical id");
}

// ── Empty Database ──

#[test]
fn test_empty_database_operations() {
    let graph = test_graph();

    // All operations should succeed on empty db
    assert!(graph.list_episodes(10, 0).unwrap().is_empty());
    assert!(graph.list_entities(None, 10).unwrap().is_empty());
    assert!(graph.search("anything", 10).unwrap().is_empty());

    let stats = graph.stats().unwrap();
    assert_eq!(stats.episode_count, 0);
    assert_eq!(stats.entity_count, 0);
    assert_eq!(stats.edge_count, 0);
}

// ── A1: MemoryType and TTL ──

#[test]
fn test_memory_type_from_entity_type_decision() {
    assert_eq!(
        MemoryType::from_entity_type("Decision"),
        MemoryType::Decision
    );
    assert_eq!(
        MemoryType::from_entity_type("decision"),
        MemoryType::Decision
    );
}

#[test]
fn test_memory_type_from_entity_type_unknown_falls_back_to_fact() {
    assert_eq!(
        MemoryType::from_entity_type("UnknownType"),
        MemoryType::Fact
    );
    assert_eq!(MemoryType::from_entity_type("Component"), MemoryType::Fact);
    assert_eq!(MemoryType::from_entity_type(""), MemoryType::Fact);
}

#[test]
fn test_memory_type_default_ttl_fact() {
    use std::time::Duration;
    assert_eq!(
        MemoryType::Fact.default_ttl(),
        Some(Duration::from_secs(90 * 86400))
    );
}

#[test]
fn test_memory_type_default_ttl_pattern_never() {
    assert_eq!(MemoryType::Pattern.default_ttl(), None);
}

#[test]
fn test_memory_type_default_ttl_experience() {
    use std::time::Duration;
    assert_eq!(
        MemoryType::Experience.default_ttl(),
        Some(Duration::from_secs(14 * 86400))
    );
}

#[test]
fn test_memory_type_default_ttl_preference() {
    use std::time::Duration;
    assert_eq!(
        MemoryType::Preference.default_ttl(),
        Some(Duration::from_secs(30 * 86400))
    );
}

#[test]
fn test_memory_type_default_ttl_decision() {
    use std::time::Duration;
    assert_eq!(
        MemoryType::Decision.default_ttl(),
        Some(Duration::from_secs(90 * 86400))
    );
}

#[test]
fn test_memory_type_from_db() {
    assert_eq!(MemoryType::from_db("fact"), MemoryType::Fact);
    assert_eq!(MemoryType::from_db("Pattern"), MemoryType::Pattern);
    assert_eq!(MemoryType::from_db("EXPERIENCE"), MemoryType::Experience);
    assert_eq!(MemoryType::from_db("unknown"), MemoryType::Fact);
}

#[test]
fn test_memory_type_display() {
    assert_eq!(format!("{}", MemoryType::Fact), "fact");
    assert_eq!(format!("{}", MemoryType::Pattern), "pattern");
    assert_eq!(format!("{}", MemoryType::Decision), "decision");
}

#[test]
fn test_entity_new_auto_sets_memory_type_and_ttl() {
    let entity = Entity::new("JWT", "Component");
    assert_eq!(entity.memory_type, MemoryType::Fact); // Component -> Fact
    assert_eq!(entity.ttl, Some(std::time::Duration::from_secs(90 * 86400)));
}

#[test]
fn test_entity_new_decision_type() {
    let entity = Entity::new("Use Postgres", "Decision");
    assert_eq!(entity.memory_type, MemoryType::Decision);
    assert_eq!(entity.ttl, Some(std::time::Duration::from_secs(90 * 86400)));
}

#[test]
fn test_entity_with_explicit_memory() {
    let entity = Entity::with_memory(
        "Recurring bug",
        "Component",
        MemoryType::Pattern,
        None, // never expires
    );
    assert_eq!(entity.memory_type, MemoryType::Pattern);
    assert_eq!(entity.ttl, None);
}

#[test]
fn test_entity_persist_and_retrieve_with_memory_type() {
    let graph = test_graph();
    let entity = Entity::new("Redis", "Component");
    let id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.memory_type, MemoryType::Fact);
    assert_eq!(
        retrieved.ttl,
        Some(std::time::Duration::from_secs(90 * 86400))
    );
}

#[test]
fn test_entity_persist_pattern_no_ttl() {
    let graph = test_graph();
    let entity = Entity::with_memory(
        "Users prefer dark mode",
        "Preference",
        MemoryType::Pattern,
        None,
    );
    let id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.memory_type, MemoryType::Pattern);
    assert_eq!(retrieved.ttl, None);
}

#[test]
fn test_edge_new_auto_sets_memory_type() {
    let edge = Edge::new("e1", "e2", "uses");
    assert_eq!(edge.memory_type, MemoryType::Fact);
    assert_eq!(edge.ttl, Some(std::time::Duration::from_secs(90 * 86400)));
}

#[test]
fn test_edge_persist_and_retrieve_with_memory_type() {
    let graph = test_graph();

    let src = Entity::new("Service A", "Component");
    let tgt = Entity::new("Postgres", "Component");
    graph.add_entity(src.clone()).unwrap();
    graph.add_entity(tgt.clone()).unwrap();

    let edge = Edge::with_memory(
        &src.id,
        &tgt.id,
        "depends on",
        MemoryType::Decision,
        Some(std::time::Duration::from_secs(90 * 86400)),
    );
    graph.add_edge(edge).unwrap();

    let edges = graph.get_edges_for_entity(&src.id).unwrap();
    let retrieved = edges.iter().find(|e| e.relation == "depends on").unwrap();
    assert_eq!(retrieved.memory_type, MemoryType::Decision);
    assert_eq!(
        retrieved.ttl,
        Some(std::time::Duration::from_secs(90 * 86400))
    );
}

// ── A2: decay_score ──

#[test]
fn test_decay_fact_age_zero_returns_base_confidence() {
    // Fact at age=0 should return base_confidence exactly
    let created_at = Utc::now();
    let ttl = Some(std::time::Duration::from_secs(90 * 86400));
    let score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Fact at age=0 should score ~1.0, got {score}"
    );
}

#[test]
fn test_decay_fact_at_ttl_scores_0_25() {
    // Fact at age=ttl with half_life=ttl/2: exp(-2*ln(2)) = 0.25
    let ttl_secs = 90u64 * 86400;
    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
    let created_at = Utc::now() - chrono::Duration::seconds(ttl_secs as i64);
    let score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
    assert!(
        (score - 0.25).abs() < 1e-6,
        "Fact at age=ttl should score ~0.25, got {score}"
    );
}

#[test]
fn test_decay_fact_at_half_ttl_scores_0_5() {
    // Fact at age=half_life (ttl/2) should score 0.5
    let ttl_secs = 90u64 * 86400;
    let half_life = ttl_secs / 2;
    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
    let created_at = Utc::now() - chrono::Duration::seconds(half_life as i64);
    let score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
    assert!(
        (score - 0.5).abs() < 1e-4,
        "Fact at half-life should score ~0.5, got {score}"
    );
}

#[test]
fn test_decay_pattern_never_decays() {
    // Pattern returns base_confidence regardless of age
    let created_at = Utc::now() - chrono::Duration::days(365);
    let score = MemoryType::Pattern.decay_score(0.8, created_at, None);
    assert_eq!(score, 0.8, "Pattern should always return base_confidence");

    // Even with a ttl provided, Pattern ignores it
    let ttl = Some(std::time::Duration::from_secs(30 * 86400));
    let score2 = MemoryType::Pattern.decay_score(0.8, created_at, ttl);
    assert_eq!(score2, 0.8, "Pattern should ignore ttl");
}

#[test]
fn test_decay_experience_linear_halfway() {
    // Experience at age=ttl/2 should score 0.5
    let ttl_secs = 14u64 * 86400;
    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
    let created_at = Utc::now() - chrono::Duration::seconds((ttl_secs / 2) as i64);
    let score = MemoryType::Experience.decay_score(1.0, created_at, ttl);
    assert!(
        (score - 0.5).abs() < 1e-4,
        "Experience at half-ttl should score ~0.5, got {score}"
    );
}

#[test]
fn test_decay_experience_at_ttl_scores_zero() {
    // Experience linear decay hits 0.0 at age=ttl
    let ttl_secs = 14u64 * 86400;
    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
    let created_at = Utc::now() - chrono::Duration::seconds(ttl_secs as i64);
    let score = MemoryType::Experience.decay_score(1.0, created_at, ttl);
    assert!(
        score.abs() < 1e-6,
        "Experience at age=ttl should score ~0.0, got {score}"
    );
}

#[test]
fn test_decay_preference_exponential() {
    // Preference at age=0 scores base_confidence
    let created_at = Utc::now();
    let ttl = Some(std::time::Duration::from_secs(30 * 86400));
    let score = MemoryType::Preference.decay_score(1.0, created_at, ttl);
    assert!(
        (score - 1.0).abs() < 1e-6,
        "Preference at age=0 should score ~1.0, got {score}"
    );

    // At age=half_life (ttl*0.7) should score ~0.5
    let ttl_secs = 30u64 * 86400;
    let half_life = (ttl_secs as f64 * 0.7) as i64;
    let created_at2 = Utc::now() - chrono::Duration::seconds(half_life);
    let ttl2 = Some(std::time::Duration::from_secs(ttl_secs));
    let score2 = MemoryType::Preference.decay_score(1.0, created_at2, ttl2);
    assert!(
        (score2 - 0.5).abs() < 1e-4,
        "Preference at half-life should score ~0.5, got {score2}"
    );
}

#[test]
fn test_decay_decision_same_as_fact() {
    // Decision uses same exponential as Fact (half_life = ttl * 0.5)
    let ttl_secs = 90u64 * 86400;
    let ttl = Some(std::time::Duration::from_secs(ttl_secs));

    let created_at = Utc::now() - chrono::Duration::seconds(ttl_secs as i64);
    let fact_score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
    let decision_score = MemoryType::Decision.decay_score(1.0, created_at, ttl);
    assert!(
        (fact_score - decision_score).abs() < 1e-10,
        "Decision and Fact should have identical decay: fact={fact_score}, decision={decision_score}"
    );
}

#[test]
fn test_decay_expired_returns_zero() {
    // Age > ttl should return 0.0
    let ttl_secs = 90u64 * 86400;
    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
    // Create 91 days ago — one day past ttl
    let created_at = Utc::now() - chrono::Duration::days(91);

    assert_eq!(MemoryType::Fact.decay_score(1.0, created_at, ttl), 0.0);
    assert_eq!(
        MemoryType::Experience.decay_score(1.0, created_at, ttl),
        0.0
    );
    assert_eq!(
        MemoryType::Preference.decay_score(1.0, created_at, ttl),
        0.0
    );
    assert_eq!(MemoryType::Decision.decay_score(1.0, created_at, ttl), 0.0);
}

#[test]
fn test_decay_ttl_none_returns_base_confidence() {
    // Non-pattern with ttl=None returns base_confidence
    let created_at = Utc::now() - chrono::Duration::days(100);
    let score = MemoryType::Fact.decay_score(0.9, created_at, None);
    assert_eq!(score, 0.9);
}

#[test]
fn test_decay_ttl_zero_returns_zero() {
    let created_at = Utc::now();
    let ttl = Some(std::time::Duration::from_secs(0));
    assert_eq!(MemoryType::Fact.decay_score(1.0, created_at, ttl), 0.0);
    assert_eq!(
        MemoryType::Experience.decay_score(1.0, created_at, ttl),
        0.0
    );
}

#[test]
fn test_decay_scores_in_range() {
    // All decay functions must return values in [0.0, 1.0]
    let types = [
        MemoryType::Fact,
        MemoryType::Pattern,
        MemoryType::Experience,
        MemoryType::Preference,
        MemoryType::Decision,
    ];
    let ages_days = [0i64, 7, 14, 30, 45, 90, 100, 365];

    for mt in &types {
        let ttl = mt.default_ttl();
        for &age in &ages_days {
            let created_at = Utc::now() - chrono::Duration::days(age);
            let score = mt.decay_score(1.0, created_at, ttl);
            assert!(
                (0.0..=1.0).contains(&score),
                "{mt:?} at age={age}d score={score} out of range"
            );
        }
    }
}

#[test]
fn test_migration_003_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // First open: fresh DB, migration runs
    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let entity = Entity::new("Test", "Component");
    let id = entity.id.clone();
    graph1.add_entity(entity).unwrap();
    drop(graph1);

    // Second open: same DB, migration re-runs (idempotent)
    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved = graph2.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.memory_type, MemoryType::Fact);
    assert_eq!(
        retrieved.ttl,
        Some(std::time::Duration::from_secs(90 * 86400))
    );
    drop(graph2);

    // Third open: verify still works after double migration
    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved2 = graph3.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved2.memory_type, MemoryType::Fact);
}

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
fn test_migration_003_reopen_safe() {
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

// ── Migration 004: usage_count and last_recalled_at ─────────────────────────────────

#[test]
fn test_migration_004_entity_fields_exist() {
    let graph = test_graph();
    let entity = Entity::new("TestComponent", "Component");
    let id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    // New fields should exist with defaults
    assert_eq!(retrieved.usage_count, 0);
    assert_eq!(retrieved.last_recalled_at, None);
}

#[test]
fn test_migration_004_edge_fields_exist() {
    let graph = test_graph();

    let e1 = Entity::new("Source", "Component");
    let e2 = Entity::new("Target", "Component");
    graph.add_entity(e1.clone()).unwrap();
    graph.add_entity(e2.clone()).unwrap();

    let edge = Edge::new(&e1.id, &e2.id, "depends_on");
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    let edges = graph.get_edges_for_entity(&e1.id).unwrap();
    let retrieved = edges.iter().find(|e| e.id == edge_id).unwrap();

    // New fields should exist with defaults
    assert_eq!(retrieved.usage_count, 0);
    assert_eq!(retrieved.last_recalled_at, None);
}

#[test]
fn test_touch_entity_increments_usage_count() {
    let graph = test_graph();
    let entity = Entity::new("TouchTest", "Component");
    let id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    // Touch the entity 3 times
    graph.touch_entity(&id).unwrap();
    graph.touch_entity(&id).unwrap();
    graph.touch_entity(&id).unwrap();

    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.usage_count, 3);
    // last_recalled_at should be set
    assert!(retrieved.last_recalled_at.is_some());
}

#[test]
fn test_touch_edge_increments_usage_count() {
    let graph = test_graph();

    let e1 = Entity::new("TouchEdgeE1", "Component");
    let e2 = Entity::new("TouchEdgeE2", "Component");
    graph.add_entity(e1.clone()).unwrap();
    graph.add_entity(e2.clone()).unwrap();

    let edge = Edge::new(&e1.id, &e2.id, "connects");
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    // Touch the edge twice
    graph.touch_edge(&edge_id).unwrap();
    graph.touch_edge(&edge_id).unwrap();

    let edges = graph.get_edges_for_entity(&e1.id).unwrap();
    let retrieved = edges.iter().find(|e| e.id == edge_id).unwrap();

    assert_eq!(retrieved.usage_count, 2);
    assert!(retrieved.last_recalled_at.is_some());
}

#[test]
fn test_migration_004_reopen_safe() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // First open: fresh DB, migrations run
    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let entity = Entity::new("Migration004Test", "Component");
    let id = entity.id.clone();
    graph1.add_entity(entity).unwrap();

    // Touch to set usage_count
    graph1.touch_entity(&id).unwrap();
    drop(graph1);

    // Second open: same DB, migrations re-run (idempotent)
    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved = graph2.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.usage_count, 1);
    assert!(retrieved.last_recalled_at.is_some());

    // Touch again and verify increment persists
    graph2.touch_entity(&id).unwrap();
    drop(graph2);

    // Third open: verify usage_count persists after double migration
    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved3 = graph3.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved3.usage_count, 2);
}

#[test]
fn test_migration_004_applies_to_existing_db() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // Simulate an old DB (before migration 004) by creating graph and manually
    // verifying the new columns exist after migration runs
    let graph = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let entity = Entity::new("OldEntity", "Decision");
    let id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    // Existing rows should get usage_count=0 and last_recalled_at=NULL
    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.usage_count, 0);
    assert_eq!(retrieved.last_recalled_at, None);

    // Edge should also get defaults
    let e1 = Entity::new("EdgeTest1", "Component");
    let e2 = Entity::new("EdgeTest2", "Component");
    graph.add_entity(e1.clone()).unwrap();
    graph.add_entity(e2.clone()).unwrap();

    let edge = Edge::new(&e1.id, &e2.id, "related");
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    let edges = graph.get_edges_for_entity(&e1.id).unwrap();
    let retrieved_edge = edges.iter().find(|e| e.id == edge_id).unwrap();
    assert_eq!(retrieved_edge.usage_count, 0);
    assert_eq!(retrieved_edge.last_recalled_at, None);
}

// ── A3: usage_count and last_recalled_at ──

#[test]
fn test_entity_new_has_zero_usage_count_and_null_recalled() {
    let entity = Entity::new("Test", "Component");
    assert_eq!(
        entity.usage_count, 0,
        "new Entity should have usage_count = 0"
    );
    assert!(
        entity.last_recalled_at.is_none(),
        "new Entity should have last_recalled_at = None"
    );
}

#[test]
fn test_entity_with_memory_has_zero_usage_count_and_null_recalled() {
    let entity = Entity::with_memory(
        "Test",
        "Component",
        MemoryType::Fact,
        Some(std::time::Duration::from_secs(86400)),
    );
    assert_eq!(
        entity.usage_count, 0,
        "with_memory Entity should have usage_count = 0"
    );
    assert!(
        entity.last_recalled_at.is_none(),
        "with_memory Entity should have last_recalled_at = None"
    );
}

#[test]
fn test_edge_new_has_zero_usage_count_and_null_recalled() {
    let edge = Edge::new("a", "b", "uses");
    assert_eq!(edge.usage_count, 0, "new Edge should have usage_count = 0");
    assert!(
        edge.last_recalled_at.is_none(),
        "new Edge should have last_recalled_at = None"
    );
}

#[test]
fn test_edge_with_memory_has_zero_usage_count_and_null_recalled() {
    let edge = Edge::with_memory(
        "a",
        "b",
        "uses",
        MemoryType::Decision,
        Some(std::time::Duration::from_secs(86400)),
    );
    assert_eq!(
        edge.usage_count, 0,
        "with_memory Edge should have usage_count = 0"
    );
    assert!(
        edge.last_recalled_at.is_none(),
        "with_memory Edge should have last_recalled_at = None"
    );
}

#[test]
fn test_touch_entity_increments_count() {
    let graph = test_graph();

    // Create an entity
    let entity = Entity::new("TouchTest", "Component");
    let id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    // Touch it the first time
    graph.touch_entity(&id).unwrap();

    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(
        retrieved.usage_count, 1,
        "usage_count should be 1 after first touch"
    );
    assert!(
        retrieved.last_recalled_at.is_some(),
        "last_recalled_at should be Some after first touch"
    );

    // Touch it a second time
    graph.touch_entity(&id).unwrap();

    let retrieved2 = graph.get_entity(&id).unwrap().unwrap();
    assert_eq!(
        retrieved2.usage_count, 2,
        "usage_count should be 2 after second touch"
    );

    // The last_recalled_at should have been updated
    assert!(
        retrieved2.last_recalled_at >= retrieved.last_recalled_at,
        "last_recalled_at should advance or stay same"
    );
}

#[test]
fn test_touch_edge_increments_count() {
    let graph = test_graph();

    // Create two entities and an edge
    let src = Entity::new("Source", "Component");
    let tgt = Entity::new("Target", "Component");
    graph.add_entity(src.clone()).unwrap();
    graph.add_entity(tgt.clone()).unwrap();

    let edge = Edge::new(&src.id, &tgt.id, "depends_on");
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    // Touch the edge
    graph.touch_edge(&edge_id).unwrap();

    let edges = graph.get_edges_for_entity(&src.id).unwrap();
    let retrieved = edges.iter().find(|e| e.id == edge_id).unwrap();
    assert_eq!(
        retrieved.usage_count, 1,
        "usage_count should be 1 after touch"
    );
    assert!(
        retrieved.last_recalled_at.is_some(),
        "last_recalled_at should be Some after touch"
    );

    // Touch again
    graph.touch_edge(&edge_id).unwrap();

    let edges2 = graph.get_edges_for_entity(&src.id).unwrap();
    let retrieved2 = edges2.iter().find(|e| e.id == edge_id).unwrap();
    assert_eq!(
        retrieved2.usage_count, 2,
        "usage_count should be 2 after second touch"
    );
}

#[test]
fn test_touch_nonexistent_returns_error() {
    let graph = test_graph();

    // Touch non-existent entity
    let result = graph.touch_entity("nonexistent-entity-id");
    assert!(
        result.is_err(),
        "touch_entity on non-existent should return error"
    );

    // Touch non-existent edge
    let result = graph.touch_edge("nonexistent-edge-id");
    assert!(
        result.is_err(),
        "touch_edge on non-existent should return error"
    );
}

#[test]
fn test_migration_004_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // First open: fresh DB, migration runs
    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let entity = Entity::new("Migration004", "Component");
    let id = entity.id.clone();
    graph1.add_entity(entity).unwrap();
    drop(graph1);

    // Second open: same DB, migration re-runs (idempotent)
    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved = graph2.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved.usage_count, 0);
    assert!(retrieved.last_recalled_at.is_none());
    drop(graph2);

    // Third open: verify still works after double migration
    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved2 = graph3.get_entity(&id).unwrap().unwrap();
    assert_eq!(retrieved2.usage_count, 0);
    assert!(retrieved2.last_recalled_at.is_none());
}

#[test]
fn test_read_paths_include_new_fields() {
    let graph = test_graph();

    // Create entity
    let entity = Entity::new("ReadTest", "Component");
    let entity_id = entity.id.clone();
    graph.add_entity(entity).unwrap();

    // Create edge
    let src = Entity::new("Src", "Component");
    let tgt = Entity::new("Tgt", "Component");
    graph.add_entity(src.clone()).unwrap();
    graph.add_entity(tgt.clone()).unwrap();
    let edge = Edge::new(&src.id, &tgt.id, "connects");
    let edge_id = edge.id.clone();
    graph.add_edge(edge).unwrap();

    // Touch both to set non-default values
    graph.touch_entity(&entity_id).unwrap();
    graph.touch_entity(&entity_id).unwrap();
    graph.touch_edge(&edge_id).unwrap();

    // Verify via get_entity
    let retrieved_entity = graph.get_entity(&entity_id).unwrap().unwrap();
    assert_eq!(retrieved_entity.usage_count, 2);
    assert!(retrieved_entity.last_recalled_at.is_some());

    // Verify via list_entities
    let entities = graph.list_entities(Some("Component"), 100).unwrap();
    let listed = entities.iter().find(|e| e.id == entity_id).unwrap();
    assert_eq!(listed.usage_count, 2);
    assert!(listed.last_recalled_at.is_some());

    // Verify via get_edges_for_entity
    let edges = graph.get_edges_for_entity(&src.id).unwrap();
    let listed_edge = edges.iter().find(|e| e.id == edge_id).unwrap();
    assert_eq!(listed_edge.usage_count, 1);
    assert!(listed_edge.last_recalled_at.is_some());

    // Verify via search_entities
    let search_results = graph.search_entities("ReadTest", 10).unwrap();
    let searched = search_results
        .iter()
        .find(|(e, _)| e.id == entity_id)
        .unwrap();
    assert_eq!(searched.0.usage_count, 2);
    assert!(searched.0.last_recalled_at.is_some());
}

// ── B1: Episode Compression Pipeline ──────────────────────────────────────

#[test]
fn test_episode_new_has_default_compression_id_none_and_experience_type() {
    let episode = Episode::builder("Test episode").build();
    assert_eq!(
        episode.compression_id, None,
        "new Episode should have compression_id = None"
    );
    assert_eq!(
        episode.memory_type,
        MemoryType::Experience,
        "new Episode should default to Experience memory_type"
    );
}

#[test]
fn test_episode_builder_with_memory_type() {
    let episode = Episode::builder("Compressed summary")
        .memory_type(MemoryType::Pattern)
        .build();
    assert_eq!(episode.memory_type, MemoryType::Pattern);
    assert_eq!(episode.compression_id, None);
}

#[test]
fn test_episode_persist_and_retrieve_with_new_fields() {
    let graph = test_graph();

    // Regular episode
    let episode = Episode::builder("Regular episode content").build();
    let id = episode.id.clone();
    graph.add_episode(episode).unwrap();

    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    assert_eq!(retrieved.compression_id, None);
    assert_eq!(retrieved.memory_type, MemoryType::Experience);

    // Episode with Pattern memory_type
    let pattern_episode = Episode::builder("Pattern summary")
        .memory_type(MemoryType::Pattern)
        .build();
    let pid = pattern_episode.id.clone();
    graph.add_episode(pattern_episode).unwrap();

    let retrieved_pattern = graph.get_episode(&pid).unwrap().unwrap();
    assert_eq!(retrieved_pattern.memory_type, MemoryType::Pattern);
}

#[test]
fn test_migration_006_columns_exist() {
    let graph = test_graph();

    // Insert an episode and verify the new columns are readable
    let episode = Episode::builder("Migration 006 test").build();
    let id = episode.id.clone();
    graph.add_episode(episode).unwrap();

    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    assert!(retrieved.compression_id.is_none());
    assert_eq!(retrieved.memory_type, MemoryType::Experience);
}

#[test]
fn test_migration_006_reopen_safe() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    // First open: fresh DB, migration 006 runs
    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let episode = Episode::builder("Migration 006 reopen test").build();
    let id = episode.id.clone();
    graph1.add_episode(episode).unwrap();
    drop(graph1);

    // Second open: same DB, migration re-runs (idempotent)
    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved = graph2.get_episode(&id).unwrap().unwrap();
    assert_eq!(retrieved.memory_type, MemoryType::Experience);
    assert_eq!(retrieved.compression_id, None);

    // Compress the episode
    let compressed_id = graph2.compress_episodes(std::slice::from_ref(&id)).unwrap();
    drop(graph2);

    // Third open: verify compression_id persists after double migration
    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
    let retrieved2 = graph3.get_episode(&id).unwrap().unwrap();
    assert_eq!(
        retrieved2.compression_id.as_deref(),
        Some(compressed_id.as_str())
    );
}

#[test]
fn test_compress_empty_list_returns_error() {
    let graph = test_graph();
    let result = graph.compress_episodes(&[]);
    assert!(
        result.is_err(),
        "compressing empty episode_ids should return error"
    );
}

#[test]
fn test_generate_compression_summary_empty_returns_error() {
    let graph = test_graph();
    let result = graph.generate_compression_summary(&[]);
    assert!(result.is_err());
}

#[test]
fn test_generate_compression_summary_produces_text() {
    let graph = test_graph();

    // Create episode objects directly (no need to insert for summary generation)
    let ep1 = Episode::builder("Debugged Docker networking issue with bridge driver").build();
    let ep2 = Episode::builder("Fixed Docker DNS resolution by configuring resolv.conf").build();
    let ep3 = Episode::builder("Resolved Docker container restart loop caused by OOM").build();

    let summary = graph
        .generate_compression_summary(&[ep1, ep2, ep3])
        .unwrap();

    // Summary should not be empty
    assert!(!summary.is_empty(), "summary should not be empty");

    // Summary should mention Docker (key topic from source episodes)
    assert!(
        summary.contains("Docker"),
        "summary should mention Docker: {}",
        summary
    );

    // Summary should mention the count
    assert!(
        summary.contains("3 episodes"),
        "summary should mention count: {}",
        summary
    );
}

#[test]
fn test_compress_episodes_creates_summary_with_fact_type() {
    let graph = test_graph();

    // Insert episodes about Docker debugging
    let ep1 = Episode::builder("Debugged Docker networking issue").build();
    let ep2 = Episode::builder("Fixed Docker DNS resolution problem").build();
    let ep3 = Episode::builder("Resolved Docker container restart loop").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    let id3 = ep3.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();
    graph.add_episode(ep3).unwrap();

    let compressed_id = graph
        .compress_episodes(&[id1.clone(), id2.clone(), id3.clone()])
        .unwrap();

    // Verify compressed episode exists with Fact type
    let compressed = graph.get_episode(&compressed_id).unwrap().unwrap();
    assert_eq!(compressed.memory_type, MemoryType::Fact);
    assert_eq!(compressed.source.as_deref(), Some("compression"));
    assert!(!compressed.content.is_empty());

    // Verify source episodes have compression_id set
    let source1 = graph.get_episode(&id1).unwrap().unwrap();
    let source2 = graph.get_episode(&id2).unwrap().unwrap();
    let source3 = graph.get_episode(&id3).unwrap().unwrap();

    assert_eq!(
        source1.compression_id.as_deref(),
        Some(compressed_id.as_str()),
        "source episode 1 should have compression_id set"
    );
    assert_eq!(
        source2.compression_id.as_deref(),
        Some(compressed_id.as_str()),
        "source episode 2 should have compression_id set"
    );
    assert_eq!(
        source3.compression_id.as_deref(),
        Some(compressed_id.as_str()),
        "source episode 3 should have compression_id set"
    );
}

#[test]
fn test_compress_episodes_merges_entity_links() {
    let graph = test_graph();

    // Create entities
    let docker = Entity::new("Docker", "Component");
    let linux = Entity::new("Linux", "Component");
    let nginx = Entity::new("Nginx", "Component");
    let docker_id = docker.id.clone();
    let linux_id = linux.id.clone();
    let nginx_id = nginx.id.clone();
    graph.add_entity(docker).unwrap();
    graph.add_entity(linux).unwrap();
    graph.add_entity(nginx).unwrap();

    // Create episodes with entity links
    let ep1 = Episode::builder("Debugged Docker networking").build();
    let ep2 = Episode::builder("Fixed Linux kernel module").build();
    let ep3 = Episode::builder("Configured Nginx reverse proxy").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    let id3 = ep3.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();
    graph.add_episode(ep3).unwrap();

    // Link entities to episodes
    graph
        .link_episode_entity(&id1, &docker_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id2, &linux_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id3, &nginx_id, None, None)
        .unwrap();
    // Docker also linked to ep3 (shared entity)
    graph
        .link_episode_entity(&id3, &docker_id, None, None)
        .unwrap();

    // Compress
    let compressed_id = graph
        .compress_episodes(&[id1.clone(), id2.clone(), id3.clone()])
        .unwrap();

    // Verify the compressed episode has entity links
    // We can't directly query episode_entities from Graph, but we can verify
    // by checking that the compressed episode shows up in search for "Docker"
    let search_results = graph.search("Docker", 10).unwrap();
    let compressed_found = search_results.iter().any(|(ep, _)| ep.id == compressed_id);
    assert!(
        compressed_found,
        "compressed episode should be findable via FTS5 search"
    );

    // Verify compressed episode has metadata with compressed_count
    let compressed = graph.get_episode(&compressed_id).unwrap().unwrap();
    let count = compressed
        .metadata
        .as_ref()
        .and_then(|m| m.get("compressed_count"))
        .and_then(|v| v.as_u64());
    assert_eq!(
        count,
        Some(3),
        "metadata should contain compressed_count = 3"
    );
}

#[test]
fn test_list_uncompressed_episodes() {
    let graph = test_graph();
    let now = Utc::now();
    let old_time = now - chrono::Duration::days(30);
    let recent_time = now - chrono::Duration::hours(1);

    // Insert old episodes
    let mut ep1 = Episode::builder("Old episode 1").build();
    ep1.recorded_at = old_time;
    let id1 = ep1.id.clone();
    graph.add_episode(ep1).unwrap();

    let mut ep2 = Episode::builder("Old episode 2").build();
    ep2.recorded_at = old_time;
    let id2 = ep2.id.clone();
    graph.add_episode(ep2).unwrap();

    // Insert a recent episode
    let mut ep3 = Episode::builder("Recent episode").build();
    ep3.recorded_at = recent_time;
    let id3 = ep3.id.clone();
    graph.add_episode(ep3).unwrap();

    // List uncompressed before "now" — should return all 3 (none compressed yet)
    let uncompressed = graph.list_uncompressed_episodes(now).unwrap();
    assert_eq!(
        uncompressed.len(),
        3,
        "all 3 episodes should be uncompressed"
    );

    // Compress the two old episodes
    graph
        .compress_episodes(&[id1.clone(), id2.clone()])
        .unwrap();

    // List uncompressed again — should only return the recent one
    let uncompressed2 = graph.list_uncompressed_episodes(now).unwrap();
    assert_eq!(
        uncompressed2.len(),
        1,
        "only the recent episode should be uncompressed"
    );
    assert_eq!(uncompressed2[0].id, id3);
}

#[test]
fn test_list_uncompressed_excludes_already_compressed() {
    let graph = test_graph();
    let future = Utc::now() + chrono::Duration::days(1);

    // Insert episodes
    let ep1 = Episode::builder("Episode A").build();
    let ep2 = Episode::builder("Episode B").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();

    // Compress episode A
    let compressed_id = graph.compress_episodes(std::slice::from_ref(&id1)).unwrap();

    // List uncompressed — should only return B
    let uncompressed = graph.list_uncompressed_episodes(future).unwrap();
    assert_eq!(uncompressed.len(), 1);
    assert_eq!(uncompressed[0].id, id2);

    // Verify compressed episode exists and has Fact type
    let compressed = graph.get_episode(&compressed_id).unwrap().unwrap();
    assert_eq!(compressed.memory_type, MemoryType::Fact);
}

#[test]
fn test_compressed_episode_searchable_via_fts() {
    let graph = test_graph();

    let ep1 = Episode::builder("Deployed Redis cluster with sentinel").build();
    let ep2 = Episode::builder("Migrated Redis to AOF persistence").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();

    let compressed_id = graph.compress_episodes(&[id1, id2]).unwrap();

    // The compressed episode should be findable via FTS5 search
    let results = graph.search("Redis", 10).unwrap();

    // Should find at least the compressed episode (source episodes remain too)
    let compressed_found = results.iter().any(|(ep, _)| ep.id == compressed_id);
    assert!(
        compressed_found,
        "compressed episode should be searchable via FTS5"
    );
}

#[test]
fn test_compress_single_episode() {
    let graph = test_graph();

    let ep = Episode::builder("Single episode to compress").build();
    let id = ep.id.clone();
    graph.add_episode(ep).unwrap();

    let compressed_id = graph.compress_episodes(std::slice::from_ref(&id)).unwrap();

    // Verify
    let source = graph.get_episode(&id).unwrap().unwrap();
    assert_eq!(
        source.compression_id.as_deref(),
        Some(compressed_id.as_str())
    );

    let compressed = graph.get_episode(&compressed_id).unwrap().unwrap();
    assert_eq!(compressed.memory_type, MemoryType::Fact);
    assert!(compressed.content.contains("Single episode"));
}

#[test]
fn test_compress_nonexistent_episode_returns_error() {
    let graph = test_graph();

    // Insert one real episode
    let ep = Episode::builder("Real episode").build();
    let id = ep.id.clone();
    graph.add_episode(ep).unwrap();

    // Try to compress with a nonexistent ID
    let result = graph.compress_episodes(&[id.clone(), "nonexistent-id".to_string()]);
    assert!(
        result.is_err(),
        "compressing with a nonexistent episode should return error"
    );
}

// ── Pattern Description (D1b) Tests ─────────────────────────────────────────

use ctxgraph::pattern::{FailingPatternDescriber, MockPatternDescriber, PatternDescriber};

#[test]
fn test_generate_pattern_description_delegates_to_describer() {
    let graph = test_graph();
    let candidate = PatternCandidate {
        id: "test-id".to_string(),
        entity_types: vec!["Component".to_string()],
        entity_pair: None,
        relation_triplet: None,
        occurrence_count: 3,
        source_groups: vec!["comp1".to_string()],
        confidence: 0.75,
        description: None,
    };

    let describer = MockPatternDescriber::new("Test behavioral description");
    let result = graph.generate_pattern_description(&candidate, &[], &describer);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test behavioral description");
}

#[test]
fn test_mock_pattern_describer_produces_behavioral_insight() {
    // MockPatternDescriber should produce descriptions that:
    // - Are NOT co-occurrence counts
    // - Mention entities/relations from the candidate
    let candidate = PatternCandidate {
        id: "test-id".to_string(),
        entity_types: vec!["Docker".to_string(), "Network".to_string()],
        entity_pair: Some(("Docker".to_string(), "Network".to_string())),
        relation_triplet: Some((
            "Docker".to_string(),
            "depends_on".to_string(),
            "Network".to_string(),
        )),
        occurrence_count: 4,
        source_groups: vec!["comp1".to_string(), "comp2".to_string()],
        confidence: 0.8,
        description: None,
    };

    let describer = MockPatternDescriber::for_candidate(&candidate);
    let description = describer.generate(&candidate, &[]).unwrap();

    // Should NOT contain co-occurrence phrasing
    assert!(
        !description.contains("appears"),
        "description should not contain 'appears': {}",
        description
    );
    assert!(
        !description.contains("times"),
        "description should not contain 'times': {}",
        description
    );
    assert!(
        !description.contains("entity type"),
        "description should not contain 'entity type': {}",
        description
    );
    assert!(
        !description.contains("count"),
        "description should not contain 'count': {}",
        description
    );

    // Should contain references to actual entities from candidate
    assert!(
        description.contains("Docker") || description.contains("docker"),
        "description should mention Docker: {}",
        description
    );
}

#[test]
fn test_mock_pattern_describer_for_triplet_candidate() {
    let candidate = PatternCandidate {
        id: "test-id".to_string(),
        entity_types: vec!["Component".to_string()],
        entity_pair: Some(("User".to_string(), "Postgres".to_string())),
        relation_triplet: Some((
            "User".to_string(),
            "connects_to".to_string(),
            "Postgres".to_string(),
        )),
        occurrence_count: 5,
        source_groups: vec!["comp1".to_string()],
        confidence: 1.0,
        description: None,
    };

    let describer = MockPatternDescriber::for_candidate(&candidate);
    let description = describer.generate(&candidate, &[]).unwrap();

    // Should mention the triplet entities
    assert!(
        description.contains("User"),
        "should mention User: {}",
        description
    );
    assert!(
        description.contains("connects_to"),
        "should mention relation: {}",
        description
    );
    assert!(
        description.contains("Postgres"),
        "should mention Postgres: {}",
        description
    );

    // Should be under 200 chars
    assert!(
        description.len() < 200,
        "description should be < 200 chars: {} (len={})",
        description,
        description.len()
    );
}

#[test]
fn test_mock_pattern_describer_for_type_candidate() {
    let candidate = PatternCandidate {
        id: "test-id".to_string(),
        entity_types: vec!["Component".to_string(), "Infrastructure".to_string()],
        entity_pair: None,
        relation_triplet: None,
        occurrence_count: 3,
        source_groups: vec!["comp1".to_string(), "comp2".to_string()],
        confidence: 0.6,
        description: None,
    };

    let describer = MockPatternDescriber::for_candidate(&candidate);
    let description = describer.generate(&candidate, &[]).unwrap();

    // Should mention the entity types
    assert!(
        description.contains("Component") || description.contains("Infrastructure"),
        "should mention entity types: {}",
        description
    );

    // Should NOT be a co-occurrence count description
    assert!(
        !description.contains("appears"),
        "should not contain 'appears': {}",
        description
    );
}

#[test]
fn test_failing_pattern_describer_returns_error() {
    let candidate = PatternCandidate {
        id: "test-id".to_string(),
        entity_types: vec!["Component".to_string()],
        entity_pair: None,
        relation_triplet: None,
        occurrence_count: 1,
        source_groups: vec![],
        confidence: 1.0,
        description: None,
    };

    let describer = FailingPatternDescriber::new("LLM unavailable");
    let result = describer.generate(&candidate, &[]);

    assert!(
        result.is_err(),
        "FailingPatternDescriber should return error"
    );
}

#[test]
fn test_extract_and_describe_patterns_with_mock() {
    let graph = test_graph();

    // Create some episodes and compress them to get compression groups
    let ep1 = Episode::builder("Docker container failed to start because network was down").build();
    let ep2 = Episode::builder("Network bridge issue prevented Docker from starting").build();
    let ep3 = Episode::builder("Docker needs network to connect to registry").build();

    graph.add_episode(ep1.clone()).unwrap();
    graph.add_episode(ep2.clone()).unwrap();
    graph.add_episode(ep3.clone()).unwrap();

    // Compress to create compression groups
    let _compressed_id = graph
        .compress_episodes(&[ep1.id.clone(), ep2.id.clone(), ep3.id.clone()])
        .unwrap();

    // Extract and describe patterns
    let describer = MockPatternDescriber::new(
        "When Docker networking issues occur, the agent typically needs to restart the container and check bridge configuration.",
    );
    let config = PatternExtractorConfig {
        min_occurrence_count: 1,
        max_patterns_per_extraction: 10,
        ..Default::default()
    };

    let patterns = graph.extract_and_describe_patterns(&config, &describer);

    assert!(
        patterns.is_ok(),
        "extract_and_describe_patterns should succeed"
    );
}

#[test]
fn test_extract_and_describe_patterns_empty_candidates_returns_empty() {
    let graph = test_graph();

    // No episodes, so no compression groups, so no pattern candidates
    let describer = MockPatternDescriber::new("This should not be called");
    let config = PatternExtractorConfig::default();

    let patterns = graph.extract_and_describe_patterns(&config, &describer);

    assert!(patterns.is_ok());
    assert!(patterns.unwrap().is_empty());
}

#[test]
fn test_store_pattern_creates_learned_pattern_entity() {
    let graph = test_graph();

    // Create and store a pattern directly via storage
    let candidate = PatternCandidate {
        id: "pattern-123".to_string(),
        entity_types: vec!["Component".to_string()],
        entity_pair: None,
        relation_triplet: None,
        occurrence_count: 3,
        source_groups: vec!["comp1".to_string()],
        confidence: 0.75,
        description: Some(
            "When Docker networking issues occur, restart the container first.".to_string(),
        ),
    };

    // Store via graph's public API
    let result = graph.store_pattern(&candidate);
    assert!(result.is_ok(), "store_pattern should succeed: {:?}", result);

    // Verify we can retrieve it
    let patterns = graph.get_patterns().unwrap();
    assert!(!patterns.is_empty(), "should have at least one pattern");

    let stored = patterns.iter().find(|p| p.id == "pattern-123").unwrap();
    assert!(
        stored.description.is_some(),
        "pattern should have description"
    );
    assert!(
        stored
            .description
            .as_ref()
            .unwrap()
            .contains("Docker networking"),
        "description should be preserved: {:?}",
        stored.description
    );
}

#[test]
fn test_get_patterns_returns_stored_patterns_with_descriptions() {
    let graph = test_graph();

    // Store a pattern
    let candidate = PatternCandidate {
        id: "pattern-456".to_string(),
        entity_types: vec!["Service".to_string()],
        entity_pair: Some(("API".to_string(), "Database".to_string())),
        relation_triplet: None,
        occurrence_count: 4,
        source_groups: vec!["comp1".to_string(), "comp2".to_string()],
        confidence: 0.8,
        description: Some(
            "The API and Database frequently experience connectivity issues together.".to_string(),
        ),
    };

    graph.store_pattern(&candidate).unwrap();

    // Retrieve patterns
    let patterns = graph.get_patterns().unwrap();

    let found = patterns.iter().find(|p| p.id == "pattern-456").unwrap();
    assert!(
        found.description.is_some(),
        "retrieved pattern should have description"
    );
    assert!(
        found.description.as_ref().unwrap().contains("connectivity"),
        "description content should be preserved"
    );
}

#[test]
fn test_llm_failure_skips_candidate_continues_pipeline() {
    let graph = test_graph();

    // Create episodes and compress
    let ep1 = Episode::builder("Service A depends on Service B").build();
    let ep2 = Episode::builder("Service B went down affecting Service A").build();

    graph.add_episode(ep1.clone()).unwrap();
    graph.add_episode(ep2.clone()).unwrap();

    graph
        .compress_episodes(&[ep1.id.clone(), ep2.id.clone()])
        .unwrap();

    // Use failing describer - should not crash, just skip
    let failing_describer = FailingPatternDescriber::new("LLM unavailable");
    let config = PatternExtractorConfig {
        min_occurrence_count: 1,
        max_patterns_per_extraction: 10,
        ..Default::default()
    };

    let result = graph.extract_and_describe_patterns(&config, &failing_describer);

    // Should succeed but return empty (candidates skipped due to LLM failure)
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_entity_name_truncated_at_word_boundary_80_chars() {
    let graph = test_graph();

    // Create a description longer than 80 chars
    let long_description = "When Docker networking issues occur in production environments, the agent should restart the container service and verify the network bridge configuration before assuming the daemon is healthy.".to_string();
    assert!(
        long_description.len() > 80,
        "test description should be > 80 chars"
    );

    let candidate = PatternCandidate {
        id: "pattern-789".to_string(),
        entity_types: vec!["Component".to_string()],
        entity_pair: None,
        relation_triplet: None,
        occurrence_count: 2,
        source_groups: vec!["comp1".to_string()],
        confidence: 0.5,
        description: Some(long_description.clone()),
    };

    // Store pattern (which truncates name at word boundary)
    graph.storage.store_pattern(&candidate).unwrap();

    // Retrieve the entity directly to check name
    let entity = graph.get_entity("pattern-789").unwrap().unwrap();

    // Name should be <= 80 chars
    assert!(
        entity.name.len() <= 80,
        "entity name should be <= 80 chars, got {} chars: {}",
        entity.name.len(),
        entity.name
    );

    // Name should end at word boundary (no partial words at end)
    // If it's exactly 80, check it doesn't end mid-word
    if entity.name.len() == 80 {
        assert!(
            !entity.name.ends_with(' '),
            "name should not end with space"
        );
        // The truncation function finds last space in first 80 chars and truncates there
        // So it should be a complete word
    }
}

#[test]
fn test_pattern_has_memory_type_pattern_and_no_ttl() {
    let graph = test_graph();

    let candidate = PatternCandidate {
        id: "pattern-ttl-test".to_string(),
        entity_types: vec!["Service".to_string()],
        entity_pair: None,
        relation_triplet: None,
        occurrence_count: 3,
        source_groups: vec!["comp1".to_string()],
        confidence: 0.75,
        description: Some("Patterns persist indefinitely.".to_string()),
    };

    graph.store_pattern(&candidate).unwrap();

    // Retrieve the entity
    let entity = graph.get_entity("pattern-ttl-test").unwrap().unwrap();

    // Check memory_type is Pattern
    assert_eq!(
        entity.memory_type,
        MemoryType::Pattern,
        "stored pattern should have MemoryType::Pattern"
    );

    // Check ttl is None (never expires)
    assert_eq!(
        entity.ttl, None,
        "stored pattern should have ttl=None (never expires)"
    );
}

#[test]
fn test_description_length_is_1_2_sentences() {
    // Test that MockPatternDescriber produces descriptions in the expected length range
    let candidate = PatternCandidate {
        id: "test".to_string(),
        entity_types: vec!["Component".to_string()],
        entity_pair: None,
        relation_triplet: Some((
            "Docker".to_string(),
            "connects_to".to_string(),
            "Network".to_string(),
        )),
        occurrence_count: 3,
        source_groups: vec!["comp1".to_string()],
        confidence: 0.75,
        description: None,
    };

    let describer = MockPatternDescriber::for_candidate(&candidate);
    let description = describer.generate(&candidate, &[]).unwrap();

    // Should be less than 200 chars (1-2 sentences)
    assert!(
        description.len() < 200,
        "description should be < 200 chars, got {}: {}",
        description.len(),
        description
    );

    // Should have at least some content
    assert!(
        description.len() > 20,
        "description should be meaningful, got {} chars: {}",
        description.len(),
        description
    );
}

#[test]
fn test_description_does_not_contain_rejected_phrases() {
    let candidate = PatternCandidate {
        id: "test".to_string(),
        entity_types: vec!["Component".to_string(), "Service".to_string()],
        entity_pair: Some(("A".to_string(), "B".to_string())),
        relation_triplet: None,
        occurrence_count: 5,
        source_groups: vec!["comp1".to_string(), "comp2".to_string()],
        confidence: 0.8,
        description: None,
    };

    let describer = MockPatternDescriber::for_candidate(&candidate);
    let description = describer.generate(&candidate, &[]).unwrap().to_lowercase();

    let rejected_phrases = [
        "appears ",
        "appeared ",
        "appears in ",
        "appeared in ",
        "entity type",
        "entity types",
        "co-occurrence",
        "count of",
        "times across",
    ];

    for phrase in rejected_phrases {
        assert!(
            !description.contains(phrase),
            "description should not contain '{}': {}",
            phrase,
            description
        );
    }
}

// ── D1a Integration Tests ──────────────────────────────────────────────────

#[test]
fn test_extract_pattern_candidates_end_to_end() {
    // Full end-to-end: create episodes → compress → add entities/edges → extract patterns
    let graph = test_graph();

    // Create episodes about Docker networking
    let ep1 = Episode::builder("Docker container failed to start because network was down").build();
    let ep2 = Episode::builder("Fixed Docker DNS resolution by checking resolv.conf").build();
    let ep3 =
        Episode::builder("Docker network bridge misconfiguration caused restart loop").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    let id3 = ep3.id.clone();

    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();
    graph.add_episode(ep3).unwrap();

    // Create entities
    let docker = Entity::new("Docker", "Component");
    let network = Entity::new("Network", "Infrastructure");
    let dns = Entity::new("DNS", "Service");
    let docker_id = docker.id.clone();
    let network_id = network.id.clone();
    let dns_id = dns.id.clone();
    graph.add_entity(docker).unwrap();
    graph.add_entity(network).unwrap();
    graph.add_entity(dns).unwrap();

    // Link entities to episodes
    graph
        .link_episode_entity(&id1, &docker_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id1, &network_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id2, &docker_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id2, &dns_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id3, &docker_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id3, &network_id, None, None)
        .unwrap();

    // Create edges between entities
    let mut edge1 = Edge::new(&docker_id, &network_id, "depends_on");
    edge1.episode_id = Some(id1.clone());
    graph.add_edge(edge1).unwrap();

    let mut edge2 = Edge::new(&docker_id, &dns_id, "uses");
    edge2.episode_id = Some(id2.clone());
    graph.add_edge(edge2).unwrap();

    let mut edge3 = Edge::new(&docker_id, &network_id, "depends_on");
    edge3.episode_id = Some(id3.clone());
    graph.add_edge(edge3).unwrap();

    // Compress episodes
    let compressed_id = graph
        .compress_episodes(&[id1.clone(), id2.clone(), id3.clone()])
        .unwrap();

    // Extract pattern candidates with low threshold
    let config = PatternExtractorConfig {
        min_occurrence_count: 1,
        min_entity_types: 1,
        max_patterns_per_extraction: 20,
    };
    let candidates = graph.extract_pattern_candidates(&config).unwrap();

    // Should find at least some candidates (Docker appears in all 3 source episodes)
    assert!(
        !candidates.is_empty(),
        "expected pattern candidates from compression group"
    );

    // Verify source_groups includes our compression group
    let has_our_group = candidates
        .iter()
        .any(|c| c.source_groups.contains(&compressed_id));
    assert!(
        has_our_group,
        "at least one candidate should reference our compression group"
    );

    // All candidates should have confidence <= 1.0
    for c in &candidates {
        assert!(
            c.confidence <= 1.0,
            "confidence should not exceed 1.0, got {}",
            c.confidence
        );
        assert!(
            c.occurrence_count <= 1,
            "occurrence_count should not exceed 1 for a single group, got {}",
            c.occurrence_count
        );
    }
}

#[test]
fn test_get_compression_groups_loads_correctly() {
    let graph = test_graph();

    // Create episodes and entities
    let ep1 = Episode::builder("First episode about Redis").build();
    let ep2 = Episode::builder("Second episode about Redis").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();

    let redis = Entity::new("Redis", "Component");
    let redis_id = redis.id.clone();
    graph.add_entity(redis).unwrap();

    graph
        .link_episode_entity(&id1, &redis_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id2, &redis_id, None, None)
        .unwrap();

    // Compress
    let compressed_id = graph
        .compress_episodes(&[id1.clone(), id2.clone()])
        .unwrap();

    // Load compression groups
    let groups = graph.get_compression_groups(Utc::now()).unwrap();

    assert_eq!(groups.len(), 1, "should have exactly 1 compression group");

    let group = &groups[0];
    assert_eq!(
        group.compression_id, compressed_id,
        "compression_id should match"
    );
    assert_eq!(
        group.source_episode_ids.len(),
        2,
        "should have 2 source episodes"
    );
    assert!(
        group.source_episode_ids.contains(&id1),
        "source_episode_ids should contain id1"
    );
    assert!(
        group.source_episode_ids.contains(&id2),
        "source_episode_ids should contain id2"
    );
    assert_eq!(group.entities.len(), 1, "should have 1 entity (Redis)");
    assert_eq!(
        group.entities[0].name, "Redis",
        "entity name should be Redis"
    );
}

#[test]
fn test_extract_pattern_candidates_with_duplicate_edges() {
    // Test that duplicate edges within a single group don't inflate occurrence_count
    let graph = test_graph();

    let ep1 = Episode::builder("Docker depends on Network").build();
    let ep2 = Episode::builder("Docker connects to Network").build();
    let id1 = ep1.id.clone();
    let id2 = ep2.id.clone();
    graph.add_episode(ep1).unwrap();
    graph.add_episode(ep2).unwrap();

    let docker = Entity::new("Docker", "Component");
    let network = Entity::new("Network", "Infrastructure");
    let docker_id = docker.id.clone();
    let network_id = network.id.clone();
    graph.add_entity(docker).unwrap();
    graph.add_entity(network).unwrap();

    graph
        .link_episode_entity(&id1, &docker_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id1, &network_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id2, &docker_id, None, None)
        .unwrap();
    graph
        .link_episode_entity(&id2, &network_id, None, None)
        .unwrap();

    // Create MULTIPLE edges between same entity pair within one group
    let mut edge1 = Edge::new(&docker_id, &network_id, "depends_on");
    edge1.episode_id = Some(id1.clone());
    graph.add_edge(edge1).unwrap();

    let mut edge2 = Edge::new(&docker_id, &network_id, "depends_on");
    edge2.episode_id = Some(id1.clone());
    graph.add_edge(edge2).unwrap();

    let mut edge3 = Edge::new(&docker_id, &network_id, "connects_to");
    edge3.episode_id = Some(id2.clone());
    graph.add_edge(edge3).unwrap();

    // Compress into one group
    graph
        .compress_episodes(&[id1.clone(), id2.clone()])
        .unwrap();

    // Extract with low threshold
    let config = PatternExtractorConfig {
        min_occurrence_count: 1,
        min_entity_types: 1,
        max_patterns_per_extraction: 20,
    };
    let candidates = graph.extract_pattern_candidates(&config).unwrap();

    // Find the pair candidate for (Docker, Network)
    let pair_candidate = candidates.iter().find(|c| {
        c.entity_pair.as_ref().map_or(false, |(a, b)| {
            (a == "Docker" && b == "Network") || (a == "Network" && b == "Docker")
        })
    });

    if let Some(pc) = pair_candidate {
        assert_eq!(
            pc.occurrence_count, 1,
            "pair occurrence_count should be 1 (one group), not inflated by duplicate edges, got {}",
            pc.occurrence_count
        );
        assert!(
            pc.confidence <= 1.0,
            "confidence should be <= 1.0, got {}",
            pc.confidence
        );
    }

    // Find the depends_on triplet candidate
    let triplet_dep = candidates.iter().find(|c| {
        c.relation_triplet.as_ref().map_or(false, |(a, r, b)| {
            a == "Docker" && r == "depends_on" && b == "Network"
        })
    });

    if let Some(tc) = triplet_dep {
        assert_eq!(
            tc.occurrence_count, 1,
            "triplet occurrence_count should be 1 (one group), not inflated by duplicate edges, got {}",
            tc.occurrence_count
        );
    }

    // Find the connects_to triplet candidate
    let triplet_conn = candidates.iter().find(|c| {
        c.relation_triplet.as_ref().map_or(false, |(a, r, b)| {
            a == "Docker" && r == "connects_to" && b == "Network"
        })
    });

    if let Some(tc) = triplet_conn {
        assert_eq!(
            tc.occurrence_count, 1,
            "connects_to triplet occurrence_count should be 1, got {}",
            tc.occurrence_count
        );
    }
}

// ── Contradiction Detection (C1) Tests ───────────────────────────────────────

#[test]
fn test_contradiction_detected_when_new_fact_conflicts() {
    // Insert two conflicting edges, verify first is invalidated
    let graph = test_graph();

    // Create entities
    let alice = Entity::new("Alice", "Person");
    let postgres = Entity::new("PostgreSQL", "Database");
    let mysql = Entity::new("MySQL", "Database");

    graph.add_entity(alice.clone()).unwrap();
    graph.add_entity(postgres.clone()).unwrap();
    graph.add_entity(mysql.clone()).unwrap();

    // First edge: Alice chose PostgreSQL
    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
    edge1.confidence = 0.9;
    edge1.fact = Some("Alice chose PostgreSQL".to_string());
    let edge1_id = edge1.id.clone();
    graph.add_edge(edge1).unwrap();

    // Second edge: Alice chose MySQL (conflicts with first)
    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
    edge2.confidence = 0.9;
    edge2.fact = Some("Alice chose MySQL".to_string());
    let edge2_for_check = edge2.clone();
    graph.add_edge(edge2).unwrap();

    // Check for contradictions
    let contradictions = graph
        .storage
        .check_contradictions(&[edge2_for_check], 0.2)
        .unwrap();

    // Should detect exactly one contradiction
    assert_eq!(contradictions.len(), 1, "Expected 1 contradiction, got {}", contradictions.len());
    assert_eq!(contradictions[0].old_edge_id, edge1_id);
    assert_eq!(contradictions[0].relation, "chose");
}

#[test]
fn test_contradiction_invalidated_edge_has_metadata() {
    // Verify contradicted_by and contradicted_at in metadata
    let graph = test_graph();

    let alice = Entity::new("Alice", "Person");
    let postgres = Entity::new("PostgreSQL", "Database");
    let mysql = Entity::new("MySQL", "Database");

    graph.add_entity(alice.clone()).unwrap();
    graph.add_entity(postgres.clone()).unwrap();
    graph.add_entity(mysql.clone()).unwrap();

    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
    edge1.confidence = 0.9;
    edge1.fact = Some("Alice chose PostgreSQL".to_string());
    let edge1_id = edge1.id.clone();
    graph.add_edge(edge1).unwrap();

    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
    edge2.confidence = 0.9;
    edge2.fact = Some("Alice chose MySQL".to_string());
    let edge2_for_check = edge2.clone();
    graph.add_edge(edge2).unwrap();

    // Check for contradictions and invalidate
    let contradictions = graph
        .storage
        .check_contradictions(&[edge2_for_check], 0.2)
        .unwrap();

    for c in &contradictions {
        graph
            .storage
            .invalidate_contradicted(&c.old_edge_id, &c.new_edge_id)
            .unwrap();
    }

    // Get the invalidated edge
    let all_edges = graph.storage.get_edges_for_entity(&alice.id).unwrap();
    let invalidated = all_edges.iter().find(|e| e.id == edge1_id).unwrap();

    // Check metadata has contradicted_by and contradicted_at
    let metadata = invalidated.metadata.as_ref().unwrap();
    assert!(
        metadata.get("contradicted_by").is_some(),
        "Expected contradicted_by in metadata"
    );
    assert!(
        metadata.get("contradicted_at").is_some(),
        "Expected contradicted_at in metadata"
    );
}

#[test]
fn test_no_contradiction_when_same_fact_inserted_twice() {
    // Insert the same fact twice — no contradiction
    let graph = test_graph();

    let alice = Entity::new("Alice", "Person");
    let postgres = Entity::new("PostgreSQL", "Database");

    graph.add_entity(alice.clone()).unwrap();
    graph.add_entity(postgres.clone()).unwrap();

    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
    edge1.confidence = 0.9;
    edge1.fact = Some("Alice chose PostgreSQL".to_string());
    graph.add_edge(edge1).unwrap();

    // Same fact, same target
    let mut edge2 = Edge::new(&alice.id, &postgres.id, "chose");
    edge2.confidence = 0.9;
    edge2.fact = Some("Alice chose PostgreSQL".to_string());
    let edge2_for_check = edge2.clone();
    graph.add_edge(edge2).unwrap();

    // Check for contradictions — should be none since target_id is the same
    let contradictions = graph
        .storage
        .check_contradictions(&[edge2_for_check], 0.2)
        .unwrap();

    assert!(
        contradictions.is_empty(),
        "Expected no contradiction for same target, got {}",
        contradictions.len()
    );
}

#[test]
fn test_get_current_edges_returns_only_newer_after_contradiction() {
    // Verify old edge is excluded from current edges after contradiction
    let graph = test_graph();

    let alice = Entity::new("Alice", "Person");
    let postgres = Entity::new("PostgreSQL", "Database");
    let mysql = Entity::new("MySQL", "Database");

    graph.add_entity(alice.clone()).unwrap();
    graph.add_entity(postgres.clone()).unwrap();
    graph.add_entity(mysql.clone()).unwrap();

    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
    edge1.confidence = 0.9;
    edge1.fact = Some("Alice chose PostgreSQL".to_string());
    let edge1_id = edge1.id.clone();
    graph.add_edge(edge1).unwrap();

    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
    edge2.confidence = 0.9;
    edge2.fact = Some("Alice chose MySQL".to_string());
    let edge2_for_check = edge2.clone();
    graph.add_edge(edge2).unwrap();

    // Check for contradictions and invalidate
    let contradictions = graph
        .storage
        .check_contradictions(&[edge2_for_check], 0.2)
        .unwrap();

    for c in &contradictions {
        graph
            .storage
            .invalidate_contradicted(&c.old_edge_id, &c.new_edge_id)
            .unwrap();
    }

    // Get current edges for Alice — should only return edge2
    let current_edges = graph
        .storage
        .get_current_edges_for_entity(&alice.id)
        .unwrap();

    // edge1 should be invalidated, edge2 should be current
    assert_eq!(current_edges.len(), 1, "Expected 1 current edge, got {}", current_edges.len());
    assert_eq!(current_edges[0].id, edge2_for_check.id);
    assert!(
        current_edges.iter().all(|e| e.id != edge1_id),
        "Old invalidated edge should not be in current edges"
    );
}

#[test]
fn test_low_confidence_edge_replaced_silently() {
    // confidence=0.1 edge replaced without contradiction
    let graph = test_graph();

    let alice = Entity::new("Alice", "Person");
    let postgres = Entity::new("PostgreSQL", "Database");
    let mysql = Entity::new("MySQL", "Database");

    graph.add_entity(alice.clone()).unwrap();
    graph.add_entity(postgres.clone()).unwrap();
    graph.add_entity(mysql.clone()).unwrap();

    // First edge with low confidence (below threshold)
    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
    edge1.confidence = 0.1; // Below 0.2 threshold
    edge1.fact = Some("Alice chose PostgreSQL".to_string());
    let edge1_id = edge1.id.clone();
    graph.add_edge(edge1).unwrap();

    // Second edge with higher confidence
    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
    edge2.confidence = 0.9;
    edge2.fact = Some("Alice chose MySQL".to_string());
    let edge2_id = edge2.id.clone();
    let edge2_for_check = edge2.clone();
    graph.add_edge(edge2).unwrap();

    // Check for contradictions — edge1 should be silently invalidated
    // not flagged as a contradiction (confidence below threshold)
    let contradictions = graph
        .storage
        .check_contradictions(&[edge2_for_check], 0.2)
        .unwrap();

    // No contradiction recorded since edge1 confidence < threshold
    assert!(
        contradictions.is_empty(),
        "Expected no contradiction for low-confidence edge, got {}",
        contradictions.len()
    );

    // But edge1 should be invalidated internally
    let current_edges = graph
        .storage
        .get_current_edges_for_entity(&alice.id)
        .unwrap();

    // Only edge2 should be current
    assert_eq!(current_edges.len(), 1);
    assert_eq!(current_edges[0].id, edge2_for_check.id);
}

#[test]
fn test_contradiction_check_empty_graph_returns_empty() {
    // Unit test for check_contradictions on empty graph
    let graph = test_graph();

    let alice = Entity::new("Alice", "Person");
    let postgres = Entity::new("PostgreSQL", "Database");

    graph.add_entity(alice.clone()).unwrap();
    graph.add_entity(postgres.clone()).unwrap();

    let edge = Edge::new(&alice.id, &postgres.id, "chose");
    let edge_for_first_check = edge.clone();

    // Check contradictions with single edge in empty graph
    let contradictions = graph
        .storage
        .check_contradictions(&[edge_for_first_check], 0.2)
        .unwrap();

    assert!(
        contradictions.is_empty(),
        "Expected no contradiction in empty graph, got {}",
        contradictions.len()
    );

    // Add edge and check again with same edge
    graph.add_edge(edge.clone()).unwrap();

    let contradictions = graph
        .storage
        .check_contradictions(&[edge], 0.2)
        .unwrap();

    // Should still be empty since they're the same (same target_id)
    assert!(
        contradictions.is_empty(),
        "Expected no contradiction for identical edges, got {}",
        contradictions.len()
    );
}

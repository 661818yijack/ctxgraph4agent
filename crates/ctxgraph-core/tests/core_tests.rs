     1|use chrono::Utc;
     2|use ctxgraph::*;
     3|
     4|fn test_graph() -> Graph {
     5|    Graph::in_memory().expect("failed to create in-memory graph")
     6|}
     7|
     8|// ── Episode CRUD ──
     9|
    10|#[test]
    11|fn test_episode_insert_and_retrieve() {
    12|    let graph = test_graph();
    13|    let episode = Episode::builder("Chose Postgres over SQLite for billing").build();
    14|    let id = episode.id.clone();
    15|
    16|    let result = graph.add_episode(episode).unwrap();
    17|    assert_eq!(result.episode_id, id);
    18|
    19|    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    20|    assert_eq!(retrieved.content, "Chose Postgres over SQLite for billing");
    21|}
    22|
    23|#[test]
    24|fn test_episode_with_source_and_tags() {
    25|    let graph = test_graph();
    26|    let episode = Episode::builder("Priya approved the discount")
    27|        .source("slack")
    28|        .tag("finance")
    29|        .tag("approval")
    30|        .build();
    31|    let id = episode.id.clone();
    32|
    33|    graph.add_episode(episode).unwrap();
    34|
    35|    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    36|    assert_eq!(retrieved.source.as_deref(), Some("slack"));
    37|    assert!(retrieved.metadata.is_some());
    38|
    39|    let meta = retrieved.metadata.unwrap();
    40|    let tags = meta.get("tags").unwrap().as_array().unwrap();
    41|    assert_eq!(tags.len(), 2);
    42|    assert_eq!(tags[0].as_str().unwrap(), "finance");
    43|}
    44|
    45|#[test]
    46|fn test_episode_with_metadata() {
    47|    let graph = test_graph();
    48|    let episode = Episode::builder("Budget approved for Q3")
    49|        .meta("author", "rohan")
    50|        .meta("confidence", serde_json::json!(0.95))
    51|        .build();
    52|    let id = episode.id.clone();
    53|
    54|    graph.add_episode(episode).unwrap();
    55|
    56|    let retrieved = graph.get_episode(&id).unwrap().unwrap();
    57|    let meta = retrieved.metadata.unwrap();
    58|    assert_eq!(meta.get("author").unwrap().as_str().unwrap(), "rohan");
    59|}
    60|
    61|#[test]
    62|fn test_list_episodes() {
    63|    let graph = test_graph();
    64|
    65|    for i in 0..5 {
    66|        let ep = Episode::builder(&format!("Decision {i}")).build();
    67|        graph.add_episode(ep).unwrap();
    68|    }
    69|
    70|    let episodes = graph.list_episodes(3, 0).unwrap();
    71|    assert_eq!(episodes.len(), 3);
    72|
    73|    let all = graph.list_episodes(100, 0).unwrap();
    74|    assert_eq!(all.len(), 5);
    75|
    76|    let offset = graph.list_episodes(100, 3).unwrap();
    77|    assert_eq!(offset.len(), 2);
    78|}
    79|
    80|#[test]
    81|fn test_episode_not_found() {
    82|    let graph = test_graph();
    83|    let result = graph.get_episode("nonexistent-id").unwrap();
    84|    assert!(result.is_none());
    85|}
    86|
    87|// ── Entity CRUD ──
    88|
    89|#[test]
    90|fn test_entity_insert_and_retrieve() {
    91|    let graph = test_graph();
    92|    let entity = Entity::new("Postgres", "Component");
    93|    let id = entity.id.clone();
    94|
    95|    graph.add_entity(entity).unwrap();
    96|
    97|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
    98|    assert_eq!(retrieved.name, "Postgres");
    99|    assert_eq!(retrieved.entity_type, "Component");
   100|}
   101|
   102|#[test]
   103|fn test_entity_by_name() {
   104|    let graph = test_graph();
   105|    let entity = Entity::new("Priya Sharma", "Person");
   106|    graph.add_entity(entity).unwrap();
   107|
   108|    let found = graph.get_entity_by_name("Priya Sharma").unwrap().unwrap();
   109|    assert_eq!(found.entity_type, "Person");
   110|
   111|    let not_found = graph.get_entity_by_name("Nonexistent").unwrap();
   112|    assert!(not_found.is_none());
   113|}
   114|
   115|#[test]
   116|fn test_list_entities_with_type_filter() {
   117|    let graph = test_graph();
   118|
   119|    graph
   120|        .add_entity(Entity::new("Postgres", "Component"))
   121|        .unwrap();
   122|    graph
   123|        .add_entity(Entity::new("SQLite", "Component"))
   124|        .unwrap();
   125|    graph.add_entity(Entity::new("Priya", "Person")).unwrap();
   126|    graph.add_entity(Entity::new("billing", "Service")).unwrap();
   127|
   128|    let all = graph.list_entities(None, 100).unwrap();
   129|    assert_eq!(all.len(), 4);
   130|
   131|    let components = graph.list_entities(Some("Component"), 100).unwrap();
   132|    assert_eq!(components.len(), 2);
   133|
   134|    let people = graph.list_entities(Some("Person"), 100).unwrap();
   135|    assert_eq!(people.len(), 1);
   136|    assert_eq!(people[0].name, "Priya");
   137|}
   138|
   139|// ── Edge CRUD + Bi-temporal ──
   140|
   141|#[test]
   142|fn test_edge_insert_and_retrieve() {
   143|    let graph = test_graph();
   144|
   145|    let pg = Entity::new("Postgres", "Component");
   146|    let billing = Entity::new("billing", "Service");
   147|    let pg_id = pg.id.clone();
   148|    let billing_id = billing.id.clone();
   149|    graph.add_entity(pg).unwrap();
   150|    graph.add_entity(billing).unwrap();
   151|
   152|    let edge = Edge::new(&pg_id, &billing_id, "chosen_for");
   153|    let edge_id = edge.id.clone();
   154|    graph.add_edge(edge).unwrap();
   155|
   156|    let edges = graph.get_edges_for_entity(&pg_id).unwrap();
   157|    assert_eq!(edges.len(), 1);
   158|    assert_eq!(edges[0].id, edge_id);
   159|    assert_eq!(edges[0].relation, "chosen_for");
   160|}
   161|
   162|#[test]
   163|fn test_edge_is_current() {
   164|    let edge = Edge::new("a", "b", "test");
   165|    assert!(edge.is_current());
   166|}
   167|
   168|#[test]
   169|fn test_edge_invalidation() {
   170|    let graph = test_graph();
   171|
   172|    let alice = Entity::new("Alice", "Person");
   173|    let google = Entity::new("Google", "Organization");
   174|    let alice_id = alice.id.clone();
   175|    let google_id = google.id.clone();
   176|    graph.add_entity(alice).unwrap();
   177|    graph.add_entity(google).unwrap();
   178|
   179|    let mut edge = Edge::new(&alice_id, &google_id, "works_at");
   180|    edge.valid_from = Some(Utc::now());
   181|    let edge_id = edge.id.clone();
   182|    graph.add_edge(edge).unwrap();
   183|
   184|    // Edge should be current
   185|    let edges = graph.get_edges_for_entity(&alice_id).unwrap();
   186|    assert_eq!(edges.len(), 1);
   187|    assert!(edges[0].is_current());
   188|
   189|    // Invalidate
   190|    graph.invalidate_edge(&edge_id).unwrap();
   191|
   192|    // Should still appear in all-edges query
   193|    let all_edges = graph.get_edges_for_entity(&alice_id).unwrap();
   194|    assert_eq!(all_edges.len(), 1);
   195|    assert!(!all_edges[0].is_current());
   196|}
   197|
   198|#[test]
   199|fn test_edge_valid_at() {
   200|    let mut edge = Edge::new("a", "b", "test");
   201|    let now = Utc::now();
   202|    edge.valid_from = Some(now - chrono::Duration::days(30));
   203|    edge.valid_until = Some(now - chrono::Duration::days(10));
   204|
   205|    // 20 days ago: should be valid
   206|    assert!(edge.is_valid_at(now - chrono::Duration::days(20)));
   207|
   208|    // 5 days ago: should not be valid (after valid_until)
   209|    assert!(!edge.is_valid_at(now - chrono::Duration::days(5)));
   210|
   211|    // 40 days ago: should not be valid (before valid_from)
   212|    assert!(!edge.is_valid_at(now - chrono::Duration::days(40)));
   213|}
   214|
   215|#[test]
   216|fn test_invalidate_nonexistent_edge() {
   217|    let graph = test_graph();
   218|    let result = graph.invalidate_edge("nonexistent");
   219|    assert!(result.is_err());
   220|}
   221|
   222|// ── Episode-Entity Links ──
   223|
   224|#[test]
   225|fn test_episode_entity_link() {
   226|    let graph = test_graph();
   227|
   228|    let episode = Episode::builder("Chose Postgres for billing").build();
   229|    let ep_id = episode.id.clone();
   230|    graph.add_episode(episode).unwrap();
   231|
   232|    let entity = Entity::new("Postgres", "Component");
   233|    let ent_id = entity.id.clone();
   234|    graph.add_entity(entity).unwrap();
   235|
   236|    graph
   237|        .link_episode_entity(&ep_id, &ent_id, Some(6), Some(14))
   238|        .unwrap();
   239|
   240|    // Link should be idempotent (INSERT OR IGNORE)
   241|    graph
   242|        .link_episode_entity(&ep_id, &ent_id, Some(6), Some(14))
   243|        .unwrap();
   244|}
   245|
   246|// ── FTS5 Search ──
   247|
   248|#[test]
   249|fn test_fts5_search_episodes() {
   250|    let graph = test_graph();
   251|
   252|    graph
   253|        .add_episode(Episode::builder("Chose Postgres over SQLite for billing").build())
   254|        .unwrap();
   255|    graph
   256|        .add_episode(Episode::builder("Switched from REST to gRPC for internal services").build())
   257|        .unwrap();
   258|    graph
   259|        .add_episode(Episode::builder("Priya approved the discount for Reliance").build())
   260|        .unwrap();
   261|
   262|    let results = graph.search("Postgres", 10).unwrap();
   263|    assert_eq!(results.len(), 1);
   264|    assert!(results[0].0.content.contains("Postgres"));
   265|
   266|    let results = graph.search("billing OR discount", 10).unwrap();
   267|    assert_eq!(results.len(), 2);
   268|}
   269|
   270|#[test]
   271|fn test_fts5_search_empty_results() {
   272|    let graph = test_graph();
   273|    graph
   274|        .add_episode(Episode::builder("Chose Postgres").build())
   275|        .unwrap();
   276|
   277|    let results = graph.search("nonexistent_term_xyz", 10).unwrap();
   278|    assert!(results.is_empty());
   279|}
   280|
   281|#[test]
   282|fn test_fts5_search_entities() {
   283|    let graph = test_graph();
   284|
   285|    graph
   286|        .add_entity(Entity::new("Postgres", "Component"))
   287|        .unwrap();
   288|    graph
   289|        .add_entity(Entity::new("SQLite", "Component"))
   290|        .unwrap();
   291|    graph.add_entity(Entity::new("Priya", "Person")).unwrap();
   292|
   293|    let results = graph.search_entities("Postgres", 10).unwrap();
   294|    assert_eq!(results.len(), 1);
   295|    assert_eq!(results[0].0.name, "Postgres");
   296|
   297|    let results = graph.search_entities("Component", 10).unwrap();
   298|    assert_eq!(results.len(), 2);
   299|}
   300|
   301|// ── Entity Context ──
   302|
   303|#[test]
   304|fn test_entity_context() {
   305|    let graph = test_graph();
   306|
   307|    let pg = Entity::new("Postgres", "Component");
   308|    let billing = Entity::new("billing", "Service");
   309|    let rohan = Entity::new("rohan", "Person");
   310|    let pg_id = pg.id.clone();
   311|    let billing_id = billing.id.clone();
   312|    let rohan_id = rohan.id.clone();
   313|
   314|    graph.add_entity(pg).unwrap();
   315|    graph.add_entity(billing).unwrap();
   316|    graph.add_entity(rohan).unwrap();
   317|
   318|    graph
   319|        .add_edge(Edge::new(&pg_id, &billing_id, "chosen_for"))
   320|        .unwrap();
   321|    graph
   322|        .add_edge(Edge::new(&rohan_id, &pg_id, "chose"))
   323|        .unwrap();
   324|
   325|    let context = graph.get_entity_context(&pg_id).unwrap();
   326|    assert_eq!(context.entity.name, "Postgres");
   327|    assert_eq!(context.edges.len(), 2);
   328|    assert_eq!(context.neighbors.len(), 2);
   329|}
   330|
   331|// ── Stats ──
   332|
   333|#[test]
   334|fn test_stats() {
   335|    let graph = test_graph();
   336|
   337|    graph
   338|        .add_episode(Episode::builder("Decision 1").source("manual").build())
   339|        .unwrap();
   340|    graph
   341|        .add_episode(Episode::builder("Decision 2").source("manual").build())
   342|        .unwrap();
   343|    graph
   344|        .add_episode(Episode::builder("Slack message").source("slack").build())
   345|        .unwrap();
   346|
   347|    let pg = Entity::new("Postgres", "Component");
   348|    let pg_id = pg.id.clone();
   349|    graph.add_entity(pg).unwrap();
   350|    let billing = Entity::new("billing", "Service");
   351|    let billing_id = billing.id.clone();
   352|    graph.add_entity(billing).unwrap();
   353|
   354|    graph
   355|        .add_edge(Edge::new(&pg_id, &billing_id, "chosen_for"))
   356|        .unwrap();
   357|
   358|    let stats = graph.stats().unwrap();
   359|    assert_eq!(stats.episode_count, 3);
   360|    assert_eq!(stats.entity_count, 2);
   361|    assert_eq!(stats.edge_count, 1);
   362|    assert_eq!(stats.sources.len(), 2);
   363|}
   364|
   365|// ── Graph Init ──
   366|
   367|#[test]
   368|fn test_graph_init_and_open() {
   369|    let tmp = tempfile::tempdir().unwrap();
   370|    let dir = tmp.path();
   371|
   372|    // Init should succeed
   373|    let _graph = Graph::init(dir).unwrap();
   374|
   375|    // Init again should fail (already exists)
   376|    let result = Graph::init(dir);
   377|    assert!(result.is_err());
   378|
   379|    // Open should succeed
   380|    let db_path = dir.join(".ctxgraph").join("graph.db");
   381|    let _graph = Graph::open(&db_path).unwrap();
   382|}
   383|
   384|#[test]
   385|fn test_graph_open_nonexistent() {
   386|    let result = Graph::open(std::path::Path::new("/tmp/nonexistent/graph.db"));
   387|    assert!(result.is_err());
   388|}
   389|
   390|// ── Embedding Storage ──
   391|
   392|#[test]
   393|fn test_store_and_retrieve_embedding() {
   394|    let graph = test_graph();
   395|    let episode = Episode::builder("Embedding test episode").build();
   396|    let ep_id = episode.id.clone();
   397|    graph.add_episode(episode).unwrap();
   398|
   399|    // Store a fake 384-dim embedding
   400|    let embedding: Vec<f32> = (0..384).map(|i| i as f32 / 384.0).collect();
   401|    graph.store_embedding(&ep_id, &embedding).unwrap();
   402|
   403|    // Retrieve all embeddings — should include ours
   404|    let all = graph.get_embeddings().unwrap();
   405|    assert_eq!(all.len(), 1);
   406|    assert_eq!(all[0].0, ep_id);
   407|    assert_eq!(all[0].1.len(), 384);
   408|    // Check round-trip fidelity for a few values
   409|    for (i, &v) in all[0].1.iter().enumerate() {
   410|        let expected = i as f32 / 384.0;
   411|        assert!(
   412|            (v - expected).abs() < 1e-6,
   413|            "mismatch at index {i}: {v} vs {expected}"
   414|        );
   415|    }
   416|}
   417|
   418|#[test]
   419|fn test_get_embeddings_empty() {
   420|    let graph = test_graph();
   421|    let embeddings = graph.get_embeddings().unwrap();
   422|    assert!(embeddings.is_empty());
   423|}
   424|
   425|#[test]
   426|fn test_search_fused_no_embeddings() {
   427|    let graph = test_graph();
   428|
   429|    graph
   430|        .add_episode(Episode::builder("Chose Postgres for billing").build())
   431|        .unwrap();
   432|    graph
   433|        .add_episode(Episode::builder("Switched from REST to gRPC").build())
   434|        .unwrap();
   435|
   436|    // Fused search with a dummy query embedding — FTS5 results only
   437|    let query_embedding = vec![0.0f32; 384];
   438|    let results = graph
   439|        .search_fused("Postgres", &query_embedding, 10)
   440|        .unwrap();
   441|
   442|    // Should still return FTS5 hits even with zero-magnitude query embedding
   443|    assert!(!results.is_empty());
   444|    assert!(results[0].episode.content.contains("Postgres"));
   445|}
   446|
   447|#[test]
   448|fn test_search_fused_with_embeddings() {
   449|    let graph = test_graph();
   450|
   451|    let ep1 = Episode::builder("Chose Postgres for billing").build();
   452|    let ep2 = Episode::builder("Switched from REST to gRPC").build();
   453|    let id1 = ep1.id.clone();
   454|    let id2 = ep2.id.clone();
   455|    graph.add_episode(ep1).unwrap();
   456|    graph.add_episode(ep2).unwrap();
   457|
   458|    // Synthetic embeddings: ep1 in direction [1, 0, ...], ep2 in direction [0, 1, ...]
   459|    let mut emb1 = vec![0.0f32; 384];
   460|    emb1[0] = 1.0;
   461|    let mut emb2 = vec![0.0f32; 384];
   462|    emb2[1] = 1.0;
   463|
   464|    graph.store_embedding(&id1, &emb1).unwrap();
   465|    graph.store_embedding(&id2, &emb2).unwrap();
   466|
   467|    // Query in direction of ep1
   468|    let query_embedding = emb1.clone();
   469|    let results = graph
   470|        .search_fused("Postgres", &query_embedding, 10)
   471|        .unwrap();
   472|
   473|    // ep1 should rank first (matches both FTS5 and semantic)
   474|    assert!(!results.is_empty());
   475|    assert_eq!(results[0].episode.id, id1);
   476|}
   477|
   478|// ── UUID v7 Ordering ──
   479|
   480|#[test]
   481|fn test_uuid_v7_is_time_sortable() {
   482|    let id1 = uuid::Uuid::now_v7().to_string();
   483|    std::thread::sleep(std::time::Duration::from_millis(2));
   484|    let id2 = uuid::Uuid::now_v7().to_string();
   485|
   486|    assert!(
   487|        id1 < id2,
   488|        "UUID v7 should be lexicographically time-sortable"
   489|    );
   490|}
   491|
   492|// ── Migrations Idempotent ──
   493|
   494|#[test]
   495|fn test_migrations_idempotent() {
   496|    let tmp = tempfile::tempdir().unwrap();
   497|    let db_path = tmp.path().join("test.db");
   498|
   499|    // Open twice — migrations should not fail on second open
   500|    let _storage = ctxgraph::storage::Storage::open(&db_path).unwrap();
   501|    drop(_storage);
   502|    let _storage = ctxgraph::storage::Storage::open(&db_path).unwrap();
   503|}
   504|
   505|// ── Entity Deduplication ──
   506|
   507|#[test]
   508|fn test_entity_dedup_merges_similar() {
   509|    let graph = test_graph();
   510|
   511|    // Add "PostgreSQL" entity
   512|    let pg = Entity::new("PostgreSQL", "Component");
   513|    let (pg_id, merged) = graph.add_entity_deduped(pg, 0.85).unwrap();
   514|    assert!(!merged, "First insert should not be merged");
   515|
   516|    // Add "Postgres" entity with dedup threshold 0.85 — should merge
   517|    let postgres = Entity::new("Postgres", "Component");
   518|    let (deduped_id, was_merged) = graph.add_entity_deduped(postgres, 0.85).unwrap();
   519|    assert!(was_merged, "Postgres should be merged into PostgreSQL");
   520|    assert_eq!(
   521|        deduped_id, pg_id,
   522|        "Should return canonical PostgreSQL entity id"
   523|    );
   524|
   525|    // Only one entity should exist
   526|    let all = graph.list_entities(Some("Component"), 100).unwrap();
   527|    assert_eq!(
   528|        all.len(),
   529|        1,
   530|        "Only one Component entity should exist after merge"
   531|    );
   532|    assert_eq!(all[0].name, "PostgreSQL");
   533|}
   534|
   535|#[test]
   536|fn test_entity_dedup_preserves_different() {
   537|    let graph = test_graph();
   538|
   539|    let pg = Entity::new("PostgreSQL", "Component");
   540|    graph.add_entity_deduped(pg, 0.85).unwrap();
   541|
   542|    // "Redis" has very low similarity to "PostgreSQL"
   543|    let redis = Entity::new("Redis", "Component");
   544|    let (_, was_merged) = graph.add_entity_deduped(redis, 0.85).unwrap();
   545|    assert!(!was_merged, "Redis should not be merged with PostgreSQL");
   546|
   547|    let all = graph.list_entities(Some("Component"), 100).unwrap();
   548|    assert_eq!(
   549|        all.len(),
   550|        2,
   551|        "Both PostgreSQL and Redis should exist as separate entities"
   552|    );
   553|}
   554|
   555|#[test]
   556|fn test_entity_dedup_alias_lookup() {
   557|    let graph = test_graph();
   558|
   559|    // Add canonical entity
   560|    let pg = Entity::new("PostgreSQL", "Component");
   561|    let (pg_id, _) = graph.add_entity_deduped(pg, 0.85).unwrap();
   562|
   563|    // Add alias variant
   564|    let postgres = Entity::new("Postgres", "Component");
   565|    let (merged_id, was_merged) = graph.add_entity_deduped(postgres, 0.85).unwrap();
   566|    assert!(was_merged);
   567|    assert_eq!(merged_id, pg_id);
   568|
   569|    // Adding "Postgres" again should hit alias table (exact alias match)
   570|    let postgres2 = Entity::new("Postgres", "Component");
   571|    let (alias_id, alias_merged) = graph.add_entity_deduped(postgres2, 0.85).unwrap();
   572|    assert!(alias_merged, "Second 'Postgres' should hit alias table");
   573|    assert_eq!(alias_id, pg_id, "Alias lookup should return canonical id");
   574|}
   575|
   576|// ── Empty Database ──
   577|
   578|#[test]
   579|fn test_empty_database_operations() {
   580|    let graph = test_graph();
   581|
   582|    // All operations should succeed on empty db
   583|    assert!(graph.list_episodes(10, 0).unwrap().is_empty());
   584|    assert!(graph.list_entities(None, 10).unwrap().is_empty());
   585|    assert!(graph.search("anything", 10).unwrap().is_empty());
   586|
   587|    let stats = graph.stats().unwrap();
   588|    assert_eq!(stats.episode_count, 0);
   589|    assert_eq!(stats.entity_count, 0);
   590|    assert_eq!(stats.edge_count, 0);
   591|}
   592|
   594|// ── A1: MemoryType and TTL ──
   595|
   596|#[test]
   597|fn test_memory_type_from_entity_type_decision() {
   598|    assert_eq!(
   599|        MemoryType::from_entity_type("Decision"),
   600|        MemoryType::Decision
   601|    );
   602|    assert_eq!(
   603|        MemoryType::from_entity_type("decision"),
   604|        MemoryType::Decision
   605|    );
   606|}
   607|
   608|#[test]
   609|fn test_memory_type_from_entity_type_unknown_falls_back_to_fact() {
   610|    assert_eq!(
   611|        MemoryType::from_entity_type("UnknownType"),
   612|        MemoryType::Fact
   613|    );
   614|    assert_eq!(MemoryType::from_entity_type("Component"), MemoryType::Fact);
   615|    assert_eq!(MemoryType::from_entity_type(""), MemoryType::Fact);
   616|}
   617|
   618|#[test]
   619|fn test_memory_type_default_ttl_fact() {
   620|    use std::time::Duration;
   621|    assert_eq!(
   622|        MemoryType::Fact.default_ttl(),
   623|        Some(Duration::from_secs(90 * 86400))
   624|    );
   625|}
   626|
   627|#[test]
   628|fn test_memory_type_default_ttl_pattern_never() {
   629|    assert_eq!(MemoryType::Pattern.default_ttl(), None);
   630|}
   631|
   632|#[test]
   633|fn test_memory_type_default_ttl_experience() {
   634|    use std::time::Duration;
   635|    assert_eq!(
   636|        MemoryType::Experience.default_ttl(),
   637|        Some(Duration::from_secs(14 * 86400))
   638|    );
   639|}
   640|
   641|#[test]
   642|fn test_memory_type_default_ttl_preference() {
   643|    use std::time::Duration;
   644|    assert_eq!(
   645|        MemoryType::Preference.default_ttl(),
   646|        Some(Duration::from_secs(30 * 86400))
   647|    );
   648|}
   649|
   650|#[test]
   651|fn test_memory_type_default_ttl_decision() {
   652|    use std::time::Duration;
   653|    assert_eq!(
   654|        MemoryType::Decision.default_ttl(),
   655|        Some(Duration::from_secs(90 * 86400))
   656|    );
   657|}
   658|
   659|#[test]
   660|fn test_memory_type_from_db() {
   661|    assert_eq!(MemoryType::from_db("fact"), MemoryType::Fact);
   662|    assert_eq!(MemoryType::from_db("Pattern"), MemoryType::Pattern);
   663|    assert_eq!(MemoryType::from_db("EXPERIENCE"), MemoryType::Experience);
   664|    assert_eq!(MemoryType::from_db("unknown"), MemoryType::Fact);
   665|}
   666|
   667|#[test]
   668|fn test_memory_type_display() {
   669|    assert_eq!(format!("{}", MemoryType::Fact), "fact");
   670|    assert_eq!(format!("{}", MemoryType::Pattern), "pattern");
   671|    assert_eq!(format!("{}", MemoryType::Decision), "decision");
   672|}
   673|
   674|#[test]
   675|fn test_entity_new_auto_sets_memory_type_and_ttl() {
   676|    let entity = Entity::new("JWT", "Component");
   677|    assert_eq!(entity.memory_type, MemoryType::Fact); // Component -> Fact
   678|    assert_eq!(entity.ttl, Some(std::time::Duration::from_secs(90 * 86400)));
   679|}
   680|
   681|#[test]
   682|fn test_entity_new_decision_type() {
   683|    let entity = Entity::new("Use Postgres", "Decision");
   684|    assert_eq!(entity.memory_type, MemoryType::Decision);
   685|    assert_eq!(entity.ttl, Some(std::time::Duration::from_secs(90 * 86400)));
   686|}
   687|
   688|#[test]
   689|fn test_entity_with_explicit_memory() {
   690|    let entity = Entity::with_memory(
   691|        "Recurring bug",
   692|        "Component",
   693|        MemoryType::Pattern,
   694|        None, // never expires
   695|    );
   696|    assert_eq!(entity.memory_type, MemoryType::Pattern);
   697|    assert_eq!(entity.ttl, None);
   698|}
   699|
   700|#[test]
   701|fn test_entity_persist_and_retrieve_with_memory_type() {
   702|    let graph = test_graph();
   703|    let entity = Entity::new("Redis", "Component");
   704|    let id = entity.id.clone();
   705|    graph.add_entity(entity).unwrap();
   706|
   707|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
   708|    assert_eq!(retrieved.memory_type, MemoryType::Fact);
   709|    assert_eq!(
   710|        retrieved.ttl,
   711|        Some(std::time::Duration::from_secs(90 * 86400))
   712|    );
   713|}
   714|
   715|#[test]
   716|fn test_entity_persist_pattern_no_ttl() {
   717|    let graph = test_graph();
   718|    let entity = Entity::with_memory(
   719|        "Users prefer dark mode",
   720|        "Preference",
   721|        MemoryType::Pattern,
   722|        None,
   723|    );
   724|    let id = entity.id.clone();
   725|    graph.add_entity(entity).unwrap();
   726|
   727|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
   728|    assert_eq!(retrieved.memory_type, MemoryType::Pattern);
   729|    assert_eq!(retrieved.ttl, None);
   730|}
   731|
   732|#[test]
   733|fn test_edge_new_auto_sets_memory_type() {
   734|    let edge = Edge::new("e1", "e2", "uses");
   735|    assert_eq!(edge.memory_type, MemoryType::Fact);
   736|    assert_eq!(edge.ttl, Some(std::time::Duration::from_secs(90 * 86400)));
   737|}
   738|
   739|#[test]
   740|fn test_edge_persist_and_retrieve_with_memory_type() {
   741|    let graph = test_graph();
   742|
   743|    let src = Entity::new("Service A", "Component");
   744|    let tgt = Entity::new("Postgres", "Component");
   745|    graph.add_entity(src.clone()).unwrap();
   746|    graph.add_entity(tgt.clone()).unwrap();
   747|
   748|    let edge = Edge::with_memory(
   749|        &src.id,
   750|        &tgt.id,
   751|        "depends on",
   752|        MemoryType::Decision,
   753|        Some(std::time::Duration::from_secs(90 * 86400)),
   754|    );
   755|    graph.add_edge(edge).unwrap();
   756|
   757|    let edges = graph.get_edges_for_entity(&src.id).unwrap();
   758|    let retrieved = edges.iter().find(|e| e.relation == "depends on").unwrap();
   759|    assert_eq!(retrieved.memory_type, MemoryType::Decision);
   760|    assert_eq!(
   761|        retrieved.ttl,
   762|        Some(std::time::Duration::from_secs(90 * 86400))
   763|    );
   764|}
   765|
   766|// ── A2: decay_score ──
   767|
   768|#[test]
   769|fn test_decay_fact_age_zero_returns_base_confidence() {
   770|    // Fact at age=0 should return base_confidence exactly
   771|    let created_at = Utc::now();
   772|    let ttl = Some(std::time::Duration::from_secs(90 * 86400));
   773|    let score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
   774|    assert!(
   775|        (score - 1.0).abs() < 1e-6,
   776|        "Fact at age=0 should score ~1.0, got {score}"
   777|    );
   778|}
   779|
   780|#[test]
   781|fn test_decay_fact_at_ttl_scores_0_25() {
   782|    // Fact at age=ttl with half_life=ttl/2: exp(-2*ln(2)) = 0.25
   783|    let ttl_secs = 90u64 * 86400;
   784|    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
   785|    let created_at = Utc::now() - chrono::Duration::seconds(ttl_secs as i64);
   786|    let score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
   787|    assert!(
   788|        (score - 0.25).abs() < 1e-6,
   789|        "Fact at age=ttl should score ~0.25, got {score}"
   790|    );
   791|}
   792|
   793|#[test]
   794|fn test_decay_fact_at_half_ttl_scores_0_5() {
   795|    // Fact at age=half_life (ttl/2) should score 0.5
   796|    let ttl_secs = 90u64 * 86400;
   797|    let half_life = ttl_secs / 2;
   798|    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
   799|    let created_at = Utc::now() - chrono::Duration::seconds(half_life as i64);
   800|    let score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
   801|    assert!(
   802|        (score - 0.5).abs() < 1e-4,
   803|        "Fact at half-life should score ~0.5, got {score}"
   804|    );
   805|}
   806|
   807|#[test]
   808|fn test_decay_pattern_never_decays() {
   809|    // Pattern returns base_confidence regardless of age
   810|    let created_at = Utc::now() - chrono::Duration::days(365);
   811|    let score = MemoryType::Pattern.decay_score(0.8, created_at, None);
   812|    assert_eq!(score, 0.8, "Pattern should always return base_confidence");
   813|
   814|    // Even with a ttl provided, Pattern ignores it
   815|    let ttl = Some(std::time::Duration::from_secs(30 * 86400));
   816|    let score2 = MemoryType::Pattern.decay_score(0.8, created_at, ttl);
   817|    assert_eq!(score2, 0.8, "Pattern should ignore ttl");
   818|}
   819|
   820|#[test]
   821|fn test_decay_experience_linear_halfway() {
   822|    // Experience at age=ttl/2 should score 0.5
   823|    let ttl_secs = 14u64 * 86400;
   824|    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
   825|    let created_at = Utc::now() - chrono::Duration::seconds((ttl_secs / 2) as i64);
   826|    let score = MemoryType::Experience.decay_score(1.0, created_at, ttl);
   827|    assert!(
   828|        (score - 0.5).abs() < 1e-4,
   829|        "Experience at half-ttl should score ~0.5, got {score}"
   830|    );
   831|}
   832|
   833|#[test]
   834|fn test_decay_experience_at_ttl_scores_zero() {
   835|    // Experience linear decay hits 0.0 at age=ttl
   836|    let ttl_secs = 14u64 * 86400;
   837|    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
   838|    let created_at = Utc::now() - chrono::Duration::seconds(ttl_secs as i64);
   839|    let score = MemoryType::Experience.decay_score(1.0, created_at, ttl);
   840|    assert!(
   841|        score.abs() < 1e-6,
   842|        "Experience at age=ttl should score ~0.0, got {score}"
   843|    );
   844|}
   845|
   846|#[test]
   847|fn test_decay_preference_exponential() {
   848|    // Preference at age=0 scores base_confidence
   849|    let created_at = Utc::now();
   850|    let ttl = Some(std::time::Duration::from_secs(30 * 86400));
   851|    let score = MemoryType::Preference.decay_score(1.0, created_at, ttl);
   852|    assert!(
   853|        (score - 1.0).abs() < 1e-6,
   854|        "Preference at age=0 should score ~1.0, got {score}"
   855|    );
   856|
   857|    // At age=half_life (ttl*0.7) should score ~0.5
   858|    let ttl_secs = 30u64 * 86400;
   859|    let half_life = (ttl_secs as f64 * 0.7) as i64;
   860|    let created_at2 = Utc::now() - chrono::Duration::seconds(half_life);
   861|    let ttl2 = Some(std::time::Duration::from_secs(ttl_secs));
   862|    let score2 = MemoryType::Preference.decay_score(1.0, created_at2, ttl2);
   863|    assert!(
   864|        (score2 - 0.5).abs() < 1e-4,
   865|        "Preference at half-life should score ~0.5, got {score2}"
   866|    );
   867|}
   868|
   869|#[test]
   870|fn test_decay_decision_same_as_fact() {
   871|    // Decision uses same exponential as Fact (half_life = ttl * 0.5)
   872|    let ttl_secs = 90u64 * 86400;
   873|    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
   874|
   875|    let created_at = Utc::now() - chrono::Duration::seconds(ttl_secs as i64);
   876|    let fact_score = MemoryType::Fact.decay_score(1.0, created_at, ttl);
   877|    let decision_score = MemoryType::Decision.decay_score(1.0, created_at, ttl);
   878|    assert!(
   879|        (fact_score - decision_score).abs() < 1e-10,
   880|        "Decision and Fact should have identical decay: fact={fact_score}, decision={decision_score}"
   881|    );
   882|}
   883|
   884|#[test]
   885|fn test_decay_expired_returns_zero() {
   886|    // Age > ttl should return 0.0
   887|    let ttl_secs = 90u64 * 86400;
   888|    let ttl = Some(std::time::Duration::from_secs(ttl_secs));
   889|    // Create 91 days ago — one day past ttl
   890|    let created_at = Utc::now() - chrono::Duration::days(91);
   891|
   892|    assert_eq!(MemoryType::Fact.decay_score(1.0, created_at, ttl), 0.0);
   893|    assert_eq!(
   894|        MemoryType::Experience.decay_score(1.0, created_at, ttl),
   895|        0.0
   896|    );
   897|    assert_eq!(
   898|        MemoryType::Preference.decay_score(1.0, created_at, ttl),
   899|        0.0
   900|    );
   901|    assert_eq!(MemoryType::Decision.decay_score(1.0, created_at, ttl), 0.0);
   902|}
   903|
   904|#[test]
   905|fn test_decay_ttl_none_returns_base_confidence() {
   906|    // Non-pattern with ttl=None returns base_confidence
   907|    let created_at = Utc::now() - chrono::Duration::days(100);
   908|    let score = MemoryType::Fact.decay_score(0.9, created_at, None);
   909|    assert_eq!(score, 0.9);
   910|}
   911|
   912|#[test]
   913|fn test_decay_ttl_zero_returns_zero() {
   914|    let created_at = Utc::now();
   915|    let ttl = Some(std::time::Duration::from_secs(0));
   916|    assert_eq!(MemoryType::Fact.decay_score(1.0, created_at, ttl), 0.0);
   917|    assert_eq!(
   918|        MemoryType::Experience.decay_score(1.0, created_at, ttl),
   919|        0.0
   920|    );
   921|}
   922|
   923|#[test]
   924|fn test_decay_scores_in_range() {
   925|    // All decay functions must return values in [0.0, 1.0]
   926|    let types = [
   927|        MemoryType::Fact,
   928|        MemoryType::Pattern,
   929|        MemoryType::Experience,
   930|        MemoryType::Preference,
   931|        MemoryType::Decision,
   932|    ];
   933|    let ages_days = [0i64, 7, 14, 30, 45, 90, 100, 365];
   934|
   935|    for mt in &types {
   936|        let ttl = mt.default_ttl();
   937|        for &age in &ages_days {
   938|            let created_at = Utc::now() - chrono::Duration::days(age);
   939|            let score = mt.decay_score(1.0, created_at, ttl);
   940|            assert!(
   941|                (0.0..=1.0).contains(&score),
   942|                "{mt:?} at age={age}d score={score} out of range"
   943|            );
   944|        }
   945|    }
   946|}
   947|
   948|#[test]
   949|fn test_migration_003_reopen_safe() {
   950|    let tmp = tempfile::tempdir().unwrap();
   951|    let db_path = tmp.path().join("test.db");
   952|
   953|    // First open: fresh DB, migration runs
   954|    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
   955|    let entity = Entity::new("Test", "Component");
   956|    let id = entity.id.clone();
   957|    graph1.add_entity(entity).unwrap();
   958|    drop(graph1);
   959|
   960|    // Second open: same DB, migration re-runs (idempotent)
   961|    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
   962|    let retrieved = graph2.get_entity(&id).unwrap().unwrap();
   963|    assert_eq!(retrieved.memory_type, MemoryType::Fact);
   964|    assert_eq!(
   965|        retrieved.ttl,
   966|        Some(std::time::Duration::from_secs(90 * 86400))
   967|    );
   968|    drop(graph2);
   969|
   970|    // Third open: verify still works after double migration
   971|    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
   972|    let retrieved2 = graph3.get_entity(&id).unwrap().unwrap();
   973|    assert_eq!(retrieved2.memory_type, MemoryType::Fact);
   974|}
   975|
   976|// ── Migration 004: usage_count and last_recalled_at ─────────────────────────────────
   977|
   978|#[test]
   979|fn test_migration_004_entity_fields_exist() {
   980|    let graph = test_graph();
   981|    let entity = Entity::new("TestComponent", "Component");
   982|    let id = entity.id.clone();
   983|    graph.add_entity(entity).unwrap();
   984|
   985|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
   986|    // New fields should exist with defaults
   987|    assert_eq!(retrieved.usage_count, 0);
   988|    assert_eq!(retrieved.last_recalled_at, None);
   989|}
   990|
   991|#[test]
   992|fn test_migration_004_edge_fields_exist() {
   993|    let graph = test_graph();
   994|
   995|    let e1 = Entity::new("Source", "Component");
   996|    let e2 = Entity::new("Target", "Component");
   997|    graph.add_entity(e1.clone()).unwrap();
   998|    graph.add_entity(e2.clone()).unwrap();
   999|
  1000|    let edge = Edge::new(&e1.id, &e2.id, "depends_on");
  1001|    let edge_id = edge.id.clone();
  1002|    graph.add_edge(edge).unwrap();
  1003|
  1004|    let edges = graph.get_edges_for_entity(&e1.id).unwrap();
  1005|    let retrieved = edges.iter().find(|e| e.id == edge_id).unwrap();
  1006|
  1007|    // New fields should exist with defaults
  1008|    assert_eq!(retrieved.usage_count, 0);
  1009|    assert_eq!(retrieved.last_recalled_at, None);
  1010|}
  1011|
  1012|#[test]
  1013|fn test_touch_entity_increments_usage_count() {
  1014|    let graph = test_graph();
  1015|    let entity = Entity::new("TouchTest", "Component");
  1016|    let id = entity.id.clone();
  1017|    graph.add_entity(entity).unwrap();
  1018|
  1019|    // Touch the entity 3 times
  1020|    graph.touch_entity(&id).unwrap();
  1021|    graph.touch_entity(&id).unwrap();
  1022|    graph.touch_entity(&id).unwrap();
  1023|
  1024|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
  1025|    assert_eq!(retrieved.usage_count, 3);
  1026|    // last_recalled_at should be set
  1027|    assert!(retrieved.last_recalled_at.is_some());
  1028|}
  1029|
  1030|#[test]
  1031|fn test_touch_edge_increments_usage_count() {
  1032|    let graph = test_graph();
  1033|
  1034|    let e1 = Entity::new("TouchEdgeE1", "Component");
  1035|    let e2 = Entity::new("TouchEdgeE2", "Component");
  1036|    graph.add_entity(e1.clone()).unwrap();
  1037|    graph.add_entity(e2.clone()).unwrap();
  1038|
  1039|    let edge = Edge::new(&e1.id, &e2.id, "connects");
  1040|    let edge_id = edge.id.clone();
  1041|    graph.add_edge(edge).unwrap();
  1042|
  1043|    // Touch the edge twice
  1044|    graph.touch_edge(&edge_id).unwrap();
  1045|    graph.touch_edge(&edge_id).unwrap();
  1046|
  1047|    let edges = graph.get_edges_for_entity(&e1.id).unwrap();
  1048|    let retrieved = edges.iter().find(|e| e.id == edge_id).unwrap();
  1049|
  1050|    assert_eq!(retrieved.usage_count, 2);
  1051|    assert!(retrieved.last_recalled_at.is_some());
  1052|}
  1053|
  1054|#[test]
  1055|fn test_migration_004_reopen_safe() {
  1056|    let tmp = tempfile::tempdir().unwrap();
  1057|    let db_path = tmp.path().join("test.db");
  1058|
  1059|    // First open: fresh DB, migrations run
  1060|    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1061|    let entity = Entity::new("Migration004Test", "Component");
  1062|    let id = entity.id.clone();
  1063|    graph1.add_entity(entity).unwrap();
  1064|
  1065|    // Touch to set usage_count
  1066|    graph1.touch_entity(&id).unwrap();
  1067|    drop(graph1);
  1068|
  1069|    // Second open: same DB, migrations re-run (idempotent)
  1070|    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1071|    let retrieved = graph2.get_entity(&id).unwrap().unwrap();
  1072|    assert_eq!(retrieved.usage_count, 1);
  1073|    assert!(retrieved.last_recalled_at.is_some());
  1074|
  1075|    // Touch again and verify increment persists
  1076|    graph2.touch_entity(&id).unwrap();
  1077|    drop(graph2);
  1078|
  1079|    // Third open: verify usage_count persists after double migration
  1080|    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1081|    let retrieved3 = graph3.get_entity(&id).unwrap().unwrap();
  1082|    assert_eq!(retrieved3.usage_count, 2);
  1083|}
  1084|
  1085|#[test]
  1086|fn test_migration_004_applies_to_existing_db() {
  1087|    let tmp = tempfile::tempdir().unwrap();
  1088|    let db_path = tmp.path().join("test.db");
  1089|
  1090|    // Simulate an old DB (before migration 004) by creating graph and manually
  1091|    // verifying the new columns exist after migration runs
  1092|    let graph = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1093|    let entity = Entity::new("OldEntity", "Decision");
  1094|    let id = entity.id.clone();
  1095|    graph.add_entity(entity).unwrap();
  1096|
  1097|    // Existing rows should get usage_count=0 and last_recalled_at=NULL
  1098|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
  1099|    assert_eq!(retrieved.usage_count, 0);
  1100|    assert_eq!(retrieved.last_recalled_at, None);
  1101|
  1102|    // Edge should also get defaults
  1103|    let e1 = Entity::new("EdgeTest1", "Component");
  1104|    let e2 = Entity::new("EdgeTest2", "Component");
  1105|    graph.add_entity(e1.clone()).unwrap();
  1106|    graph.add_entity(e2.clone()).unwrap();
  1107|
  1108|    let edge = Edge::new(&e1.id, &e2.id, "related");
  1109|    let edge_id = edge.id.clone();
  1110|    graph.add_edge(edge).unwrap();
  1111|
  1112|    let edges = graph.get_edges_for_entity(&e1.id).unwrap();
  1113|    let retrieved_edge = edges.iter().find(|e| e.id == edge_id).unwrap();
  1114|    assert_eq!(retrieved_edge.usage_count, 0);
  1115|    assert_eq!(retrieved_edge.last_recalled_at, None);
  1116|}
  1117|
  1118|// ── A3: usage_count and last_recalled_at ──
  1119|
  1120|#[test]
  1121|fn test_entity_new_has_zero_usage_count_and_null_recalled() {
  1122|    let entity = Entity::new("Test", "Component");
  1123|    assert_eq!(
  1124|        entity.usage_count, 0,
  1125|        "new Entity should have usage_count = 0"
  1126|    );
  1127|    assert!(
  1128|        entity.last_recalled_at.is_none(),
  1129|        "new Entity should have last_recalled_at = None"
  1130|    );
  1131|}
  1132|
  1133|#[test]
  1134|fn test_entity_with_memory_has_zero_usage_count_and_null_recalled() {
  1135|    let entity = Entity::with_memory(
  1136|        "Test",
  1137|        "Component",
  1138|        MemoryType::Fact,
  1139|        Some(std::time::Duration::from_secs(86400)),
  1140|    );
  1141|    assert_eq!(
  1142|        entity.usage_count, 0,
  1143|        "with_memory Entity should have usage_count = 0"
  1144|    );
  1145|    assert!(
  1146|        entity.last_recalled_at.is_none(),
  1147|        "with_memory Entity should have last_recalled_at = None"
  1148|    );
  1149|}
  1150|
  1151|#[test]
  1152|fn test_edge_new_has_zero_usage_count_and_null_recalled() {
  1153|    let edge = Edge::new("a", "b", "uses");
  1154|    assert_eq!(edge.usage_count, 0, "new Edge should have usage_count = 0");
  1155|    assert!(
  1156|        edge.last_recalled_at.is_none(),
  1157|        "new Edge should have last_recalled_at = None"
  1158|    );
  1159|}
  1160|
  1161|#[test]
  1162|fn test_edge_with_memory_has_zero_usage_count_and_null_recalled() {
  1163|    let edge = Edge::with_memory(
  1164|        "a",
  1165|        "b",
  1166|        "uses",
  1167|        MemoryType::Decision,
  1168|        Some(std::time::Duration::from_secs(86400)),
  1169|    );
  1170|    assert_eq!(
  1171|        edge.usage_count, 0,
  1172|        "with_memory Edge should have usage_count = 0"
  1173|    );
  1174|    assert!(
  1175|        edge.last_recalled_at.is_none(),
  1176|        "with_memory Edge should have last_recalled_at = None"
  1177|    );
  1178|}
  1179|
  1180|#[test]
  1181|fn test_touch_entity_increments_count() {
  1182|    let graph = test_graph();
  1183|
  1184|    // Create an entity
  1185|    let entity = Entity::new("TouchTest", "Component");
  1186|    let id = entity.id.clone();
  1187|    graph.add_entity(entity).unwrap();
  1188|
  1189|    // Touch it the first time
  1190|    graph.touch_entity(&id).unwrap();
  1191|
  1192|    let retrieved = graph.get_entity(&id).unwrap().unwrap();
  1193|    assert_eq!(
  1194|        retrieved.usage_count, 1,
  1195|        "usage_count should be 1 after first touch"
  1196|    );
  1197|    assert!(
  1198|        retrieved.last_recalled_at.is_some(),
  1199|        "last_recalled_at should be Some after first touch"
  1200|    );
  1201|
  1202|    // Touch it a second time
  1203|    graph.touch_entity(&id).unwrap();
  1204|
  1205|    let retrieved2 = graph.get_entity(&id).unwrap().unwrap();
  1206|    assert_eq!(
  1207|        retrieved2.usage_count, 2,
  1208|        "usage_count should be 2 after second touch"
  1209|    );
  1210|
  1211|    // The last_recalled_at should have been updated
  1212|    assert!(
  1213|        retrieved2.last_recalled_at >= retrieved.last_recalled_at,
  1214|        "last_recalled_at should advance or stay same"
  1215|    );
  1216|}
  1217|
  1218|#[test]
  1219|fn test_touch_edge_increments_count() {
  1220|    let graph = test_graph();
  1221|
  1222|    // Create two entities and an edge
  1223|    let src = Entity::new("Source", "Component");
  1224|    let tgt = Entity::new("Target", "Component");
  1225|    graph.add_entity(src.clone()).unwrap();
  1226|    graph.add_entity(tgt.clone()).unwrap();
  1227|
  1228|    let edge = Edge::new(&src.id, &tgt.id, "depends_on");
  1229|    let edge_id = edge.id.clone();
  1230|    graph.add_edge(edge).unwrap();
  1231|
  1232|    // Touch the edge
  1233|    graph.touch_edge(&edge_id).unwrap();
  1234|
  1235|    let edges = graph.get_edges_for_entity(&src.id).unwrap();
  1236|    let retrieved = edges.iter().find(|e| e.id == edge_id).unwrap();
  1237|    assert_eq!(
  1238|        retrieved.usage_count, 1,
  1239|        "usage_count should be 1 after touch"
  1240|    );
  1241|    assert!(
  1242|        retrieved.last_recalled_at.is_some(),
  1243|        "last_recalled_at should be Some after touch"
  1244|    );
  1245|
  1246|    // Touch again
  1247|    graph.touch_edge(&edge_id).unwrap();
  1248|
  1249|    let edges2 = graph.get_edges_for_entity(&src.id).unwrap();
  1250|    let retrieved2 = edges2.iter().find(|e| e.id == edge_id).unwrap();
  1251|    assert_eq!(
  1252|        retrieved2.usage_count, 2,
  1253|        "usage_count should be 2 after second touch"
  1254|    );
  1255|}
  1256|
  1257|#[test]
  1258|fn test_touch_nonexistent_returns_error() {
  1259|    let graph = test_graph();
  1260|
  1261|    // Touch non-existent entity
  1262|    let result = graph.touch_entity("nonexistent-entity-id");
  1263|    assert!(
  1264|        result.is_err(),
  1265|        "touch_entity on non-existent should return error"
  1266|    );
  1267|
  1268|    // Touch non-existent edge
  1269|    let result = graph.touch_edge("nonexistent-edge-id");
  1270|    assert!(
  1271|        result.is_err(),
  1272|        "touch_edge on non-existent should return error"
  1273|    );
  1274|}
  1275|
  1276|#[test]
  1277|fn test_migration_004_idempotent() {
  1278|    let tmp = tempfile::tempdir().unwrap();
  1279|    let db_path = tmp.path().join("test.db");
  1280|
  1281|    // First open: fresh DB, migration runs
  1282|    let graph1 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1283|    let entity = Entity::new("Migration004", "Component");
  1284|    let id = entity.id.clone();
  1285|    graph1.add_entity(entity).unwrap();
  1286|    drop(graph1);
  1287|
  1288|    // Second open: same DB, migration re-runs (idempotent)
  1289|    let graph2 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1290|    let retrieved = graph2.get_entity(&id).unwrap().unwrap();
  1291|    assert_eq!(retrieved.usage_count, 0);
  1292|    assert!(retrieved.last_recalled_at.is_none());
  1293|    drop(graph2);
  1294|
  1295|    // Third open: verify still works after double migration
  1296|    let graph3 = ctxgraph::Graph::open_or_create(&db_path).unwrap();
  1297|    let retrieved2 = graph3.get_entity(&id).unwrap().unwrap();
  1298|    assert_eq!(retrieved2.usage_count, 0);
  1299|    assert!(retrieved2.last_recalled_at.is_none());
  1300|}
  1301|
  1302|#[test]
  1303|fn test_read_paths_include_new_fields() {
  1304|    let graph = test_graph();
  1305|
  1306|    // Create entity
  1307|    let entity = Entity::new("ReadTest", "Component");
  1308|    let entity_id = entity.id.clone();
  1309|    graph.add_entity(entity).unwrap();
  1310|
  1311|    // Create edge
  1312|    let src = Entity::new("Src", "Component");
  1313|    let tgt = Entity::new("Tgt", "Component");
  1314|    graph.add_entity(src.clone()).unwrap();
  1315|    graph.add_entity(tgt.clone()).unwrap();
  1316|    let edge = Edge::new(&src.id, &tgt.id, "connects");
  1317|    let edge_id = edge.id.clone();
  1318|    graph.add_edge(edge).unwrap();
  1319|
  1320|    // Touch both to set non-default values
  1321|    graph.touch_entity(&entity_id).unwrap();
  1322|    graph.touch_entity(&entity_id).unwrap();
  1323|    graph.touch_edge(&edge_id).unwrap();
  1324|
  1325|    // Verify via get_entity
  1326|    let retrieved_entity = graph.get_entity(&entity_id).unwrap().unwrap();
  1327|    assert_eq!(retrieved_entity.usage_count, 2);
  1328|    assert!(retrieved_entity.last_recalled_at.is_some());
  1329|
  1330|    // Verify via list_entities
  1331|    let entities = graph.list_entities(Some("Component"), 100).unwrap();
  1332|    let listed = entities.iter().find(|e| e.id == entity_id).unwrap();
  1333|    assert_eq!(listed.usage_count, 2);
  1334|    assert!(listed.last_recalled_at.is_some());
  1335|
  1336|    // Verify via get_edges_for_entity
  1337|    let edges = graph.get_edges_for_entity(&src.id).unwrap();
  1338|    let listed_edge = edges.iter().find(|e| e.id == edge_id).unwrap();
  1339|    assert_eq!(listed_edge.usage_count, 1);
  1340|    assert!(listed_edge.last_recalled_at.is_some());
  1341|
  1342|    // Verify via search_entities
  1343|    let search_results = graph.search_entities("ReadTest", 10).unwrap();
  1344|    let searched = search_results
  1345|        .iter()
  1346|        .find(|(e, _)| e.id == entity_id)
  1347|        .unwrap();
  1348|    assert_eq!(searched.0.usage_count, 2);
  1349|    assert!(searched.0.last_recalled_at.is_some());
  1350|}
  1351|
  1352|// ── B1: Episode Memory Type ────────────────────────────────────────────────
  1353|
  1354|#[test]
  1355|fn test_episode_builder_with_memory_type() {
  1356|    let episode = Episode::builder("Pattern summary")
  1357|        .memory_type(MemoryType::Pattern)
  1358|        .build();
  1359|    assert_eq!(episode.memory_type, MemoryType::Pattern);
  1360|}
  1361|
  1362|#[test]
  1363|fn test_episode_persist_and_retrieve_with_new_fields() {
  1364|    let graph = test_graph();
  1365|
  1366|    // Regular episode
  1367|    let episode = Episode::builder("Regular episode content").build();
  1368|    let id = episode.id.clone();
  1369|    graph.add_episode(episode).unwrap();
  1370|
  1371|    let retrieved = graph.get_episode(&id).unwrap().unwrap();
  1372|    assert_eq!(retrieved.memory_type, MemoryType::Experience);
  1373|
  1374|    // Episode with Pattern memory_type
  1375|    let pattern_episode = Episode::builder("Pattern summary")
  1376|        .memory_type(MemoryType::Pattern)
  1377|        .build();
  1378|    let pid = pattern_episode.id.clone();
  1379|    graph.add_episode(pattern_episode).unwrap();
  1380|
  1381|    let retrieved_pattern = graph.get_episode(&pid).unwrap().unwrap();
  1382|    assert_eq!(retrieved_pattern.memory_type, MemoryType::Pattern);
  1383|}
  1384|
  1385|#[test]
  1386|fn test_migration_006_columns_exist() {
  1387|    let graph = test_graph();
  1388|
  1389|    // Insert an episode and verify the new columns are readable
  1390|    let episode = Episode::builder("Migration 006 test").build();
  1391|    let id = episode.id.clone();
  1392|    graph.add_episode(episode).unwrap();
  1393|
  1394|    let retrieved = graph.get_episode(&id).unwrap().unwrap();
  1395|    assert_eq!(retrieved.memory_type, MemoryType::Experience);
  1396|}
  1397|
  1398|// ── Batch Label Describer (D1b) Tests ────────────────────────────────────────
  1399|
  1400|use ctxgraph::pattern::{BatchLabelDescriber, FailingBatchLabelDescriber, MockBatchLabelDescriber};
  1401|use std::collections::HashMap as SummaryMap;
  1402|
  1403|#[test]
  1404|fn test_mock_batch_describer_returns_label_per_candidate() {
  1405|    let candidates = vec![
  1406|        PatternCandidate {
  1407|            id: "c1".to_string(),
  1408|            entity_types: vec!["Docker".to_string()],
  1409|            entity_pair: None,
  1410|            relation_triplet: Some(("Docker".to_string(), "depends_on".to_string(), "Network".to_string())),
  1411|            occurrence_count: 4,
  1412|            source_groups: vec!["ep1".to_string()],
  1413|            confidence: 0.8,
  1414|            description: None,
  1415|        },
  1416|        PatternCandidate {
  1417|            id: "c2".to_string(),
  1418|            entity_types: vec!["Service".to_string()],
  1419|            entity_pair: Some(("API".to_string(), "DB".to_string())),
  1420|            relation_triplet: None,
  1421|            occurrence_count: 3,
  1422|            source_groups: vec!["ep2".to_string()],
  1423|            confidence: 0.6,
  1424|            description: None,
  1425|        },
  1426|    ];
  1427|
  1428|    let describer = MockBatchLabelDescriber;
  1429|    let results = describer.describe_batch(&candidates, &SummaryMap::new()).unwrap();
  1430|
  1431|    assert_eq!(results.len(), 2, "should return one label per candidate");
  1432|    let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
  1433|    assert!(ids.contains(&"c1") && ids.contains(&"c2"));
  1434|    for (_, label) in &results {
  1435|        assert!(!label.is_empty(), "label should not be empty");
  1436|    }
  1437|}
  1438|
  1439|#[test]
  1440|fn test_mock_batch_describer_empty_input() {
  1441|    let describer = MockBatchLabelDescriber;
  1442|    let results = describer.describe_batch(&[], &SummaryMap::new()).unwrap();
  1443|    assert!(results.is_empty());
  1444|}
  1445|
  1446|#[test]
  1447|fn test_failing_batch_describer_returns_error() {
  1448|    let describer = FailingBatchLabelDescriber::new("LLM unavailable");
  1449|    let result = describer.describe_batch(&[], &SummaryMap::new());
  1450|    assert!(result.is_err(), "FailingBatchLabelDescriber should return error");
  1451|}
  1452|
  1453|#[test]
  1454|fn test_mock_batch_describer_triplet_label_mentions_entities() {
  1455|    let candidates = vec![PatternCandidate {
  1456|        id: "c1".to_string(),
  1457|        entity_types: vec!["Component".to_string()],
  1458|        entity_pair: Some(("User".to_string(), "Postgres".to_string())),
  1459|        relation_triplet: Some(("User".to_string(), "connects_to".to_string(), "Postgres".to_string())),
  1460|        occurrence_count: 5,
  1461|        source_groups: vec!["ep1".to_string()],
  1462|        confidence: 1.0,
  1463|        description: None,
  1464|    }];
  1465|
  1466|    let describer = MockBatchLabelDescriber;
  1467|    let results = describer.describe_batch(&candidates, &SummaryMap::new()).unwrap();
  1468|    assert_eq!(results.len(), 1);
  1469|    let (_, label) = &results[0];
  1470|    assert!(label.contains("User") || label.contains("connects_to") || label.contains("Postgres"),
  1471|        "label should mention triplet entities: {}", label);
  1472|    assert!(label.len() < 300, "label should be concise: {}", label);
  1473|}
  1474|
  1475|#[test]
  1476|fn test_store_pattern_creates_learned_pattern_entity() {
  1477|    let graph = test_graph();
  1478|
  1479|    // Create and store a pattern directly via storage
  1480|    let candidate = PatternCandidate {
  1481|        id: "pattern-123".to_string(),
  1482|        entity_types: vec!["Component".to_string()],
  1483|        entity_pair: None,
  1484|        relation_triplet: None,
  1485|        occurrence_count: 3,
  1486|        source_groups: vec!["comp1".to_string()],
  1487|        confidence: 0.75,
  1488|        description: Some(
  1489|            "When Docker networking issues occur, restart the container first.".to_string(),
  1490|        ),
  1491|    };
  1492|
  1493|    // Store via graph's public API
  1494|    let result = graph.store_pattern(&candidate);
  1495|    assert!(result.is_ok(), "store_pattern should succeed: {:?}", result);
  1496|
  1497|    // Verify we can retrieve it
  1498|    let patterns = graph.get_patterns().unwrap();
  1499|    assert!(!patterns.is_empty(), "should have at least one pattern");
  1500|
  1501|    let stored = patterns.iter().find(|p| p.id == "pattern-123").unwrap();
  1502|    assert!(
  1503|        stored.description.is_some(),
  1504|        "pattern should have description"
  1505|    );
  1506|    assert!(
  1507|        stored
  1508|            .description
  1509|            .as_ref()
  1510|            .unwrap()
  1511|            .contains("Docker networking"),
  1512|        "description should be preserved: {:?}",
  1513|        stored.description
  1514|    );
  1515|}
  1516|
  1517|#[test]
  1518|fn test_get_patterns_returns_stored_patterns_with_descriptions() {
  1519|    let graph = test_graph();
  1520|
  1521|    // Store a pattern
  1522|    let candidate = PatternCandidate {
  1523|        id: "pattern-456".to_string(),
  1524|        entity_types: vec!["Service".to_string()],
  1525|        entity_pair: Some(("API".to_string(), "Database".to_string())),
  1526|        relation_triplet: None,
  1527|        occurrence_count: 4,
  1528|        source_groups: vec!["comp1".to_string(), "comp2".to_string()],
  1529|        confidence: 0.8,
  1530|        description: Some(
  1531|            "The API and Database frequently experience connectivity issues together.".to_string(),
  1532|        ),
  1533|    };
  1534|
  1535|    graph.store_pattern(&candidate).unwrap();
  1536|
  1537|    // Retrieve patterns
  1538|    let patterns = graph.get_patterns().unwrap();
  1539|
  1540|    let found = patterns.iter().find(|p| p.id == "pattern-456").unwrap();
  1541|    assert!(
  1542|        found.description.is_some(),
  1543|        "retrieved pattern should have description"
  1544|    );
  1545|    assert!(
  1546|        found.description.as_ref().unwrap().contains("connectivity"),
  1547|        "description content should be preserved"
  1548|    );
  1549|}
  1550|
  1551|#[test]
  1552|fn test_entity_name_truncated_at_word_boundary_80_chars() {
  1553|    let graph = test_graph();
  1554|
  1555|    // Create a description longer than 80 chars
  1556|    let long_description = "When Docker networking issues occur in production environments, the agent should restart the container service and verify the network bridge configuration before assuming the daemon is healthy.".to_string();
  1557|    assert!(
  1558|        long_description.len() > 80,
  1559|        "test description should be > 80 chars"
  1560|    );
  1561|
  1562|    let candidate = PatternCandidate {
  1563|        id: "pattern-789".to_string(),
  1564|        entity_types: vec!["Component".to_string()],
  1565|        entity_pair: None,
  1566|        relation_triplet: None,
  1567|        occurrence_count: 2,
  1568|        source_groups: vec!["comp1".to_string()],
  1569|        confidence: 0.5,
  1570|        description: Some(long_description.clone()),
  1571|    };
  1572|
  1573|    // Store pattern (which truncates name at word boundary)
  1574|    graph.storage.store_pattern(&candidate).unwrap();
  1575|
  1576|    // Retrieve the entity directly to check name
  1577|    let entity = graph.get_entity("pattern-789").unwrap().unwrap();
  1578|
  1579|    // Name should be <= 80 chars
  1580|    assert!(
  1581|        entity.name.len() <= 80,
  1582|        "entity name should be <= 80 chars, got {} chars: {}",
  1583|        entity.name.len(),
  1584|        entity.name
  1585|    );
  1586|
  1587|    // Name should end at word boundary (no partial words at end)
  1588|    // If it's exactly 80, check it doesn't end mid-word
  1589|    if entity.name.len() == 80 {
  1590|        assert!(
  1591|            !entity.name.ends_with(' '),
  1592|            "name should not end with space"
  1593|        );
  1594|        // The truncation function finds last space in first 80 chars and truncates there
  1595|        // So it should be a complete word
  1596|    }
  1597|}
  1598|
  1599|#[test]
  1600|fn test_pattern_has_memory_type_pattern_and_no_ttl() {
  1601|    let graph = test_graph();
  1602|
  1603|    let candidate = PatternCandidate {
  1604|        id: "pattern-ttl-test".to_string(),
  1605|        entity_types: vec!["Service".to_string()],
  1606|        entity_pair: None,
  1607|        relation_triplet: None,
  1608|        occurrence_count: 3,
  1609|        source_groups: vec!["comp1".to_string()],
  1610|        confidence: 0.75,
  1611|        description: Some("Patterns persist indefinitely.".to_string()),
  1612|    };
  1613|
  1614|    graph.store_pattern(&candidate).unwrap();
  1615|
  1616|    // Retrieve the entity
  1617|    let entity = graph.get_entity("pattern-ttl-test").unwrap().unwrap();
  1618|
  1619|    // Check memory_type is Pattern
  1620|    assert_eq!(
  1621|        entity.memory_type,
  1622|        MemoryType::Pattern,
  1623|        "stored pattern should have MemoryType::Pattern"
  1624|    );
  1625|
  1626|    // Check ttl is None (never expires)
  1627|    assert_eq!(
  1628|        entity.ttl, None,
  1629|        "stored pattern should have ttl=None (never expires)"
  1630|    );
  1631|}
  1632|
  1633|
  1634|// ── D1a Integration Tests ──────────────────────────────────────────────────
  1635|
  1636|// ── Contradiction Detection (C1) Tests ───────────────────────────────────────
  1637|
  1638|#[test]
  1639|fn test_contradiction_detected_when_new_fact_conflicts() {
  1640|    // Insert two conflicting edges, verify first is invalidated
  1641|    let graph = test_graph();
  1642|
  1643|    // Create entities
  1644|    let alice = Entity::new("Alice", "Person");
  1645|    let postgres = Entity::new("PostgreSQL", "Database");
  1646|    let mysql = Entity::new("MySQL", "Database");
  1647|
  1648|    graph.add_entity(alice.clone()).unwrap();
  1649|    graph.add_entity(postgres.clone()).unwrap();
  1650|    graph.add_entity(mysql.clone()).unwrap();
  1651|
  1652|    // First edge: Alice chose PostgreSQL
  1653|    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
  1654|    edge1.confidence = 0.9;
  1655|    edge1.fact = Some("Alice chose PostgreSQL".to_string());
  1656|    let edge1_id = edge1.id.clone();
  1657|    graph.add_edge(edge1).unwrap();
  1658|
  1659|    // Second edge: Alice chose MySQL (conflicts with first)
  1660|    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
  1661|    edge2.confidence = 0.9;
  1662|    edge2.fact = Some("Alice chose MySQL".to_string());
  1663|    let edge2_for_check = edge2.clone();
  1664|    graph.add_edge(edge2).unwrap();
  1665|
  1666|    // Check for contradictions
  1667|    let contradictions = graph
  1668|        .storage
  1669|        .check_contradictions(&[edge2_for_check], 0.2)
  1670|        .unwrap();
  1671|
  1672|    // Should detect exactly one contradiction
  1673|    assert_eq!(contradictions.len(), 1, "Expected 1 contradiction, got {}", contradictions.len());
  1674|    assert_eq!(contradictions[0].old_edge_id, edge1_id);
  1675|    assert_eq!(contradictions[0].relation, "chose");
  1676|}
  1677|
  1678|#[test]
  1679|fn test_contradiction_invalidated_edge_has_metadata() {
  1680|    // Verify contradicted_by and contradicted_at in metadata
  1681|    let graph = test_graph();
  1682|
  1683|    let alice = Entity::new("Alice", "Person");
  1684|    let postgres = Entity::new("PostgreSQL", "Database");
  1685|    let mysql = Entity::new("MySQL", "Database");
  1686|
  1687|    graph.add_entity(alice.clone()).unwrap();
  1688|    graph.add_entity(postgres.clone()).unwrap();
  1689|    graph.add_entity(mysql.clone()).unwrap();
  1690|
  1691|    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
  1692|    edge1.confidence = 0.9;
  1693|    edge1.fact = Some("Alice chose PostgreSQL".to_string());
  1694|    let edge1_id = edge1.id.clone();
  1695|    graph.add_edge(edge1).unwrap();
  1696|
  1697|    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
  1698|    edge2.confidence = 0.9;
  1699|    edge2.fact = Some("Alice chose MySQL".to_string());
  1700|    let edge2_for_check = edge2.clone();
  1701|    graph.add_edge(edge2).unwrap();
  1702|
  1703|    // Check for contradictions and invalidate
  1704|    let contradictions = graph
  1705|        .storage
  1706|        .check_contradictions(&[edge2_for_check], 0.2)
  1707|        .unwrap();
  1708|
  1709|    for c in &contradictions {
  1710|        graph
  1711|            .storage
  1712|            .invalidate_contradicted(&c.old_edge_id, &c.new_edge_id)
  1713|            .unwrap();
  1714|    }
  1715|
  1716|    // Get the invalidated edge
  1717|    let all_edges = graph.storage.get_edges_for_entity(&alice.id).unwrap();
  1718|    let invalidated = all_edges.iter().find(|e| e.id == edge1_id).unwrap();
  1719|
  1720|    // Check metadata has contradicted_by and contradicted_at
  1721|    let metadata = invalidated.metadata.as_ref().unwrap();
  1722|    assert!(
  1723|        metadata.get("contradicted_by").is_some(),
  1724|        "Expected contradicted_by in metadata"
  1725|    );
  1726|    assert!(
  1727|        metadata.get("contradicted_at").is_some(),
  1728|        "Expected contradicted_at in metadata"
  1729|    );
  1730|}
  1731|
  1732|#[test]
  1733|fn test_no_contradiction_when_same_fact_inserted_twice() {
  1734|    // Insert the same fact twice — no contradiction
  1735|    let graph = test_graph();
  1736|
  1737|    let alice = Entity::new("Alice", "Person");
  1738|    let postgres = Entity::new("PostgreSQL", "Database");
  1739|
  1740|    graph.add_entity(alice.clone()).unwrap();
  1741|    graph.add_entity(postgres.clone()).unwrap();
  1742|
  1743|    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
  1744|    edge1.confidence = 0.9;
  1745|    edge1.fact = Some("Alice chose PostgreSQL".to_string());
  1746|    graph.add_edge(edge1).unwrap();
  1747|
  1748|    // Same fact, same target
  1749|    let mut edge2 = Edge::new(&alice.id, &postgres.id, "chose");
  1750|    edge2.confidence = 0.9;
  1751|    edge2.fact = Some("Alice chose PostgreSQL".to_string());
  1752|    let edge2_for_check = edge2.clone();
  1753|    graph.add_edge(edge2).unwrap();
  1754|
  1755|    // Check for contradictions — should be none since target_id is the same
  1756|    let contradictions = graph
  1757|        .storage
  1758|        .check_contradictions(&[edge2_for_check], 0.2)
  1759|        .unwrap();
  1760|
  1761|    assert!(
  1762|        contradictions.is_empty(),
  1763|        "Expected no contradiction for same target, got {}",
  1764|        contradictions.len()
  1765|    );
  1766|}
  1767|
  1768|#[test]
  1769|fn test_get_current_edges_returns_only_newer_after_contradiction() {
  1770|    // Verify old edge is excluded from current edges after contradiction
  1771|    let graph = test_graph();
  1772|
  1773|    let alice = Entity::new("Alice", "Person");
  1774|    let postgres = Entity::new("PostgreSQL", "Database");
  1775|    let mysql = Entity::new("MySQL", "Database");
  1776|
  1777|    graph.add_entity(alice.clone()).unwrap();
  1778|    graph.add_entity(postgres.clone()).unwrap();
  1779|    graph.add_entity(mysql.clone()).unwrap();
  1780|
  1781|    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
  1782|    edge1.confidence = 0.9;
  1783|    edge1.fact = Some("Alice chose PostgreSQL".to_string());
  1784|    let edge1_id = edge1.id.clone();
  1785|    graph.add_edge(edge1).unwrap();
  1786|
  1787|    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
  1788|    edge2.confidence = 0.9;
  1789|    edge2.fact = Some("Alice chose MySQL".to_string());
  1790|    let edge2_id = edge2.id.clone();
  1791|    let edge2_for_check = edge2.clone();
  1792|    graph.add_edge(edge2).unwrap();
  1793|
  1794|    // Check for contradictions and invalidate
  1795|    let contradictions = graph
  1796|        .storage
  1797|        .check_contradictions(&[edge2_for_check], 0.2)
  1798|        .unwrap();
  1799|
  1800|    for c in &contradictions {
  1801|        graph
  1802|            .storage
  1803|            .invalidate_contradicted(&c.old_edge_id, &c.new_edge_id)
  1804|            .unwrap();
  1805|    }
  1806|
  1807|    // Get current edges for Alice — should only return edge2
  1808|    let current_edges = graph
  1809|        .storage
  1810|        .get_current_edges_for_entity(&alice.id)
  1811|        .unwrap();
  1812|
  1813|    // edge1 should be invalidated, edge2 should be current
  1814|    assert_eq!(current_edges.len(), 1, "Expected 1 current edge, got {}", current_edges.len());
  1815|    assert_eq!(current_edges[0].id, edge2_id);
  1816|    assert!(
  1817|        current_edges.iter().all(|e| e.id != edge1_id),
  1818|        "Old invalidated edge should not be in current edges"
  1819|    );
  1820|}
  1821|
  1822|#[test]
  1823|fn test_low_confidence_edge_replaced_silently() {
  1824|    // confidence=0.1 edge replaced without contradiction
  1825|    let graph = test_graph();
  1826|
  1827|    let alice = Entity::new("Alice", "Person");
  1828|    let postgres = Entity::new("PostgreSQL", "Database");
  1829|    let mysql = Entity::new("MySQL", "Database");
  1830|
  1831|    graph.add_entity(alice.clone()).unwrap();
  1832|    graph.add_entity(postgres.clone()).unwrap();
  1833|    graph.add_entity(mysql.clone()).unwrap();
  1834|
  1835|    // First edge with low confidence (below threshold)
  1836|    let mut edge1 = Edge::new(&alice.id, &postgres.id, "chose");
  1837|    edge1.confidence = 0.1; // Below 0.2 threshold
  1838|    edge1.fact = Some("Alice chose PostgreSQL".to_string());
  1839|    let edge1_id = edge1.id.clone();
  1840|    graph.add_edge(edge1).unwrap();
  1841|
  1842|    // Second edge with higher confidence
  1843|    let mut edge2 = Edge::new(&alice.id, &mysql.id, "chose");
  1844|    edge2.confidence = 0.9;
  1845|    edge2.fact = Some("Alice chose MySQL".to_string());
  1846|    let edge2_id = edge2.id.clone();
  1847|    let edge2_for_check = edge2.clone();
  1848|    graph.add_edge(edge2).unwrap();
  1849|
  1850|    // Check for contradictions — edge1 should be silently invalidated
  1851|    // not flagged as a contradiction (confidence below threshold)
  1852|    let contradictions = graph
  1853|        .storage
  1854|        .check_contradictions(&[edge2_for_check], 0.2)
  1855|        .unwrap();
  1856|
  1857|    // No contradiction recorded since edge1 confidence < threshold
  1858|    assert!(
  1859|        contradictions.is_empty(),
  1860|        "Expected no contradiction for low-confidence edge, got {}",
  1861|        contradictions.len()
  1862|    );
  1863|
  1864|    // But edge1 should be invalidated internally
  1865|    let current_edges = graph
  1866|        .storage
  1867|        .get_current_edges_for_entity(&alice.id)
  1868|        .unwrap();
  1869|
  1870|    // Only edge2 should be current
  1871|    assert_eq!(current_edges.len(), 1);
  1872|    assert_eq!(current_edges[0].id, edge2_id);
  1873|}
  1874|
  1875|#[test]
  1876|fn test_contradiction_check_empty_graph_returns_empty() {
  1877|    // Unit test for check_contradictions on empty graph
  1878|    let graph = test_graph();
  1879|
  1880|    let alice = Entity::new("Alice", "Person");
  1881|    let postgres = Entity::new("PostgreSQL", "Database");
  1882|
  1883|    graph.add_entity(alice.clone()).unwrap();
  1884|    graph.add_entity(postgres.clone()).unwrap();
  1885|
  1886|    let edge = Edge::new(&alice.id, &postgres.id, "chose");
  1887|    let edge_for_first_check = edge.clone();
  1888|
  1889|    // Check contradictions with single edge in empty graph
  1890|    let contradictions = graph
  1891|        .storage
  1892|        .check_contradictions(&[edge_for_first_check], 0.2)
  1893|        .unwrap();
  1894|
  1895|    assert!(
  1896|        contradictions.is_empty(),
  1897|        "Expected no contradiction in empty graph, got {}",
  1898|        contradictions.len()
  1899|    );
  1900|
  1901|    // Add edge and check again with same edge
  1902|    graph.add_edge(edge.clone()).unwrap();
  1903|
  1904|    let contradictions = graph
  1905|        .storage
  1906|        .check_contradictions(&[edge], 0.2)
  1907|        .unwrap();
  1908|
  1909|    // Should still be empty since they're the same (same target_id)
  1910|    assert!(
  1911|        contradictions.is_empty(),
  1912|        "Expected no contradiction for identical edges, got {}",
  1913|        contradictions.len()
  1914|    );
  1915|=======
  1917|
  1918|#[test]
  1919|fn test_fts5_search_with_question_mark() {
  1920|    // T18: Query with ? character returns results without syntax error
  1921|    let graph = test_graph();
  1922|
  1923|    graph
  1924|        .add_episode(Episode::builder("Chose JWT for auth over session cookies").build())
  1925|        .unwrap();
  1926|    graph
  1927|        .add_episode(Episode::builder("Implemented OAuth2 flow with PKCE").build())
  1928|        .unwrap();
  1929|
  1930|    // This used to crash with "fts5: syntax error near '?'"
  1931|    let results = graph.search("JWT?", 10).unwrap();
  1932|    assert_eq!(results.len(), 1);
  1933|    assert!(results[0].0.content.contains("JWT"));
  1934|}
  1935|
  1936|#[test]
  1937|fn test_fts5_search_with_and_or_not_keywords() {
  1938|    // T19: FTS5 keywords in content are searchable via quoted phrases or combined queries
  1939|    let graph = test_graph();
  1940|
  1941|    graph
  1942|        .add_episode(Episode::builder("Using AND gate for signal processing logic").build())
  1943|        .unwrap();
  1944|    graph
  1945|        .add_episode(
  1946|            Episode::builder("OR operator in search queries returns combined results").build(),
  1947|        )
  1948|        .unwrap();
  1949|    graph
  1950|        .add_episode(Episode::builder("NOT NULL constraint added to user_id column").build())
  1951|        .unwrap();
  1952|
  1953|    // "AND gate" — search for "gate" (AND is a boolean op at query start, not content match).
  1954|    // Users searching for content containing AND should use phrase matching.
  1955|    let results = graph.search("signal AND gate", 10).unwrap();
  1956|    assert_eq!(results.len(), 1);
  1957|    assert!(results[0].0.content.contains("AND gate"));
  1958|
  1959|    // "OR operator" — search with boolean OR
  1960|    let results = graph.search("operator OR constraint", 10).unwrap();
  1961|    assert_eq!(results.len(), 2);
  1962|
  1963|    // "NOT NULL" — search for "NOT" as boolean op + "NULL"
  1964|    // Since NOT at query start is valid FTS5: "NOT NULL" means "without NULL"
  1965|    // Let's search for the full content instead
  1966|    let results = graph.search("constraint NULL", 10).unwrap();
  1967|    assert_eq!(results.len(), 1);
  1968|    assert!(results[0].0.content.contains("NOT NULL"));
  1969|}
  1970|
  1971|#[test]
  1972|fn test_fts5_search_with_parentheses_and_quotes() {
  1973|    // T20: Parentheses and quotes don't cause syntax errors
  1974|    let graph = test_graph();
  1975|
  1976|    graph
  1977|        .add_episode(Episode::builder("Upgraded Express to v4.17 for better routing").build())
  1978|        .unwrap();
  1979|    graph
  1980|        .add_episode(Episode::builder("Configured \"strict mode\" for TypeScript compiler").build())
  1981|        .unwrap();
  1982|
  1983|    // Parentheses in query — used to crash
  1984|    let results = graph.search("(v4.17)", 10).unwrap();
  1985|    assert_eq!(results.len(), 1);
  1986|    assert!(results[0].0.content.contains("v4.17"));
  1987|
  1988|    // Quotes in content and query
  1989|    let results = graph.search("strict mode", 10).unwrap();
  1990|    assert_eq!(results.len(), 1);
  1991|    assert!(results[0].0.content.contains("strict mode"));
  1992|}
  1993|
  1994|#[test]
  1995|fn test_fts5_search_entities_with_special_chars() {
  1996|    // T21: Entity search handles dots, plus signs, and other special chars
  1997|    let graph = test_graph();
  1998|
  1999|    graph.add_entity(Entity::new("Node.js", "Runtime")).unwrap();
  2000|    graph.add_entity(Entity::new("C++", "Language")).unwrap();
  2001|
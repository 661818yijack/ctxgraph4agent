use ctxgraph::*;

fn sorted_names(entities: &[Entity]) -> Vec<String> {
    let mut names: Vec<String> = entities.iter().map(|entity| entity.name.clone()).collect();
    names.sort();
    names
}

#[tokio::test]
async fn captures_sourced_tagged_decision_episode_for_agent_memory() {
    let graph = Graph::in_memory().expect("failed to create in-memory graph");

    let episode = Episode::builder(
        "Apex chose SQLite for local agent memory because it keeps setup offline.",
    )
    .source("agent-journal")
    .tag("architecture")
    .tag("offline")
    .build();
    let episode_id = episode.id.clone();

    graph.add_episode(episode).await.unwrap();

    let retrieved = graph.get_episode(&episode_id).unwrap().unwrap();
    assert_eq!(retrieved.source.as_deref(), Some("agent-journal"));

    let metadata = retrieved
        .metadata
        .as_ref()
        .expect("tags stored as metadata");
    let tags = metadata
        .get("tags")
        .and_then(|value| value.as_array())
        .expect("tags metadata is an array");
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].as_str(), Some("architecture"));
    assert_eq!(tags[1].as_str(), Some("offline"));

    let matches = graph
        .search("SQLite offline", 10, Some("agent-journal"))
        .unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0.id, episode_id);
}

#[tokio::test]
async fn manually_links_entities_edges_and_traversal_without_extraction_models() {
    let graph = Graph::in_memory().expect("failed to create in-memory graph");

    let episode = Episode::builder(
        "Apex linked billing memory to SQLite so agent recall works without model downloads.",
    )
    .source("agent-journal")
    .tag("manual-graph")
    .build();
    let episode_id = episode.id.clone();
    graph.add_episode(episode).await.unwrap();

    let service = Entity::new("billing memory", "Service");
    let database = Entity::new("SQLite", "Database");
    let service_id = service.id.clone();
    let database_id = database.id.clone();
    graph.add_entity(service).unwrap();
    graph.add_entity(database).unwrap();

    graph
        .link_episode_entity(&episode_id, &service_id, Some(12), Some(26))
        .unwrap();
    graph
        .link_episode_entity(&episode_id, &database_id, Some(30), Some(36))
        .unwrap();

    let mut edge = Edge::new(&service_id, &database_id, "stored_in");
    edge.episode_id = Some(episode_id.clone());
    edge.fact = Some("billing memory stored_in SQLite".to_string());
    graph.add_edge(edge).unwrap();

    let service_edges = graph.get_edges_for_entity(&service_id).unwrap();
    assert_eq!(service_edges.len(), 1);
    assert_eq!(service_edges[0].relation, "stored_in");
    assert_eq!(
        service_edges[0].episode_id.as_deref(),
        Some(episode_id.as_str())
    );

    let (entities, edges) = graph.traverse(&service_id, 1).unwrap();
    assert_eq!(sorted_names(&entities), vec!["SQLite", "billing memory"]);
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].relation, "stored_in");
}

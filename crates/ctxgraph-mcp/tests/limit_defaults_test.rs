use ctxgraph::{Entity, Episode, Graph};
use ctxgraph_mcp::tools::ToolContext;
use serde_json::{Value, json};

fn array_len(value: &Value) -> usize {
    value.as_array().map(|items| items.len()).unwrap_or(0)
}

#[tokio::test]
async fn search_limit_zero_uses_default_results() {
    let graph = Graph::in_memory().expect("in-memory graph");
    graph
        .add_episode(Episode::builder("PostgreSQL powers billing search").build())
        .await
        .expect("episode inserted");
    let tools = ToolContext::new(graph, None);

    let default_results = tools
        .search(json!({"query": "PostgreSQL"}))
        .await
        .expect("default search succeeds");
    let zero_limit_results = tools
        .search(json!({"query": "PostgreSQL", "limit": 0}))
        .await
        .expect("zero-limit search succeeds");
    let positive_limit_results = tools
        .search(json!({"query": "PostgreSQL", "limit": 1}))
        .await
        .expect("positive-limit search succeeds");

    assert_eq!(array_len(&zero_limit_results), array_len(&default_results));
    assert_eq!(array_len(&positive_limit_results), 1);
}

#[tokio::test]
async fn list_entities_limit_zero_uses_default_before_offset() {
    let graph = Graph::in_memory().expect("in-memory graph");
    graph
        .add_entity(Entity::new("PostgreSQL", "Database"))
        .expect("entity inserted");
    graph
        .add_entity(Entity::new("Redis", "Database"))
        .expect("entity inserted");
    let tools = ToolContext::new(graph, None);

    let default_page = tools
        .list_entities(json!({}))
        .await
        .expect("default list succeeds");
    let zero_limit_page = tools
        .list_entities(json!({"limit": 0}))
        .await
        .expect("zero-limit list succeeds");
    let offset_page = tools
        .list_entities(json!({"limit": 0, "offset": 1}))
        .await
        .expect("zero-limit offset list succeeds");

    assert_eq!(zero_limit_page["count"], default_page["count"]);
    assert_eq!(offset_page["count"], json!(1));
}

#[tokio::test]
async fn export_graph_limit_zero_uses_default_cap() {
    let graph = Graph::in_memory().expect("in-memory graph");
    graph
        .add_entity(Entity::new("PostgreSQL", "Database"))
        .expect("entity inserted");
    let tools = ToolContext::new(graph, None);

    let default_export = tools
        .export_graph(json!({}))
        .await
        .expect("default export succeeds");
    let zero_limit_export = tools
        .export_graph(json!({"limit": 0}))
        .await
        .expect("zero-limit export succeeds");

    assert_eq!(
        zero_limit_export["entity_count"],
        default_export["entity_count"]
    );
    assert_eq!(zero_limit_export["entity_count"], json!(1));
}

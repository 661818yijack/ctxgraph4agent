#[test]
fn agent_memory_example_source_documents_full_workflow() {
    let source = include_str!("../examples/agent_memory_workflow.rs");

    for marker in [
        "// 1. Record memory episodes",
        "// 2. List recent memory",
        "// 3. Search prior decisions",
        "// 4. Traverse graph context",
    ] {
        assert!(source.contains(marker), "missing workflow marker: {marker}");
    }

    assert!(source.contains("Episode::builder"));
    assert!(source.contains("graph.add_episode"));
    assert!(source.contains("graph.list_episodes"));
    assert!(source.contains("graph.search"));
    assert!(source.contains("graph.traverse"));
}

#[test]
fn agent_memory_usage_guide_points_to_runnable_example() {
    let guide = include_str!("../../../docs/AGENT_MEMORY_WORKFLOW.md");

    assert!(guide.contains("cargo run -p ctxgraph --example agent_memory_workflow"));
    assert!(guide.contains("Record episodes"));
    assert!(guide.contains("Search memory"));
    assert!(guide.contains("Traverse graph context"));
}

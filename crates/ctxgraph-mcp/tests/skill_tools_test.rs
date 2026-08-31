use chrono::Utc;
use ctxgraph::{Graph, Skill, SkillScope};
use ctxgraph_mcp::tools::{ToolContext, tools_list};
use serde_json::{Value, json};
use tokio::runtime::Runtime;

fn test_skill(id: &str, owner: &str, scope: SkillScope) -> Skill {
    Skill {
        id: id.to_string(),
        name: format!("skill-{id}"),
        description: format!("description for {id}"),
        trigger_condition: "when testing MCP skill tools".to_string(),
        action: "return deterministic skill data".to_string(),
        success_count: 2,
        failure_count: 0,
        confidence: 1.0,
        superseded_by: None,
        created_at: Utc::now(),
        entity_types: vec!["Technology".to_string()],
        provenance: None,
        scope,
        created_by_agent: owner.to_string(),
    }
}

fn parse_tool_result(value: Value) -> Value {
    let text = value["content"][0]["text"].as_str().expect("tool text");
    serde_json::from_str(text).expect("tool result JSON")
}

#[test]
fn tools_list_advertises_skill_management_tools() {
    let payload = tools_list();
    let tools = payload["tools"].as_array().expect("tools array");

    let list_tool = tools
        .iter()
        .find(|tool| tool["name"] == "list_skills")
        .expect("list_skills should be advertised");
    assert_eq!(
        list_tool["inputSchema"]["properties"]["agent"]["type"],
        "string"
    );

    let share_tool = tools
        .iter()
        .find(|tool| tool["name"] == "share_skill")
        .expect("share_skill should be advertised");
    assert_eq!(
        share_tool["inputSchema"]["properties"]["skill_id"]["type"],
        "string"
    );
    assert_eq!(share_tool["inputSchema"]["required"][0], "skill_id");
}

#[test]
fn list_skills_returns_default_agent_private_and_shared_only() {
    let graph = Graph::in_memory().expect("graph");
    graph
        .storage
        .create_skill(&test_skill(
            "assistant-private",
            "assistant",
            SkillScope::Private,
        ))
        .expect("create assistant skill");
    graph
        .storage
        .create_skill(&test_skill("other-private", "other", SkillScope::Private))
        .expect("create other skill");
    graph
        .storage
        .create_skill(&test_skill("shared", "other", SkillScope::Shared))
        .expect("create shared skill");
    let mut superseded = test_skill("superseded", "assistant", SkillScope::Private);
    superseded.superseded_by = Some("assistant-private".to_string());
    graph
        .storage
        .create_skill(&superseded)
        .expect("create superseded skill");

    let ctx = ToolContext::new(graph, None);
    let result = parse_tool_result(ctxgraph_mcp::tools::tool_result(
        Runtime::new().unwrap().block_on(ctx.list_skills(json!({}))),
    ));

    let ids: Vec<&str> = result["skills"]
        .as_array()
        .expect("skills")
        .iter()
        .map(|skill| skill["id"].as_str().expect("id"))
        .collect();

    assert!(ids.contains(&"assistant-private"));
    assert!(ids.contains(&"shared"));
    assert!(!ids.contains(&"other-private"));
    assert!(!ids.contains(&"superseded"));
    assert_eq!(result["agent"], "assistant");
    assert_eq!(result["count"], 2);
}

#[test]
fn list_skills_respects_explicit_agent() {
    let graph = Graph::in_memory().expect("graph");
    graph
        .storage
        .create_skill(&test_skill("apex-private", "apex", SkillScope::Private))
        .expect("create apex skill");
    graph
        .storage
        .create_skill(&test_skill(
            "assistant-private",
            "assistant",
            SkillScope::Private,
        ))
        .expect("create assistant skill");

    let ctx = ToolContext::new(graph, None);
    let result = parse_tool_result(ctxgraph_mcp::tools::tool_result(
        Runtime::new()
            .unwrap()
            .block_on(ctx.list_skills(json!({"agent":"apex"}))),
    ));
    let ids: Vec<&str> = result["skills"]
        .as_array()
        .expect("skills")
        .iter()
        .map(|skill| skill["id"].as_str().expect("id"))
        .collect();

    assert_eq!(result["agent"], "apex");
    assert!(ids.contains(&"apex-private"));
    assert!(!ids.contains(&"assistant-private"));
}

#[test]
fn share_skill_marks_private_skill_shared_and_is_idempotent() {
    let graph = Graph::in_memory().expect("graph");
    graph
        .storage
        .create_skill(&test_skill("private", "assistant", SkillScope::Private))
        .expect("create private skill");

    let ctx = ToolContext::new(graph, None);
    let first = parse_tool_result(ctxgraph_mcp::tools::tool_result(
        Runtime::new()
            .unwrap()
            .block_on(ctx.share_skill(json!({"skill_id":"private"}))),
    ));
    assert_eq!(first["ok"], true);
    assert_eq!(first["skill_id"], "private");
    assert_eq!(first["scope"], "shared");

    let second = parse_tool_result(ctxgraph_mcp::tools::tool_result(
        Runtime::new()
            .unwrap()
            .block_on(ctx.share_skill(json!({"skill_id":"private"}))),
    ));
    assert_eq!(second["ok"], true);
    assert_eq!(second["skill_id"], "private");
    assert_eq!(second["scope"], "shared");

    let rt = Runtime::new().unwrap();
    let graph = rt.block_on(ctx.graph.lock());
    let skill = graph
        .storage
        .get_skill("private")
        .expect("get skill")
        .expect("skill exists");
    assert_eq!(skill.scope, SkillScope::Shared);
}

#[test]
fn share_skill_reports_missing_skill_id() {
    let ctx = ToolContext::new(Graph::in_memory().expect("graph"), None);
    let result = parse_tool_result(ctxgraph_mcp::tools::tool_result(
        Runtime::new()
            .unwrap()
            .block_on(ctx.share_skill(json!({"skill_id":"missing"}))),
    ));

    assert!(
        result["error"]
            .as_str()
            .expect("error")
            .contains("skill not found")
    );
}

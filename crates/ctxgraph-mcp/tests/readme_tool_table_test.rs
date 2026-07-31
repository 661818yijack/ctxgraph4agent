use std::collections::BTreeSet;

use ctxgraph_mcp::tools::tools_list;

const README: &str = include_str!("../../../README.md");

fn readme_mcp_section() -> &'static str {
    README
        .split("## MCP Server")
        .nth(1)
        .expect("README must have MCP Server section")
        .split("## Rust SDK")
        .next()
        .expect("README MCP Server section must precede Rust SDK section")
}

fn documented_tool_names() -> BTreeSet<String> {
    readme_mcp_section()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("| `") {
                return None;
            }
            let name = trimmed.split('`').nth(1)?;
            Some(name.to_string())
        })
        .collect()
}

fn schema_tool_names() -> BTreeSet<String> {
    tools_list()["tools"]
        .as_array()
        .expect("tools_list must expose a tools array")
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("each tool must have a string name")
                .to_string()
        })
        .collect()
}

#[test]
fn readme_mcp_tool_table_matches_exported_schema_names() {
    assert_eq!(documented_tool_names(), schema_tool_names());
}

#[test]
fn readme_mcp_tool_table_uses_canonical_unprefixed_names() {
    for name in documented_tool_names() {
        assert!(
            !name.starts_with("ctxgraph_"),
            "README must document canonical MCP schema name, not stale prefixed alias: {name}"
        );
    }
}

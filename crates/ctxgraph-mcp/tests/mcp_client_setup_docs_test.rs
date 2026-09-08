use ctxgraph_mcp::tools::tools_list;

const MCP_CLIENT_SETUP_DOC: &str = include_str!("../../../docs/MCP_CLIENT_SETUP.md");

#[test]
fn mcp_client_setup_guide_mentions_every_registered_tool() {
    let tools = tools_list();
    let tool_names = tools["tools"]
        .as_array()
        .expect("tools_list returns a tools array")
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .expect("registered MCP tool has a string name")
        })
        .collect::<Vec<_>>();

    for tool_name in tool_names {
        assert!(
            MCP_CLIENT_SETUP_DOC.contains(&format!("`{tool_name}`")),
            "docs/MCP_CLIENT_SETUP.md should document the registered MCP tool `{tool_name}`"
        );
    }
}

#[test]
fn mcp_client_setup_guide_documents_local_stdio_config() {
    assert!(
        MCP_CLIENT_SETUP_DOC.contains("\"command\": \"ctxgraph-mcp\""),
        "guide should include a copyable stdio config using the ctxgraph-mcp binary"
    );
    assert!(
        MCP_CLIENT_SETUP_DOC.contains("No API key is required"),
        "guide should state local MCP operation has no mandatory API key"
    );
    assert!(
        MCP_CLIENT_SETUP_DOC.contains("cargo build -p ctxgraph-mcp"),
        "guide should show how to verify/build the MCP server binary from source"
    );
}

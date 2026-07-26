use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

struct McpProcess {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

impl McpProcess {
    fn spawn(db_path: &str) -> Self {
        let binary = env!("CARGO_BIN_EXE_ctxgraph-mcp");
        let mut child = Command::new(binary)
            .arg("--db")
            .arg(db_path)
            .env("CTXGRAPH_NO_EMBED", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn ctxgraph-mcp binary");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        Self {
            child,
            stdin,
            reader: BufReader::new(stdout),
        }
    }

    fn send(&mut self, msg: Value) {
        let line = serde_json::to_string(&msg).unwrap() + "\n";
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn recv(&mut self) -> Value {
        let mut line = String::new();
        self.reader.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim()).expect("invalid JSON response")
    }
}

impl Drop for McpProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn test_mcp_initialize_handshake() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db").to_str().unwrap().to_string();

    let mut mcp = McpProcess::spawn(&db);

    mcp.send(json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {"name": "test", "version": "0.1"}
        }
    }));

    let resp = mcp.recv();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert!(
        resp["result"]["serverInfo"]["name"]
            .as_str()
            .unwrap()
            .contains("ctxgraph")
    );
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn test_mcp_tools_list() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db").to_str().unwrap().to_string();
    let mut mcp = McpProcess::spawn(&db);

    // initialize first
    mcp.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    mcp.recv(); // consume response

    mcp.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}));
    let resp = mcp.recv();

    let tools = resp["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"add_episode"));
    assert!(names.contains(&"search"));
    assert!(names.contains(&"get_decision"));
    assert!(names.contains(&"traverse"));
    assert!(names.contains(&"find_precedents"));
}

fn add_episode(mcp: &mut McpProcess, id: u64, text: &str, source: Option<&str>) {
    let mut arguments = json!({"text": text});
    if let Some(source) = source {
        arguments["source"] = json!(source);
    }

    mcp.send(json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "add_episode",
            "arguments": arguments
        }
    }));
    let add_resp = mcp.recv();
    assert_eq!(add_resp["id"], id);
    let content = &add_resp["result"]["content"][0]["text"];
    let result: Value = serde_json::from_str(content.as_str().unwrap()).unwrap();
    assert!(result["episode_id"].is_string());
}

fn search(mcp: &mut McpProcess, id: u64, arguments: Value) -> Vec<Value> {
    mcp.send(json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": "search",
            "arguments": arguments
        }
    }));
    let search_resp = mcp.recv();
    assert_eq!(search_resp["id"], id);
    let search_content = &search_resp["result"]["content"][0]["text"];
    serde_json::from_str::<Vec<Value>>(search_content.as_str().unwrap()).unwrap()
}

#[test]
fn test_mcp_add_episode_and_search() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db").to_str().unwrap().to_string();
    let mut mcp = McpProcess::spawn(&db);

    mcp.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    mcp.recv();

    add_episode(
        &mut mcp,
        2,
        "Chose PostgreSQL over MySQL for the billing service",
        None,
    );

    let results = search(
        &mut mcp,
        3,
        json!({"query": "PostgreSQL billing", "limit": 5}),
    );
    assert!(!results.is_empty(), "search should return results");
}

#[test]
fn test_mcp_search_accepts_topk_alias_for_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db").to_str().unwrap().to_string();
    let mut mcp = McpProcess::spawn(&db);

    mcp.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    mcp.recv();

    add_episode(&mut mcp, 2, "PostgreSQL billing alpha", Some("slack"));
    add_episode(&mut mcp, 3, "PostgreSQL billing beta", Some("slack"));
    add_episode(&mut mcp, 4, "PostgreSQL billing gamma", Some("email"));

    let topk_results = search(
        &mut mcp,
        5,
        json!({"query": "PostgreSQL billing", "topk": 1}),
    );
    assert_eq!(topk_results.len(), 1, "topk should cap search results");

    let canonical_limit_results = search(
        &mut mcp,
        6,
        json!({"query": "PostgreSQL billing", "limit": 2, "topk": 1}),
    );
    assert_eq!(
        canonical_limit_results.len(),
        2,
        "canonical limit should take precedence over topk"
    );

    let source_results = search(
        &mut mcp,
        7,
        json!({"query": "PostgreSQL billing", "source": "slack", "topk": 2}),
    );
    assert_eq!(source_results.len(), 2);
    assert!(source_results.iter().all(|item| item["source"] == "slack"));
}

#[test]
fn test_mcp_tools_list_documents_search_topk_alias() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db").to_str().unwrap().to_string();
    let mut mcp = McpProcess::spawn(&db);

    mcp.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    mcp.recv();

    mcp.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}));
    let resp = mcp.recv();
    let tools = resp["result"]["tools"].as_array().unwrap();
    let search_tool = tools
        .iter()
        .find(|tool| tool["name"] == "search")
        .expect("search tool should be registered");
    assert!(
        search_tool["inputSchema"]["properties"]
            .get("topk")
            .is_some(),
        "search schema should document topk as an accepted alias"
    );
}

#[test]
fn test_mcp_unknown_method_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("test.db").to_str().unwrap().to_string();
    let mut mcp = McpProcess::spawn(&db);

    mcp.send(json!({"jsonrpc":"2.0","id":1,"method":"nonexistent","params":{}}));
    let resp = mcp.recv();
    assert!(resp["error"].is_object());
    assert_eq!(resp["error"]["code"], -32601);
}

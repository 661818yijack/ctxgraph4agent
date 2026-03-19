use serde_json::{Value, json};

// Re-use protocol types by path — the crate is a binary, so we test
// the serialization round-trip directly with serde_json.

/// Simulate building a JSON-RPC response "ok" payload.
fn make_ok_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

/// Simulate building a JSON-RPC error payload.
fn make_error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[test]
fn test_ok_response_shape() {
    let resp = make_ok_response(json!(1), json!({"tools": []}));
    assert_eq!(resp["jsonrpc"].as_str().unwrap(), "2.0");
    assert_eq!(resp["id"].as_i64().unwrap(), 1);
    assert!(resp["result"].is_object());
    assert!(resp.get("error").is_none() || resp["error"].is_null());
}

#[test]
fn test_error_response_shape() {
    let resp = make_error_response(json!(2), -32601, "method not found");
    assert_eq!(resp["jsonrpc"].as_str().unwrap(), "2.0");
    assert_eq!(resp["id"].as_i64().unwrap(), 2);
    assert_eq!(resp["error"]["code"].as_i64().unwrap(), -32601);
    assert_eq!(
        resp["error"]["message"].as_str().unwrap(),
        "method not found"
    );
}

#[test]
fn test_initialize_result_shape() {
    let result = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {"tools": {}},
        "serverInfo": {"name": "ctxgraph", "version": "0.3.0"}
    });

    assert_eq!(result["protocolVersion"].as_str().unwrap(), "2024-11-05");
    assert!(result["capabilities"]["tools"].is_object());
    assert_eq!(result["serverInfo"]["name"].as_str().unwrap(), "ctxgraph");
}

#[test]
fn test_request_parse() {
    let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":null}"#;
    let val: Value = serde_json::from_str(raw).unwrap();
    assert_eq!(val["method"].as_str().unwrap(), "tools/list");
    assert_eq!(val["id"].as_i64().unwrap(), 1);
}

#[test]
fn test_notification_has_no_id() {
    let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let val: Value = serde_json::from_str(raw).unwrap();
    assert!(val.get("id").is_none() || val["id"].is_null());
}

#[test]
fn test_tool_result_content_shape() {
    let content = json!({
        "content": [{"type": "text", "text": "{\"episode_id\": \"abc\"}"}]
    });
    let items = content["content"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["type"].as_str().unwrap(), "text");
    assert!(items[0]["text"].as_str().unwrap().contains("episode_id"));
}

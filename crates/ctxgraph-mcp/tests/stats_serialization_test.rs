use serde_json::{Map, Number, Value, json};

/// Helper that the stats() handler should use to convert Vec<(String, usize)>
/// into a JSON object { key: count }.
/// This mirrors the implementation in tools.rs — keep in sync.
fn vec_pairs_to_json_object(pairs: &[(String, usize)]) -> Value {
    let mut map = Map::new();
    for (key, val) in pairs {
        map.insert(key.clone(), Value::Number(Number::from(*val)));
    }
    Value::Object(map)
}

#[test]
fn test_vec_pairs_to_json_object_basic() {
    let pairs = vec![
        ("fact".to_string(), 10),
        ("experience".to_string(), 5),
        ("pattern".to_string(), 3),
    ];
    let result = vec_pairs_to_json_object(&pairs);

    // Must be an object, not an array
    assert!(result.is_object(), "Expected JSON object, got: {result}");

    let obj = result.as_object().unwrap();
    assert_eq!(obj.len(), 3);
    assert_eq!(obj["fact"].as_u64().unwrap(), 10);
    assert_eq!(obj["experience"].as_u64().unwrap(), 5);
    assert_eq!(obj["pattern"].as_u64().unwrap(), 3);
}

#[test]
fn test_vec_pairs_to_json_object_empty() {
    let pairs: Vec<(String, usize)> = vec![];
    let result = vec_pairs_to_json_object(&pairs);

    // Empty input must produce empty object {}, not empty array []
    assert!(result.is_object(), "Expected empty object, got: {result}");
    assert!(result.as_object().unwrap().is_empty());
}

#[test]
fn test_stats_response_sources_is_object() {
    // Simulates the stats response shape that tools.rs::stats() produces
    let sources = vec![("meeting".to_string(), 3), ("code-review".to_string(), 1)];
    let stats_response = json!({
        "episodes": 4,
        "entities": 12,
        "edges": 8,
        "sources": vec_pairs_to_json_object(&sources),
    });

    let sources_val = &stats_response["sources"];
    assert!(
        sources_val.is_object(),
        "sources must be a JSON object, got: {sources_val}"
    );
    assert_eq!(sources_val["meeting"].as_u64().unwrap(), 3);
    assert_eq!(sources_val["code-review"].as_u64().unwrap(), 1);
}

#[test]
fn test_stats_response_total_entities_by_type_is_object() {
    let by_type = vec![
        ("fact".to_string(), 8),
        ("experience".to_string(), 3),
        ("pattern".to_string(), 1),
    ];
    let stats_response = json!({
        "total_entities_by_type": vec_pairs_to_json_object(&by_type),
    });

    let val = &stats_response["total_entities_by_type"];
    assert!(
        val.is_object(),
        "total_entities_by_type must be a JSON object, got: {val}"
    );
    assert_eq!(val["fact"].as_u64().unwrap(), 8);
}

#[test]
fn test_stats_response_decayed_entities_by_type_is_object() {
    let decayed = vec![("fact".to_string(), 2), ("experience".to_string(), 1)];
    let stats_response = json!({
        "decayed_entities_by_type": vec_pairs_to_json_object(&decayed),
    });

    let val = &stats_response["decayed_entities_by_type"];
    assert!(
        val.is_object(),
        "decayed_entities_by_type must be a JSON object, got: {val}"
    );
    assert_eq!(val["fact"].as_u64().unwrap(), 2);
}

#[test]
fn test_stats_values_are_integers_not_strings() {
    let pairs = vec![("source".to_string(), 42)];
    let result = vec_pairs_to_json_object(&pairs);

    let val = &result["source"];
    assert!(val.is_number(), "Value must be an integer, got: {val}");
    assert_eq!(val.as_u64().unwrap(), 42);
}

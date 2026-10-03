const ENTITIES_COMMAND_SOURCE: &str = include_str!("../src/commands/entities.rs");

#[test]
fn empty_entities_guidance_avoids_stale_version_promise() {
    assert!(
        !ENTITIES_COMMAND_SOURCE.contains("v0.2"),
        "empty entity guidance should not mention stale v0.2 auto-extraction promises"
    );
}

#[test]
fn empty_entities_guidance_points_to_current_log_workflow() {
    assert!(
        ENTITIES_COMMAND_SOURCE.contains("ctxgraph log"),
        "empty entity guidance should tell users to create episodes with `ctxgraph log`"
    );
    assert!(
        ENTITIES_COMMAND_SOURCE.contains("Rust API"),
        "empty entity guidance should preserve the manual Rust API insertion path"
    );
}

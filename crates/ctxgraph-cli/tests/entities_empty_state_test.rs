use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fresh_project_dir() -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "ctxgraph-cli-empty-entities-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("failed to create temp project directory");
    dir
}

#[test]
fn empty_entities_message_points_to_current_workflow() {
    let project_dir = fresh_project_dir();
    let binary = env!("CARGO_BIN_EXE_ctxgraph");

    let init_output = Command::new(binary)
        .arg("init")
        .current_dir(&project_dir)
        .output()
        .expect("failed to run ctxgraph init");
    assert!(
        init_output.status.success(),
        "ctxgraph init failed: {}",
        String::from_utf8_lossy(&init_output.stderr)
    );

    let output = Command::new(binary)
        .args(["entities", "list"])
        .current_dir(&project_dir)
        .output()
        .expect("failed to run ctxgraph entities list");
    assert!(
        output.status.success(),
        "ctxgraph entities list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No entities found."),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("ctxgraph log"),
        "empty state should point users at the current logging workflow: {stdout}"
    );
    assert!(
        stdout.contains("ctxgraph models download"),
        "empty state should mention optional model setup for extraction: {stdout}"
    );
    assert!(
        !stdout.contains("v0.2"),
        "empty state should not mention obsolete future-version wording: {stdout}"
    );

    fs::remove_dir_all(&project_dir).ok();
}

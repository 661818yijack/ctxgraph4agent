use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CLI crate should have workspace parent")
        .parent()
        .expect("crates directory should have workspace parent")
        .to_path_buf()
}

fn run_ctxgraph(workdir: &std::path::Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ctxgraph"))
        .args(args)
        .current_dir(workdir)
        .output()
        .expect("ctxgraph command should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "ctxgraph {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        stdout,
        stderr
    );
    stdout.into_owned()
}

#[test]
fn decision_memory_guide_is_discoverable_from_readme() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("README should exist");
    let guide = fs::read_to_string(root.join("docs/use-cases/decision-memory-cli.md"))
        .expect("decision memory guide should exist");

    assert!(
        readme.contains("docs/use-cases/decision-memory-cli.md"),
        "README should link to the tested CLI decision-memory guide"
    );
    for required in [
        "ctxgraph init",
        "ctxgraph log",
        "ctxgraph query",
        "ctxgraph stats",
        "No API key required",
    ] {
        assert!(
            guide.contains(required),
            "guide should mention required workflow text: {required}"
        );
    }
}

#[test]
fn decision_memory_cli_workflow_runs_offline() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    let sandbox = std::env::temp_dir().join(format!("ctxgraph-decision-memory-{unique}"));
    fs::create_dir_all(&sandbox).expect("sandbox should be created");

    run_ctxgraph(&sandbox, &["init", "--name", "decision-memory-demo"]);
    run_ctxgraph(
        &sandbox,
        &[
            "log",
            "We chose SQLite for agent memory because it keeps the context graph local, cheap, and easy to back up.",
            "--source",
            "architecture-notes",
            "--tags",
            "decision,memory,sqlite",
        ],
    );

    let query = run_ctxgraph(
        &sandbox,
        &["query", "SQLite", "--source", "architecture-notes"],
    );
    assert!(
        query.contains("SQLite"),
        "query should return the logged decision: {query}"
    );
    assert!(
        query.contains("architecture-notes"),
        "query should preserve the source in output: {query}"
    );

    let stats = run_ctxgraph(&sandbox, &["stats"]);
    assert!(
        stats.contains("Episodes:"),
        "stats should include episode count: {stats}"
    );

    fs::remove_dir_all(&sandbox).expect("sandbox cleanup should succeed");
}

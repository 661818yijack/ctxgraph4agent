use std::fs;
use std::path::PathBuf;

fn workspace_manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("Cargo.toml")
}

#[test]
fn workspace_repository_points_to_active_github_repo() {
    let manifest = fs::read_to_string(workspace_manifest()).expect("read workspace Cargo.toml");
    let value: toml::Value = toml::from_str(&manifest).expect("parse workspace Cargo.toml");

    let repository = value
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("repository"))
        .and_then(toml::Value::as_str)
        .expect("workspace.package.repository is present");

    assert_eq!(repository, "https://github.com/661818yijack/ctxgraph4agent");
    assert!(
        !repository.contains("rohansx/ctxgraph"),
        "workspace repository metadata must not point at the stale upstream repo"
    );
}

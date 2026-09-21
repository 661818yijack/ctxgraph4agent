use std::fs;
use std::path::Path;

use toml::Value;

const ACTIVE_REPOSITORY: &str = "https://github.com/661818yijack/ctxgraph4agent";
const STALE_REPOSITORY: &str = "https://github.com/rohansx/ctxgraph";

fn workspace_manifest_path() -> std::path::PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = crate_dir
        .parent()
        .expect("crate directory should have a workspace parent");
    let workspace_dir = crates_dir
        .parent()
        .expect("workspace manifest directory should exist");

    workspace_dir.join("Cargo.toml")
}

#[test]
fn workspace_package_repository_points_to_active_github_repo() {
    let manifest =
        fs::read_to_string(workspace_manifest_path()).expect("read workspace Cargo.toml");
    let parsed: Value = toml::from_str(&manifest).expect("parse workspace Cargo.toml");

    let repository = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("repository"))
        .and_then(Value::as_str)
        .expect("workspace.package.repository should be set");

    assert_eq!(repository, ACTIVE_REPOSITORY);
    assert_ne!(repository, STALE_REPOSITORY);
}

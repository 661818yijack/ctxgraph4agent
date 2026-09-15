use std::fs;
use std::path::{Path, PathBuf};

const STALE_REPO_URL: &str = "https://github.com/rohansx/ctxgraph";
const ACTIVE_REPO_URL: &str = "https://github.com/661818yijack/ctxgraph4agent";

#[test]
fn blog_docs_do_not_link_to_stale_repository_home() {
    let repo_root = repo_root();
    let blog_dir = repo_root.join("docs/blog");

    let mut stale_matches = Vec::new();
    let mut active_link_count = 0;
    for path in markdown_files(&blog_dir) {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if content.contains(STALE_REPO_URL) {
            stale_matches.push(path.strip_prefix(&repo_root).unwrap().display().to_string());
        }
        active_link_count += content.matches(ACTIVE_REPO_URL).count();
    }

    assert!(
        stale_matches.is_empty(),
        "blog docs should link to {ACTIVE_REPO_URL}; stale {STALE_REPO_URL} found in: {}",
        stale_matches.join(", ")
    );
    assert!(
        active_link_count > 0,
        "blog docs should contain active repository links to {ACTIVE_REPO_URL}"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("ctxgraph-core crate should live two levels below repo root")
        .to_path_buf()
}

fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_markdown_files(dir, &mut files);
    files.sort();
    files
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(dir).unwrap_or_else(|err| panic!("failed to read {}: {err}", dir.display()))
    {
        let entry = entry.expect("blog directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "md") {
            files.push(path);
        }
    }
}

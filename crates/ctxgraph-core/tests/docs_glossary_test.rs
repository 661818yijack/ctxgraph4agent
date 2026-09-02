use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("ctxgraph-core should be under crates/")
        .parent()
        .expect("crates/ should be under the repository root")
        .to_path_buf()
}

#[test]
fn glossary_defines_agent_facing_memory_terms() {
    let glossary_path = repo_root().join("docs/GLOSSARY.md");
    let glossary = std::fs::read_to_string(&glossary_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", glossary_path.display()));

    let required_terms = [
        "# ctxgraph Glossary",
        "## Core graph terms",
        "### Episode",
        "### Entity",
        "### Edge",
        "### Source",
        "### Memory type",
        "## Lifecycle and retrieval terms",
        "### TTL",
        "### Decay score",
        "### Soft-expired memory",
        "### Re-verification",
        "### Retrieval candidate",
    ];

    for term in required_terms {
        assert!(
            glossary.contains(term),
            "docs/GLOSSARY.md should define `{term}` for agent integrators"
        );
    }
}

#[test]
fn readme_links_to_glossary() {
    let readme_path = repo_root().join("README.md");
    let readme = std::fs::read_to_string(&readme_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", readme_path.display()));

    assert!(
        readme.contains("[Glossary](docs/GLOSSARY.md)"),
        "README.md should link to docs/GLOSSARY.md so new agent developers can find the terminology guide"
    );
}

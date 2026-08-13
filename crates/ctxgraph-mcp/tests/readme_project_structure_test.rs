use std::collections::BTreeSet;
use std::path::Path;

fn readme_project_structure_crates() -> BTreeSet<String> {
    let readme = include_str!("../../../README.md");
    let project_structure = readme
        .split("## Project Structure")
        .nth(1)
        .expect("README has Project Structure section")
        .split("## Contributing")
        .next()
        .expect("README has Contributing section after Project Structure");

    project_structure
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let rest = trimmed.strip_prefix("+-- ")?;
            let (crate_name, _) = rest.split_once('/')?;
            Some(crate_name.trim().to_string())
        })
        .collect()
}

fn actual_workspace_crates() -> BTreeSet<String> {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates");
    std::fs::read_dir(crates_dir)
        .expect("workspace crates directory exists")
        .map(|entry| {
            let entry = entry.expect("crate directory entry is readable");
            entry
                .file_name()
                .into_string()
                .expect("crate directory name is valid UTF-8")
        })
        .collect()
}

#[test]
fn readme_project_structure_lists_existing_crates() {
    let documented = readme_project_structure_crates();
    let actual = actual_workspace_crates();

    let stale: BTreeSet<_> = documented.difference(&actual).cloned().collect();
    assert!(
        stale.is_empty(),
        "README Project Structure lists crates that do not exist: {stale:?}"
    );
}

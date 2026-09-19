use std::fs;
use std::path::PathBuf;

fn guide_text() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let guide_path = manifest_dir
        .parent()
        .expect("cli crate has workspace parent")
        .parent()
        .expect("workspace has repository root")
        .join("docs")
        .join("usage")
        .join("backup-restore.md");

    fs::read_to_string(&guide_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", guide_path.display()))
}

#[test]
fn backup_restore_guide_covers_safe_backup_workflow() {
    let guide = guide_text();

    for required in [
        "# Backup and Restore ctxgraph Databases",
        ".ctxgraph/graph.db",
        "BACKUP_DIR=\"backups/ctxgraph-$(date -u +%Y%m%dT%H%M%SZ)\"",
        "mkdir -p \"$BACKUP_DIR\"",
        "cp .ctxgraph/graph.db \"$BACKUP_DIR/graph.db\"",
        "test -f \"$BACKUP_DIR/graph.db\"",
    ] {
        assert!(
            guide.contains(required),
            "backup guide is missing required text: {required}"
        );
    }
}

#[test]
fn backup_restore_guide_covers_restore_guardrails() {
    let guide = guide_text();

    for required in [
        "Stop active writers before restoring",
        "PRE_RESTORE_DIR=\"backups/pre-restore-$(date -u +%Y%m%dT%H%M%SZ)\"",
        "cp .ctxgraph/graph.db \"$PRE_RESTORE_DIR/graph.db\"",
        "cp \"$BACKUP_DIR/graph.db\" .ctxgraph/graph.db",
        "Do not restore over a database while `ctxgraph mcp start`",
    ] {
        assert!(
            guide.contains(required),
            "restore guide is missing required text: {required}"
        );
    }
}

#[test]
fn backup_restore_guide_covers_read_only_verification() {
    let guide = guide_text();

    for required in [
        "Read-only verification",
        "ctxgraph stats",
        "ctxgraph query \"",
        "ctxgraph decisions list --limit 5",
        "These checks read existing data; they do not write new episodes.",
    ] {
        assert!(
            guide.contains(required),
            "verification guide is missing required text: {required}"
        );
    }
}

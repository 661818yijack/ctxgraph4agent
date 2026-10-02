pub mod decisions;
pub mod entities;
pub mod init;
pub mod learn;
pub mod log;
pub mod mcp;
pub mod models;
pub mod query;
pub mod reverify;
pub mod stats;

use std::env;
use std::path::PathBuf;

use ctxgraph::Graph;

/// Find and open the nearest .ctxgraph/graph.db, searching up from cwd.
/// If extraction models are available, loads the extraction pipeline.
pub fn open_graph() -> ctxgraph::Result<Graph> {
    let db_path = find_db()?;
    let mut graph = Graph::open(&db_path)?;

    if let Some(models_dir) = find_models_dir(&db_path) {
        // Look for ctxgraph.toml in the project root next to .ctxgraph/.
        let config_path = config_path_for_db(&db_path);

        let result = if let Some(ref cfg) = config_path {
            if cfg.exists() {
                graph.load_extraction_pipeline_from_config(&models_dir, cfg)
            } else {
                graph.load_extraction_pipeline(&models_dir)
            }
        } else {
            graph.load_extraction_pipeline(&models_dir)
        };

        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!(
                    "ctxgraph: extraction pipeline not loaded: {e}\n\
                     hint: place ONNX model files in {}",
                    models_dir.display()
                );
            }
        }
    }

    Ok(graph)
}

/// Locate models directory by checking (in order):
/// 1. `CTXGRAPH_MODELS_DIR` env var
/// 2. `~/.cache/ctxgraph/models`
/// 3. `.ctxgraph/models` next to the database
fn find_models_dir(db_path: &std::path::Path) -> Option<PathBuf> {
    // 1. Env var override
    if let Ok(val) = env::var("CTXGRAPH_MODELS_DIR") {
        let p = PathBuf::from(val);
        if p.is_dir() {
            return Some(p);
        }
    }

    // 2. ~/.cache/ctxgraph/models
    if let Ok(home) = env::var("HOME") {
        let p = PathBuf::from(home).join(".cache/ctxgraph/models");
        if p.is_dir() {
            return Some(p);
        }
    }

    // 3. .ctxgraph/models relative to the found .ctxgraph dir
    if let Some(ctxgraph_dir) = db_path.parent() {
        let p = ctxgraph_dir.join("models");
        if p.is_dir() {
            return Some(p);
        }
    }

    None
}

/// Resolve the project-root ctxgraph.toml path from a discovered graph database.
///
/// The supported layout is `<project>/.ctxgraph/graph.db`, with config at
/// `<project>/ctxgraph.toml`.
fn config_path_for_db(db_path: &std::path::Path) -> Option<PathBuf> {
    let ctxgraph_dir = db_path.parent()?;
    if ctxgraph_dir.file_name()? != ".ctxgraph" {
        return None;
    }
    let project_root = ctxgraph_dir.parent()?;
    Some(project_root.join("ctxgraph.toml"))
}

fn find_db() -> ctxgraph::Result<PathBuf> {
    let mut dir = env::current_dir().map_err(ctxgraph::CtxGraphError::Io)?;

    loop {
        let candidate = dir.join(".ctxgraph").join("graph.db");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }

    Err(ctxgraph::CtxGraphError::NotFound(
        "no .ctxgraph/ found in current or parent directories. Run `ctxgraph init` first."
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    #[test]
    fn config_path_for_db_points_to_project_root_toml() {
        let db_path = Path::new("/work/project/.ctxgraph/graph.db");

        assert_eq!(
            super::config_path_for_db(db_path),
            Some(PathBuf::from("/work/project/ctxgraph.toml"))
        );
    }

    #[test]
    fn config_path_for_db_rejects_unexpected_db_layout() {
        let db_path = Path::new("/work/project/graph.db");

        assert_eq!(super::config_path_for_db(db_path), None);
    }

    #[test]
    fn example_mentions_supported_project_root_location() {
        let example = include_str!("../../../../ctxgraph.toml.example");

        assert!(
            example.contains("Copy this file to ctxgraph.toml in your project root"),
            "example should document the path that open_graph actually reads"
        );
        assert!(
            !example.contains("Copy this file to .ctxgraph/ctxgraph.toml"),
            "example must not point users at the unsupported .ctxgraph/ctxgraph.toml path"
        );
    }
}

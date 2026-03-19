mod protocol;
mod server;
mod tools;

use std::path::PathBuf;

use ctxgraph::Graph;
use ctxgraph_embed::EmbedEngine;
use server::McpServer;

/// Parse `--db <path>` from argv or fall back to CTXGRAPH_DB env var.
/// Default: `.ctxgraph/graph.db` relative to the current directory.
fn resolve_db_path() -> PathBuf {
    // Check env var first
    if let Ok(val) = std::env::var("CTXGRAPH_DB") {
        return PathBuf::from(val);
    }

    // Parse --db <path> from argv
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--db" {
            if let Some(path) = args.get(i + 1) {
                return PathBuf::from(path);
            }
        }
        i += 1;
    }

    // Default
    PathBuf::from(".ctxgraph/graph.db")
}

#[tokio::main]
async fn main() {
    eprintln!("ctxgraph-mcp v0.3.0 starting on stdio");

    let db_path = resolve_db_path();
    eprintln!("ctxgraph-mcp: using database at {}", db_path.display());

    // Open or init graph
    let graph = if db_path.exists() {
        match Graph::open(&db_path) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("ctxgraph-mcp: failed to open graph: {e}");
                std::process::exit(1);
            }
        }
    } else {
        // Auto-init if database doesn't exist
        let dir = db_path.parent().and_then(|p| p.parent()).unwrap_or(std::path::Path::new("."));
        eprintln!("ctxgraph-mcp: initializing new graph at {}", dir.display());
        match Graph::init(dir) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("ctxgraph-mcp: failed to init graph: {e}");
                std::process::exit(1);
            }
        }
    };

    // Initialize embedding engine (downloads model on first use)
    eprintln!("ctxgraph-mcp: loading embedding model (all-MiniLM-L6-v2)...");
    let embed = match EmbedEngine::new() {
        Ok(e) => e,
        Err(err) => {
            eprintln!("ctxgraph-mcp: failed to load embedding model: {err}");
            std::process::exit(1);
        }
    };
    eprintln!("ctxgraph-mcp: embedding model ready");

    let server = McpServer::new(graph, embed);
    server.run().await;
}

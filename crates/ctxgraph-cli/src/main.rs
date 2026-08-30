mod commands;
mod display;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ctxgraph", about = "Local-first context graph engine")]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize ctxgraph in the current directory
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Log a decision or event
    Log {
        /// The text to log
        text: String,

        /// Source of this information
        #[arg(short, long)]
        source: Option<String>,

        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Search the context graph
    Query {
        /// Search query text
        text: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Only show results after this date (ISO-8601)
        #[arg(long)]
        after: Option<String>,

        /// Filter by source
        #[arg(long)]
        source: Option<String>,
    },

    /// List and show entities
    Entities {
        #[command(subcommand)]
        action: EntitiesAction,
    },

    /// List and show decisions
    Decisions {
        #[command(subcommand)]
        action: DecisionsAction,
    },

    /// Show graph statistics
    Stats,

    /// Run the learning pipeline to extract patterns and create skills
    Learn {
        /// Show what would be learned without persisting
        #[arg(long)]
        dry_run: bool,

        /// Skill scope: private or shared
        #[arg(long, default_value = "private", value_name = "private|shared")]
        scope: String,

        /// Maximum number of skills to create per run
        #[arg(long, default_value = "50")]
        limit: usize,

        /// Agent name (for scope tracking)
        #[arg(long, default_value = "assistant")]
        agent: String,

        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Manage ONNX models
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },

    /// List and manage stale memories for re-verification
    Reverify {
        #[command(subcommand)]
        action: ReverifyAction,
    },

    /// Run the MCP server (JSON-RPC over stdio)
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Start the MCP server on stdio
    Start {
        /// Path to the graph database (overrides CTXGRAPH_DB env var)
        #[arg(long)]
        db: Option<String>,
    },
}

#[derive(Subcommand)]
enum ModelsAction {
    /// Download ONNX models required for extraction
    Download,
}

#[derive(Subcommand)]
enum ReverifyAction {
    /// List stale memories for re-verification
    List {
        /// Decay score threshold (memories below this are shown)
        #[arg(short, long, default_value = "0.7")]
        threshold: f64,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: usize,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Output format: text or json
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Renew a specific memory (reset TTL)
    Renew {
        /// Memory ID
        id: String,

        /// Memory type (fact, experience, preference, decision)
        #[arg(long)]
        memory_type: String,
    },

    /// Update a memory's content and/or type
    Update {
        /// Memory ID
        id: String,

        /// New content
        #[arg(long)]
        content: Option<String>,

        /// New memory type
        #[arg(long)]
        memory_type: Option<String>,
    },

    /// Immediately delete a memory
    Expire {
        /// Memory ID
        id: String,
    },
}

#[derive(Subcommand)]
enum EntitiesAction {
    /// List all entities
    List {
        /// Filter by entity type
        #[arg(short = 't', long = "type")]
        entity_type: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Show details for a specific entity
    Show {
        /// Entity ID or name
        id: String,
    },
}

#[derive(Subcommand)]
enum DecisionsAction {
    /// List all decisions
    List {
        /// Only show decisions after this date
        #[arg(long)]
        after: Option<String>,

        /// Filter by source
        #[arg(long)]
        source: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show full decision trace
    Show {
        /// Decision/episode ID
        id: String,
    },
}

fn parse_learn_scope(scope: &str) -> ctxgraph::Result<ctxgraph::SkillScope> {
    match scope {
        "private" => Ok(ctxgraph::SkillScope::Private),
        "shared" => Ok(ctxgraph::SkillScope::Shared),
        other => Err(ctxgraph::CtxGraphError::InvalidInput(format!(
            "invalid learn scope '{other}'; expected 'private' or 'shared'"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_learn_scope;

    #[test]
    fn learn_scope_accepts_documented_values() {
        assert_eq!(
            parse_learn_scope("private").unwrap(),
            ctxgraph::SkillScope::Private
        );
        assert_eq!(
            parse_learn_scope("shared").unwrap(),
            ctxgraph::SkillScope::Shared
        );
    }

    #[test]
    fn learn_scope_rejects_unknown_values() {
        let error = parse_learn_scope("sharedd").unwrap_err();

        match error {
            ctxgraph::CtxGraphError::InvalidInput(message) => {
                assert!(message.contains("invalid learn scope 'sharedd'"));
                assert!(message.contains("private"));
                assert!(message.contains("shared"));
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { name } => commands::init::run(name),
        Commands::Log { text, source, tags } => commands::log::run(text, source, tags).await,
        Commands::Query {
            text,
            limit,
            after,
            source,
        } => commands::query::run(text, limit, after, source),
        Commands::Entities { action } => match action {
            EntitiesAction::List { entity_type, limit } => {
                commands::entities::list(entity_type, limit)
            }
            EntitiesAction::Show { id } => commands::entities::show(id),
        },
        Commands::Decisions { action } => match action {
            DecisionsAction::List {
                after,
                source,
                limit,
            } => commands::decisions::list(after, source, limit),
            DecisionsAction::Show { id } => commands::decisions::show(id),
        },
        Commands::Stats => commands::stats::run(),
        Commands::Learn {
            dry_run,
            scope,
            limit,
            agent,
            format,
        } => match parse_learn_scope(&scope) {
            Ok(scope) => {
                commands::learn::run(commands::learn::LearnOptions {
                    dry_run,
                    scope,
                    limit,
                    agent,
                    format,
                })
                .await
            }
            Err(error) => Err(error),
        },
        Commands::Models { action } => match action {
            ModelsAction::Download => commands::models::download(),
        },
        Commands::Reverify { action } => match action {
            ReverifyAction::List {
                threshold,
                limit,
                offset,
                format,
            } => commands::reverify::list(commands::reverify::ReverifyListOptions {
                threshold,
                limit,
                offset,
                format,
            }),
            ReverifyAction::Renew { id, memory_type } => {
                commands::reverify::renew(commands::reverify::ReverifyRenewOptions {
                    id,
                    memory_type,
                })
            }
            ReverifyAction::Update {
                id,
                content,
                memory_type,
            } => commands::reverify::update(commands::reverify::ReverifyUpdateOptions {
                id,
                content,
                memory_type,
            }),
            ReverifyAction::Expire { id } => {
                commands::reverify::expire(commands::reverify::ReverifyExpireOptions { id })
            }
        },
        Commands::Mcp { action } => match action {
            McpAction::Start { db } => commands::mcp::start(db).await,
        },
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

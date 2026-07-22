//! Core SDK for creating, writing, and querying a local ctxgraph database.
//!
//! ```rust
//! use ctxgraph::{Episode, Graph};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let project = tempfile::tempdir()?;
//!     let graph = Graph::init(project.path())?;
//!
//!     graph
//!         .add_episode(Episode::builder("Chose Postgres for billing").build())
//!         .await?;
//!
//!     let results = graph.search("Postgres", 10, None)?;
//!     assert_eq!(results.len(), 1);
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod graph;
pub mod pattern;
pub mod skill;
pub mod storage;
pub mod types;

pub use error::{CtxGraphError, Result};
pub use graph::Graph;
pub use pattern::{
    BatchLabelDescriber, FailingBatchLabelDescriber, MockBatchLabelDescriber, PatternExtractor,
};
pub use skill::SkillCreator;
pub use types::*;

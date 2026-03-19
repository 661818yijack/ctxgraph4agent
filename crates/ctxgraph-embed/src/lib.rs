mod encoder;
pub use encoder::EmbedEngine;

#[derive(Debug, thiserror::Error)]
pub enum EmbedError {
    #[error("model init failed: {0}")]
    ModelInit(String),
    #[error("encoding failed: {0}")]
    Encoding(String),
}

#[derive(Debug, thiserror::Error)]
pub enum HarvestError {
    #[error("source '{source_id}' failed: {reason}")]
    SourceFailed { source_id: String, reason: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error in '{path}': {reason}")]
    Parse { path: String, reason: String },
}

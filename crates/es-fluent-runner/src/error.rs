#[derive(Debug, thiserror::Error)]
pub enum RunnerIoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid runner request: {0}")]
    InvalidRunnerRequest(String),
    #[error("conflicting transaction plans target the same path: {path}")]
    TransactionConflict { path: std::path::PathBuf },
    #[error("filesystem path changed after the transaction was planned: {path}")]
    TransactionChanged { path: std::path::PathBuf },
    #[error("transaction commit failed and was rolled back: {0}")]
    TransactionCommit(String),
    #[error(
        "transaction commit failed ({commit_error}) and rollback also failed ({rollback_error})"
    )]
    TransactionRollback {
        commit_error: String,
        rollback_error: String,
    },
    #[error("invalid inventory source package '{source_package}' for {source_type}: {reason}")]
    InvalidInventorySourcePackage {
        source_package: String,
        source_type: String,
        reason: String,
    },
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RunnerIoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid runner request: {0}")]
    InvalidRunnerRequest(String),
    #[error("{0}")]
    Message(String),
}

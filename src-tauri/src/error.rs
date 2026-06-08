use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum FuseError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("metadata error: {0}")]
    Metadata(String),
    #[error("playback error: {0}")]
    Playback(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("app path error: {0}")]
    AppPath(String),
    #[error("state lock failed")]
    Lock,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

impl From<FuseError> for CommandError {
    fn from(value: FuseError) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

pub type FuseResult<T> = Result<T, FuseError>;
pub type CommandResult<T> = Result<T, CommandError>;

use thiserror::Error;
use std::error::Error as StdError;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    #[error("Failed to parse secrets JSON: {0}")]
    SecretsParseFailed(#[from] serde_json::Error),

    #[error("GitHub API error: {0}")]
    GitHubApiError(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    #[error("Sodium error: {0}")]
    SodiumError(String),

    #[error("Unknown error occurred: {0}")]
    Unknown(String),
}

impl From<Box<dyn StdError>> for AppError {
    fn from(error: Box<dyn StdError>) -> Self {
        AppError::Unknown(error.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
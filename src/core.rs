use serde::{Deserialize, Serialize};
use std::error::Error as StdError;
use thiserror::Error;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub enum SecretStatus {
    New,
    Existing,
    Deleted,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Secret {
    pub name: String,
    pub value: String,
    #[serde(skip_deserializing)]
    pub status: Option<SecretStatus>,
}

#[derive(Clone)]
pub struct SecretDetails {
    pub name: String,
    pub value: String,
    pub created_at: String,
    pub updated_at: String,
    pub status: SecretStatus,
}

pub trait SecretsManager {
    fn get_secrets(&self) -> &Vec<Secret>;
    fn get_secret_details(&self, index: usize) -> Option<SecretDetails>;
    fn manage_secrets(&self) -> Result<(), AppError>;
}

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
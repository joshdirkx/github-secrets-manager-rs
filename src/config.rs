use crate::core::{AppError, AppResult, Secret};
use dotenv::dotenv;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub organization: String,
    pub repository: String,
    pub token: String,
    pub secrets: Vec<Secret>,
}

impl Config {
    pub fn load() -> AppResult<Self> {
        dotenv().ok();

        let organization = env::var("GITHUB_ORGANIZATION")
            .map_err(|_| AppError::EnvVarNotFound("GITHUB_ORGANIZATION".to_string()))?;
        let repository = env::var("GITHUB_REPOSITORY")
            .map_err(|_| AppError::EnvVarNotFound("GITHUB_REPOSITORY".to_string()))?;
        let token = env::var("GITHUB_TOKEN")
            .map_err(|_| AppError::EnvVarNotFound("GITHUB_TOKEN".to_string()))?;
        let secrets_json = env::var("GITHUB_SECRETS")
            .map_err(|_| AppError::EnvVarNotFound("GITHUB_SECRETS".to_string()))?;

        let secrets: Vec<Secret> = serde_json::from_str(&secrets_json)?;

        Ok(Config {
            organization,
            repository,
            token,
            secrets,
        })
    }
}
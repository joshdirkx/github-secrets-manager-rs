mod github_client;
mod secrets_manager;

use dotenv::dotenv;
use github_client::GitHubClient;
use secrets_manager::{Secret, SecretsManager};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let organization = env::var("GITHUB_ORGANIZATION")?;
    let repository = env::var("GITHUB_REPOSITORY")?;
    let token = env::var("GITHUB_TOKEN")?;
    let secrets_json = env::var("GITHUB_SECRETS")?;

    let secrets: Vec<Secret> = serde_json::from_str(&secrets_json)?;
    let client = GitHubClient::new(&organization, &repository, &token);

    let public_key = client.get_public_key().await?;
    let existing_secrets = client.get_existing_secrets().await?;

    let secrets_manager = SecretsManager::new(secrets, existing_secrets, public_key, &client);
    secrets_manager.manage_secrets().await?;

    Ok(())
}

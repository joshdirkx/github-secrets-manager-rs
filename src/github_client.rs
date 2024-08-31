use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Deserialize)]
pub struct PublicKeyResponse {
    pub key_id: String,
    pub key: String,
}

#[derive(Deserialize)]
pub struct ExistingSecret {
    pub name: String,
}

#[derive(Deserialize)]
struct SecretListResponse {
    secrets: Vec<ExistingSecret>,
}

#[derive(Serialize)]
struct UpdateSecretRequest {
    encrypted_value: String,
    key_id: String,
}

pub struct GitHubClient {
    client: reqwest::Client,
    organization: String,
    repository: String,
    token: String,
}

impl GitHubClient {
    pub fn new(organization: &str, repository: &str, token: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            organization: organization.to_string(),
            repository: repository.to_string(),
            token: token.to_string(),
        }
    }

    pub async fn get_public_key(&self) -> Result<PublicKeyResponse, Box<dyn Error>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets/public-key",
            self.organization, self.repository
        );

        let response = self
            .client
            .get(&url)
            .header(USER_AGENT, "github-secrets-manager")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .send()
            .await?;

        if response.status().is_success() {
            let public_key = response.json::<PublicKeyResponse>().await?;
            Ok(public_key)
        } else {
            Err(Box::new(response.error_for_status().unwrap_err()))
        }
    }

    pub async fn get_existing_secrets(&self) -> Result<Vec<ExistingSecret>, Box<dyn Error>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets",
            self.organization, self.repository
        );

        let response = self
            .client
            .get(&url)
            .header(USER_AGENT, "github-secrets-manager")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .send()
            .await?;

        if response.status().is_success() {
            let secret_list = response.json::<SecretListResponse>().await?;
            Ok(secret_list.secrets)
        } else {
            Err(Box::new(response.error_for_status().unwrap_err()))
        }
    }

    pub async fn upsert_secret(
        &self,
        secret_name: &str,
        encrypted_value: String,
        key_id: String,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets/{}",
            self.organization, self.repository, secret_name
        );

        let update_secret_req = UpdateSecretRequest { encrypted_value, key_id };

        let response = self
            .client
            .put(&url)
            .header(USER_AGENT, "github-secrets-manager")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .header(CONTENT_TYPE, "application/json")
            .json(&update_secret_req)
            .send()
            .await?;

        if response.status().is_success() {
            println!("Secret '{}' updated successfully!", secret_name);
            Ok(())
        } else {
            Err(Box::new(response.error_for_status().unwrap_err()))
        }
    }

    pub async fn delete_secret(&self, secret_name: &str) -> Result<(), Box<dyn Error>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets/{}",
            self.organization, self.repository, secret_name
        );

        let response = self
            .client
            .delete(&url)
            .header(USER_AGENT, "github-secrets-manager")
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .send()
            .await?;

        if response.status().is_success() {
            println!("Secret '{}' deleted successfully!", secret_name);
            Ok(())
        } else {
            Err(Box::new(response.error_for_status().unwrap_err()))
        }
    }
}

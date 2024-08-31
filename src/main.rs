use base64::engine::general_purpose;
use base64::Engine;
use dotenv::{dotenv};
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::{box_, sealedbox};
use std::env;

#[derive(Deserialize)]
struct Secret {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct PublicKeyResponse {
    key_id: String,
    key: String,
}

#[derive(Serialize)]
struct UpdateSecretRequest {
    encrypted_value: String,
    key_id: String,
}

#[derive(Deserialize)]
struct SecretListResponse {
    secrets: Vec<ExistingSecret>,
}

#[derive(Deserialize)]
struct ExistingSecret {
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let organization = env::var("GITHUB_ORGANIZATION")
        .expect("GITHUB_ORGANIZATION environment variable is not set");
    let repository =
        env::var("GITHUB_REPOSITORY").expect("GITHUB_REPOSITORY environment variable is not set");
    let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN environment variable is not set");
    let secrets_json =
        env::var("GITHUB_SECRETS").expect("GITHUB_SECRETS environment variable is not set");

    let secrets: Vec<Secret> = serde_json::from_str(&secrets_json)?;

    let client = reqwest::Client::new();
    let public_key_url = format!(
        "https://api.github.com/repos/{}/{}/actions/secrets/public-key",
        organization, repository
    );

    let public_key_response = client
        .get(&public_key_url)
        .header(USER_AGENT, "github-secrets-manager")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await?;

    let public_key: PublicKeyResponse;

    if public_key_response.status().is_success() {
        let public_key_text = public_key_response.text().await?;
        println!("Retrieved public key");

        public_key = serde_json::from_str::<PublicKeyResponse>(&public_key_text)?;
    } else {
        println!(
            "Failed to fetch public key: {}",
            public_key_response.status()
        );
        let error_text = public_key_response.text().await?;
        panic!("Error response: {}", error_text);
    }

    let public_key_bytes = general_purpose::STANDARD.decode(public_key.key)?;
    let pk = box_::PublicKey::from_slice(&public_key_bytes).unwrap();

    // Fetch the existing secrets from GitHub
    let secrets_list_url = format!(
        "https://api.github.com/repos/{}/{}/actions/secrets",
        organization, repository
    );

    let secrets_list_response = client
        .get(&secrets_list_url)
        .header(USER_AGENT, "github-secrets-manager")
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .send()
        .await?;

    let existing_secrets: Vec<ExistingSecret> = if secrets_list_response.status().is_success() {
        let secrets_list_text = secrets_list_response.text().await?;
        let secrets_list: SecretListResponse = serde_json::from_str(&secrets_list_text)?;
        secrets_list.secrets
    } else {
        println!(
            "Failed to fetch secrets list: {}",
            secrets_list_response.status()
        );
        let error_text = secrets_list_response.text().await?;
        panic!("Error response: {}", error_text);
    };

    // Track new, updated, and deleted secrets
    let mut new_secrets = Vec::new();
    let mut updated_secrets = Vec::new();
    let mut secrets_to_delete = Vec::new();

    for secret in &secrets {
        match existing_secrets.iter().find(|s| s.name == secret.name) {
            Some(_) => updated_secrets.push(secret),
            None => new_secrets.push(secret),
        }
    }

    for existing_secret in &existing_secrets {
        if !secrets.iter().any(|s| s.name == existing_secret.name) {
            secrets_to_delete.push(&existing_secret.name);
        }
    }

    // Output the results
    if !new_secrets.is_empty() {
        println!("New secrets to be added:");
        for secret in &new_secrets {
            println!("- {}", secret.name);
        }
    }

    if !updated_secrets.is_empty() {
        println!("Existing secrets to be updated:");
        for secret in &updated_secrets {
            println!("- {}", secret.name);
        }
    }

    if !secrets_to_delete.is_empty() {
        println!("Secrets to be deleted:");
        for secret_name in &secrets_to_delete {
            println!("- {}", secret_name);
        }
    }

    // Update or add secrets
    for secret in new_secrets.iter().chain(updated_secrets.iter()) {
        let sealed_box = sealedbox::seal(secret.value.as_bytes(), &pk);
        let encrypted_value = general_purpose::STANDARD.encode(&sealed_box);

        let update_secret_url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets/{}",
            organization, repository, secret.name
        );

        let update_secret_req = UpdateSecretRequest {
            encrypted_value,
            key_id: public_key.key_id.clone(),
        };

        let update_resp = client
            .put(&update_secret_url)
            .header(USER_AGENT, "github-secrets-manager")
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .header(CONTENT_TYPE, "application/json")
            .json(&update_secret_req)
            .send()
            .await?;

        if update_resp.status().is_success() {
            if new_secrets.iter().any(|s| s.name == secret.name) {
                println!("Secret '{}' created successfully!", secret.name);
            } else {
                println!("Secret '{}' updated successfully!", secret.name);
            }
        } else {
            println!(
                "Failed to update secret '{}' with status {} and error {:?}",
                secret.name,
                update_resp.status(),
                update_resp.text().await?
            );
        }
    }

    // Delete secrets that are no longer in the JSON
    for secret_name in secrets_to_delete {
        let delete_secret_url = format!(
            "https://api.github.com/repos/{}/{}/actions/secrets/{}",
            organization, repository, secret_name
        );

        let delete_resp = client
            .delete(&delete_secret_url)
            .header(USER_AGENT, "github-secrets-manager")
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .header(ACCEPT, "application/vnd.github.v3+json")
            .send()
            .await?;

        if delete_resp.status().is_success() {
            println!("Secret '{}' deleted successfully!", secret_name);
        } else {
            println!(
                "Failed to delete secret '{}' with status {} and error {:?}",
                secret_name,
                delete_resp.status(),
                delete_resp.text().await?
            );
        }
    }

    Ok(())
}

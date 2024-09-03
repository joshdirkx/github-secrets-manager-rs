use base64::engine::general_purpose;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::{box_, sealedbox};

use crate::github_client::{ExistingSecret, GitHubClient, PublicKeyResponse};
use std::error::Error;

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

pub struct SecretsManager<'a> {
    secrets: Vec<Secret>,
    existing_secrets: Vec<ExistingSecret>,
    public_key: PublicKeyResponse,
    client: &'a GitHubClient,
}

impl<'a> SecretsManager<'a> {
    pub fn new(
        mut secrets: Vec<Secret>,
        existing_secrets: Vec<ExistingSecret>,
        public_key: PublicKeyResponse,
        client: &'a GitHubClient,
    ) -> Self {
        let mut manager = Self {
            secrets,
            existing_secrets,
            public_key,
            client,
        };
        manager.update_secret_statuses();
        manager
    }

    fn update_secret_statuses(&mut self) {
        // Update statuses for secrets in our list
        for secret in &mut self.secrets {
            if self.existing_secrets.iter().any(|s| s.name == secret.name) {
                secret.status = Some(SecretStatus::Existing);
            } else {
                secret.status = Some(SecretStatus::New);
            }
        }

        // Add deleted secrets
        for existing in &self.existing_secrets {
            if !self.secrets.iter().any(|s| s.name == existing.name) {
                self.secrets.push(Secret {
                    name: existing.name.clone(),
                    value: String::new(), // We don't know the value
                    status: Some(SecretStatus::Deleted),
                });
            }
        }
    }

    pub fn get_secrets(&self) -> &Vec<Secret> {
        &self.secrets
    }

    pub fn get_secret_details(&self, index: usize) -> Option<SecretDetails> {
        self.secrets.get(index).map(|secret| {
            let existing = self.existing_secrets.iter().find(|s| s.name == secret.name);
            SecretDetails {
                name: secret.name.clone(),
                value: if secret.status == Some(SecretStatus::Deleted) {
                    "Unknown (Deleted)".to_string()
                } else {
                    secret.value.clone()
                },
                created_at: existing.map_or_else(|| "N/A".to_string(), |s| s.created_at.clone()),
                updated_at: existing.map_or_else(|| "N/A".to_string(), |s| s.updated_at.clone()),
                status: secret.status.clone().expect("Secret status is missing"),
            }
        })
    }

    pub async fn manage_secrets(&self) -> Result<(), Box<dyn Error>> {
        let pk = self.decode_public_key()?;

        let (new_secrets, updated_secrets, secrets_to_delete) = self.categorize_secrets();

        self.print_secrets_to_manage(&new_secrets, &updated_secrets, &secrets_to_delete);

        self.upsert_secrets(&pk, &new_secrets, &updated_secrets).await?;
        self.delete_secrets(secrets_to_delete).await?;

        Ok(())
    }

    fn decode_public_key(&self) -> Result<box_::PublicKey, Box<dyn Error>> {
        let public_key_bytes = general_purpose::STANDARD.decode(&self.public_key.key)?;
        let pk = box_::PublicKey::from_slice(&public_key_bytes).unwrap();
        Ok(pk)
    }

    fn categorize_secrets(
        &self,
    ) -> (
        Vec<&Secret>,
        Vec<&Secret>,
        Vec<&String>,
    ) {
        let mut new_secrets = Vec::new();
        let mut updated_secrets = Vec::new();
        let mut secrets_to_delete = Vec::new();

        for secret in &self.secrets {
            match secret.status {
                Some(SecretStatus::New) => new_secrets.push(secret),
                Some(SecretStatus::Existing) => updated_secrets.push(secret),
                Some(SecretStatus::Deleted) => secrets_to_delete.push(&secret.name),
                _ => {}
            }
        }

        (new_secrets, updated_secrets, secrets_to_delete)
    }

    fn print_secrets_to_manage(
        &self,
        new_secrets: &Vec<&Secret>,
        updated_secrets: &Vec<&Secret>,
        secrets_to_delete: &Vec<&String>,
    ) {
        if !new_secrets.is_empty() {
            println!("New secrets to be added:");
            for secret in new_secrets {
                println!("- {}", secret.name);
            }
        }

        if !updated_secrets.is_empty() {
            println!("Existing secrets to be updated:");
            for secret in updated_secrets {
                println!("- {}", secret.name);
            }
        }

        if !secrets_to_delete.is_empty() {
            println!("Secrets to be deleted:");
            for secret_name in secrets_to_delete {
                println!("- {}", secret_name);
            }
        }
    }

    async fn upsert_secrets(
        &self,
        pk: &box_::PublicKey,
        new_secrets: &Vec<&Secret>,
        updated_secrets: &Vec<&Secret>,
    ) -> Result<(), Box<dyn Error>> {
        for secret in new_secrets.iter().chain(updated_secrets.iter()) {
            let sealed_box = sealedbox::seal(secret.value.as_bytes(), &pk);
            let encrypted_value = general_purpose::STANDARD.encode(&sealed_box);

            self.client
                .upsert_secret(&secret.name, encrypted_value, self.public_key.key_id.clone())
                .await?;
        }

        Ok(())
    }

    async fn delete_secrets(&self, secrets_to_delete: Vec<&String>) -> Result<(), Box<dyn Error>> {
        for secret_name in secrets_to_delete {
            self.client.delete_secret(secret_name).await?;
        }

        Ok(())
    }
}
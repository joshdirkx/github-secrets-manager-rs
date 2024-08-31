use base64::engine::general_purpose;
use base64::Engine;
use serde::Deserialize;
use sodiumoxide::crypto::{box_, sealedbox};

use crate::github_client::{ExistingSecret, GitHubClient, PublicKeyResponse};
use std::error::Error;

#[derive(Deserialize)]
pub struct Secret {
    pub name: String,
    pub value: String,
}

pub struct SecretsManager<'a> {
    secrets: Vec<Secret>,
    existing_secrets: Vec<ExistingSecret>,
    public_key: PublicKeyResponse,
    client: &'a GitHubClient,
}

impl<'a> SecretsManager<'a> {
    pub fn new(
        secrets: Vec<Secret>,
        existing_secrets: Vec<ExistingSecret>,
        public_key: PublicKeyResponse,
        client: &'a GitHubClient,
    ) -> Self {
        Self {
            secrets,
            existing_secrets,
            public_key,
            client,
        }
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
            match self
                .existing_secrets
                .iter()
                .find(|s| s.name == secret.name)
            {
                Some(_) => updated_secrets.push(secret),
                None => new_secrets.push(secret),
            }
        }

        for existing_secret in &self.existing_secrets {
            if !self
                .secrets
                .iter()
                .any(|s| s.name == existing_secret.name)
            {
                secrets_to_delete.push(&existing_secret.name);
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

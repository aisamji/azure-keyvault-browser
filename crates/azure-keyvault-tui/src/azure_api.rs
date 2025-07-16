// An internal module since there is no official SDK to get the list of ALL keyvaults
use anyhow::{Result, anyhow};
use azure_core::credentials::TokenCredential;
use azure_identity::{AzureCliCredential, AzureCliCredentialOptions};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyVault {
    pub id: String,
    pub name: String,
    pub _location: String,
    #[serde(rename = "type")]
    pub _resource_type: String,
    pub properties: KeyVaultProperties,
}

impl KeyVault {
    /// Extracts the resource group name from the Azure resource ID.
    ///
    /// Azure resource IDs follow the format:
    /// `/subscriptions/{subscription}/resourceGroups/{resourceGroup}/providers/Microsoft.KeyVault/vaults/{name}`
    pub fn resource_group(&self) -> &str {
        self.id
            .split('/')
            .collect::<Vec<&str>>()
            .get(4)
            .unwrap_or(&"Unknown")
    }

    /// Returns the vault URL for use with Azure Key Vault secrets client.
    pub fn vault_url(&self) -> &str {
        &self.properties.vault_uri
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyVaultProperties {
    pub vault_uri: String,
    pub _tenant_id: String,
    pub _sku: KeyVaultSku,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyVaultSku {
    pub _family: String,
    pub _name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyVaultListResponse {
    pub value: Vec<KeyVault>,
    #[serde(rename = "nextLink")]
    pub _next_link: Option<String>,
}

pub async fn list_key_vaults(subscription_id: &str, access_token: &str) -> Result<Vec<KeyVault>> {
    let url = format!(
        "https://management.azure.com/subscriptions/{}/providers/Microsoft.KeyVault/vaults?api-version=2022-07-01",
        subscription_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to list key vaults: HTTP {}",
            response.status()
        ));
    }

    let list_response: KeyVaultListResponse = response.json().await?;
    Ok(list_response.value)
}

pub async fn get_access_token_for_subscription(subscription_id: &str) -> Result<String> {
    let credential = AzureCliCredential::new(Some(AzureCliCredentialOptions {
        subscription: Some(subscription_id.to_string()),
        ..Default::default()
    }))?;
    let scopes = ["https://management.azure.com/.default"];

    let token_response = credential.get_token(&scopes, None).await?;

    Ok(token_response.token.secret().to_string())
}

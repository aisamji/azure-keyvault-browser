use anyhow::{Result, anyhow};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyVault {
    pub id: String,
    pub name: String,
    pub location: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub properties: KeyVaultProperties,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyVaultProperties {
    pub vault_uri: String,
    pub tenant_id: String,
    pub sku: KeyVaultSku,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KeyVaultSku {
    pub family: String,
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KeyVaultListResponse {
    pub value: Vec<KeyVault>,
    #[serde(rename = "nextLink")]
    pub next_link: Option<String>,
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
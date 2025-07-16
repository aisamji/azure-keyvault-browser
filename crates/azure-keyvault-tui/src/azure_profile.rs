use std::{fmt::Display, path::PathBuf, process::Command, str::FromStr};

use anyhow::{Result, anyhow};
use dirs::home_dir;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AzureProfile {
    pub installation_id: String,
    pub subscriptions: Vec<AzureSubscription>,
}

impl AzureProfile {
    /// Tries to read an AzureProfile object from the azure cli's config file.
    ///
    /// The file should be in JSON format. Using UTF-8.
    pub fn try_from_config() -> Result<Self> {
        let profile_path = home_dir()
            .ok_or(anyhow!("Could not find home directory"))?
            .join(PathBuf::from_str(".azure/azureProfile.json")?);

        AzureProfile::try_from_file(profile_path.to_str().ok_or(anyhow!(
            "{:?} resulted in an invalid filename.",
            profile_path
        ))?)
    }

    /// Tries to read an AzureProfile object from the given file.
    ///
    /// The file should be in JSON format. Using UTF-8.
    pub fn try_from_file(filepath: &str) -> Result<Self> {
        let bytes = std::fs::read(filepath)?;
        let raw_json = bytes
            .iter()
            .filter_map(|x| {
                // Ignore undisplayable bytes, otherwise convert them into the equivalent char.
                if x.is_ascii_alphanumeric() || x.is_ascii_punctuation() {
                    Some((*x as char).into())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .concat();

        Ok(serde_json::from_str(raw_json.as_str())?)
    }

    /// Gets the Azure CLI version.
    ///
    /// Returns an error if Azure CLI is not installed or not accessible.
    pub fn get_azure_cli_version() -> Result<String> {
        let output = Command::new("az")
            .arg("version")
            .arg("--output")
            .arg("json")
            .output()
            .map_err(|e| anyhow!("Failed to execute 'az version --output json'. Please ensure Azure CLI is installed and accessible in PATH. Error: {}", e))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Azure CLI command failed. Please ensure Azure CLI is properly installed."
            ));
        }

        let version_output = String::from_utf8(output.stdout)
            .map_err(|e| anyhow!("Failed to parse Azure CLI version output: {}", e))?;

        let version_info: serde_json::Value = serde_json::from_str(&version_output)
            .map_err(|e| anyhow!("Failed to parse Azure CLI version JSON: {}", e))?;

        let version = version_info["azure-cli"]
            .as_str()
            .ok_or_else(|| anyhow!("Could not find 'azure-cli' version in response"))?;

        Ok(version.to_string())
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AzureSubscription {
    pub id: String,
    pub name: String,
    pub _state: String,
    pub user: AzureCredential,
    pub is_default: bool,
    pub tenant_id: String,
    pub tenant_display_name: Option<String>,
    pub _tenant_default_domain: Option<String>,
    pub _environment_name: String,
}

impl AzureSubscription {
    /// Returns the tenant display name with fallback to tenant ID.
    ///
    /// Tries tenant_display_name first, then falls back to tenant_id.
    pub fn tenant_display_name(&self) -> &str {
        self.tenant_display_name
            .as_ref()
            .filter(|name| !name.is_empty())
            .unwrap_or(&self.tenant_id)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AzureCredential {
    User { name: String },
}

impl Display for AzureCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AzureCredential::User { name } => write!(f, "User ({})", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_file() -> Result<()> {
        AzureProfile::try_from_file("tests/azureProfile.json")
            .inspect_err(|e| eprintln!("{}", e))?;
        Ok(())
    }
}

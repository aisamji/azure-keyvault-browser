use std::{path::PathBuf, str::FromStr};

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
                    Some((x.clone() as char).into())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
            .concat();

        Ok(serde_json::from_str(raw_json.as_str())?)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct AzureSubscription {
    pub id: String,
    pub name: String,
    pub state: String,
    pub user: AzureCredential,
    pub is_default: bool,
    pub tenant_id: String,
    pub environment_name: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum AzureCredential {
    User { name: String },
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

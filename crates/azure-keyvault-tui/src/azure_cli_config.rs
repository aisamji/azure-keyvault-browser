use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AzureProfile {
    installation_id: String,
    subscriptions: Vec<AzureSubscription>,
}

impl AzureProfile {
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AzureSubscription {
    id: String,
    name: String,
    state: String,
    user: AzureCredential,
    is_default: bool,
    tenant_id: String,
    environment_name: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AzureCredential {
    User { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Mark test only available for those with AZ CLI installed.
    // TODO: Modify to work on any computer.
    #[test]
    fn test_from_file() -> Result<()> {
        AzureProfile::try_from_file("/home/alisamji/.azure/azureProfile.json")
            .inspect_err(|e| eprintln!("{}", e))?;
        Ok(())
    }

    // TODO: Add test for parsing directly from string
}

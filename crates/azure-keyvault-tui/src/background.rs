use async_spawn_macros::{TaskSpec, background_task};
use azure_identity::{AzureCliCredential, AzureCliCredentialOptions};
use azure_security_keyvault_secrets::SecretClient;
use futures::StreamExt;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    azure_api::{get_access_token_for_subscription, list_key_vaults},
    tui::TuiEvent,
};

/// Represents different types of background tasks that can be launched.
///
/// Each [`TaskSpec`] contains the necessary parameters and other information to be able to call
/// the function associated with the specified background task.
#[derive(TaskSpec)]
#[taskspec(message_type = "TuiEvent")]
pub enum BackgroundTask {
    /// Lists Key Vaults for the given subscription ID.
    #[taskspec(callback = "load_key_vaults")]
    LoadKeyVaults { subscription_id: String },
    /// Lists Secrets for the given key vault URL.
    #[taskspec(callback = "load_secrets")]
    LoadSecrets { key_vault_url: String, subscription_id: String },
}

/// Launches requested tasks in the background and waits for tasks to finish before exiting.
///
/// Continously launches new tokio tasks based on the [`TaskSpec`] receieved from the given
/// [`Receiver`]. Passes a clone of the given [`Sender`] to the launched background tasks so the
/// background tasks can send messages to the TUI thread to request state modifications.
pub async fn manager(mut rx_bg_task: Receiver<BackgroundTask>, tx_tui_event: Sender<TuiEvent>) {
    let mut spawned_tasks = vec![];
    // Stay alive only while main thread is alive
    while let Some(task_spec) = rx_bg_task.recv().await {
        // Spawn a new task or thread for new BackgroundTasks and add them to the Vec to keep track
        // of them.
        // TODO: There might be a better way to keep track of them.
        if let Some(handle) = task_spec.spawn_task(&tx_tui_event) {
            spawned_tasks.push(handle);
        }
    }

    // Wait for all background tasks to finish.
    for handle in spawned_tasks {
        let _ = handle.await.inspect_err(|e| {
            // Only use eprintln when TUI is shutting down
            eprintln!("Background task error during shutdown: {:?}", e);
        });
    }
}

/// Lists Key Vaults for the given subscription and sends the result to the TUI.
#[background_task(message_type = "TuiEvent", abort_with = "TuiEvent::SetErrorStatus")]
async fn load_key_vaults(subscription_id: String) {
    // Show loading status
    notify!(TuiEvent::SetSuccessStatus(
        "Loading Key Vaults...".to_string(),
    ));

    // Get access token for the subscription
    let access_token = match get_access_token_for_subscription(&subscription_id).await {
        Ok(token) => token,
        Err(e) => {
            abort!("Failed to get access token: {}", e);
        }
    };

    // List key vaults using the token
    match list_key_vaults(&subscription_id, &access_token).await {
        Ok(key_vaults) => notify!(TuiEvent::KeyVaultsLoaded(key_vaults)),
        Err(e) => abort!("Failed to load Key Vaults: {}", e),
    }
}

/// Lists Secrets for the given key vault and sends the result to the TUI.
#[background_task(message_type = "TuiEvent", abort_with = "TuiEvent::SetErrorStatus")]
async fn load_secrets(key_vault_url: String, subscription_id: String) {
    // Show loading status
    notify!(TuiEvent::SetSuccessStatus(
        "Loading Secrets...".to_string(),
    ));


    // Create Azure CLI credential with the subscription
    let credential = match AzureCliCredential::new(Some(AzureCliCredentialOptions {
        subscription: Some(subscription_id),
        ..Default::default()
    })) {
        Ok(cred) => cred,
        Err(e) => {
            abort!("Failed to create Azure CLI credential: {}", e);
        }
    };

    // Create the secrets client
    let client = match SecretClient::new(&key_vault_url, credential, None) {
        Ok(client) => client,
        Err(e) => {
            abort!("Failed to create SecretClient: {}", e);
        }
    };

    // List secret properties
    match client.list_secret_properties(None) {
        Ok(mut iter) => {
            let mut secrets = Vec::new();
            loop {
                match iter.next().await {
                    Some(Ok(secret_property)) => {
                        secrets.push(secret_property);
                    }
                    Some(Err(e)) => {
                        abort!("Failed to load secrets: {}", e);
                    }
                    None => break,
                }
            }
            notify!(TuiEvent::SecretsLoaded(secrets));
        }
        Err(e) => {
            abort!("Failed to create secrets iterator: {}", e);
        }
    }
}

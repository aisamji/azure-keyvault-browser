use tokio::sync::mpsc::{Receiver, Sender};

use crate::tui::TuiEvent;

/// Represents different types of background tasks that can be launched.
///
/// Each [`TaskSpec`] contains the necessary parameters and other information to be able to call
/// the function associated with the specified background task.
pub enum TaskSpec {
    /// Lists Key Vaults for the given subscription ID.
    ListKeyVaults { subscription_id: String },
}

/// Launches requested tasks in the background and waits for tasks to finish before exiting.
///
/// Continously launches new tokio tasks based on the [`TaskSpec`] receieved from the given
/// [`Receiver`]. Passes a clone of the given [`Sender`] to the launched background tasks so the
/// background tasks can send messages to the TUI thread to request state modifications.
pub async fn manager(mut rx_bg_task: Receiver<TaskSpec>, tx_tui_event: Sender<TuiEvent>) {
    let mut spawned_tasks = vec![];
    // Stay alive only while main thread is alive
    while let Some(task_spec) = rx_bg_task.recv().await {
        // Spawn a new task or thread for new BackgroundTasks and add them to the Vec to keep track
        // of them.
        // TODO: There might be a better way to keep track of them.
        let handle = match task_spec {
            TaskSpec::ListKeyVaults { subscription_id } => {
                tokio::task::spawn(list_key_vaults(tx_tui_event.clone(), subscription_id))
            }
        };
        spawned_tasks.push(handle);
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
async fn list_key_vaults(
    tx: Sender<TuiEvent>,
    subscription_id: String,
) -> Result<(), tokio::sync::mpsc::error::SendError<TuiEvent>> {
    // Show loading status
    tx.send(TuiEvent::SetSuccessStatus(
        "Loading Key Vaults...".to_string(),
    ))
    .await?;

    // Get access token for the subscription
    let access_token =
        match crate::azure_api::get_access_token_for_subscription(&subscription_id).await {
            Ok(token) => token,
            Err(e) => {
                tx.send(TuiEvent::SetErrorStatus(format!(
                    "Failed to get access token: {}",
                    e
                )))
                .await?;
                return Ok(());
            }
        };

    // List key vaults using the token
    match crate::azure_api::list_key_vaults(&subscription_id, &access_token).await {
        Ok(key_vaults) => {
            tx.send(TuiEvent::KeyVaultsLoaded(key_vaults)).await?;
        }
        Err(e) => {
            tx.send(TuiEvent::SetErrorStatus(format!(
                "Failed to load Key Vaults: {}",
                e
            )))
            .await?;
        }
    }

    Ok(())
}

use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::tui::TuiEvent;

/// Represents different types of background tasks that can be launched.
///
/// Each [`TaskSpec`] contains the necessary parameters and other information to be able to call
/// the function associated with the specified background task.
pub enum TaskSpec {
    /// A dummy task that sleeps for a 5 seconds.
    SleepTest,
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
            TaskSpec::SleepTest => tokio::task::spawn(sleep_test(tx_tui_event.clone())),
        };
        spawned_tasks.push(handle);
    }

    // Wait for all background tasks to finish.
    for handle in spawned_tasks {
        let _ = handle.await.inspect_err(|e| eprintln!("{:?}", e));
    }
}

/// A dummy function that sleeps for 5 seconds. Sends state modification requests before and after sleeping.
async fn sleep_test(tx: Sender<TuiEvent>) {
    match tx.send(TuiEvent::ModifyCount(1)).await {
        Ok(_) => {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if tx.send(TuiEvent::ModifyCount(-1)).await.is_err() {
                // TUI thread is dead now. Send the log message to stderr.
                eprintln!("Task completed.");
            }
        }
        Err(_) => {
            // TUI is dead before we could even start. Skip doing anything.
        }
    }
}

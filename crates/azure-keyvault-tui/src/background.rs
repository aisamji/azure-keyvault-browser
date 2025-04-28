use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::tui::TuiEvent;

pub enum TaskSpec {
    SleepTest,
}

pub async fn manager(rx_bg_task: &mut Receiver<TaskSpec>, tx_tui_event: Sender<TuiEvent>) {
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

async fn sleep_test(tx: Sender<TuiEvent>) {
    match tx.send(TuiEvent::ModifyCount(1)).await {
        Ok(_) => {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let _ = tx.send(TuiEvent::ModifyCount(-1)).await;
        }
        Err(_) => {
            // TUI is dead. Skip doing anything.
        }
    }
}

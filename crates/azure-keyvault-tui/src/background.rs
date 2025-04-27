use std::time::Duration;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::app::AppEvent;

pub enum BackgroundTask {
    SleepTest,
}

pub async fn manager(rx: &mut Receiver<BackgroundTask>, tx: Sender<AppEvent>) {
    let mut spawned_tasks = vec![];
    while let Some(task_spec) = rx.recv().await {
        // Spawn a new task or thread for new BackgroundTasks and them to the Vec to keep track of
        // them.
        // TODO: There might be a better way to keep track of them.
        let handle = match task_spec {
            BackgroundTask::SleepTest => tokio::task::spawn(sleep_test(tx.clone())),
        };
        spawned_tasks.push(handle);
    }

    // Wait for all background tasks to finish.
    for handle in spawned_tasks {
        match handle.await {
            Ok(_) => {}
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

async fn sleep_test(tx: Sender<AppEvent>) {
    match tx.send(AppEvent::ModifyCount(1)).await {
        Ok(_) => {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let _ = tx.send(AppEvent::ModifyCount(-1)).await;
        }
        Err(_) => {
            // TUI is dead. Skip doing anything.
        }
    }
}

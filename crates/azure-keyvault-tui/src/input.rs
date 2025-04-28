use crossterm::event::{Event, EventStream, KeyCode};
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc::Sender;

use crate::{background::BackgroundTask, tui::TuiEvent};

pub async fn wait_for_inputs(tx_tui_event: Sender<TuiEvent>, tx_bg_task: Sender<BackgroundTask>) {
    // We use expects in this closure because we want to crash the application if an error
    // occurs.
    // TODO: Maybe better to send a notification that read failed. In what situations does read
    // fail?

    let mut reader = EventStream::new();

    loop {
        if let Some(result) = reader.next().fuse().await {
            match result.expect("Could not receive events from terminal.") {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Char('q') => {
                        let _ = tx_tui_event.send(TuiEvent::KillMe).await;
                        // We don't care if we get a SendError since that just means the UI thread
                        // is already dead.
                        break;
                    }
                    KeyCode::Char('t') => {
                        tx_bg_task
                            .send(BackgroundTask::SleepTest)
                            .await
                            .expect("Cannot communicate with background task manager. Thread is dead or channel has been accidentally closed.");
                    }
                    _ => {
                        // Not handled
                    }
                },
                _ => {
                    // Not handled
                }
            }
        }
    }
}

use crossterm::event::EventStream;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc::Sender;

use crate::tui::TuiEvent;

pub async fn wait_for_inputs(tx_tui_event: Sender<TuiEvent>) {
    // We use expects in this closure because we want to crash the application if an error
    // occurs.
    // TODO: Maybe better to send a notification that read failed. In what situations does read
    // fail?

    let mut reader = EventStream::new();

    loop {
        if let Some(result) = reader.next().fuse().await {
            let event =
                TuiEvent::UserInteraction(result.expect("Could not receive events from terminal."));
            let _ = tx_tui_event.send(event).await;
        }
    }
}

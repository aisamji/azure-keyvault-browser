use crossterm::event::EventStream;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc::Sender;

use crate::tui::TuiEvent;

/// Continuously forwards [`crossterm::event::Event`]s to the TUI thread using the given [`Sender`]
pub async fn forwarder(tx_tui_event: Sender<TuiEvent>) {
    // We use expects in this function because we want to crash the application if an error
    // occurs.
    // TODO: Maybe better to send a notification that read failed. In what situations does read
    // fail?

    let mut reader = EventStream::new();

    loop {
        if let Some(result) = reader.next().fuse().await {
            let event = TuiEvent::TerminalEvent(
                result.expect("Could not receive events from terminal."),
            );
            let _ = tx_tui_event.send(event).await;
        }
    }
}

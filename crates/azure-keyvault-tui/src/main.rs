use std::io;

use anyhow::Result;
use tokio::{sync::mpsc, task};

mod background;
mod input;
mod tui;

use tui::Tui;

/// The application entrypoint
///
/// Launches all threads and tasks necessary for the operation of the TUI.
#[tokio::main]
async fn main() -> Result<()> {
    // TODO: Use Clap to parse arguments and configure the application before launching the TUI.
    let mut app = Tui::default();

    // All rx must be mut, but the function calls take care of that.
    let (tx_tui_event, rx_tui_event) = mpsc::channel(10);
    let (tx_bg_task, rx_bg_task) = mpsc::channel(10);

    // Start main UI loop in separate thread
    let tui_handle = task::spawn_blocking(move || -> io::Result<()> {
        let mut terminal = ratatui::init();
        let app_result = app.run(&mut terminal, rx_tui_event, tx_bg_task);
        ratatui::restore();
        app_result
    });

    // Start Input loop in separate thread
    let tx_tui_event_clone = tx_tui_event.clone();
    tokio::spawn(input::forwarder(tx_tui_event_clone));

    // Start Background Task Manager
    let bg_man_handle = tokio::spawn(background::manager(rx_bg_task, tx_tui_event));

    // Wait for the main event loop to complete or crash
    let result = tui_handle.await?;
    eprintln!("Waiting on background tasks to complete.");
    bg_man_handle.await?;
    eprintln!("Background tasks finished.");
    Ok(result?)
}

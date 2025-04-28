use std::io;

use anyhow::Result;
use tokio::{sync::mpsc, task};

mod background;
mod input;
mod tui;

use tui::Tui;

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: Use Clap to parse arguments and configure the application before launching the TUI.
    let mut app = Tui::default();

    let (tx_tui_event, mut rx_tui_event) = mpsc::channel(10);
    let (tx_bg_task, mut rx_bg_task) = mpsc::channel(10);

    // Start main UI loop in separate thread
    let tui_handle = task::spawn_blocking(move || -> io::Result<()> {
        let mut terminal = ratatui::init();
        let app_result = app.run(&mut terminal, &mut rx_tui_event, tx_bg_task);
        // TODO: Send Kill to input thread. Might be able to leverage crossterm::event::poll.
        ratatui::restore();
        app_result
    });

    // Start Input loop in separate thread
    let tx_tui_event_clone = tx_tui_event.clone();
    tokio::spawn(async move {
        input::wait_for_inputs(tx_tui_event_clone).await;
    });

    // Start Background Task Manager
    let bg_man_handle = tokio::spawn(async move {
        background::manager(&mut rx_bg_task, tx_tui_event).await;
    });

    // Wait for the main event loop to complete or crash
    let result = tui_handle.await?;
    eprintln!("Waiting on background tasks to complete.");
    bg_man_handle.await?;
    eprintln!("Background tasks finished.");
    Ok(result?)
}

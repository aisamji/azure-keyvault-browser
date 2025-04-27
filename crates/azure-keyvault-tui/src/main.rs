use std::{io, time::Duration};

use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize as _,
    text::{Line, Span},
};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task,
};

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: Use Clap to parse arguments and configure the application before launching the TUI.
    let mut app = App::default();

    let (tx_to_tui, mut rx_for_tui) = mpsc::channel(10);
    let (tx_to_bg_man, mut rx_for_bg_man) = mpsc::channel(10);

    // Start main UI loop in separate thread
    let tui_handle = task::spawn_blocking(move || -> io::Result<()> {
        let mut terminal = ratatui::init();
        let app_result = app.run(&mut terminal, &mut rx_for_tui);
        // TODO: Send Kill to input thread. Might be able to leverage crossterm::event::poll.
        ratatui::restore();
        app_result
    });

    // Start Input loop in separate thread
    let tx_to_tui_for_input = tx_to_tui.clone();
    task::spawn_blocking(move || {
        // We use expects in this closure because we want to crash the application if an error
        // occurs.
        // TODO: Maybe better to send a notification that read failed. In what situations does read
        // fail?

        loop {
            match crossterm::event::read().expect("Could not receive events from terminal.") {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Char('q') => {
                        let _ = tx_to_tui_for_input.blocking_send(AppEvent::KillMe);
                        // We don't care if we get a SendError since that just means the UI thread
                        // is already dead.
                        break;
                    }
                    KeyCode::Char('t') => {
                        tx_to_bg_man
                            .blocking_send(BackgroundTask::SleepTest)
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
    });

    // Start Background Task Manager
    let bg_man_handle = task::spawn(async move {
        let mut spawned_tasks = vec![];
        loop {
            match rx_for_bg_man.recv().await {
                Some(task_spec) => {
                    let handle = match task_spec {
                        BackgroundTask::SleepTest => task::spawn(sleep_test(tx_to_tui.clone())),
                    };
                    spawned_tasks.push(handle);
                }
                None => break, // No more senders available.
            }
        }
        for handle in spawned_tasks {
            match handle.await {
                Ok(_) => {}
                Err(e) => eprintln!("{:?}", e),
            }
        }
    });

    // Wait for the main event loop to complete or crash
    let result = tui_handle.await?;
    eprintln!("Waiting on background tasks to complete.");
    bg_man_handle.await?;
    Ok(result?)
}

#[derive(Default)]
struct App {
    active_tasks: i16,
}

impl App {
    fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: &mut Receiver<AppEvent>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            match rx.blocking_recv() {
                Some(app_event) => match app_event {
                    AppEvent::KillMe => break,
                    AppEvent::ModifyCount(inc) => {
                        self.active_tasks += inc;
                    }
                },
                // If all senders have somehow been closed, we should kill this thread as well.
                None => break,
            }
        }

        Ok(())
    }

    fn render(&self, frame: &mut Frame<'_>) {
        let line = Line::from(vec![
            Span::from("Hello World! I have "),
            Span::from(self.active_tasks.to_string()).bold(),
            Span::from(" tasks running in the background."),
        ]);
        frame.render_widget(line, frame.area());
    }
}

enum AppEvent {
    KillMe,
    ModifyCount(i16),
}

enum BackgroundTask {
    SleepTest,
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

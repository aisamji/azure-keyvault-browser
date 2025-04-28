use std::io;

use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize as _,
    text::{Line, Span},
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::background::BackgroundTask;

// All state mutations should be done in the run method only to avoid deadlocks.
#[derive(Default)]
pub struct Tui {
    active_tasks: i16,
}

impl Tui {
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: &mut Receiver<TuiEvent>,
        tx_bg_task: Sender<BackgroundTask>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            match rx.blocking_recv() {
                Some(tui_event) => match tui_event {
                    TuiEvent::ModifyCount(inc) => {
                        self.active_tasks += inc;
                    }
                    TuiEvent::UserInteraction(event) => {
                        match event {
                            Event::Key(key_event) => match key_event.code {
                                KeyCode::Char('q') => {
                                    // Quit
                                    break;
                                }
                                KeyCode::Char('t') => {
                                    // Launch a new background task
                                    tx_bg_task
                                        .blocking_send(BackgroundTask::SleepTest)
                                        .expect("Cannot communicate with background task manager. Thread is dead or channel has been accidentally closed.");
                                    // TODO: Do not use expect. Find a better solution. Print error
                                    // out to TUI
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

pub enum TuiEvent {
    /// Represents a interactive  by the user that needs to be processed by the system.
    /// This variant wraps a [`crossterm::event::Event`] type that encapsulates user interactions.
    UserInteraction(Event),
    ModifyCount(i16),
}

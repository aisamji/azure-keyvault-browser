use std::io;

use crossterm::event::{Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::Stylize as _,
    text::{Line, Span},
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::background::TaskSpec;

/// Represents different types of events that can occur in the Terminal User Interface (TUI).
///
/// This enum represents various events that the TUI thread needs to handle, including
/// terminal (i.e. [`crossterm`]) events and requests to modify state from the background tasks.
/// Background tasks should not modify the TUI state directly to avoid race condtions and must send
/// a [`TuiEvent`] describing the type of modification needs to be made.
///
/// # Example
///
/// ```rust
/// use crossterm::event::{read, Event};
/// use tokio::sync::mpsc;
///
/// let tx, _ = mpsc::channel(10);
/// let event: Event = read()?;
/// tx.blocking_send(TuiEvent::TerminalInteraction(event))?;
/// ```
pub enum TuiEvent {
    /// Represents an interactive event made by the user.
    TerminalEvent(Event),
    /// Requests a modification of [`Tui::active_tasks`] by the specified amount.
    ModifyCount(i16),
}

// All state mutations should be done in the run method only to avoid deadlocks.
/// Represents the app state and handles state modification as well as rendering to the terminal.
///
/// Contains a `run` function that is used to start the main loop. The fields of this struct should
/// not be modified by any threads other than the one executing [`Self::run`]. Any modification
/// requests should be sent to the appropriate [`Sender`] channel.
#[derive(Default)]
pub struct Tui {
    /// The number of active background tasks in the application.
    active_tasks: i16,
}

impl Tui {
    /// Redraws the terminal every time a [`TuiEvent`] is received.
    ///
    /// Be sure to only call this function with [`tokio::task::spawn_blocking`]. This function
    /// watches for [`TuiEvent`]s from the given [`Receiver`] in an infinite loop. Based on the
    /// `TuiEvent` received, the function does one of three things: modify the state (i.e. fields
    /// on the [`Tui`] instance, launch one or background tasks by sending [`TaskSpec`]s to the
    /// given [`Sender`]s, or break out of the infinite loop (i.e. quit the application).
    ///
    /// The terminal is redrawn after processing each [`TuiEvent`].
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        mut rx: Receiver<TuiEvent>,
        tx_bg_task: Sender<TaskSpec>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            match rx.blocking_recv() {
                Some(tui_event) => match tui_event {
                    TuiEvent::ModifyCount(inc) => {
                        self.active_tasks += inc;
                    }
                    TuiEvent::TerminalEvent(event) => {
                        if self.process_terminal_event(&event, &tx_bg_task) {
                            break;
                        }
                    }
                },
                // If all senders of TuiEvents have somehow been closed, we should kill this thread as well.
                // TODO: Should probably return an error since this means the input thread has
                // crashed.
                None => break,
            }
        }

        Ok(())
    }

    /// Renders [`ratatui::widgets::Widget`]s on the specified [`Frame`].
    ///
    /// A private helper function that should only be called from [`Tui::run`].
    fn render(&self, frame: &mut Frame<'_>) {
        let hello = Line::from(vec![
            Span::from("Hello World! I have "),
            Span::from(self.active_tasks.to_string()).bold().green(),
            Span::from(" tasks running in the background."),
        ])
        .centered();
        let instructions = Line::from(vec![
            Span::from("<Q>").blue().bold(),
            Span::from(" Quit  "),
            Span::from("<T>").blue().bold(),
            Span::from(" Launch Background Task"),
        ])
        .centered();

        let layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ]);
        let [_, hello_area, instructions_area, _] = layout.areas(frame.area());
        frame.render_widget(hello, hello_area);
        frame.render_widget(instructions, instructions_area);
    }

    /// Handles crossterm [`Event`]s. Returns `true` if the TUI should quit.
    ///
    /// A private helper function that mutates the app's internal state or launches a background
    /// task by using the given [`Sender`]. Returns a value indicating whether the TUI should quit.
    /// May use the current app state to determine what action to take in response to the [`Event`].
    fn process_terminal_event(&mut self, event: &Event, tx_bg_task: &Sender<TaskSpec>) -> bool {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Char('q') => {
                    // Quit
                    return true;
                }
                KeyCode::Char('t') => {
                    // Launch a new background task
                    tx_bg_task
                        .blocking_send(TaskSpec::SleepTest)
                        .expect("Cannot communicate with background task manager. Thread is dead or channel has been accidentally closed.");
                    // TODO: Do not use expect. Find a better solution. Print error
                    // out to TUI
                }
                _ => {
                    // Other key combinations not handled
                }
            },
            _ => {
                // Other events not handled
            }
        }

        return false;
    }
}

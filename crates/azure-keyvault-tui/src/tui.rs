use std::io;

use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize as _,
    text::{Line, Span},
};
use tokio::sync::mpsc::Receiver;

#[derive(Default)]
pub struct Tui {
    active_tasks: i16,
}

impl Tui {
    pub fn run(
        &mut self,
        terminal: &mut DefaultTerminal,
        rx: &mut Receiver<TuiEvent>,
    ) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;
            match rx.blocking_recv() {
                Some(tui_event) => match tui_event {
                    TuiEvent::KillMe => break,
                    TuiEvent::ModifyCount(inc) => {
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

pub enum TuiEvent {
    KillMe,
    ModifyCount(i16),
}

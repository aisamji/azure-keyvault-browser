use std::io;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style, Stylize as _},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    azure_api::KeyVault,
    azure_profile::{AzureProfile, AzureSubscription},
    background::TaskSpec,
};

/// Represents the different screens/views available in the TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    /// Main screen showing key vaults for the selected subscription.
    KeyVaults,
    /// Screen showing the list of available subscriptions.
    Subscriptions,
}

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
    /// Key Vaults have been successfully loaded.
    KeyVaultsLoaded(Vec<KeyVault>),
    /// Sets a success status message to display to the user.
    SetSuccessStatus(String),
    /// Sets an error status message to display to the user.
    SetErrorStatus(String),
    /// Clears the current status.
    ClearStatus,
}

// All state mutations should be done in the run method only to avoid deadlocks.
/// Represents the app state and handles state modification as well as rendering to the terminal.
///
/// Contains a `run` function that is used to start the main loop. The fields of this struct should
/// not be modified by any threads other than the one executing [`Self::run`]. Any modification
/// requests should be sent to the appropriate [`Sender`] channel.
pub struct TuiState {
    /// List of all available subscriptions.
    subscriptions: Vec<AzureSubscription>,
    /// The currently selected subscription.
    selected_subscription: Option<AzureSubscription>,
    /// List of loaded key vaults.
    key_vaults: Vec<KeyVault>,
    /// The currently activated key vault.
    selected_key_vault: Option<KeyVault>,
    /// The current screen being displayed.
    current_screen: Screen,
    /// Azure CLI version.
    azure_cli_version: String,
    /// Current status to display to the user.
    status: Option<Result<String, String>>,

    // States for Widgets
    /// Table state for table selections.
    table_state: TableState,
}

impl Default for TuiState {
    fn default() -> Self {
        // Get Azure CLI version first, crash if not available
        let azure_cli_version = AzureProfile::get_azure_cli_version().unwrap_or_else(|e| {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        });

        let subscriptions = AzureProfile::try_from_config()
            .ok()
            .map(|ap| ap.subscriptions)
            .unwrap_or_default();

        let selected_subscription = subscriptions.iter().find(|s| s.is_default).cloned();

        // Initialize subscriptions table state with default selection if available
        let current_screen = if selected_subscription.is_some() {
            Screen::KeyVaults
        } else {
            Screen::Subscriptions
        };

        Self {
            subscriptions,
            selected_subscription,
            key_vaults: Vec::new(),
            table_state: TableState::default(),
            selected_key_vault: None,
            current_screen,
            azure_cli_version,
            status: None,
        }
    }
}

impl TuiState {
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
        // Trigger initial Key Vault listing if we have a default subscription
        if let Some(subscription) = &self.selected_subscription {
            let _ = tx_bg_task.blocking_send(TaskSpec::ListKeyVaults {
                subscription_id: subscription.id.clone(),
            });
        }

        loop {
            terminal.draw(|f| {
                self.render(f);
            })?;
            match rx.blocking_recv() {
                Some(tui_event) => match tui_event {
                    TuiEvent::TerminalEvent(event) => {
                        if self.process_terminal_event(&event, &tx_bg_task) {
                            break;
                        }
                    }
                    TuiEvent::KeyVaultsLoaded(key_vaults) => {
                        self.key_vaults = key_vaults;
                        self.status =
                            Some(Ok(format!("Loaded {} key vaults", self.key_vaults.len())));
                    }
                    TuiEvent::SetSuccessStatus(message) => {
                        self.status = Some(Ok(message));
                    }
                    TuiEvent::SetErrorStatus(message) => {
                        self.status = Some(Err(message));
                    }
                    TuiEvent::ClearStatus => {
                        self.status = None;
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
    fn render(&mut self, frame: &mut Frame<'_>) {
        // Define areas/layout
        let layout = Layout::vertical([
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Length(2),
        ]);
        let [header, body_area, status_area] = layout.areas(frame.area());
        let header_layout = Layout::horizontal([
            Constraint::Fill(2),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ]);
        let [metadata_area, global_keymaps_area, local_keymaps_area] = header_layout.areas(header);

        // Render Metadata
        let metadata = Text::from(vec![
            Line::from(vec![
                Span::from("Tenant ID: ").bold(),
                Span::from(
                    self.selected_subscription
                        .as_ref()
                        .map(|s| s.tenant_id.as_str())
                        .unwrap_or("None"),
                ),
            ]),
            Line::from(vec![
                Span::from("Subscription: ").bold(),
                Span::from(
                    self.selected_subscription
                        .as_ref()
                        .map(|s| format!("{} ({})", s.name, s.id))
                        .unwrap_or("None".to_string()),
                ),
            ]),
            Line::from(vec![
                Span::from("Resource Group: ").bold(),
                Span::from(
                    self.selected_key_vault
                        .as_ref()
                        .map(|kv| kv.resource_group())
                        .unwrap_or("None"),
                ),
            ]),
            Line::from(vec![
                Span::from("Key Vault: ").bold(),
                Span::from(
                    self.selected_key_vault
                        .as_ref()
                        .map(|kv| kv.name.as_str())
                        .unwrap_or("None"),
                ),
            ]),
            Line::from(vec![
                Span::from("AZKV Version: ").bold(),
                Span::from(env!("CARGO_PKG_VERSION")),
            ]),
            Line::from(vec![
                Span::from("Azure CLI: ").bold(),
                Span::from(&self.azure_cli_version),
            ]),
        ]);
        frame.render_widget(metadata, metadata_area);

        // Render Instructions
        let global_keymaps = Text::from(vec![
            Line::from(vec![
                Span::from("<S>").bold().cyan(),
                Span::from(" Subscriptions"),
            ]),
            Line::from(vec![
                Span::from("<K>").bold().cyan(),
                Span::from(" Key Vaults"),
            ]),
            Line::from(vec![Span::from("<k>").bold().cyan(), Span::from(" Keys")]),
            Line::from(vec![
                Span::from("<s>").bold().cyan(),
                Span::from(" Secrets"),
            ]),
            Line::from(vec![
                Span::from("<c>").bold().cyan(),
                Span::from(" Certificates"),
            ]),
            Line::from(vec![Span::from("<q>").bold().cyan(), Span::from(" Quit")]),
        ]);
        frame.render_widget(global_keymaps, global_keymaps_area);

        // Render Body based on current screen
        match self.current_screen {
            Screen::KeyVaults => {
                let table = keyvaults_as_table(self.key_vaults.as_slice());
                frame.render_stateful_widget(table, body_area, &mut self.table_state);
            }
            Screen::Subscriptions => {
                let table = subscriptions_as_table(self.subscriptions.as_slice());
                frame.render_stateful_widget(table, body_area, &mut self.table_state);
            }
        }

        // Render Status Bar
        if let Some(ref status_result) = self.status {
            let (status_text, status_style) = match status_result {
                Ok(text) => (text.as_str(), Style::default().fg(Color::Green)),
                Err(text) => (text.as_str(), Style::default().fg(Color::Red)),
            };
            let status_paragraph = Paragraph::new(status_text)
                .style(status_style)
                .wrap(Wrap { trim: true });
            frame.render_widget(status_paragraph, status_area);
        }
    }

    /// Handles crossterm [`Event`]s. Returns `true` if the TUI should quit.
    ///
    /// A private helper function that mutates the app's internal state or launches a background
    /// task by using the given [`Sender`]. Returns a value indicating whether the TUI should quit.
    /// Blockuse the current app state to determine what action to take in response to the [`Event`].
    fn process_terminal_event(&mut self, event: &Event, tx_bg_task: &Sender<TaskSpec>) -> bool {
        match event {
            Event::Key(key_event) => match key_event.code {
                KeyCode::Char('q') => {
                    // Quit
                    return true;
                }
                KeyCode::Char('S') if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.load_screen(Screen::Subscriptions, tx_bg_task);
                }
                KeyCode::Char('K') if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                    self.load_screen(Screen::KeyVaults, tx_bg_task);
                }
                KeyCode::Up => match self.current_screen {
                    Screen::KeyVaults | Screen::Subscriptions => {
                        self.table_state.select_previous();
                    }
                },
                KeyCode::Down => match self.current_screen {
                    Screen::KeyVaults | Screen::Subscriptions => {
                        self.table_state.select_next();
                    }
                },
                KeyCode::Enter => {
                    match self.current_screen {
                        Screen::KeyVaults => {
                            if let Some(selected_index) = self.table_state.selected() {
                                if let Some(key_vault) = self.key_vaults.get(selected_index) {
                                    self.selected_key_vault = Some(key_vault.clone());
                                    self.status = Some(Ok(format!(
                                        "Activated Key Vault: {}",
                                        key_vault.name
                                    )));
                                    // TODO: Automatically switch to Secrets screen.
                                }
                            }
                        }
                        Screen::Subscriptions => {
                            if let Some(selected_index) = self.table_state.selected()
                            {
                                if let Some(subscription) = self.subscriptions.get(selected_index) {
                                    self.selected_subscription = Some(subscription.clone());
                                    self.selected_key_vault = None;
                                    self.load_screen(Screen::KeyVaults, tx_bg_task);
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Other key combinations not handled
                }
            },
            _ => {
                // Other events not handled
            }
        }

        false
    }

    /// Switch to the given [`Screen`], loading data as necessary.
    ///
    /// Launches a background task to load the data asynchronously.
    fn load_screen(&mut self, screen: Screen, tx_bg_task: &Sender<TaskSpec>) {
        self.current_screen = screen.clone();
        self.status = None;

        // Load new list for table/screen.
        match screen {
            Screen::KeyVaults => {
                // Trigger key vault loading if we have a selected subscription
                if let Some(subscription) = &self.selected_subscription {
                    self.key_vaults = vec![];
                    self.table_state = TableState::default();
                    let _ = tx_bg_task.blocking_send(TaskSpec::ListKeyVaults {
                        subscription_id: subscription.id.clone(),
                    });
                } else {
                    self.status = Some(Err(
                        "No subscription selected. Please select a subscription first.".to_string(),
                    ));
                }
            }
            Screen::Subscriptions => {
                // No need to load anything, the file is cached upon startup.
            }
        }
    }
}

fn subscriptions_as_table(subscriptions: &[AzureSubscription]) -> Table<'_> {
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().bold()),
        Cell::from("ID").style(Style::default().bold()),
        Cell::from("Tenant ID").style(Style::default().bold()),
        Cell::from("Auth").style(Style::default().bold()),
    ]);

    let rows: Vec<Row> = subscriptions
        .iter()
        .map(|subscription| {
            Row::new(vec![
                Cell::from(subscription.name.clone()),
                Cell::from(subscription.id.clone()),
                Cell::from(subscription.tenant_id.clone()),
                Cell::from(subscription.user.to_string()),
            ])
        })
        .collect();

    Table::new(
        rows,
        [
            Constraint::Fill(2),
            Constraint::Fill(3),
            Constraint::Fill(3),
            Constraint::Fill(2),
        ],
    )
    .header(header)
    .block(
        Block::new()
            .borders(Borders::all())
            .title_alignment(Alignment::Center)
            .title(Line::from(" Subscriptions ")),
    )
    .row_highlight_style(Style::default().bg(Color::Blue))
}

fn keyvaults_as_table(keyvaults: &[KeyVault]) -> Table<'_> {
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().bold()),
        Cell::from("Resource Group").style(Style::default().bold()),
    ]);

    let rows: Vec<Row> = keyvaults
        .iter()
        .map(|key_vault| {
            Row::new(vec![
                Cell::from(key_vault.name.clone()),
                Cell::from(key_vault.resource_group()),
            ])
        })
        .collect();

    Table::new(rows, [Constraint::Fill(1), Constraint::Fill(1)])
        .header(header)
        .block(
            Block::new()
                .borders(Borders::all())
                .title_alignment(Alignment::Center)
                .title(Line::from(" Key Vaults ")),
        )
        .row_highlight_style(Style::default().bg(Color::Blue))
}

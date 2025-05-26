use std::io;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style, Stylize as _},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
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
    KeyVaultsLoaded(Vec<crate::azure_api::KeyVault>),
    /// Sets the status message to display to the user.
    SetStatusMessage(String),
    /// Clears the current status message.
    ClearStatusMessage,
}

// All state mutations should be done in the run method only to avoid deadlocks.
/// Represents the app state and handles state modification as well as rendering to the terminal.
///
/// Contains a `run` function that is used to start the main loop. The fields of this struct should
/// not be modified by any threads other than the one executing [`Self::run`]. Any modification
/// requests should be sent to the appropriate [`Sender`] channel.
pub struct Tui {
    /// List of all available subscriptions.
    subscriptions: Vec<AzureSubscription>,
    /// Index of the currently selected subscription.
    selected_subscription_index: Option<usize>,
    /// List of loaded key vaults.
    key_vaults: Vec<crate::azure_api::KeyVault>,
    /// Index of the currently selected key vault.
    selected_key_vault_index: Option<usize>,
    /// The current screen being displayed.
    current_screen: Screen,
    /// Azure CLI version.
    azure_cli_version: String,
    /// Current status message to display to the user.
    status_message: Option<String>,
}

impl Default for Tui {
    fn default() -> Self {
        // Get Azure CLI version first, crash if not available
        let azure_cli_version = AzureProfile::get_azure_cli_version()
            .unwrap_or_else(|e| {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            });

        let subscriptions = AzureProfile::try_from_config()
            .ok()
            .map(|ap| ap.subscriptions)
            .unwrap_or_default();
        
        let selected_subscription_index = subscriptions
            .iter()
            .position(|s| s.is_default);

        let current_screen = if selected_subscription_index.is_some() {
            Screen::KeyVaults
        } else {
            Screen::Subscriptions
        };

        Self {
            subscriptions,
            selected_subscription_index,
            key_vaults: Vec::new(),
            selected_key_vault_index: None,
            current_screen,
            azure_cli_version,
            status_message: None,
        }
    }
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
        // Trigger initial Key Vault listing if we have a default subscription
        if let Some(subscription) = self.selected_subscription_index
            .and_then(|idx| self.subscriptions.get(idx)) {
            let _ = tx_bg_task.blocking_send(TaskSpec::ListKeyVaults {
                subscription_id: subscription.id.clone(),
            });
        }

        loop {
            terminal.draw(|f| self.render(f))?;
            match rx.blocking_recv() {
                Some(tui_event) => match tui_event {
                    TuiEvent::TerminalEvent(event) => {
                        if self.process_terminal_event(&event, &tx_bg_task) {
                            break;
                        }
                    }
                    TuiEvent::KeyVaultsLoaded(key_vaults) => {
                        self.key_vaults = key_vaults;
                        self.selected_key_vault_index = None; // Clear selection when new data loads
                        self.status_message = Some(format!("Loaded {} key vaults", self.key_vaults.len()));
                    }
                    TuiEvent::SetStatusMessage(message) => {
                        self.status_message = Some(message);
                    }
                    TuiEvent::ClearStatusMessage => {
                        self.status_message = None;
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
        // Define areas/layout
        let layout = Layout::vertical([
            Constraint::Length(6), 
            Constraint::Fill(1), 
            Constraint::Length(2)
        ]);
        let [header, body_area, status_area] = layout.areas(frame.area());
        let header_layout = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ]);
        let [metadata_area, global_keymaps_area, local_keymaps_area] = header_layout.areas(header);

        // Render Metadata
        let current_subscription = self.selected_subscription_index
            .and_then(|idx| self.subscriptions.get(idx));
        
        let metadata = Text::from(vec![
            Line::from(vec![
                Span::from("Subscription: ").bold(),
                Span::from(
                    current_subscription
                        .map(|s| format!("{} ({})", s.name, s.id))
                        .unwrap_or("None".to_string()),
                ),
            ]),
            Line::from(vec![
                Span::from("Resource Group: ").bold(),
                Span::from("None"),
            ]),
            Line::from(vec![
                Span::from("Tenant ID: ").bold(),
                Span::from(
                    current_subscription
                        .map(|s| s.tenant_id.as_str())
                        .unwrap_or("None"),
                ),
            ]),
            Line::from(vec![Span::from("Key Vault: ").bold(), Span::from("None")]),
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
                let header = Row::new(vec![
                    Cell::from("Name").style(Style::default().bold()),
                    Cell::from("Resource Group").style(Style::default().bold()),
                ]);

                let rows: Vec<Row> = self.key_vaults
                    .iter()
                    .enumerate()
                    .map(|(idx, key_vault)| {
                        // Extract resource group from the resource ID
                        // Format: /subscriptions/{subscription}/resourceGroups/{resourceGroup}/providers/Microsoft.KeyVault/vaults/{name}
                        let resource_group = key_vault.id
                            .split('/')
                            .collect::<Vec<&str>>()
                            .get(4)
                            .unwrap_or(&"Unknown")
                            .to_string();
                        
                        let style = if Some(idx) == self.selected_key_vault_index {
                            Style::default().bg(Color::Blue)
                        } else {
                            Style::default()
                        };
                        
                        Row::new(vec![
                            Cell::from(key_vault.name.clone()),
                            Cell::from(resource_group),
                        ]).style(style)
                    })
                    .collect();

                let table = Table::new(rows, [
                    Constraint::Fill(1),
                    Constraint::Fill(1),
                ])
                .header(header)
                .block(
                    Block::new()
                        .borders(Borders::all())
                        .title_alignment(Alignment::Center)
                        .title(Line::from(" Key Vaults "))
                );

                frame.render_widget(table, body_area);
            }
            Screen::Subscriptions => {
                let header = Row::new(vec![
                    Cell::from("Name").style(Style::default().bold()),
                    Cell::from("ID").style(Style::default().bold()),
                    Cell::from("Tenant ID").style(Style::default().bold()),
                    Cell::from("Auth").style(Style::default().bold()),
                ]);

                let rows: Vec<Row> = self.subscriptions
                    .iter()
                    .enumerate()
                    .map(|(idx, subscription)| {
                        let style = if Some(idx) == self.selected_subscription_index {
                            Style::default().bg(Color::Blue)
                        } else {
                            Style::default()
                        };
                        
                        Row::new(vec![
                            Cell::from(subscription.name.clone()),
                            Cell::from(subscription.id.clone()),
                            Cell::from(subscription.tenant_id.clone()),
                            Cell::from(subscription.user.to_string()),
                        ]).style(style)
                    })
                    .collect();

                let table = Table::new(rows, [
                    Constraint::Fill(2),
                    Constraint::Fill(3),
                    Constraint::Fill(3),
                    Constraint::Fill(2),
                ])
                .header(header)
                .block(
                    Block::new()
                        .borders(Borders::all())
                        .title_alignment(Alignment::Center)
                        .title(Line::from(" Subscriptions "))
                );

                frame.render_widget(table, body_area);
            }
        }

        // Render Status Bar
        if let Some(ref status_text) = self.status_message {
            let status_style = if status_text.contains("Error") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };
            let status_paragraph = Paragraph::new(status_text.as_str())
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
                    // Switch to subscriptions screen
                    self.current_screen = Screen::Subscriptions;
                    self.status_message = None;
                }
                KeyCode::Char('K') if key_event.modifiers.contains(KeyModifiers::SHIFT) => {
                    // Switch to key vaults screen
                    self.current_screen = Screen::KeyVaults;
                    self.status_message = None;
                    
                    // Trigger key vault loading if we have a selected subscription
                    if let Some(subscription) = self.selected_subscription_index
                        .and_then(|idx| self.subscriptions.get(idx)) {
                        let _ = tx_bg_task.blocking_send(TaskSpec::ListKeyVaults {
                            subscription_id: subscription.id.clone(),
                        });
                    } else {
                        self.status_message = Some("No subscription selected. Please select a subscription first.".to_string());
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

        return false;
    }
}

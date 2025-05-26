# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

- **Build**: `cargo build` or `cargo build --release`
- **Run**: `cargo run` (runs the main TUI application)
- **Test**: `cargo test`
- **Check**: `cargo check` (fast compilation check without building)
- **Clippy**: `cargo clippy` (Rust linter)
- **Format**: `cargo fmt` (code formatting)

## Architecture Overview

This is a Rust TUI (Terminal User Interface) application for browsing Azure Key Vaults built with ratatui and tokio.

### Thread Architecture
The application uses a multi-threaded architecture with message passing:

1. **Main TUI Thread** (`tui::Tui::run`): Handles rendering and state management. Runs in `spawn_blocking` since ratatui is synchronous.
2. **Input Thread** (`input::forwarder`): Captures terminal events (keystrokes, resize, etc.) and forwards them to the TUI thread via `TuiEvent::TerminalEvent`.
3. **Background Task Manager** (`background::manager`): Manages async operations to keep the UI responsive. Receives `TaskSpec`s from TUI thread and spawns appropriate background tasks.

### Message Passing
- `TuiEvent`: Events sent TO the TUI thread (terminal events, state modification requests)
- `TaskSpec`: Background task specifications sent FROM TUI thread to background manager

### Key Modules
- `tui.rs`: Main application state and rendering logic
- `background.rs`: Background task management and definitions
- `input.rs`: Terminal event capture and forwarding

### State Management Rules
- Only the TUI thread (`Tui::run`) should modify application state
- Background tasks communicate via `TuiEvent`s, never modify state directly
- This design ensures thread safety without mutexes

### Adding New Features
1. Define background tasks in `background.rs` with appropriate `TaskSpec` variants
2. Add TUI event handling in `tui.rs` for state updates from background tasks
3. Update `process_terminal_event` for new user interactions
4. Follow the message-passing pattern to maintain thread safety

## Workspace Structure
This is a Cargo workspace with the main application in `crates/azure-keyvault-tui/` and a placeholder xtask in `xtask/`.

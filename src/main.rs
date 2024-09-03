mod config;
mod errors;
mod github_client;
mod secrets_manager;
mod tui;

use crate::config::Config;
use crate::errors::AppResult;
use crate::github_client::GitHubClient;
use crate::secrets_manager::SecretsManager;
use crate::tui::Tui;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;

#[tokio::main]
async fn main() -> AppResult<()> {
    let config = Config::load()?;
    let client = GitHubClient::new(&config.organization, &config.repository, &config.token);

    let public_key = client.get_public_key().await?;
    let existing_secrets = client.get_existing_secrets().await?;

    let secrets_manager = SecretsManager::new(config.secrets, existing_secrets, public_key, &client);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create TUI and run it
    let mut tui = Tui::new(&secrets_manager);
    let res = tui.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error in TUI: {:?}", err);
    }

    // Perform actual secret management after TUI closes
    secrets_manager.manage_secrets().await?;

    Ok(())
}
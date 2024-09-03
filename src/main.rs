mod github_client;
mod secrets_manager;
mod tui;

use dotenv::dotenv;
use github_client::GitHubClient;
use secrets_manager::{Secret, SecretsManager};
use std::{env, io};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tui::Tui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let organization = env::var("GITHUB_ORGANIZATION")?;
    let repository = env::var("GITHUB_REPOSITORY")?;
    let token = env::var("GITHUB_TOKEN")?;
    let secrets_json = env::var("GITHUB_SECRETS")?;

    let secrets: Vec<Secret> = serde_json::from_str(&secrets_json)?;
    let client = GitHubClient::new(&organization, &repository, &token);

    let public_key = client.get_public_key().await?;
    let existing_secrets = client.get_existing_secrets().await?;

let secrets_manager = SecretsManager::new(secrets, existing_secrets, public_key, &client);

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
        println!("{:?}", err)
    }

    // Perform actual secret management after TUI closes
    secrets_manager.manage_secrets().await?;

    Ok(())
}
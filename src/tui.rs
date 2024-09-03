use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::io;
use crate::secrets_manager::{SecretsManager, SecretStatus};

enum NavDirection {
    Up,
    Down,
}

enum AppState {
    ListView,
    DetailsView,
}

pub struct StatusMessage {
    pub content: String,
    pub style: Style,
    pub expiry: Option<Instant>,
}

impl StatusMessage {
    fn new(content: String, style: Style, duration: Option<Duration>) -> Self {
        let expiry = duration.map(|d| Instant::now() + d);
        Self {
            content,
            style,
            expiry,
        }
    }

    fn is_expired(&self) -> bool {
        self.expiry.map_or(false, |expiry| Instant::now() > expiry)
    }
}

pub struct ColorScheme {
    pub new: Color,
    pub existing: Color,
    pub deleted: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        ColorScheme {
            new: Color::Green,
            existing: Color::White,
            deleted: Color::Red,
        }
    }
}

pub struct Tui<'a> {
    secrets_manager: &'a SecretsManager<'a>,
    selected_index: usize,
    app_state: AppState,
    status_message: Option<StatusMessage>,
    color_scheme: ColorScheme,
}

impl<'a> Tui<'a> {
    pub fn new(secrets_manager: &'a SecretsManager) -> Self {
        Self {
            secrets_manager,
            selected_index: 0,
            app_state: AppState::ListView,
            status_message: None,
            color_scheme: ColorScheme::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => self.move_selection(NavDirection::Up),
                    KeyCode::Down => self.move_selection(NavDirection::Down),
                    KeyCode::Enter => self.toggle_view(),
                    _ => {}
                }
            }

            self.clear_expired_status_message();
        }
    }

    fn move_selection(&mut self, direction: NavDirection) {
        let secrets_len = self.secrets_manager.get_secrets().len();
        match direction {
            NavDirection::Up => {
                self.selected_index = self.selected_index.saturating_sub(1);
            }
            NavDirection::Down => {
                if self.selected_index < secrets_len.saturating_sub(1) {
                    self.selected_index += 1;
                }
            }
        }
    }

    fn toggle_view(&mut self) {
        self.app_state = match self.app_state {
            AppState::ListView => AppState::DetailsView,
            AppState::DetailsView => AppState::ListView,
        };
        self.set_status_message("View toggled".to_string(), Style::default().fg(Color::Yellow), Some(Duration::from_secs(3)));
    }

    pub fn set_status_message(&mut self, content: String, style: Style, duration: Option<Duration>) {
        self.status_message = Some(StatusMessage::new(content, style, duration));
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    fn clear_expired_status_message(&mut self) {
        if let Some(status) = &self.status_message {
            if status.is_expired() {
                self.status_message = None;
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.size());

        let title = Paragraph::new("GitHub Secrets Manager")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        match self.app_state {
            AppState::ListView => self.render_secrets_list(f, chunks[1]),
            AppState::DetailsView => self.render_secret_details(f, chunks[1]),
        }

        let footer = match self.app_state {
            AppState::ListView => "↑↓: Navigate | Enter: View Details | q: Quit",
            AppState::DetailsView => "Enter: Back to List | q: Quit",
        };

        let footer = Paragraph::new(footer)
            .style(Style::default().fg(Color::LightCyan))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);

        if let Some(status) = &self.status_message {
            let status_widget = Paragraph::new(status.content.clone())
                .style(status.style)
                .block(Block::default());
            f.render_widget(status_widget, chunks[3]);
        }
    }

    fn render_secrets_list(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let secrets: Vec<ListItem> = self
            .secrets_manager
            .get_secrets()
            .iter()
            .map(|s| {
                let color = match s.status {
                    Some(SecretStatus::New) => self.color_scheme.new,
                    Some(SecretStatus::Existing) => self.color_scheme.existing,
                    Some(SecretStatus::Deleted) => self.color_scheme.deleted,
                    None => self.color_scheme.existing, // Default to existing if status is None
                };
                ListItem::new(Span::styled(
                    &s.name,
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ))
            })
            .collect();

        let secrets_list = List::new(secrets)
            .block(Block::default().borders(Borders::ALL).title("Secrets"))
            .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD));

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        f.render_stateful_widget(secrets_list, area, &mut list_state);
    }

    fn render_secret_details(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        if let Some(secret_details) = self.secrets_manager.get_secret_details(self.selected_index) {
            let status_color = match secret_details.status {
                SecretStatus::New => self.color_scheme.new,
                SecretStatus::Existing => self.color_scheme.existing,
                SecretStatus::Deleted => self.color_scheme.deleted,
            };

            let details = Text::from(vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&secret_details.name),
                ]),
                Line::from(vec![
                    Span::styled("Value: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&secret_details.value),
                ]),
                Line::from(vec![
                    Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{:?}", secret_details.status),
                        Style::default().fg(status_color),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Created At: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&secret_details.created_at),
                ]),
                Line::from(vec![
                    Span::styled("Updated At: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&secret_details.updated_at),
                ]),
            ]);

            let details_widget = Paragraph::new(details)
                .block(Block::default().borders(Borders::ALL).title("Secret Details"))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(details_widget, area);
        } else {
            let error_message = Paragraph::new("Secret details not available")
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title("Error"));
            f.render_widget(error_message, area);
        }
    }
}
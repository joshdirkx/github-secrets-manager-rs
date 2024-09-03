use std::time::{Duration, Instant};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect, Margin},
    style::{Color, Modifier, Style},
    text::{Span, Text, Line},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Clear},
    Frame, Terminal,
};
use std::io;
use crate::core::{SecretsManager, SecretStatus};

#[derive(Clone)]
enum AppState {
    ListView,
    DetailsView,
}

struct StatusMessage {
    content: String,
    style: Style,
    expiry: Option<Instant>,
}

struct ColorScheme {
    new: Color,
    existing: Color,
    deleted: Color,
}

struct ConfirmationDialog {
    message: String,
    yes_text: String,
    no_text: String,
}

pub struct Tui<'a> {
    secrets_manager: &'a dyn SecretsManager,
    selected_index: usize,
    app_state: AppState,
    status_message: Option<StatusMessage>,
    color_scheme: ColorScheme,
    confirmation_dialog: Option<ConfirmationDialog>,
}

impl<'a> Tui<'a> {
    pub fn new(secrets_manager: &'a dyn SecretsManager) -> Self {
        Self {
            secrets_manager,
            selected_index: 0,
            app_state: AppState::ListView,
            status_message: None,
            color_scheme: ColorScheme { new: Color::Green, existing: Color::White, deleted: Color::Red },
            confirmation_dialog: None,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;
            if let Event::Key(key) = event::read()? {
                if self.handle_input(key.code) {
                    return Ok(());
                }
            }
            self.clear_expired_status_message();
        }
    }

    fn handle_input(&mut self, key: KeyCode) -> bool {
        match (self.app_state.clone(), key) {
            (_, KeyCode::Char('q')) => self.show_confirmation_dialog("Are you sure you want to quit?", "Yes", "No"),
            (AppState::ListView, KeyCode::Up) => self.move_selection(-1),
            (AppState::ListView, KeyCode::Down) => self.move_selection(1),
            (_, KeyCode::Enter) => self.toggle_view(),
            (_, KeyCode::Char('y')) | (_, KeyCode::Char('Y')) if self.confirmation_dialog.is_some() => {
                self.hide_confirmation_dialog();
                return true;
            },
            (_, KeyCode::Char('n')) | (_, KeyCode::Char('N')) if self.confirmation_dialog.is_some() => self.hide_confirmation_dialog(),
            _ => {}
        }
        false
    }

    fn move_selection(&mut self, delta: isize) {
        let len = self.secrets_manager.get_secrets().len();
        self.selected_index = (self.selected_index as isize + delta).rem_euclid(len as isize) as usize;
    }

    fn toggle_view(&mut self) {
        self.app_state = match self.app_state {
            AppState::ListView => AppState::DetailsView,
            AppState::DetailsView => AppState::ListView,
        };
        self.set_status_message("View toggled", Style::default().fg(Color::Yellow), Some(Duration::from_secs(3)));
    }

    pub fn set_status_message(&mut self, content: impl Into<String>, style: Style, duration: Option<Duration>) {
        self.status_message = Some(StatusMessage {
            content: content.into(),
            style,
            expiry: duration.map(|d| Instant::now() + d),
        });
    }

    fn clear_expired_status_message(&mut self) {
        if let Some(status) = &self.status_message {
            if status.expiry.map_or(false, |expiry| Instant::now() > expiry) {
                self.status_message = None;
            }
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3), Constraint::Length(1)])
            .split(f.size());

        f.render_widget(self.title_widget(), chunks[0]);
        match self.app_state {
            AppState::ListView => self.render_secrets_list(f, chunks[1]),
            AppState::DetailsView => self.render_secret_details(f, chunks[1]),
        }
        f.render_widget(self.footer_widget(), chunks[2]);
        if let Some(status) = &self.status_message {
            f.render_widget(Paragraph::new(status.content.clone()).style(status.style), chunks[3]);
        }
        if let Some(dialog) = &self.confirmation_dialog {
            self.render_confirmation_dialog(f, dialog);
        }
    }

    fn title_widget(&self) -> Paragraph {
        Paragraph::new("GitHub Secrets Manager")
            .style(Style::default().fg(Color::Cyan))
            .block(Block::default().borders(Borders::ALL))
    }

    fn footer_widget(&self) -> Paragraph {
        let footer_text = match self.app_state {
            AppState::ListView => "↑↓: Navigate | Enter: View Details | q: Quit",
            AppState::DetailsView => "Enter: Back to List | q: Quit",
        };
        Paragraph::new(footer_text)
            .style(Style::default().fg(Color::LightCyan))
            .block(Block::default().borders(Borders::ALL))
    }

    fn render_secrets_list(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self.secrets_manager.get_secrets().iter()
            .map(|s| {
                let color = match s.status {
                    Some(SecretStatus::New) => self.color_scheme.new,
                    Some(SecretStatus::Existing) => self.color_scheme.existing,
                    Some(SecretStatus::Deleted) => self.color_scheme.deleted,
                    None => self.color_scheme.existing,
                };
                ListItem::new(Span::styled(&s.name, Style::default().fg(color).add_modifier(Modifier::BOLD)))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Secrets"))
            .highlight_style(Style::default().bg(Color::LightGreen).add_modifier(Modifier::BOLD));

        let mut state = ListState::default();
        state.select(Some(self.selected_index));
        f.render_stateful_widget(list, area, &mut state);
    }

    fn render_secret_details(&self, f: &mut Frame, area: Rect) {
        if let Some(secret_details) = self.secrets_manager.get_secret_details(self.selected_index) {
            let status_color = match secret_details.status {
                SecretStatus::New => self.color_scheme.new,
                SecretStatus::Existing => self.color_scheme.existing,
                SecretStatus::Deleted => self.color_scheme.deleted,
            };

            let details = Text::from(vec![
                Line::from(vec![Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&secret_details.name)]),
                Line::from(vec![Span::styled("Value: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&secret_details.value)]),
                Line::from(vec![Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)), Span::styled(format!("{:?}", secret_details.status), Style::default().fg(status_color))]),
                Line::from(vec![Span::styled("Created At: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&secret_details.created_at)]),
                Line::from(vec![Span::styled("Updated At: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&secret_details.updated_at)]),
            ]);

            f.render_widget(
                Paragraph::new(details)
                    .block(Block::default().borders(Borders::ALL).title("Secret Details"))
                    .wrap(ratatui::widgets::Wrap { trim: true }),
                area
            );
        } else {
            f.render_widget(
                Paragraph::new("Secret details not available")
                    .style(Style::default().fg(Color::Red))
                    .block(Block::default().borders(Borders::ALL).title("Error")),
                area
            );
        }
    }

    fn render_confirmation_dialog(&self, f: &mut Frame, dialog: &ConfirmationDialog) {
        let area = self.centered_rect(60, 20, f.size());
        f.render_widget(Clear, area);
        f.render_widget(Block::default().borders(Borders::ALL).style(Style::default().bg(Color::DarkGray)), area);

        let inner_area = area.inner(Margin::new(1, 1));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
            .split(inner_area);

        f.render_widget(Paragraph::new(&*dialog.message).alignment(ratatui::layout::Alignment::Center), chunks[0]);

        let options = Line::from(vec![
            Span::styled(format!("(Y) {}", dialog.yes_text), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("   "),
            Span::styled(format!("(N) {}", dialog.no_text), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        ]);
        f.render_widget(Paragraph::new(options).alignment(ratatui::layout::Alignment::Center), chunks[2]);
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage((100 - percent_y) / 2), Constraint::Percentage(percent_y), Constraint::Percentage((100 - percent_y) / 2)])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage((100 - percent_x) / 2), Constraint::Percentage(percent_x), Constraint::Percentage((100 - percent_x) / 2)])
            .split(popup_layout[1])[1]
    }

    pub fn show_confirmation_dialog(&mut self, message: impl Into<String>, yes_text: impl Into<String>, no_text: impl Into<String>) {
        self.confirmation_dialog = Some(ConfirmationDialog {
            message: message.into(),
            yes_text: yes_text.into(),
            no_text: no_text.into(),
        });
    }

    pub fn hide_confirmation_dialog(&mut self) {
        self.confirmation_dialog = None;
    }
}
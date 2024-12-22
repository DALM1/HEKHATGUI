use crossterm::{
  event::{self, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use reqwest::Client;
use serde_json::json;
use std::{io::{self, Write}, time::Duration};
use tui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout},
  widgets::{Block, Borders, Paragraph},
  Terminal,
};

// Log file
const LOG_FILE: &str = "client_debug.log";
const ASCII_ART: &str = r#"
▄████████    ▄████████  ███     █▄   ▄██████▄       ███  v1.0
███    ███   ███    ███ ███    ███ ███    ███  ▀█████████▄
███    █▀    ███    ███ ███    ███ ███    ███     ▀███▀▀██
▄███▄▄▄      ▄███▄▄▄▄██▀███    ███ ███    ███      ███   ▀
▀▀███▀▀▀     ▀▀███▀▀▀▀▀ ███    ███ ███    ███     ████▀
███    █▄  ▀███████████ ███    ███ ███    ███      ███
███    ███   ███    ███ ███    ███ ███    ███      ███
██████████   ███    ███ ████████▀   ▀██████▀      ▄████▀
▀▀▀    ███ by DALM†™
"#;

fn log_debug(message: &str) -> io::Result<()> {
  let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
  let log_message = format!("[{}] {}\n", timestamp, message);

  let mut file = std::fs::OpenOptions::new()
      .create(true)
      .append(true)
      .open(LOG_FILE)?;

  file.write_all(log_message.as_bytes())?;
  Ok(())
}

struct UIState {
  username: String,
  password: String,
  collecting_username: bool,
  error_message: Option<String>,
}

pub async fn run_client() -> Result<(), Box<dyn std::error::Error>> {
  log_debug("Starting client application")?;

  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen)?;

  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  let mut state = UIState {
      username: String::new(),
      password: String::new(),
      collecting_username: true,
      error_message: None,
  };

  loop {
      terminal.draw(|f| {
          let chunks = Layout::default()
              .direction(Direction::Vertical)
              .constraints([
                  Constraint::Percentage(40),
                  Constraint::Percentage(30),
                  Constraint::Percentage(30),
              ])
              .split(f.size());

          let ascii_art = Paragraph::new(ASCII_ART)
              .block(Block::default().borders(Borders::ALL));

          let input_message = if state.collecting_username {
              "Enter your username:"
          } else {
              "Enter your password:"
          };

          let input_content = if state.collecting_username {
              state.username.clone()
          } else {
              "*".repeat(state.password.len())
          };

          let input_widget = Paragraph::new(input_content)
              .block(Block::default().borders(Borders::ALL).title(input_message));

          let error_widget = if let Some(error) = &state.error_message {
              Paragraph::new(error.as_str())
                  .block(Block::default().borders(Borders::ALL).title("Error"))
          } else {
              Paragraph::new("")
                  .block(Block::default().borders(Borders::ALL))
          };

          f.render_widget(ascii_art, chunks[0]);
          f.render_widget(input_widget, chunks[1]);
          f.render_widget(error_widget, chunks[2]);
      })?;

      if let Event::Key(key) = event::read()? {
          match key.code {
              KeyCode::Char(c) => {
                  if state.collecting_username {
                      state.username.push(c);
                  } else {
                      state.password.push(c);
                  }
                  state.error_message = None;
              }
              KeyCode::Backspace => {
                  if state.collecting_username {
                      state.username.pop();
                  } else {
                      state.password.pop();
                  }
              }
              KeyCode::Enter => {
                  if state.collecting_username {
                      if state.username.is_empty() {
                          state.error_message = Some("Username cannot be empty".to_string());
                          continue;
                      }
                      state.collecting_username = false;
                  } else {
                      if state.password.is_empty() {
                          state.error_message = Some("Password cannot be empty".to_string());
                          continue;
                      }
                      break;
                  }
              }
              KeyCode::Esc => {
                  log_debug("User cancelled login")?;
                  cleanup_terminal()?;
                  return Ok(());
              }
              _ => {}
          }
      }
  }

  log_debug(&format!("Attempting login for user: {}", state.username))?;

  let client = Client::builder()
      .timeout(Duration::from_secs(10))
      .build()?;

  let response = client
      .post("http://127.0.0.1:4000/api/login")
      .json(&json!({
          "username": state.username,
          "password": state.password,
      }))
      .send()
      .await;

  match response {
      Ok(resp) => {
          if resp.status().is_success() {
              log_debug("Login successful")?;
              cleanup_terminal()?;
              println!("Login successful!");
          } else {
              let error_msg = format!("Login failed with status: {}", resp.status());
              log_debug(&error_msg)?;
              cleanup_terminal()?;
              println!("Invalid credentials. Please try again.");
          }
      }
      Err(err) => {
          let error_msg = format!("Connection error: {}", err);
          log_debug(&error_msg)?;
          cleanup_terminal()?;
          eprintln!("Failed to connect to server: {}", err);
      }
  }

  Ok(())
}

fn cleanup_terminal() -> io::Result<()> {
  disable_raw_mode()?;
  execute!(io::stdout(), LeaveAlternateScreen)?;
  Ok(())
}

use crossterm::{
  event::{self, Event, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout},
  widgets::{Block, Borders, Paragraph},
  Terminal,
};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::TcpStream,
};

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

pub async fn run_client() {
  let client = match TcpStream::connect("127.0.0.1:4000").await {
      Ok(c) => c,
      Err(e) => {
          eprintln!("Failed to connect to server: {}", e);
          return;
      }
  };

  let client = Arc::new(Mutex::new(client));

  enable_raw_mode().unwrap();
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen).unwrap();
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend).unwrap();

  let messages = Arc::new(Mutex::new(vec!["Welcome to hхеeкkхhаaтt(Hekat Bruof)".to_string()]));
  let input = Arc::new(Mutex::new(String::new()));

  let messages_clone = Arc::clone(&messages);
  let client_clone = Arc::clone(&client);
  tokio::spawn(async move {
      let mut buffer = [0; 1024];
      loop {
          let mut client = client_clone.lock().await;
          match client.read(&mut buffer).await {
              Ok(0) => break,
              Ok(n) => {
                  let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                  let mut messages = messages_clone.lock().await;
                  messages.push(msg);
              }
              Err(e) => {
                  eprintln!("Failed to read from server: {}", e);
                  break;
              }
          }
      }
  });

  loop {
      let messages_to_display = {
          let messages = messages.lock().await;
          messages.clone()
      };

      let input_to_display = {
          let input = input.lock().await;
          input.clone()
      };

      terminal
          .draw(|f| {
              let chunks = Layout::default()
                  .direction(Direction::Vertical)
                  .constraints(
                      [
                          Constraint::Percentage(70),
                          Constraint::Percentage(10),
                          Constraint::Percentage(20),
                      ]
                      .as_ref(),
                  )
                  .split(f.size());

              let ascii_art =
                  Paragraph::new(ASCII_ART).block(Block::default().borders(Borders::ALL));

              let messages_widget = Paragraph::new(messages_to_display.join("\n"))
                  .block(Block::default().borders(Borders::ALL).title("Messages"));

              let input_widget = Paragraph::new(input_to_display)
                  .block(Block::default().borders(Borders::ALL).title("Type your message"));

              f.render_widget(ascii_art, chunks[0]);
              f.render_widget(messages_widget, chunks[1]);
              f.render_widget(input_widget, chunks[2]);
          })
          .unwrap();

      if let Event::Key(key) = event::read().unwrap() {
          match key.code {
              KeyCode::Char(c) => {
                  let mut input = input.lock().await;
                  input.push(c);
              }
              KeyCode::Backspace => {
                  let mut input = input.lock().await;
                  input.pop();
              }
              KeyCode::Enter => {
                  let mut input = input.lock().await;
                  let msg = input.clone();
                  input.clear();

                  let mut client = client.lock().await;
                  if let Err(e) = client.write_all(msg.as_bytes()).await {
                      eprintln!("Failed to send message: {}", e);
                  }
              }
              KeyCode::Char('q') => break,
              _ => {}
          }
      }
  }

  disable_raw_mode().unwrap();
  execute!(io::stdout(), LeaveAlternateScreen).unwrap();
}

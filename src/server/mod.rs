use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    sync::Arc,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, RwLock},
};

#[derive(Serialize, Deserialize)]
struct User {
    username: String,
    id: u16,
}

type Clients = Arc<RwLock<HashMap<String, mpsc::Sender<String>>>>;

pub async fn run_server() {
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

    let listener = TcpListener::bind("0.0.0.0:3630")
        .await
        .expect("Failed to bind port, Ensure no other process is using it.");

    println!("AURA running lightning fast on port 3630...");

    loop {
        if let Ok((socket, _)) = listener.accept().await {
            let clients = clients.clone();
            tokio::spawn(async move {
                handle_connection(socket, clients).await;
            });
        }
    }
}

async fn handle_connection(socket: tokio::net::TcpStream, clients: Clients) {
    let (reader, mut writer) = tokio::io::split(socket);
    let (tx, mut rx) = mpsc::channel::<String>(100);

    let username = assign_id_and_load_username().unwrap_or_else(|| "Guest0".to_string());

    {
        let mut clients_write = clients.write().await;
        clients_write.insert(username.clone(), tx);
    }

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            writer.write_all(msg.as_bytes()).await.unwrap();
        }
    });

    let mut lines = tokio::io::BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let command_parts: Vec<&str> = line.splitn(2, ' ').collect();

        match command_parts[0] {
            "/msg" => {
                if command_parts.len() == 2 {
                    let args: Vec<&str> = command_parts[1].splitn(2, ' ').collect();
                    if args.len() == 2 {
                        let target_user = args[0];
                        let private_message = args[1];

                        let clients_read = clients.read().await;
                        if let Some(sender) = clients_read.get(target_user) {
                            let _ = sender
                                .send(format!("[Private from {}]: {}", username, private_message))
                                .await;
                        }
                    }
                }
            }
            "/channel" => {
                if command_parts.len() == 2 {
                    let channel_name = command_parts[1];
                    let clients_read = clients.read().await;
                    for (name, sender) in clients_read.iter() {
                        if name != &username {
                            let _ = sender
                                .send(format!("[{} joined channel {}]", username, channel_name))
                                .await;
                        }
                    }
                }
            }
            _ => {
                let broadcast_clients = clients.read().await;
                for (name, sender) in broadcast_clients.iter() {
                    if name != &username {
                        let _ = sender.send(format!("[{}]: {}", username, line)).await;
                    }
                }
            }
        }
    }

    {
        let mut clients_write = clients.write().await;
        clients_write.remove(&username);
    }
}

fn assign_id_and_load_username() -> Option<String> {
    let mut users = load_users();
    let ip_address = "127.0.0.1";

    if let Some(user) = users.get(ip_address) {
        return Some(user.username.clone());
    }

    let mut rng = rand::thread_rng();
    let new_id = rng.gen::<u16>();
    let username = format!("Guest{}", new_id);
    users.insert(ip_address.to_string(), User { username: username.clone(), id: new_id });
    save_users(&users);
    Some(username)
}

fn load_users() -> HashMap<String, User> {
    let file_path = "users.json";
    let mut file = match OpenOptions::new().read(true).create(true).write(true).open(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open {}: {}", file_path, e);
            return HashMap::new();
        }
    };

    let mut data = String::new();
    if file.read_to_string(&mut data).is_err() || data.trim().is_empty() {
        return HashMap::new();
    }

    match serde_json::from_str(&data) {
        Ok(users) => users,
        Err(_) => {
            eprintln!("Failed to parse {}: Invalid JSON format. Starting with empty data.", file_path);
            HashMap::new()
        }
    }
}

fn save_users(users: &HashMap<String, User>) {
    let file_path = "users.json";
    let mut file = match File::create(file_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create {}: {}", file_path, e);
            return;
        }
    };

    let data = match serde_json::to_string(users) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Failed to serialize users: {}", e);
            return;
        }
    };

    if let Err(e) = file.write_all(data.as_bytes()) {
        eprintln!("Failed to write to {}: {}", file_path, e);
    }
}

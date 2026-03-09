pub mod network;
pub mod capture;
pub mod input;
pub mod audio;
pub mod tunnel;
pub mod connection;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use serde::Serialize;
use std::sync::Mutex;
use tauri::{State, Manager};

#[derive(Serialize)]
struct LocalInfo {
    id: String,
    password: String,
}

struct AppState {
    local_id: Mutex<String>,
    local_password: Mutex<String>,
    public_address: Mutex<String>,
    input_tx: Mutex<Option<tokio::sync::mpsc::UnboundedSender<InputEvent>>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct InputEvent {
    pub event_type: String,
    pub payload: String,
}

#[tauri::command]
fn get_local_info(state: State<AppState>) -> LocalInfo {
    let id = state.local_id.lock().unwrap().clone();
    let pw = state.local_password.lock().unwrap().clone();
    LocalInfo { id, password: pw }
}

#[tauri::command]
fn get_tunnel_address(state: State<AppState>) -> String {
    state.public_address.lock().unwrap().clone()
}

#[tauri::command]
fn handle_input_event(event_type: String, payload: String, state: State<AppState>) {
    if let Some(tx) = state.input_tx.lock().unwrap().as_ref() {
        let _ = tx.send(InputEvent { event_type, payload });
    }
}

#[tauri::command]
async fn start_connection(remote_id: String, port: u16, password: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    // Strip all whitespace
    let remote_id = remote_id.replace(" ", "").trim().to_string();
    
    // Address resolution
    let addr = if remote_id.contains(':') {
        // Full address provided
        remote_id.clone()
    } else if remote_id.contains('.') {
        // IP address or domain
        format!("{}:{}", remote_id, port)
    } else if remote_id.len() == 6 {
        // Standard 6-digit ID
        format!("bore.pub:{}", port)
    } else {
        // Fallback or local
        format!("127.0.0.1:{}", port)
    };
    
    println!("Connecting to: {}", addr);

    // Spawn a client connection task
    tokio::spawn(async move {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        {
            let state = app_handle.state::<AppState>();
            let mut tx_lock = state.input_tx.lock().unwrap();
            *tx_lock = Some(tx);
        }
        let _ = connection::ConnectionManager::start_client(addr, password, app_handle, rx).await;
    });

    Ok(format!("Connecting to {}", remote_id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let local_id_val = format!("{:06}", rand::random::<u32>() % 900_000 + 100_000);
    let local_id = local_id_val.clone();
    let local_password = format!("{:04X}", rand::random::<u16>());

    tauri::Builder::default()
        .manage(AppState {
            local_id: Mutex::new(local_id),
            local_password: Mutex::new(local_password),
            public_address: Mutex::new("Starting tunnel...".to_string()),
            input_tx: Mutex::new(None),
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_local_info, get_tunnel_address, start_connection, handle_input_event])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = app.state::<AppState>();
            let local_id_str = state.local_id.lock().unwrap().clone();
            let pw_str = state.local_password.lock().unwrap().clone();
            
            tokio::spawn(async move {
                let parsed_local_id: u64 = local_id_str.parse().unwrap_or(0);
                let host_port = 10000 + (parsed_local_id % 55535) as u16;
                
                // Spawn host server
                let pw_host = pw_str.clone();
                tokio::spawn(async move {
                    let _ = connection::ConnectionManager::start_host(host_port, pw_host).await;
                });

                // Start Tunnel
                let manager = tunnel::TunnelManager::new();
                if let Ok((host, port)) = manager.start(host_port).await {
                    let full_addr = format!("{}:{}", host, port);
                    println!("Tunnel started: {}", full_addr);
                    
                    // Store in state
                    let state = handle.state::<AppState>();
                    let mut addr_lock = state.public_address.lock().unwrap();
                    *addr_lock = full_addr.clone();
                    drop(addr_lock);

                    use tauri::Emitter;
                    let _ = handle.emit("tunnel-ready", full_addr);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

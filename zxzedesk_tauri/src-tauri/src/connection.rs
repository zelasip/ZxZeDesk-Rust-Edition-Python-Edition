use tokio::net::{TcpListener, TcpStream};
use crate::network::{send_message, recv_message, MsgType};
use crate::capture::capture_screen;
use crate::input::{handle_mouse_event, handle_key_event};
use crate::InputEvent;
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
struct FramePayload {
    data: Vec<u8>,
}

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn start_host(port: u16, password: String) -> Result<(), String> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                // Auth check
                if let Ok(Some((MsgType::Auth, payload))) = recv_message(&mut socket).await {
                    let received_pass = String::from_utf8_lossy(&payload).to_string();
                    if received_pass == password {
                        let _ = send_message(&mut socket, MsgType::AuthOk, &[]).await;
                        
                        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(30));
                        loop {
                            tokio::select! {
                                _ = interval.tick() => {
                                    if let Ok(frame) = capture_screen() {
                                        if send_message(&mut socket, MsgType::Frame, &frame).await.is_err() {
                                            break;
                                        }
                                    }
                                }
                                msg = recv_message(&mut socket) => {
                                    match msg {
                                        Ok(Some((MsgType::MouseEvent, payload))) => handle_mouse_event(&payload),
                                        Ok(Some((MsgType::KeyEvent, payload))) => handle_key_event(&payload),
                                        Ok(None) | Err(_) => break,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    } else {
                        let _ = send_message(&mut socket, MsgType::AuthFail, &[]).await;
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn start_client(
        addr: String, 
        password: String, 
        app_handle: tauri::AppHandle,
        mut input_rx: tokio::sync::mpsc::UnboundedReceiver<InputEvent>
    ) -> Result<(), String> {
        let _ = app_handle.emit("connection-status", "Connecting");
        
        let mut stream = match TcpStream::connect(&addr).await {
            Ok(s) => s,
            Err(e) => {
                let _ = app_handle.emit("connection-status", "Failed");
                return Err(e.to_string());
            }
        };

        // Send Auth
        if let Err(e) = send_message(&mut stream, MsgType::Auth, password.as_bytes()).await {
             let _ = app_handle.emit("connection-status", "Failed");
             return Err(e.to_string());
        }

        // Wait for AuthOk
        match recv_message(&mut stream).await {
            Ok(Some((MsgType::AuthOk, _))) => {
                let _ = app_handle.emit("connection-status", "Success");
                
                loop {
                    tokio::select! {
                        msg = recv_message(&mut stream) => {
                            if let Ok(Some((MsgType::Frame, payload))) = msg {
                                let _ = app_handle.emit("video-frame", FramePayload { data: payload.to_vec() });
                            } else {
                                break;
                            }
                        }
                        Some(event) = input_rx.recv() => {
                            let msg_type = if event.event_type.starts_with("mouse") { MsgType::MouseEvent } else { MsgType::KeyEvent };
                            let payload = serde_json::to_vec(&event).unwrap_or_default();
                            if send_message(&mut stream, msg_type, &payload).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Ok(Some((MsgType::AuthFail, _))) => {
                let _ = app_handle.emit("connection-status", "Failed");
                return Err("Authentication failed".into());
            }
            _ => {
                let _ = app_handle.emit("connection-status", "Failed");
                return Err("Unexpected response from server".into());
            }
        }
        
        Ok(())
    }
}

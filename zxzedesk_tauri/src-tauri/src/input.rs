use enigo::{Enigo, MouseControllable, KeyboardControllable, MouseButton, Key};
use crate::InputEvent;
use serde_json::Value;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref ENIGO: Mutex<Enigo> = Mutex::new(Enigo::new());
}

pub fn handle_mouse_event(payload: &[u8]) {
    let event: Result<InputEvent, _> = serde_json::from_slice(payload);
    if let Ok(event) = event {
        let data: Value = serde_json::from_str(&event.payload).unwrap_or(Value::Null);
        let mut enigo = ENIGO.lock().unwrap();

        match event.event_type.as_str() {
            "mouse_move" => {
                if let (Some(x), Some(y)) = (data["x"].as_f64(), data["y"].as_f64()) {
                    let (w, h) = enigo.main_display_size();
                    enigo.mouse_move_to((x * w as f64) as i32, (y * h as f64) as i32);
                }
            }
            "mouse_down" => {
                let btn = match data["button"].as_i64() {
                    Some(0) => MouseButton::Left,
                    Some(1) => MouseButton::Middle,
                    Some(2) => MouseButton::Right,
                    _ => MouseButton::Left,
                };
                enigo.mouse_down(btn);
            }
            "mouse_up" => {
                let btn = match data["button"].as_i64() {
                    Some(0) => MouseButton::Left,
                    Some(1) => MouseButton::Middle,
                    Some(2) => MouseButton::Right,
                    _ => MouseButton::Left,
                };
                enigo.mouse_up(btn);
            }
            _ => {}
        }
    }
}

pub fn handle_key_event(payload: &[u8]) {
    let event: Result<InputEvent, _> = serde_json::from_slice(payload);
    if let Ok(event) = event {
        let data: Value = serde_json::from_str(&event.payload).unwrap_or(Value::Null);
        let mut enigo = ENIGO.lock().unwrap();

        if let Some(key_str) = data["key"].as_str() {
            let key = match key_str {
                "Enter" => Key::Return,
                "Backspace" => Key::Backspace,
                "Escape" => Key::Escape,
                "Control" => Key::Control,
                "Shift" => Key::Shift,
                "Alt" => Key::Alt,
                "Meta" => Key::Meta,
                _ if key_str.len() == 1 => Key::Layout(key_str.chars().next().unwrap()),
                _ => return, 
            };

            if event.event_type == "key_down" {
                enigo.key_down(key);
            } else {
                enigo.key_up(key);
            }
        }
    }
}

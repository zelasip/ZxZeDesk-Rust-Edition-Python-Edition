use std::process::Command;
use std::process::Stdio;
use std::io::{BufReader, BufRead};
use tokio::task;
use regex::Regex;

pub struct TunnelManager {
    // Scaffold for managing bore.exe
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&self, local_port: u16) -> Result<(String, u16), String> {
        // Find bore.exe
        let current_exe = std::env::current_exe().unwrap_or_default();
        let exe_dir = current_exe.parent().unwrap_or(&std::env::current_dir().unwrap_or_default()).to_path_buf();
        
        let mut bore_path = None;
        
        // Search strategy:
        // 1. Same directory as current exe
        // 2. "ZxZeDesk Python Version/bore.exe" in parent directories
        
        if exe_dir.join("bore.exe").exists() {
            bore_path = Some(exe_dir.join("bore.exe"));
        } else {
            let mut check_dir = Some(exe_dir.as_path());
            while let Some(dir) = check_dir {
                let candidate = dir.join("ZxZeDesk Python Version").join("bore.exe");
                if candidate.exists() {
                    bore_path = Some(candidate);
                    break;
                }
                check_dir = dir.parent();
            }
        }

        let bore_path = bore_path.ok_or_else(|| "bore.exe not found in any expected location".to_string())?;

        let mut child = Command::new(&bore_path)
            .arg("local")
            .arg(local_port.to_string())
            .arg("--to")
            .arg("bore.pub")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;

        let stdout = child.stdout.take().unwrap();
        
        let (tx, rx) = tokio::sync::oneshot::channel();

        task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            let re = Regex::new(r"bore\.pub:(\d+)").unwrap();

            for line in reader.lines() {
                if let Ok(line) = line {
                    if let Some(caps) = re.captures(&line) {
                        if let Ok(port) = caps[1].parse::<u16>() {
                            let _ = tx.send(Ok(("bore.pub".to_string(), port)));
                            return;
                        }
                    } else if line.to_lowercase().contains("error") && !line.to_lowercase().contains("pipe closed") {
                        // Only report errors that aren't "pipe closed" (which often happens during normal startup checks)
                        // and ensure it's not a successful line that happens to contain "error"
                        let _ = tx.send(Err(line));
                        return;
                    }
                }
            }
        });

        rx.await.map_err(|e| e.to_string())?
    }
}

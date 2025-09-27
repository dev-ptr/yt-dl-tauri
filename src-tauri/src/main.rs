#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Window;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![download_url, quit_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::Emitter;

#[tauri::command]
fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}
#[tauri::command]
async fn download_url(window: Window, url: String, f_path: String, mp3_only: bool) -> Result<(), String> {

    let output_template = format!("{}/%(title)s.%(ext)s", f_path);

    // Build yt-dlp args
    let mut args = vec!["-o", &output_template, &url];
    if mp3_only {
        args = vec!["-x", "--audio-format", "mp3", "-o", &output_template, &url];
    }

    let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp process: {}", e))?;

    // Capture stdout for progress
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    // Send full log line to frontend
                    let _ = window.emit("download-log", line.clone());

                    // Optionally parse progress % from line like:
                    // [download]  45.3% of 3.45MiB at 1.23MiB/s ETA 00:10
                    if let Some(percent) = parse_progress_percent(&line) {
                        let _ = window.emit("download-progress", percent);
                    }
                }
                Err(e) => {
                    let _ = window.emit("download-error", format!("Error reading output: {}", e));
                }
            }
        }
    }

    // Wait for process to exit
    let status = child
        .wait()
        .map_err(|e| format!("Failed to wait on process: {}", e))?;

    let code = status.code().unwrap_or(-1);
    if code == 0 {
        let _ = window.emit("download-complete", code);
        Ok(())
    } else {
        let _ = window.emit(
            "download-error",
            format!("yt-dlp exited with code {}", code),
        );
        Err(format!("yt-dlp exited with code {}", code))
    }
}

// Helper to parse progress percentage from a yt-dlp output line
fn parse_progress_percent(line: &str) -> Option<u8> {
    if line.contains("[download]") && line.contains('%') {
        // Example line: "[download]  45.3% of 3.45MiB at 1.23MiB/s ETA 00:10"
        let start = line.find(']').unwrap_or(0) + 1;
        let slice = &line[start..];
        for word in slice.split_whitespace() {
            if word.ends_with('%') {
                let percent_str = word.trim_end_matches('%');
                if let Ok(p) = percent_str.parse::<f32>() {
                    return Some(p.round() as u8);
                }
            }
        }
    }
    None
}

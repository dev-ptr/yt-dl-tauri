#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod config;
use config::{ConfigManager, UserConfig};

use tauri::Window;
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())  
        .invoke_handler(tauri::generate_handler![
            download_url, 
            quit_app,
            get_config,
            update_config,
            get_download_dir,
            fetch_video_title
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

use scraper::{Html, Selector};
use std::time::Duration;

#[tauri::command]
fn fetch_video_title(url: String) -> Result<String, String> {
    println!("Fetching URL: {}", url); 
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let body = client
        .get(&url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;

    let document = Html::parse_document(&body);

    let selector = Selector::parse(r#"meta[property="og:title"]"#).unwrap();
    if let Some(element) = document.select(&selector).next() {
        if let Some(content) = element.value().attr("content") {
            return Ok(content.to_string());
        }
    }

    Ok(url)
}

#[tauri::command]
fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> Result<UserConfig, String> {
    ConfigManager::load_config(&app_handle)
}

#[tauri::command]
fn update_config(app_handle: tauri::AppHandle, new_config: UserConfig) -> Result<(), String> {
    ConfigManager::save_config(&app_handle, &new_config)
}

#[tauri::command]
fn get_download_dir(app_handle: tauri::AppHandle) -> Result<String, String> {
    ConfigManager::get_download_dir(&app_handle)
}
#[tauri::command]
async fn download_url(
    window: Window,
    url: String,
    f_path: String,
    mp3_only: bool,
    enable_playlist: bool,
    sponsorblock: bool
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};
    use tauri::Emitter;

    let output_template = format!("{}/%(title)s.%(ext)s", f_path);

    let mut args = vec![
        "--newline",
        "-o", &output_template,
        &url,
    ];

    if mp3_only {
        args.push("-x");
        args.push("--audio-format");
        args.push("mp3");
    }
    if enable_playlist { args.push("--yes-playlist"); } else { args.push("--no-playlist"); }
    if sponsorblock {
        args.push("--sponsorblock-remove");
        args.push("all");
    }
    let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;
    let _ = window.emit("download-progress", 0u8);

    // Read stdout for progress
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);

    let mut last_percent: u8 = 0; 
    let mut line = String::new();

    while reader.read_line(&mut line).map_err(|e| e.to_string())? > 0 {
        let l = line.trim_end().to_string();
        let _ = window.emit("download-log", l.clone());

        if let Some(p) = parse_progress_percent(&l) {
            let p = p.min(100);
            if p > last_percent {
                last_percent = p;
                let _ = window.emit("download-progress", p);
            }
        }

        line.clear();
    }

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn({
            let window = window.clone();
            move || {
                let err_reader = BufReader::new(stderr);
                for l in err_reader.lines().flatten() {
                    let _ = window.emit("download-log", l);
                }
            }
        });
    }

    let status = child.wait().map_err(|e| format!("wait failed: {}", e))?;
    let code = status.code().unwrap_or(-1);

    if code == 0 {
        let _ = window.emit("download-complete", code);
        Ok(())
    } else {
        let _ = window.emit("download-error", format!("yt-dlp exited with code {}", code));
        Err(format!("yt-dlp exited with code {}", code))
    }
}

fn parse_progress_percent(line: &str) -> Option<u8> {
    if line.contains("[download]") && line.contains('%') {
        let start = line.find(']').unwrap_or(0) + 1;
        let slice = &line[start..];
        for word in slice.split_whitespace() {
            if let Some(stripped) = word.strip_suffix('%') {
                if let Ok(p) = stripped.parse::<f32>() {
                    // use floor to reduce flicker
                    return Some(p.floor() as u8);
                }
            }
        }
    }
    None
}

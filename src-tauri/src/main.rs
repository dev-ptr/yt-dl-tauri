#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod config;
mod binary_manager;

use config::{ConfigManager, UserConfig};
use binary_manager::{BinaryManager, BinaryStatus};

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
            fetch_video_title,
            check_binaries,
            download_ytdlp,
            download_ffmpeg,
            download_all_binaries
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
    app_handle: tauri::AppHandle,
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

    // Get path to yt-dlp binary (prefers bundled, falls back to system)
    let status = BinaryManager::check_binaries(&app_handle)?;

    if !status.yt_dlp_installed {
        return Err("yt-dlp not found. Please install yt-dlp or download binaries.".to_string());
    }

    let ytdlp_path = status.yt_dlp_path.unwrap_or_else(|| "yt-dlp".to_string());

    let output_template = format!("{}/%(title)s.%(ext)s", f_path);

    let mut args = vec![
        "--newline",
        "-o", &output_template,
        &url,
    ];

    // Only pass --ffmpeg-location if we have a bundled ffmpeg (not system PATH)
    if let Some(ref ffmpeg_path) = status.ffmpeg_path {
        if !ffmpeg_path.eq("ffmpeg") {
            args.insert(1, "--ffmpeg-location");
            args.insert(2, ffmpeg_path);
        }
    }

    if mp3_only {
        args.push("-x");
        args.push("--audio-format");
        args.push("mp3");
    } else {
        // Force merge to MKV for video downloads
        args.push("--merge-output-format");
        args.push("mkv");
    }
    if enable_playlist { args.push("--yes-playlist"); } else { args.push("--no-playlist"); }
    if sponsorblock {
        args.push("--sponsorblock-remove");
        args.push("all");
    }
    let mut child = Command::new(&ytdlp_path)
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

        if l.contains("[download]") && l.contains("Destination:") {
            last_percent = 0;
        }

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
        // Clear macOS quarantine attribute from downloaded files
        #[cfg(target_os = "macos")]
        if let Err(e) = clear_quarantine_attr(&f_path) {
            eprintln!("Warning: Failed to clear quarantine attribute: {}", e);
        }

        let _ = window.emit("download-complete", code);
        Ok(())
    } else {
        let _ = window.emit("download-error", format!("yt-dlp exited with code {}", code));
        Err(format!("yt-dlp exited with code {}", code))
    }
}

/// Clears macOS Gatekeeper quarantine attribute from files in directory
#[cfg(target_os = "macos")]
fn clear_quarantine_attr(dir_path: &str) -> Result<(), String> {
    use std::process::Command;
    use std::path::Path;

    let path = Path::new(dir_path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", dir_path));
    }

    // Run: xattr -dr com.apple.quarantine <path>
    let output = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine", dir_path])
        .output()
        .map_err(|e| format!("Failed to execute xattr: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Non-zero exit is OK if attribute doesn't exist
        if !stderr.contains("No such xattr") {
            return Err(format!("xattr failed: {}", stderr));
        }
    }

    Ok(())
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

#[tauri::command]
fn check_binaries(app_handle: tauri::AppHandle) -> Result<BinaryStatus, String> {
    BinaryManager::check_binaries(&app_handle)
}

#[tauri::command]
async fn download_ytdlp(app_handle: tauri::AppHandle) -> Result<(), String> {
    BinaryManager::download_ytdlp(&app_handle).await
}

#[tauri::command]
async fn download_ffmpeg(app_handle: tauri::AppHandle) -> Result<(), String> {
    BinaryManager::download_ffmpeg(&app_handle).await
}

#[tauri::command]
async fn download_all_binaries(app_handle: tauri::AppHandle) -> Result<(), String> {
    BinaryManager::download_ytdlp(&app_handle).await?;
    BinaryManager::download_ffmpeg(&app_handle).await?;
    Ok(())
}

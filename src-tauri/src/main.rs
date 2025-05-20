#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
  )]
  use tauri::{Window, Emitter};
  use std::process::{Command, Stdio};
  use std::io::{BufRead, BufReader};
  use std::thread;
  
  #[tauri::command]
  fn quit_app(app_handle: tauri::AppHandle) {
    app_handle.exit(0);
  }
  
  #[tauri::command]
  fn download_url(window: Window, url: String, mp3_only: bool) {
    // Spawn a thread to avoid blocking UI
    thread::spawn(move || {
      let mut args = vec!["-o", "%(title)s.%(ext)s"];
      if mp3_only {
        args.push("-x");
        args.push("--audio-format");
        args.push("mp3");
      }
      args.push(&url);
  
      // Spawn yt-dlp process
      let mut child = Command::new("yt-dlp")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start yt-dlp process");
  
      let stdout = child.stdout.take().unwrap();
      let reader = BufReader::new(stdout);
  
      for line in reader.lines() {
        if let Ok(line) = line {
          // Emit each line back to frontend
          window.emit("download-log", line).unwrap();
        }
      }
  
      let status = child.wait().expect("failed to wait on yt-dlp");
      let code = status.code().unwrap_or(-1);
  
      window.emit("download-complete", code).unwrap();
    });
  }
  
  fn main() {
    tauri::Builder::default()
      .invoke_handler(tauri::generate_handler![download_url, quit_app])
      .run(tauri::generate_context!())
      .expect("error while running tauri application");
  }
  
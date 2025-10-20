use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use crate::config::ConfigManager;

pub struct BinaryManager;

#[derive(Debug, Clone, serde::Serialize)]
pub struct BinaryStatus {
    pub yt_dlp_installed: bool,
    pub ffmpeg_installed: bool,
    pub yt_dlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
}

impl BinaryManager {
    fn get_binaries_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;

        let binaries_dir = data_dir.join("binaries");

        if !binaries_dir.exists() {
            fs::create_dir_all(&binaries_dir)
                .map_err(|e| format!("Failed to create binaries directory: {}", e))?;
        }

        Ok(binaries_dir)
    }

    // Get expected binary name with platform-specific extension
    fn get_binary_name(name: &str) -> String {
        if cfg!(windows) {
            format!("{}.exe", name)
        } else {
            name.to_string()
        }
    }

    // Get path to yt-dlp binary
    pub fn get_ytdlp_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
        let binaries_dir = Self::get_binaries_dir(app_handle)?;
        Ok(binaries_dir.join(Self::get_binary_name("yt-dlp")))
    }

    // Get path to ffmpeg binary
    pub fn get_ffmpeg_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
        let binaries_dir = Self::get_binaries_dir(app_handle)?;
        Ok(binaries_dir.join(Self::get_binary_name("ffmpeg")))
    }

    // Get path to ffprobe binary
    pub fn get_ffprobe_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
        let binaries_dir = Self::get_binaries_dir(app_handle)?;
        Ok(binaries_dir.join(Self::get_binary_name("ffprobe")))
    }

    // Check if a binary exists and is executable
    fn is_binary_valid(path: &PathBuf) -> bool {
        if !path.exists() {
            return false;
        }

        // On Unix, check if executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(path) {
                let permissions = metadata.permissions();
                return permissions.mode() & 0o111 != 0;
            }
            false
        }

        #[cfg(not(unix))]
        {
            true
        }
    }

    // Make binary executable on Unix systems
    #[cfg(unix)]
    fn make_executable(path: &PathBuf) -> Result<(), String> {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)
            .map_err(|e| format!("Failed to read file permissions: {}", e))?
            .permissions();

        perms.set_mode(0o755);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to set executable permission: {}", e))?;

        Ok(())
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &PathBuf) -> Result<(), String> {
        Ok(())
    }

    // Get download URL for yt-dlp based on current OS
    fn get_ytdlp_download_url() -> Result<String, String> {
        let url = if cfg!(target_os = "windows") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
        } else if cfg!(target_os = "macos") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
        } else if cfg!(target_os = "linux") {
            "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
        } else {
            return Err("Unsupported operating system".to_string());
        };

        Ok(url.to_string())
    }

    // Get download URL for ffmpeg based on current OS
    fn get_ffmpeg_download_url() -> Result<String, String> {
        // Using builds from GitHub releases or official builds
        let url = if cfg!(target_os = "windows") {
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
        } else if cfg!(target_os = "macos") {
            "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip"
        } else if cfg!(target_os = "linux") {
            // Use GitHub release with direct binary download
            "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz"
        } else {
            return Err("Unsupported operating system".to_string());
        };

        Ok(url.to_string())
    }

    pub async fn download_ytdlp(app_handle: &tauri::AppHandle) -> Result<(), String> {
        use tauri::Emitter;
        use futures_util::StreamExt;

        let url = Self::get_ytdlp_download_url()?;
        let dest_path = Self::get_ytdlp_path(app_handle)?;

        let _ = app_handle.emit("binary-download-status", "Downloading yt-dlp...");
        println!("Downloading yt-dlp from: {}", url);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to download yt-dlp: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        let mut buffer = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
            buffer.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
                let _ = app_handle.emit("binary-download-progress", ("yt-dlp", percent));
            }
        }

        fs::write(&dest_path, buffer)
            .map_err(|e| format!("Failed to write binary file: {}", e))?;

        Self::make_executable(&dest_path)?;

        let _ = app_handle.emit("binary-download-status", "yt-dlp downloaded successfully");
        println!("yt-dlp downloaded successfully to: {}", dest_path.display());
        Ok(())
    }

    pub async fn download_ffmpeg(app_handle: &tauri::AppHandle) -> Result<(), String> {
        use tauri::Emitter;
        use futures_util::StreamExt;

        let ffmpeg_path = Self::get_ffmpeg_path(app_handle)?;
        let ffprobe_path = Self::get_ffprobe_path(app_handle)?;

        let _ = app_handle.emit("binary-download-status", "Downloading ffmpeg...");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // For macOS, download ffmpeg and ffprobe separately
        if cfg!(target_os = "macos") {
            println!("Downloading ffmpeg...");
            let _ = app_handle.emit("binary-download-status", "Downloading ffmpeg binary...");
            let ffmpeg_url = "https://evermeet.cx/ffmpeg/getrelease/ffmpeg/zip";

            let response = client.get(ffmpeg_url).send().await
                .map_err(|e| format!("Failed to download ffmpeg: {}", e))?;
            if !response.status().is_success() {
                return Err(format!("Download failed with status: {}", response.status()));
            }

            let total_size = response.content_length().unwrap_or(0);
            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();
            let mut buffer = Vec::new();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
                buffer.extend_from_slice(&chunk);
                downloaded += chunk.len() as u64;

                if total_size > 0 {
                    let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
                    let _ = app_handle.emit("binary-download-progress", ("ffmpeg", percent));
                }
            }

            Self::extract_single_binary_macos(&buffer, &ffmpeg_path, "ffmpeg")?;
            Self::make_executable(&ffmpeg_path)?;

            println!("Downloading ffprobe...");
            let _ = app_handle.emit("binary-download-status", "Downloading ffprobe binary...");
            let ffprobe_url = "https://evermeet.cx/ffmpeg/getrelease/ffprobe/zip";
            let response = client.get(ffprobe_url).send().await
                .map_err(|e| format!("Failed to download ffprobe: {}", e))?;
            if !response.status().is_success() {
                return Err(format!("Download failed with status: {}", response.status()));
            }
            let bytes = response.bytes().await
                .map_err(|e| format!("Failed to read ffprobe bytes: {}", e))?;
            Self::extract_single_binary_macos(&bytes, &ffprobe_path, "ffprobe")?;
            Self::make_executable(&ffprobe_path)?;
        } else {
            // For Windows/Linux, download single archive with both binaries
            let url = Self::get_ffmpeg_download_url()?;
            println!("Downloading ffmpeg from: {}", url);

            let response = client.get(&url).send().await
                .map_err(|e| format!("Failed to download ffmpeg: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Download failed with status: {}", response.status()));
            }

            let total_size = response.content_length().unwrap_or(0);
            let mut downloaded: u64 = 0;
            let mut stream = response.bytes_stream();
            let mut buffer = Vec::new();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| format!("Failed to read chunk: {}", e))?;
                buffer.extend_from_slice(&chunk);
                downloaded += chunk.len() as u64;

                if total_size > 0 {
                    let percent = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
                    let _ = app_handle.emit("binary-download-progress", ("ffmpeg", percent));
                }
            }

            if cfg!(target_os = "windows") {
                Self::extract_ffmpeg_windows(&buffer, &ffmpeg_path, &ffprobe_path)?;
            } else if cfg!(target_os = "linux") {
                Self::extract_ffmpeg_linux(&buffer, &ffmpeg_path, &ffprobe_path)?;
            }

            Self::make_executable(&ffmpeg_path)?;
            Self::make_executable(&ffprobe_path)?;
        }

        let _ = app_handle.emit("binary-download-status", "ffmpeg downloaded successfully");
        println!("ffmpeg and ffprobe downloaded successfully");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn extract_ffmpeg_windows(bytes: &[u8], ffmpeg_path: &PathBuf, ffprobe_path: &PathBuf) -> Result<(), String> {
        use std::io::Cursor;
        use zip::ZipArchive;

        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| format!("Failed to read zip archive: {}", e))?;

        let mut ffmpeg_found = false;
        let mut ffprobe_found = false;

        // Extract both ffmpeg.exe and ffprobe.exe from the archive
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;

            if file.name().ends_with("ffmpeg.exe") {
                let mut buffer = Vec::new();
                std::io::copy(&mut file, &mut buffer)
                    .map_err(|e| format!("Failed to extract ffmpeg: {}", e))?;

                fs::write(ffmpeg_path, buffer)
                    .map_err(|e| format!("Failed to write ffmpeg binary: {}", e))?;
                ffmpeg_found = true;
            } else if file.name().ends_with("ffprobe.exe") {
                let mut buffer = Vec::new();
                std::io::copy(&mut file, &mut buffer)
                    .map_err(|e| format!("Failed to extract ffprobe: {}", e))?;

                fs::write(ffprobe_path, buffer)
                    .map_err(|e| format!("Failed to write ffprobe binary: {}", e))?;
                ffprobe_found = true;
            }

            if ffmpeg_found && ffprobe_found {
                return Ok(());
            }
        }

        if !ffmpeg_found {
            return Err("ffmpeg.exe not found in archive".to_string());
        }
        if !ffprobe_found {
            return Err("ffprobe.exe not found in archive".to_string());
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn extract_ffmpeg_windows(_bytes: &[u8], _ffmpeg_path: &PathBuf, _ffprobe_path: &PathBuf) -> Result<(), String> {
        Err("Windows extraction not supported on this platform".to_string())
    }

    #[cfg(target_os = "linux")]
    fn extract_ffmpeg_linux(bytes: &[u8], ffmpeg_path: &PathBuf, ffprobe_path: &PathBuf) -> Result<(), String> {
        use std::io::Cursor;
        use tar::Archive;
        use xz2::read::XzDecoder;

        let cursor = Cursor::new(bytes);
        let decompressor = XzDecoder::new(cursor);
        let mut archive = Archive::new(decompressor);

        let mut ffmpeg_found = false;
        let mut ffprobe_found = false;

        // Extract and find both ffmpeg and ffprobe binaries
        for entry in archive
            .entries()
            .map_err(|e| format!("Failed to read tar entries: {}", e))?
        {
            let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {}", e))?;
            let path = entry
                .path()
                .map_err(|e| format!("Failed to get entry path: {}", e))?;

            // Look for ffmpeg binary (usually in bin/ subdirectory)
            if path.file_name() == Some(std::ffi::OsStr::new("ffmpeg")) {
                let mut buffer = Vec::new();
                std::io::copy(&mut entry, &mut buffer)
                    .map_err(|e| format!("Failed to extract ffmpeg: {}", e))?;

                fs::write(ffmpeg_path, buffer)
                    .map_err(|e| format!("Failed to write ffmpeg binary: {}", e))?;
                ffmpeg_found = true;
            } else if path.file_name() == Some(std::ffi::OsStr::new("ffprobe")) {
                let mut buffer = Vec::new();
                std::io::copy(&mut entry, &mut buffer)
                    .map_err(|e| format!("Failed to extract ffprobe: {}", e))?;

                fs::write(ffprobe_path, buffer)
                    .map_err(|e| format!("Failed to write ffprobe binary: {}", e))?;
                ffprobe_found = true;
            }

            if ffmpeg_found && ffprobe_found {
                return Ok(());
            }
        }

        if !ffmpeg_found {
            return Err("ffmpeg binary not found in archive".to_string());
        }
        if !ffprobe_found {
            return Err("ffprobe binary not found in archive".to_string());
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn extract_ffmpeg_linux(_bytes: &[u8], _ffmpeg_path: &PathBuf, _ffprobe_path: &PathBuf) -> Result<(), String> {
        Err("Linux extraction not supported on this platform".to_string())
    }

    #[cfg(target_os = "macos")]
    fn extract_single_binary_macos(bytes: &[u8], dest_path: &PathBuf, binary_name: &str) -> Result<(), String> {
        use std::io::Cursor;
        use zip::ZipArchive;

        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| format!("Failed to read zip archive: {}", e))?;

        // evermeet.cx provides single binary per archive
        // Look for the binary (usually the only file or first file without subdirectory)
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;

            // Check if this is the binary we're looking for
            if file.name().ends_with(binary_name) && !file.name().contains('/') {
                let mut buffer = Vec::new();
                std::io::copy(&mut file, &mut buffer)
                    .map_err(|e| format!("Failed to extract {}: {}", binary_name, e))?;

                fs::write(dest_path, buffer)
                    .map_err(|e| format!("Failed to write {} binary: {}", binary_name, e))?;

                return Ok(());
            }
        }

        // If not found with exact name, try first file (fallback)
        if archive.len() > 0 {
            let mut file = archive.by_index(0)
                .map_err(|e| format!("Failed to read zip entry: {}", e))?;

            let mut buffer = Vec::new();
            std::io::copy(&mut file, &mut buffer)
                .map_err(|e| format!("Failed to extract {}: {}", binary_name, e))?;

            fs::write(dest_path, buffer)
                .map_err(|e| format!("Failed to write {} binary: {}", binary_name, e))?;

            return Ok(());
        }

        Err(format!("{} not found in archive", binary_name))
    }

    #[cfg(not(target_os = "macos"))]
    fn extract_single_binary_macos(_bytes: &[u8], _dest_path: &PathBuf, _binary_name: &str) -> Result<(), String> {
        Err("macOS extraction not supported on this platform".to_string())
    }

    // Check if binary exists in system PATH
    fn is_in_system_path(binary_name: &str) -> bool {
        use std::process::Command;

        Command::new(binary_name)
            .arg("--version")
            .output()
            .is_ok()
    }

    // Check status of binaries (respects use_system_binaries config)
    pub fn check_binaries(app_handle: &tauri::AppHandle) -> Result<BinaryStatus, String> {
        let config = ConfigManager::load_config(app_handle)?;
        let use_system = config.use_system_binaries;

        let ytdlp_path = Self::get_ytdlp_path(app_handle)?;
        let ffmpeg_path = Self::get_ffmpeg_path(app_handle)?;

        // Check bundled binaries
        let ytdlp_bundled = Self::is_binary_valid(&ytdlp_path);
        let ffmpeg_bundled = Self::is_binary_valid(&ffmpeg_path);

        // Check system PATH (only if use_system_binaries is enabled)
        let ytdlp_system = use_system && Self::is_in_system_path("yt-dlp");
        let ffmpeg_system = use_system && Self::is_in_system_path("ffmpeg");

        // Priority: bundled first, then system (if enabled)
        let ytdlp_valid = ytdlp_bundled || ytdlp_system;
        let ffmpeg_valid = ffmpeg_bundled || ffmpeg_system;

        Ok(BinaryStatus {
            yt_dlp_installed: ytdlp_valid,
            ffmpeg_installed: ffmpeg_valid,
            yt_dlp_path: if ytdlp_bundled {
                Some(ytdlp_path.to_str()
                    .ok_or("yt-dlp path contains invalid UTF-8")?
                    .to_string())
            } else if ytdlp_system {
                Some("yt-dlp".to_string())
            } else {
                None
            },
            ffmpeg_path: if ffmpeg_bundled {
                Some(ffmpeg_path.to_str()
                    .ok_or("ffmpeg path contains invalid UTF-8")?
                    .to_string())
            } else if ffmpeg_system {
                Some("ffmpeg".to_string())
            } else {
                None
            },
        })
    }
}

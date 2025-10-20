use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct UserConfig {
    pub download_dir: Option<String>,
    pub font_size: u8,
    pub remember_queue: bool,
}

impl UserConfig {
    pub fn new() -> Self {
        Self {
            download_dir: None,
            font_size: 14,
            remember_queue: true,
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    fn get_config_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
        let config_dir = app_handle.path()
            .app_config_dir()
            .map_err(|e| format!("Failed to get app config directory: {}", e))?;
        
        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        
        Ok(config_dir.join("config.json"))
    }

    pub fn load_config(app_handle: &tauri::AppHandle) -> Result<UserConfig, String> {
        let config_path = Self::get_config_path(app_handle)?;
        
        if !config_path.exists() {
            return Ok(UserConfig::new());
        }
        
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        let config: UserConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config file: {}", e))?;
        
        Ok(config)
    }

    pub fn save_config(app_handle: &tauri::AppHandle, config: &UserConfig) -> Result<(), String> {
        let config_path = Self::get_config_path(app_handle)?;
        
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        
        Ok(())
    }

    pub fn get_download_dir(app_handle: &tauri::AppHandle) -> Result<String, String> {
        let config = Self::load_config(app_handle)?;
        
        if let Some(dir) = config.download_dir {
            // Validate that the directory exists and is writable
            let path = PathBuf::from(&dir);
            if path.exists() && path.is_dir() {
                // Check if writable by trying to create a temp file
                let test_file = path.join(".ytdl_test");
                if fs::write(&test_file, "test").is_ok() {
                    let _ = fs::remove_file(&test_file);
                    return Ok(dir);
                }
            }
        }
        
        // Fallback to default downloads directory
        let home_dir = dirs_next::home_dir()
            .ok_or("Failed to get home directory")?;
        let default_dir = home_dir.join("Downloads");

        // Create default directory if it doesn't exist
        if !default_dir.exists() {
            fs::create_dir_all(&default_dir)
                .map_err(|e| format!("Failed to create default downloads directory: {}", e))?;
        }

        Ok(default_dir.to_str()
            .ok_or("Downloads path contains invalid UTF-8")?
            .to_string())
    }
}
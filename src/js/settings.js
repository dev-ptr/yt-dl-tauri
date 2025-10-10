import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

class SettingsManager {
    constructor() {
      this.modal = document.getElementById('settingsModal');
      this.closeBtn = document.querySelector('.close');
      this.cancelBtn = document.getElementById('cancelSettingsBtn');
      this.saveBtn = document.getElementById('saveSettingsBtn');
      this.browseBtn = document.getElementById('settingsBrowseBtn');
      this.resetBtn = document.getElementById('resetDownloadDirBtn');
      
      this.downloadDirInput = document.getElementById('settingsDownloadDir');
      this.fontSizeInput = document.getElementById('fontSizeInput');
      this.rememberQueueCheckbox = document.getElementById('rememberQueueCheckbox');
      
      this.setupEventListeners();
      
      document.addEventListener('settingsModalOpened', () => {
        this.loadCurrentSettings();
      });
    }

  setupEventListeners() {
    this.closeBtn.onclick = () => this.closeModal();
    this.cancelBtn.onclick = () => this.closeModal();
    window.onclick = (event) => {
      if (event.target === this.modal) {
        this.closeModal();
      }
    };

    this.saveBtn.onclick = () => this.saveSettings();

    this.browseBtn.onclick = () => this.browseDownloadDir();

    this.resetBtn.onclick = () => this.resetDownloadDir();

    this.fontSizeInput.oninput = () => this.updateFontSizePreview();
  }

  async loadCurrentSettings() {
    try {
      const config = await invoke('get_config');
      
      this.downloadDirInput.value = config.download_dir || '';
      this.fontSizeInput.value = config.font_size || 14;
      this.rememberQueueCheckbox.checked = config.remember_queue || false;
      
      this.updateFontSizePreview();
    } catch (error) {
      console.error('Failed to load settings:', error);
      // Fallback to defaults
      this.downloadDirInput.value = '';
      this.fontSizeInput.value = 14;
      this.rememberQueueCheckbox.checked = false;
      this.updateFontSizePreview();
    }
  }

  async saveSettings() {
    try {
      const config = {
        download_dir: this.downloadDirInput.value || null,
        font_size: Math.max(8, Math.min(20, parseInt(this.fontSizeInput.value) || 14)),
        remember_queue: this.rememberQueueCheckbox.checked
      };
  
      await invoke('update_config', { newConfig: config });
      
      // Apply font size globally immediately
      document.documentElement.style.setProperty('--font-size', `${config.font_size}px`);
      
      // Update main download directory input immediately
      const mainFolderInput = document.getElementById('folderInput');
      if (config.download_dir) {
        mainFolderInput.value = config.download_dir;
      } else {
        // If no download dir set, get default from backend
        try {
          const defaultDir = await invoke('get_download_dir');
          mainFolderInput.value = defaultDir;
        } catch (error) {
          console.error('Failed to get default directory:', error);
        }
      }
      
      this.closeModal();
      alert('Settings saved successfully!');
    } catch (error) {
      console.error('Failed to save settings:', error);
      alert('Failed to save settings: ' + error);
    }
  }

  async browseDownloadDir() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
      });
      
      if (selected) {
        this.downloadDirInput.value = selected;
      }
    } catch (error) {
      console.error('Failed to browse directory:', error);
    }
  }

  async resetDownloadDir() {
    try {
      const defaultDir = await invoke('get_download_dir');
      this.downloadDirInput.value = defaultDir;
    } catch (error) {
      console.error('Failed to get default directory:', error);
    }
  }

  updateFontSizePreview() {
    const fontSize = this.fontSizeInput.value;
    const preview = document.querySelector('.font-size-preview');
    preview.style.fontSize = `${fontSize}px`;
  }

  openModal() {
    this.modal.style.display = 'block';
    this.loadCurrentSettings();
  }

  closeModal() {
    this.modal.style.display = 'none';
  }

  // Method to get current settings from backend
  async getCurrentSettings() {
    try {
      return await invoke('get_config');
    } catch (error) {
      console.error('Failed to load settings:', error);
      return {
        download_dir: null,
        font_size: 14,
        remember_queue: true
      };
    }
  }
}

export { SettingsManager };
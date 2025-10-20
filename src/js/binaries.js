import { invoke } from '@tauri-apps/api/core';

/**
 * Binary manager for frontend
 * Handles checking and downloading required binaries (yt-dlp, ffmpeg)
 */
export class BinaryManager {
  /**
   * Check status of required binaries
   * @returns {Promise<{yt_dlp_installed: boolean, ffmpeg_installed: boolean, yt_dlp_path: string|null, ffmpeg_path: string|null}>}
   */
  static async checkBinaries() {
    try {
      return await invoke('check_binaries');
    } catch (error) {
      console.error('Failed to check binaries:', error);
      throw error;
    }
  }

  /**
   * Download yt-dlp binary
   * @returns {Promise<void>}
   */
  static async downloadYtDlp() {
    try {
      await invoke('download_ytdlp');
    } catch (error) {
      console.error('Failed to download yt-dlp:', error);
      throw error;
    }
  }

  /**
   * Download ffmpeg binary
   * @returns {Promise<void>}
   */
  static async downloadFfmpeg() {
    try {
      await invoke('download_ffmpeg');
    } catch (error) {
      console.error('Failed to download ffmpeg:', error);
      throw error;
    }
  }

  /**
   * Download all missing binaries
   * @param {Function} onProgress - Callback for progress updates
   * @returns {Promise<void>}
   */
  static async downloadMissingBinaries(onProgress) {
    const status = await this.checkBinaries();

    if (!status.yt_dlp_installed) {
      if (onProgress) onProgress('Downloading yt-dlp...');
      await this.downloadYtDlp();
    }

    if (!status.ffmpeg_installed) {
      if (onProgress) onProgress('Downloading ffmpeg...');
      await this.downloadFfmpeg();
    }
  }

  /**
   * Check if all required binaries are installed
   * @returns {Promise<boolean>}
   */
  static async areAllBinariesInstalled() {
    const status = await this.checkBinaries();
    return status.yt_dlp_installed && status.ffmpeg_installed;
  }
}

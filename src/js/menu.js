import { Menu, MenuItem, Submenu } from "@tauri-apps/api/menu";
import { exit } from "@tauri-apps/plugin-process";
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';

export async function setupMenu() {
  const fileSubmenu = await Submenu.new({
    text: "File",
    items: [
      await MenuItem.new({
        id: "settings",
        text: "Settings",
        action: () => {
          // Open settings modal using the settings manager
          if (window.settingsManager) {
            window.settingsManager.openModal();
          } else {
            // Fallback if settingsManager isn't available
            const settingsModal = document.getElementById('settingsModal');
            if (settingsModal) {
              settingsModal.style.display = 'block';
              const event = new Event('settingsModalOpened');
              document.dispatchEvent(event);
            }
          }
        },
      }),
      await MenuItem.new({
        id: "download_binaries",
        text: "Download Binaries",
        action: async () => {
          if (confirm("Download yt-dlp and ffmpeg binaries?\n\nThis may take a few minutes.")) {
            const log = document.getElementById('log');
            if (log) {
              log.textContent += 'Downloading binaries...\n';
            }
            try {
              await invoke('download_all_binaries');
              if (log) {
                log.textContent += 'Binaries downloaded successfully!\n';
              }
              alert('Binaries downloaded successfully!');
            } catch (error) {
              console.error('Failed to download binaries:', error);
              if (log) {
                log.textContent += `Failed: ${error}\n`;
              }
              alert(`Failed to download binaries: ${error}`);
            }
          }
        },
      }),
      await MenuItem.new({
        id: "quit",
        text: "Quit",
        action: () => {
          exit(0);
        },
      }),
    ],
  });

  const editSubmenu = await Submenu.new({
    text: "Help",
    items: [
      await MenuItem.new({
        id: "supported_sites",
        text: "Supported Sites",
        action: async () => {
            try {
            await openUrl("https://ytdl-org.github.io/youtube-dl/supportedsites.html");
            } catch (err) {
            console.error("Failed to open URL:", err);
            }
        },
      }),
      await MenuItem.new({
        id: "about",
        text: "About",
        action: () => alert("About clicked"),
      }),
    ],
  });

  const menu = await Menu.new({
    items: [fileSubmenu, editSubmenu],
  });

  await menu.setAsAppMenu();
}

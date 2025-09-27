import { Menu, MenuItem, Submenu } from "@tauri-apps/api/menu";
import { exit } from "@tauri-apps/plugin-process";
import { openUrl } from '@tauri-apps/plugin-opener';

export async function setupMenu() {
  const fileSubmenu = await Submenu.new({
    text: "File",
    items: [
      await MenuItem.new({
        id: "settings",
        text: "Settings",
        action: () => {
          alert("Settings");
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

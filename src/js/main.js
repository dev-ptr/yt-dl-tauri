import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog';
import { setupMenu } from "./menu.js";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { SettingsManager } from "./settings.js";

let appWindow;
let settingsManager;

async function initializeApp() {
  appWindow = await getCurrentWindow();
  settingsManager = new SettingsManager();
  window.settingsManager = settingsManager;
}

const toggleBtn = document.getElementById("toggleLogBtn");
const logContainer = document.getElementById("logContainer");
let isLogVisible = false;
let editingIndex = -1;
let originalUrl = '';
let isProgrammaticChange = false;


async function loadInitialSettings() {
  try {
    const config = await invoke('get_config');
    document.documentElement.style.setProperty('--font-size', `${config.font_size}px`);
    
    // Set download directory
    if (config.download_dir) {
      document.getElementById('folderInput').value = config.download_dir;
    } else {
      const defaultDir = await invoke('get_download_dir');
      document.getElementById('folderInput').value = defaultDir;
    }
    
    // Load saved queue if remember_queue is enabled
    await loadQueueFromStorage();
    
    console.log('Settings loaded:', config);
  } catch (error) {
    console.error('Failed to load initial settings:', error);
    document.documentElement.style.setProperty('--font-size', '14px');
  }
}
document.addEventListener('DOMContentLoaded', async () => {
  await initializeApp();
  await loadInitialSettings();
});

toggleBtn.addEventListener("click", async () => {
  isLogVisible = !isLogVisible;

  if (isLogVisible) {
    logContainer.style.display = "block";
    toggleBtn.textContent = "▲ Hide Log";

    await appWindow.setSize(new LogicalSize(800, 800));

  } else {
    logContainer.style.display = "none";
    toggleBtn.textContent = "▼ Show Log";

   await appWindow.setSize(new LogicalSize(800, 600));
  }
});

setupMenu();
// DOM elements
const queue = [];
let processing = false;

const removeSelect = document.getElementById("removeSelect");
const urlInput = document.getElementById('urlInput')
const mp3OnlyCheckbox = document.getElementById('mp3Only')
const sponsorblockCheckbox = document.getElementById('sponsorblock')
const enablePlayistCheckbox = document.getElementById('enablePlaylist')
const downloadBtn = document.getElementById('downloadBtn')
const addToQueueBtn = document.getElementById('addToQueueBtn')
const browseBtn = document.getElementById('browseBtn')
const folderPath = document.getElementById('folderInput')
const clearQueueBtn = document.getElementById('clearQueueBtn')
const log = document.getElementById('log')
const statusText = document.getElementById("statusText");
const statusPercent = document.getElementById("statusPercent");

let isDownloading = false;
let currentItem = null; 


clearQueueBtn.addEventListener('click', async () => {
  queue.length = 0;  
  localStorage.removeItem('ytdl_queue');
  editingIndex = -1;
  originalUrl = '';
  urlInput.value = '';
  addToQueueBtn.textContent = 'Add to queue';
  
  updateQueueDisplay();
  await saveQueueToStorage();
});
async function hideLogContainer() {
  const container = document.getElementById("logContainer");
  container.style.display = "none";
  logVisible = false;
}
// Reset status bar
function resetStatus() {
  statusText.textContent = "Status: Idle";
  statusPercent.textContent = "";
}
async function saveQueueToStorage() {
  try {
    console.log('saveQueueToStorage called, current queue length:', queue.length);
    
    if (window.settingsManager) {
      const settings = await window.settingsManager.getCurrentSettings();
      console.log('Settings for queue save:', settings);
      
      if (settings.remember_queue) {
        if (queue.length > 0) {
          const queueJson = JSON.stringify(queue);
          localStorage.setItem('ytdl_queue', queueJson);
          console.log('Queue saved to localStorage:', queue.length, 'items');
        } else {
          // Save empty queue to localStorage to clear it
          localStorage.setItem('ytdl_queue', JSON.stringify([]));
          console.log('Empty queue saved to localStorage');
        }
      } else {
        localStorage.removeItem('ytdl_queue');
        console.log('Queue remembering disabled, cleared localStorage');
      }
    }
  } catch (error) {
    console.error('Failed to save queue:', error);
  }
}

async function loadQueueFromStorage() {
  try {
    if (window.settingsManager) {
      const settings = await window.settingsManager.getCurrentSettings();
      if (settings.remember_queue) {
        const savedQueue = localStorage.getItem('ytdl_queue');
        if (savedQueue) {
          const parsedQueue = JSON.parse(savedQueue);
          if (Array.isArray(parsedQueue)) {
            // Validate each item has required properties
            const validItems = parsedQueue.filter(item => 
              item && typeof item === 'object' && item.url && item.fPath
            );
            queue.push(...validItems);
            updateQueueDisplay();
            console.log('Loaded saved queue:', parsedQueue.length, 'items');
          }
        }
      }
    }
  } catch (error) {
    console.error('Failed to load queue:', error);
  }
}

// Browse for folder
browseBtn.addEventListener('click', async () => {
  const file = await open({ multiple: false, directory: true });
  if (file) folderPath.value = file;
});

addToQueueBtn.addEventListener('click', async () => {
  const fPath = folderPath.value.trim();
  if (!fPath) {
    alert('Please select a download folder');
    return;
  }

  const url = urlInput.value.trim();
  if (!url) {
    alert('Please enter a URL');
    return;
  }
  let title = url;
  try {
    title = await invoke('fetch_video_title', { url });
  } catch (e) {
    console.warn('Could not fetch title, falling back to URL');
  }
  const mp3Only = mp3OnlyCheckbox.checked;
  const enablePlaylist = enablePlayistCheckbox.checked;
  const sponsorblock = sponsorblockCheckbox.checked;
  const item = { url, title, fPath, mp3Only, enablePlaylist, sponsorblock };

  if (editingIndex >= 0) {
    queue[editingIndex] = item;
    log.textContent += `Updated queue item: ${url}\n`;
  } else {
    if (queue.some(item => item.url === url)) {
      alert('Already in queue');
      return;
    }
    queue.push(item);
    log.textContent += `Added to queue: ${url}\n`;
  }

  updateQueueDisplay();
  await saveQueueToStorage();
  
  // Reset form
  urlInput.value = '';
  editingIndex = -1;
  originalUrl = '';
  addToQueueBtn.textContent = 'Add to queue';
  
  log.scrollTop = log.scrollHeight;
});

downloadBtn.addEventListener('click', async () => {
  if (queue.length === 0) {
    log.textContent += 'Queue is empty\n';
    log.scrollTop = log.scrollHeight;
    alert('Queue is empty');
    return;
  }
  if (isDownloading) {
    log.textContent += 'A download is already in progress\n'
    return;
  }
  log.textContent += 'Starting download...\n';
  log.scrollTop = log.scrollHeight;

  isDownloading = true;
  downloadBtn.disabled = true;

  if (!processing) {
    processQueue();
  }
});


document.getElementById("removeBtn").addEventListener("click", async () => {
  const selected = Array.from(removeSelect.selectedOptions).map(opt => parseInt(opt.value, 10));
  selected.sort((a, b) => b - a).forEach(i => {
    if (i >= 0) queue.splice(i, 1);
  });
  
  // Reset editing state when items are removed
  editingIndex = -1;
  urlInput.value = '';
  originalUrl = '';
  addToQueueBtn.textContent = 'Add to queue';
  
  updateQueueDisplay();
  await saveQueueToStorage();
});

// Update queue select box
function updateQueueDisplay() {
  removeSelect.innerHTML = '';
  queue.forEach((item, index) => {
    const opt = document.createElement("option");
    opt.value = index;
    opt.text = item.title || item.url;
    removeSelect.appendChild(opt);
  });
}
removeSelect.addEventListener('change', () => {
  const selectedIndex = parseInt(removeSelect.value);
  if (selectedIndex >= 0 && selectedIndex < queue.length) {
    editQueueItem(selectedIndex);
  }
});

function editQueueItem(index) {
  if (index >= 0 && index < queue.length) {
    const item = queue[index];
    editingIndex = index;
    originalUrl = item.url;
    
    // Set flag to prevent triggering input listener
    isProgrammaticChange = true;
    
    // Populate form with item data
    urlInput.value = item.url;
    folderPath.value = item.fPath;
    mp3OnlyCheckbox.checked = item.mp3Only;
    enablePlayistCheckbox.checked = item.enablePlaylist;
    sponsorblockCheckbox.checked = item.sponsorblock;
    
    addToQueueBtn.textContent = 'Update Item';
    
  }
}

async function processQueue() {
  processing = true;

  while (queue.length > 0) {
    currentItem = queue.shift();
    updateQueueDisplay();

    statusText.textContent = `Processing "${currentItem.title}"…`;
    statusPercent.textContent = "";

    try {
      await processDownload(currentItem);
    } catch (err) {
      // already handled by listeners
    }
    await saveQueueToStorage();
  }

  currentItem = null;
  resetStatus();
  processing = false;
}

// Trigger backend download
async function processDownload(item) {
  try {
    await invoke('download_url', {
      url: item.url,
      fPath: item.fPath,
      mp3Only: item.mp3Only,
      enablePlaylist: item.enablePlaylist,
      sponsorblock: item.sponsorblock
    });
  } catch (error) {
    throw new Error(`Download failed for ${item.url}: ${error}`);
  }
}

// 🔹 Event listeners
(async () => {
await listen('download-progress', event => {
  const percent = event.payload;
  const rawTitle = currentItem?.title || currentItem?.url || "Unknown";

  statusText.textContent = `Downloading "${rawTitle}"`;
  statusPercent.textContent = `${percent}%`;

  log.textContent += `Progress: ${percent}%\n`;
  log.scrollTop = log.scrollHeight;
});
urlInput.addEventListener('input', () => {
  if (editingIndex >= 0) {
    editingIndex = -1;
    originalUrl = '';
    addToQueueBtn.textContent = 'Add to queue';
  }
});
// Add escape key listener to cancel editing
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && editingIndex >= 0) {
    // Cancel editing
    urlInput.value = '';
    editingIndex = -1;
    originalUrl = '';
    addToQueueBtn.textContent = 'Add to queue';
  }
});
await listen('download-complete', event => {
  const rawTitle = currentItem?.title || currentItem?.url || "Unknown";
  statusText.textContent = `✅ Completed "${rawTitle}"`;
  statusPercent.textContent = "";

  log.textContent += `Download completed with code ${event.payload}\n`;
  log.scrollTop = log.scrollHeight;

  isDownloading = false;
  downloadBtn.disabled = false;
});

await listen('download-error', event => {
  const rawTitle = currentItem?.title || currentItem?.url || "Unknown";
  statusText.textContent = `❌ Failed "${rawTitle}"`;
  statusPercent.textContent = "";

  log.textContent += `ERROR: ${event.payload}\n`;
  log.scrollTop = log.scrollHeight;

  isDownloading = false;
  downloadBtn.disabled = false;
});

await listen('download-log', event => {
  console.log('[download-log]', event.payload)
  log.textContent += event.payload + '\n'
  log.scrollTop = log.scrollHeight
});
})()

// init
resetStatus();

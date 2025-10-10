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

// State
let isDownloading = false

browseBtn.addEventListener('click', async () => {
  const file = await open({
    multiple: false,
    directory: true,
  });
  if (file) {
    document.getElementById('folderInput').value = file;
  }
});
clearQueueBtn.addEventListener('click', async () => {
  queue.length = 0;  
  updateQueueDisplay();
  // Clear saved queue
  localStorage.removeItem('ytdl_queue');
});
async function hideLogContainer() {
  const container = document.getElementById("logContainer");
  container.style.display = "none";
  logVisible = false;

  const width = document.body.scrollWidth;
  const height = document.body.scrollHeight;
  await appWindow.setSize({ width, height });
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

  if (queue.some(item => item.url === url)) {
    alert('This URL is already in the queue. Skipping...');
    urlInput.value = '';
    return;
  }

  const mp3Only = mp3OnlyCheckbox.checked;
  const enablePlaylist = enablePlayistCheckbox.checked;
  const sponsorblock = sponsorblockCheckbox.checked;
  const item = { url, fPath, mp3Only, enablePlaylist, sponsorblock };

  queue.push(item);
  updateQueueDisplay();
  await saveQueueToStorage();

  log.textContent += `Added to queue: ${url}\n`;
  log.scrollTop = log.scrollHeight;
  urlInput.value = ''
});

downloadBtn.addEventListener('click', async () => {
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
  updateQueueDisplay();
  await saveQueueToStorage();
});

function updateQueueDisplay() {
  console.log("Current queue:", queue);
  removeSelect.innerHTML = ''; // Clear all existing options
  queue.forEach((item, index) => {
    const opt = document.createElement("option");
    opt.value = index;
    opt.text = item.url;
    removeSelect.appendChild(opt);
  });
}

async function processQueue() {
  processing = true;

  while (queue.length > 0) {
    const item = queue.shift(); // Get first item
    updateQueueDisplay();
    log.textContent += `Processing  ${item.url}...\n`
    log.scrollTop = log.scrollHeight;
    try {
      await processDownload(item);
      log.textContent += `Finished  ${item.url}\n`
      log.scrollTop = log.scrollHeight;

    } catch (err) {
      log.textContent += `Failed to download ${item.url}: ${err.message}`
      log.scrollTop = log.scrollHeight;

    }
    await saveQueueToStorage();
  }

  processing = false;
}

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

;(async () => {
  await listen('download-progress', event => {
    const percent = event.payload
    log.textContent += `Progress: ${percent}%\n`
    log.scrollTop = log.scrollHeight
  })

  await listen('download-error', event => {
    log.textContent += `ERROR: ${event.payload}\n`
    log.scrollTop = log.scrollHeight;
    isDownloading = false
    downloadBtn.disabled = false
  })

  await listen('download-log', event => {
    console.log('[download-log]', event.payload)
    log.textContent += event.payload + '\n'
    log.scrollTop = log.scrollHeight
  })

  await listen('download-complete', event => {
    log.textContent += `\nDownload completed with code ${event.payload}\n`
    log.scrollTop = log.scrollHeight;
    isDownloading = false
    downloadBtn.disabled = false
  })
})()

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog';

// DOM elements
const urlInput = document.getElementById('urlInput')
const mp3OnlyCheckbox = document.getElementById('mp3Only')
const downloadBtn = document.getElementById('downloadBtn')
const quitBtn = document.getElementById('quitBtn')
const browseBtn = document.getElementById('browseBtn')
const folderPath = document.getElementById('folderInput');


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
downloadBtn.addEventListener('click', async () => {
  if (isDownloading) {
    log.textContent += 'A download is already in progress\n'
    return
  }
  const fPath = folderPath.value.trim()
  if (!fPath) {
    alert('Please select a download folder');
    return;
  }
  const url = urlInput.value.trim()
  if (!url) {
    alert('Please enter a URL')
    return
  }

  log.textContent = 'Starting download...\n'
  log.scrollTop = log.scrollHeight;

  isDownloading = true
  downloadBtn.disabled = true

  try {
    await invoke('download_url', { 
      url, 
      fPath,
      mp3Only: mp3OnlyCheckbox.checked 
    })
  } catch (error) {
    log.textContent += `Error starting download: ${error}\n`
    log.scrollTop = log.scrollHeight;
    isDownloading = false
    downloadBtn.disabled = false
  }
})

quitBtn.addEventListener('click', async () => {
  try {
    await invoke('quit_app')
  } catch (error) {
    console.error('Failed to quit:', error)
  }
})
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

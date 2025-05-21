import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

// DOM elements
const urlInput = document.getElementById('urlInput')
const mp3OnlyCheckbox = document.getElementById('mp3Only')
const downloadBtn = document.getElementById('downloadBtn')
const quitBtn = document.getElementById('quitBtn')
const log = document.getElementById('log')

// State
let isDownloading = false

downloadBtn.addEventListener('click', async () => {
  if (isDownloading) {
    log.textContent += 'A download is already in progress\n'
    return
  }

  const url = urlInput.value.trim()
  if (!url) {
    alert('Please enter a URL')
    return
  }

  log.textContent = 'Starting download...\n'
  isDownloading = true
  downloadBtn.disabled = true

  try {
    await invoke('download_url', { 
      url, 
      mp3_only: mp3OnlyCheckbox.checked 
    })
  } catch (error) {
    log.textContent += `Error starting download: ${error}\n`
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

// Listen for download logs
const unlistenLog = await listen('download-log', event => {
  log.textContent += event.payload + '\n'
  log.scrollTop = log.scrollHeight
})

// Listen for completion
const unlistenComplete = await listen('download-complete', event => {
  log.textContent += `\nProcess completed with code ${event.payload}\n`
  isDownloading = false
  downloadBtn.disabled = false
})

// Cleanup when needed (e.g., if using SPA)
// window.addEventListener('beforeunload', () => {
//   unlistenLog()
//   unlistenComplete()
// })
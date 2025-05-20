import { defineConfig } from 'vite'

export default defineConfig({
  optimizeDeps: {
    exclude: ['@tauri-apps/api']
  }
})

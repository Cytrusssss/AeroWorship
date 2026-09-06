import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  root: 'src',
  clearScreen: false,
  plugins: [vue()],
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
    fs: {
      allow: ['.', '../node_modules'],
    },
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: 'chrome105',
    sourcemap: false,
    rollupOptions: {
      input: {
        main: 'index.html',
        output: 'output.html',
      },
    },
  },
})

import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// Two entry points, two bundles (PRD §6.3, §6.13). This is not a build
// optimisation: the 28 MB Projector Output line in the PRD §5.1 memory budget
// is only defensible while `output.html` carries the renderer and nothing else,
// so `src/output/**` must never import from `src/main/**`.
export default defineConfig({
  // `index.html` and `output.html` live in `src/` per PRD §6.13, so that is the
  // Vite root. `outDir` below is relative to it and lands on `<repo>/dist`,
  // which is what `src-tauri/tauri.conf.json` already points `frontendDist` at.
  root: 'src',
  // Do not wipe the Tauri CLI's own output when it prints the build banner.
  clearScreen: false,
  plugins: [vue()],
  server: {
    // Must match `devUrl` in tauri.conf.json exactly; failing loudly on a busy
    // port beats Tauri silently opening a blank window on 1421.
    port: 1420,
    strictPort: true,
    watch: {
      // Rust sources are Cargo's business; watching them would restart HMR on
      // every `cargo` write.
      ignored: ['**/src-tauri/**'],
    },
    fs: {
      // Vite derives the default `allow` from the workspace root, which with
      // `root: 'src'` is the whole repository — `/@fs/<repo>/src-tauri/...`
      // would then serve Rust sources, `Cargo.lock` and `capabilities/*.json`
      // for as long as `npm run dev` runs. The dev server binds to loopback, so
      // that is local-only exposure and this is hardening rather than a fix,
      // but nothing outside these two directories is ever a legitimate request.
      // Entries are resolved against `root`; Vite appends its own client
      // directory automatically, and it already sits under `node_modules/`.
      allow: ['.', '../node_modules'],
    },
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    // WebView2 on Windows 10 1809+ / 11 is evergreen Chromium (NFR-17).
    target: 'chrome105',
    // PRD §5.1 lists "no source maps or dev tooling in release builds" as one
    // of the tactics the memory and installer budgets assume. Stated rather
    // than left to the default so a future default change cannot silently undo
    // it.
    sourcemap: false,
    rollupOptions: {
      // Relative to `root`.
      input: {
        main: 'index.html',
        output: 'output.html',
      },
    },
  },
})

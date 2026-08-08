import { defineConfig } from 'vitest/config'

// A file of its own rather than a `test` block inside `vite.config.ts`: that
// file describes the shipped bundles (two entry points, no source maps, a
// pinned target) and nothing in it should have to be read past to understand
// what gets built. Vitest prefers this file when both exist, so the build
// config is never loaded during a test run either.
export default defineConfig({
  test: {
    // Tests sit next to the module they exercise. Rust owns the logic whose
    // correctness matters (PRD §6.1), so this suite stays small on purpose —
    // it covers build tooling and, later, frontend behaviour that genuinely
    // lives in the webview. `tests/{unit,integration,perf}` in PRD §6.13 is a
    // separate, Rust-side tree.
    include: ['scripts/**/*.test.{js,ts}', 'src/**/*.test.{js,ts}'],
    // No `passWithNoTests`: an empty run means the suite stopped being wired
    // up, and that should look like a failure rather than a green tick.
    environment: 'node',
    // Explicit imports from 'vitest' instead of ambient globals, so the test
    // files type-check under the same tsconfig as everything else.
    globals: false,
  },
})

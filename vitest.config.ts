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
    // it covers build tooling and frontend logic that needs no DOM. Component
    // tests are outside what this config can run: `environment` below is
    // 'node', and no DOM implementation is installed. Rendering a component
    // here fails on `document is not defined` rather than passing hollowly, so
    // the limit is visible; widening it is a deliberate choice about the
    // installer budget (NFR-16) and belongs to the item that first needs it.
    //
    // `tests/**` is in the list too. That tree is mostly the Rust-side one of
    // PRD §6.13, but `tests/unit/` is also named as a home for frontend tests
    // and now holds one, and a Vitest file dropped there that this config never
    // collected would leave `npm test` green while testing nothing — the
    // ADR-0020 failure again.
    //
    // The extension set is the one `SOURCE_EXTENSIONS` in `eslint.config.js`
    // carries, minus `.vue`, which is a component rather than a test file.
    // `tsconfig.json` lists the same three directories with the same spellings,
    // so everything collected here is also read by `npm run typecheck`: while
    // this stopped at `{js,ts}` for `scripts/**` and that file stopped at
    // `scripts/**/*.js`, a `scripts/x.test.ts` would have run in the suite and
    // never been type-checked at all.
    include: [
      'scripts/**/*.test.{js,mjs,cjs,jsx,ts,mts,cts,tsx}',
      'src/**/*.test.{js,mjs,cjs,jsx,ts,mts,cts,tsx}',
      'tests/**/*.test.{js,mjs,cjs,jsx,ts,mts,cts,tsx}',
    ],
    // No `passWithNoTests`: an empty run means the suite stopped being wired
    // up, and that should look like a failure rather than a green tick.
    environment: 'node',
    // Explicit imports from 'vitest' instead of ambient globals, so the test
    // files type-check under the same tsconfig as everything else.
    globals: false,
  },
})

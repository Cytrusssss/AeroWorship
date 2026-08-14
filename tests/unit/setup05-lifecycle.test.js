// @ts-check

/**
 * Three things ADR-0019 and ADR-0023 promise about how these guards and perf
 * commands are *wired up*, none of which the pure-function suites can see
 * because they never invoke `npm` itself:
 *
 *   - `pretest` really aborts `npm test` when it fails. ADR-0023's own doc
 *     for `check-fixture-content.js` states this in prose ("a real, recurring
 *     safety net ... just not a synchronous block on `git commit`") but nothing
 *     had verified the npm lifecycle mechanism actually behaves that way on
 *     this machine's npm — a failing pre-script that gets silently ignored
 *     would make the whole guard decorative.
 *   - the exact commands ADR-0019 requires for `tests/perf/` live in
 *     `package.json` and only there, carrying the specific flags that fixed
 *     the bug ADR-0019 exists to name (`--release`, `-p aeroworship`,
 *     `--features tauri/custom-protocol` for the crate that touches the
 *     Tauri shell). A silent edit that drops one of those flags reopens
 *     exactly the failure ADR-0019 already paid to close, and nothing else
 *     in the suite would notice.
 *   - `tests/integration/.gitkeep` and `tests/perf/.gitkeep` are actually
 *     tracked, so the directories PRD §6.13 requires survive a fresh clone
 *     (git does not track empty directories on its own).
 */

import { execFileSync } from 'node:child_process'
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')

/** @type {{ scripts: Record<string, string> }} */
const pkg = JSON.parse(readFileSync(join(ROOT, 'package.json'), 'utf8'))

describe('pretest wiring (ADR-0023)', () => {
  it("package.json's pretest runs the real fixture content guard, not a stand-in", () => {
    expect(pkg.scripts.pretest).toBe('node scripts/check-fixture-content.js')
  })

  it(
    'npm actually aborts the "test" script when "pretest" exits non-zero',
    { timeout: 30_000 },
    () => {
      // A synthetic package unrelated to the real guard's logic — this proves
      // the *lifecycle mechanism* npm provides on this machine, which is what
      // check-fixture-content.js's own doc leans on when it says a failing
      // pretest is "a real, recurring safety net". If npm's own behaviour ever
      // changed (or were invoked in a way that skips pre/post hooks, e.g.
      // `npm test --ignore-scripts`), this is the test that would catch it —
      // the real pretest script's own correctness is proven separately in
      // check-fixture-content.e2e.test.js.
      const dir = mkdtempSync(join(tmpdir(), 'aeroworship-pretest-lifecycle-'))
      try {
        writeFileSync(
          join(dir, 'package.json'),
          JSON.stringify({
            name: 'pretest-lifecycle-scratch',
            private: true,
            scripts: {
              pretest: `${JSON.stringify(process.execPath)} -e "process.exit(1)"`,
              test: `${JSON.stringify(process.execPath)} -e "require('fs').writeFileSync('ran.marker','1')"`,
            },
          }),
        )
        let threw = false
        try {
          execFileSync('npm', ['test'], { cwd: dir, encoding: 'utf8' })
        } catch {
          threw = true
        }
        expect(threw, 'npm test must exit non-zero when pretest fails').toBe(true)
        expect(() => readFileSync(join(dir, 'ran.marker'))).toThrow()
      } finally {
        rmSync(dir, { recursive: true, force: true })
      }
    },
  )
})

describe('tests/perf command definitions (ADR-0019)', () => {
  it('test:perf:shell keeps every flag ADR-0019 names for the crate that touches the Tauri shell', () => {
    // Pinned to the exact string, not a substring match on each flag
    // separately: ADR-0019's table names this exact command as the verified
    // jalan keluar, and a reformatting that reordered flags without dropping
    // any would still be a silent deviation from "one place" worth catching.
    expect(pkg.scripts['test:perf:shell']).toBe(
      'cargo test --release --manifest-path src-tauri/Cargo.toml -p aeroworship --features tauri/custom-protocol',
    )
  })

  it('test:perf:core stays on the domain crate the custom-protocol guard never touches', () => {
    expect(pkg.scripts['test:perf:core']).toBe(
      'cargo build --release --manifest-path src-tauri/Cargo.toml -p aeroworship-core',
    )
  })
})

// `git ls-files` over the whole working tree is a single external process,
// but the default 5 s `testTimeout` gave it zero margin: on this machine it
// has been measured exceeding 5 s on its own even without another agent's
// suite competing for the disk (see PROGRESS.md, SETUP-05 tester entry
// 2026-08-13 10:25 — this exact test passed only because the machine
// happened to be fast that run, not because 5 s was ever a safe budget).
// Sized generously per-`describe`, following the precedent in
// `tests/unit/gitignore-guard.test.js` and `check-fixture-content.e2e.test.js`,
// rather than raising `vitest.config.ts`'s global default, which every future
// fast-failing test in the suite would then also have to pay for.
const GIT_LS_FILES_TIMEOUT_MS = 30_000

describe(
  'tests/{integration,perf} scaffolding (PRD §6.13)',
  { timeout: GIT_LS_FILES_TIMEOUT_MS },
  () => {
    it('both .gitkeep placeholders are tracked in the git index', () => {
      const tracked = execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
        .split('\n')
        .map((line) => line.replace(/\r$/, ''))
      expect(tracked).toContain('tests/integration/.gitkeep')
      expect(tracked).toContain('tests/perf/.gitkeep')
    })
  },
)

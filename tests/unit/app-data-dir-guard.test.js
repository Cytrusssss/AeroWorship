// @ts-check

/**
 * ADR-0025 puts the data root at `%APPDATA%\AeroWorship`, resolved as
 * `app.path().data_dir()` plus the literal leaf `"AeroWorship"` — deliberately
 * *not* Tauri's own `app_data_dir()`, which appends the bundle identifier and
 * would give `%APPDATA%\id.aeroworship.app`. The two directories are both
 * valid, both silent, and would sit side by side with no error and no warning
 * from anything if a second call site ever resolved the other one. The ADR
 * names the guard against that in prose: "jangan pernah memanggil
 * `app_data_dir()` langsung di tempat lain" (`src-tauri/src/db.rs`). Today that
 * guard is the prose itself — nothing in the repository fails if a future
 * command handler calls `app.path().app_data_dir()` directly, which is exactly
 * the ADR-0020 failure class this project otherwise refuses to accept.
 *
 * This asks the real `.rs` source tree whether an actual call to
 * `app_data_dir(...)` exists anywhere — not whether the name is *mentioned*,
 * which `db.rs`'s own doc comments do three times over, explaining why not to
 * call it. The distinguishing mark is the open paren: a call reads
 * `app_data_dir(`, a doc link or a plain mention never does. At the time this
 * test was written there is no legitimate call site anywhere in the tree
 * either — `db.rs` resolves the root through `app.path().data_dir()` plus the
 * `"AeroWorship"` constant, never through `app_data_dir()` — so the assertion
 * is simply "zero calls, full stop", with no per-file exception list to keep
 * in sync.
 */

import { readdirSync, readFileSync, statSync } from 'node:fs'
import { extname, join, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')
const SRC_TAURI = resolve(ROOT, 'src-tauri')

/**
 * Directories under `src-tauri/` that are generated or vendored rather than
 * authored: `target/` is Cargo's build output (and can itself contain crate
 * source pulled from the registry, which this guard has no business judging),
 * `gen/` is Tauri's regenerated capability schema output (`.gitignore` already
 * excludes both).
 */
const EXCLUDED_DIR_NAMES = new Set(['target', 'gen', 'node_modules'])

/** A real call: the open paren is what separates it from a doc mention. */
const CALL_PATTERN = /app_data_dir\(/

/**
 * @param {string} dir Absolute directory to walk.
 * @returns {string[]} Absolute paths of every `.rs` file found, recursively.
 */
function collectRustFiles(dir) {
  /** @type {string[]} */
  const files = []
  for (const entry of readdirSync(dir)) {
    if (EXCLUDED_DIR_NAMES.has(entry)) continue
    const full = join(dir, entry)
    const stat = statSync(full)
    if (stat.isDirectory()) {
      files.push(...collectRustFiles(full))
    } else if (stat.isFile() && extname(full) === '.rs') {
      files.push(full)
    }
  }
  return files
}

describe('app_data_dir() call guard (ADR-0025)', () => {
  it('finds at least one .rs file to check, so a passing suite cannot mean an empty walk', () => {
    const files = collectRustFiles(SRC_TAURI)
    expect(files.length).toBeGreaterThan(0)
  })

  it('is never called anywhere in src-tauri/, including inside db.rs itself', () => {
    const files = collectRustFiles(SRC_TAURI)
    const offenders = files
      .map((file) => ({ file, text: readFileSync(file, 'utf8') }))
      .filter(({ text }) => CALL_PATTERN.test(text))
      .map(({ file }) => file)

    expect(offenders).toEqual([])
  })

  // Negative control: proves the regex itself would catch a real call, so the
  // assertion above is "zero calls found" and not "the pattern never matches
  // anything".
  it('the call pattern does match a synthetic call, so the guard above is not vacuous', () => {
    expect(CALL_PATTERN.test('let root = app.path().app_data_dir()?;')).toBe(true)
  })

  // Negative control in the other direction: a doc comment naming the
  // function without calling it must not trip the guard, or every mention in
  // db.rs explaining the ban would itself fail it.
  it('a bare mention of the name, with no call parens, does not match', () => {
    expect(CALL_PATTERN.test('/// must not call `app_data_dir` directly')).toBe(false)
  })
})

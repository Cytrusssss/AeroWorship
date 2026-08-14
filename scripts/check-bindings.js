// @ts-check

/**
 * `pretypecheck` hook: fails when the committed TypeScript bindings in
 * `src/shared/bindings/` are not what the Rust types would generate right now
 * (NFR-33).
 *
 * How it works: run the generator a second time, into a throwaway directory,
 * and compare. The throwaway directory is the point. `.cargo/config.toml` sends
 * an ordinary `cargo test` run straight into `src/shared/bindings/`, so a check
 * that simply ran the generator and looked at the result would have *repaired*
 * the drift it was asked to detect and then reported success. Cargo does not
 * `force` its `[env]` entries, so setting `TS_RS_EXPORT_DIR` here from the
 * outside wins, and the committed files are never touched by this script.
 *
 * Why `pretypecheck` and not `pretest`: `npm run typecheck` is the gate that
 * consumes these files. A stale binding does not make `vue-tsc` fail — it makes
 * it succeed, against yesterday's contract. Hanging the check on the lifecycle
 * of the command it protects means the two cannot be run apart. The cost is
 * that `npm run typecheck` now needs a Rust toolchain; that is a real cost and
 * it is the reason `npm test` was left alone.
 *
 * Deliberately thin — spawn, read, compare, print, exit. Every decision (what
 * counts as drift, where two files first diverge, when an empty run is a
 * failure) lives in `bindings-guard.js` so it can be unit-tested without cargo.
 */

import { spawnSync } from 'node:child_process'
import { mkdtemp, readdir, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, relative, sep } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

import {
  compareBindings,
  describeFirstDifference,
  summariseComparison,
} from './bindings-guard.js'

// Resolved from this module's own location rather than the working directory,
// so the check behaves the same whether npm, a CI runner or a human started it.
const repoRoot = fileURLToPath(new URL('../', import.meta.url))
const committedDir = join(repoRoot, 'src', 'shared', 'bindings')
const manifestPath = join(repoRoot, 'src-tauri', 'Cargo.toml')

/**
 * The cargo invocation that produces the bindings.
 *
 * `--workspace` is not tidiness — see ADR-0020. `--manifest-path` names the
 * *package* `aeroworship`, so without it this would compile and run the shell
 * crate's tests only and never reach `aeroworship-core`, where every exported
 * type lives today. It would then find no bindings, which `summariseComparison`
 * turns into a failure rather than a false pass; but the failure would read as
 * "the generator produced nothing" and send the reader looking in the wrong
 * place entirely.
 *
 * `export_bindings` is a test-name filter, and it is what keeps this hook from
 * re-running the whole Rust suite on every `npm run typecheck`. It is safe only
 * because an empty result is a failure here: `cargo test <filter>` exits 0 when
 * the filter matches nothing, so a filter that stopped matching — a ts-rs
 * release that renames its generated tests, or the last `#[ts(export)]` being
 * removed — must not be able to read as "clean".
 *
 * No `--release`: the release profile is rejected outright by the
 * `custom-protocol` guard in `src-tauri/src/lib.rs` (ADR-0019). Nothing here
 * wants it — this is a code generator, not a measurement.
 */
const CARGO_ARGS = [
  'test',
  '--workspace',
  '--manifest-path',
  manifestPath,
  'export_bindings',
]

/**
 * Reads every `.ts` file under `dir`, keyed by its path relative to `dir` with
 * `/` separators, so the two sides of the comparison are keyed identically on
 * Windows and elsewhere.
 *
 * Only `.ts` files: `src/shared/bindings/` also holds a `.gitkeep`, which is
 * not part of the contract and is not something the generator will ever emit.
 * Non-files are skipped outright rather than being read and failing with
 * `EISDIR` — a directory named `Foo.ts` is not a binding.
 *
 * A missing directory is reported as empty rather than thrown. For the
 * generated side that is the honest reading (the generator wrote nothing, which
 * the guard treats as a failure); for the committed side it means the
 * diagnostic below says which files are missing instead of printing an ENOENT
 * stack.
 *
 * @param {string} dir
 * @returns {Promise<Map<string, string>>}
 */
async function collectBindings(dir) {
  /** @type {Map<string, string>} */
  const files = new Map()

  /** @type {import('node:fs').Dirent[]} */
  let entries
  try {
    entries = await readdir(dir, { recursive: true, withFileTypes: true })
  } catch {
    return files
  }

  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith('.ts')) continue
    const absolute = join(entry.parentPath, entry.name)
    const key = relative(dir, absolute).split(sep).join('/')
    files.set(key, await readFile(absolute, 'utf8'))
  }

  return files
}

/** @returns {Promise<number>} Process exit code. */
async function main() {
  const scratch = await mkdtemp(join(tmpdir(), 'aeroworship-bindings-'))

  try {
    console.log('bindings guard: regenerating the Rust→TypeScript contract…')

    const result = spawnSync('cargo', CARGO_ARGS, {
      cwd: repoRoot,
      // Cargo's `[env]` table does not override variables already present in
      // the environment, so this redirects the export away from the committed
      // directory without editing any configuration.
      env: { ...process.env, TS_RS_EXPORT_DIR: scratch },
      encoding: 'utf8',
    })

    if (result.error) {
      console.error(
        'bindings guard: could not run `cargo` — the generator never ran, so ' +
          'nothing about the committed bindings was verified. A Rust toolchain ' +
          'is required by this check because the TypeScript in ' +
          'src/shared/bindings/ is generated from Rust (NFR-33).',
      )
      console.error(String(result.error))
      return 1
    }

    if (result.status !== 0) {
      console.error(
        `bindings guard: \`cargo ${CARGO_ARGS.join(' ')}\` exited ${result.status}. ` +
          'The generator did not complete, so the committed bindings were not ' +
          'checked — fix the Rust build first; this is not a finding about ' +
          'src/shared/bindings/.',
      )
      if (result.stdout) console.error(result.stdout)
      if (result.stderr) console.error(result.stderr)
      return 1
    }

    const generated = await collectBindings(scratch)
    const committed = await collectBindings(committedDir)

    const comparison = compareBindings(generated, committed)
    const { exitCode, inspected, generatedNothing } = summariseComparison(comparison)

    if (generatedNothing) {
      console.error(
        'bindings guard: the generator produced no files at all, so there was ' +
          'nothing to compare and nothing was verified.\n' +
          `\`cargo ${CARGO_ARGS.join(' ')}\` succeeded, which means the test-name ` +
          'filter matched nothing — cargo exits 0 in that case. Either the last ' +
          '`#[ts(export)]` was removed from the Rust types, or ts-rs no longer ' +
          'names its generated tests `export_bindings_*` and the filter in this ' +
          'file has to be updated.',
      )
      return 1
    }

    for (const name of comparison.differing) {
      const difference = describeFirstDifference(
        generated.get(name) ?? '',
        committed.get(name) ?? '',
      )
      console.error(`bindings guard: src/shared/bindings/${name} is out of date.`)
      if (difference) {
        console.error(`  first difference at line ${difference.line}:`)
        console.error(`    generated now: ${formatLine(difference.generated)}`)
        console.error(`    committed:     ${formatLine(difference.committed)}`)
      }
    }

    for (const name of comparison.missing) {
      console.error(
        `bindings guard: src/shared/bindings/${name} is generated but not committed. ` +
          'A Rust type gained `#[ts(export)]` — or was renamed — without its ' +
          'binding being checked in.',
      )
    }

    for (const name of comparison.unexpected) {
      console.error(
        `bindings guard: src/shared/bindings/${name} is committed but the generator ` +
          'does not produce it. Either the Rust type behind it was removed or ' +
          'renamed and this file is its leftover, or the file was written by ' +
          'hand — that directory is generated output (PRD §6.13, "do not edit").',
      )
    }

    if (exitCode !== 0) {
      console.error(
        `\nbindings guard: the committed contract does not match the Rust types.\n` +
          'Regenerate it with `npm run bindings:generate` and commit the result ' +
          'in the same change as the Rust edit. Committed generated files are how ' +
          'the frontend reads the contract without a Rust toolchain; when they ' +
          'lag behind, `npm run typecheck` passes against a contract the backend ' +
          'no longer speaks and no gate says a word (NFR-33).',
      )
      return exitCode
    }

    console.log(
      `bindings guard: ${inspected} generated file(s) match the Rust types ` +
        `(${[...generated.keys()].join(', ')}).`,
    )
    return 0
  } finally {
    await rm(scratch, { recursive: true, force: true })
  }
}

/**
 * @param {string | null} line
 * @returns {string} The line as it should appear in a diagnostic — `(no such
 *   line)` when one file simply ends before the other, which is what an added
 *   or removed field at the end of a type looks like.
 */
function formatLine(line) {
  return line === null ? '(no such line)' : JSON.stringify(line)
}

process.exitCode = await main()

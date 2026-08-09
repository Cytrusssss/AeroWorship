// @ts-check

/**
 * `postbuild` hook: fails the build when a document in `dist/` carries markup
 * the production CSP would reject (ADR-0015), or when it carries nothing at all.
 *
 * Deliberately thin — read, call, print, exit. Every decision (which documents
 * to open, which expected ones are absent, what counts as a violation, whether
 * a document loads anything, and when the guard may return 0) lives in
 * `dist-html-guard.js` so it can be unit-tested without touching the file
 * system. What stays here is the file system access, the wording of the
 * diagnostics, and `EXPECTED_DOCUMENTS`.
 */

import { readdir, readFile } from 'node:fs/promises'
import { join } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

import {
  findCspViolations,
  formatViolation,
  hasModuleScript,
  selectDocuments,
  summariseDocuments,
} from './dist-html-guard.js'

// Resolved from this module's own location rather than the working directory,
// so the check behaves the same whether npm, the Tauri CLI or a human started
// it. `dist/` is where `vite.config.ts` points `build.outDir`.
//
// Converted to a path once, here, and joined with `join` from then on. The
// URL form is a resolver, not a path concatenator: `new URL(name, distDir)`
// percent-decodes, so a document actually named `a%41.html` resolved to
// `aA.html` and — if that file existed and was clean — was reported clean
// without ever being opened. Every name below comes out of `readdir`, so this
// is not reachable today; it is fail-open where the rest of this guard is
// fail-closed, which is the wrong direction to leave lying around.
const distDir = fileURLToPath(new URL('../dist/', import.meta.url))

/**
 * The documents `build.rollupOptions.input` in `vite.config.ts` declares, named
 * here rather than discovered. Discovery alone cannot tell "the build emitted
 * both windows and they are clean" apart from "the build lost an entry point
 * and the one that survived is clean" — a mistyped `input` key or a deleted
 * entry would ship half the application past a green gate. That is the same
 * class of silent pass as ADR-0020.
 *
 * Adding a third entry point means editing `vite.config.ts` **and** this list.
 *
 * Both names are bare, which is a claim that the document lands at the root of
 * `dist/` — true today because `build.rollupOptions.input` names its sources at
 * the root of `src/`. Move an entry point into a subdirectory and its output
 * moves with it, at which point the name here stops matching and the message
 * below says "the build dropped a window" about a window that is present. The
 * fix then is to write the new relative path (`pages/output.html`) here, not to
 * drop the entry from the list.
 */
const EXPECTED_DOCUMENTS = ['index.html', 'output.html']

/** @returns {Promise<number>} Process exit code. */
async function main() {
  /** @type {string[]} */
  let entries
  try {
    // Recursive: nothing stops Vite — or a plugin added later — from emitting a
    // document into a subdirectory, and a top-level-only scan would leave those
    // unopened while still printing "clean".
    entries = await readdir(distDir, { recursive: true })
  } catch (error) {
    console.error(
      `dist HTML guard: cannot read ${distDir} — run \`npm run build\` first.`,
    )
    console.error(String(error))
    return 1
  }

  // Selection is a pure function so it can be tested; see `selectDocuments`
  // for what it does to the raw listing and why.
  const { documents, missing } = selectDocuments(entries, EXPECTED_DOCUMENTS)

  // An empty `dist/` means the build produced nothing to check, which is a
  // failure of this guard's premise rather than a clean run.
  if (documents.length === 0) {
    console.error(`dist HTML guard: no *.html found in ${distDir}; nothing was verified.`)
    return 1
  }

  if (missing.length > 0) {
    console.error(
      `dist HTML guard: expected document(s) absent from ${distDir}: ` +
        `${missing.join(', ')}.\n` +
        `Found instead: ${documents.join(', ')}.\n` +
        'Each absent name is an entry point declared in `build.rollupOptions.input` ' +
        '(vite.config.ts), so its absence means the build dropped a window — not ' +
        'that there was less to check. Confirm the `input` key and its source HTML ' +
        'under `src/` still exist. If the entry point was removed deliberately, ' +
        'remove it from `EXPECTED_DOCUMENTS` in this file in the same change.',
    )
    return 1
  }

  /** @type {import('./dist-html-guard.js').DocumentReport[]} */
  const reports = []
  for (const name of documents) {
    /** @type {string} */
    let html
    try {
      html = await readFile(join(distDir, name), 'utf8')
    } catch (error) {
      // A recursive `readdir` lists directories too, so a *directory* named
      // `x.html` reaches this loop and fails with `EISDIR`. Unhandled, that
      // prints a stack trace with this script's frames in it and reads like a
      // bug in the guard rather than a finding about `dist/`. The exit code was
      // non-zero either way; what this earns is a diagnostic that names the
      // document and says what to do about it.
      console.error(
        `dist HTML guard: cannot read dist/${name}; it was listed in dist/ but could ` +
          'not be opened as a document — a directory with an `.html` name, a broken ' +
          'symlink or a permission problem. Nothing about it was verified.',
      )
      console.error(String(error))
      reports.push({ name, violations: null, loadsModule: false })
      continue
    }

    const violations = findCspViolations(html)
    for (const violation of violations) {
      console.error(formatViolation(`dist/${name}`, violation))
    }

    const loadsModule = hasModuleScript(html)
    if (!loadsModule) {
      console.error(
        `dist HTML guard: dist/${name} loads no module — it carries no ` +
          '`<script type="module" src="…">`, so nothing in it will ever run.',
      )
    }

    reports.push({ name, violations, loadsModule })
  }

  // Counting and the exit code are a pure function so both can be tested; see
  // `summariseDocuments` for why the second half of that matters.
  const {
    violations: violationCount,
    unreadable,
    blank,
    exitCode,
  } = summariseDocuments(reports)

  if (violationCount > 0) {
    console.error(
      `\ndist HTML guard: ${violationCount} violation(s) across ${documents.length} document(s).\n` +
        "The production CSP is `style-src 'self'; script-src 'self'` " +
        '(src-tauri/tauri.conf.json, ADR-0015): inline styles and inline scripts ' +
        'are dropped at runtime, so the window would ship unstyled or with its ' +
        'script never running. Move the CSS into an SFC `<style>` block — Vite ' +
        'extracts those to a linked stylesheet — and the script into a module ' +
        'under `src/`.',
    )
  }

  // Deliberately not phrased as a CSP finding. Both failures print through the
  // same stream, so the reader has to be able to tell "your document contains
  // something forbidden" from "your document contains nothing that runs"
  // without opening this script to work out which one fired.
  if (blank > 0) {
    console.error(
      `\ndist HTML guard: ${blank} of ${documents.length} document(s) load no module.\n` +
        'This is not a CSP violation: nothing in those documents is forbidden, ' +
        'there is just nothing in them that runs. Every other check here asks ' +
        'whether something forbidden is present, so a document truncated to ' +
        'zero bytes — or emitted without its entry script — passes all of them ' +
        'while the window still opens black in front of a congregation. Check ' +
        'that the entry in `build.rollupOptions.input` (vite.config.ts) points ' +
        'at a source HTML file that references its module under `src/`.',
    )
  }

  // Counted apart from violations: an unopened document is a document nothing
  // is known about, and calling it a violation would send the reader looking
  // for inline styles that were never read.
  if (unreadable > 0) {
    console.error(
      `\ndist HTML guard: ${unreadable} of ${documents.length} document(s) could not be read, ` +
        'so they were not checked at all.',
    )
  }

  if (exitCode !== 0) return exitCode

  console.log(
    `dist HTML guard: ${documents.length} document(s) clean (${documents.join(', ')}).`,
  )
  return exitCode
}

process.exitCode = await main()

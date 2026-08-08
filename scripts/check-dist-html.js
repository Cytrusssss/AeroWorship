// @ts-check

/**
 * `postbuild` hook: fails the build when a document in `dist/` carries markup
 * the production CSP would reject (ADR-0015).
 *
 * Deliberately thin — read, call, exit. The scanning logic lives in
 * `dist-html-guard.js` so it can be unit-tested without touching the file
 * system.
 */

import { readdir, readFile } from 'node:fs/promises'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

import { findCspViolations, formatViolation } from './dist-html-guard.js'

// Resolved from this module's own location rather than the working directory,
// so the check behaves the same whether npm, the Tauri CLI or a human started
// it. `dist/` is where `vite.config.ts` points `build.outDir`.
const distDir = new URL('../dist/', import.meta.url)

/** @returns {Promise<number>} Process exit code. */
async function main() {
  /** @type {string[]} */
  let entries
  try {
    entries = await readdir(distDir)
  } catch (error) {
    console.error(
      `dist HTML guard: cannot read ${fileURLToPath(distDir)} — run \`npm run build\` first.`,
    )
    console.error(String(error))
    return 1
  }

  const documents = entries.filter((name) => name.toLowerCase().endsWith('.html')).sort()

  // An empty `dist/` means the build produced nothing to check, which is a
  // failure of this guard's premise rather than a clean run.
  if (documents.length === 0) {
    console.error(
      `dist HTML guard: no *.html found in ${fileURLToPath(distDir)}; nothing was verified.`,
    )
    return 1
  }

  let failures = 0
  for (const name of documents) {
    const html = await readFile(new URL(name, distDir), 'utf8')
    const violations = findCspViolations(html)
    if (violations.length === 0) continue

    failures += violations.length
    for (const violation of violations) {
      console.error(formatViolation(`dist/${name}`, violation))
    }
  }

  if (failures > 0) {
    console.error(
      `\ndist HTML guard: ${failures} violation(s) across ${documents.length} document(s).\n` +
        "The production CSP is `style-src 'self'; script-src 'self'` " +
        '(src-tauri/tauri.conf.json, ADR-0015): inline styles and inline scripts ' +
        'are dropped at runtime, so the window would ship unstyled or inert. ' +
        'Move the CSS into an SFC `<style>` block — Vite extracts those to a ' +
        'linked stylesheet — and the script into a module under `src/`.',
    )
    return 1
  }

  console.log(
    `dist HTML guard: ${documents.length} document(s) clean (${documents.join(', ')}).`,
  )
  return 0
}

process.exitCode = await main()

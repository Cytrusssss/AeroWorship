// @ts-check

/**
 * `prelint` hook: fails when the three lists that describe this app's IPC
 * surface disagree — the `#[tauri::command]` definitions under
 * `src-tauri/src/`, the `generate_handler!` registrations, and the committed
 * inventory `src-tauri/commands.inventory.md` (ADR-0039, NFR-14).
 *
 * Why `prelint` and not one of the other five gates. This guard reads Rust
 * source as text and a Markdown file: no toolchain, no build output, no git
 * index — so it belongs on the cheapest gate that always runs, and `npm run
 * lint` is exactly the gate whose job is "the source obeys rules the compiler
 * does not enforce". `cargo clippy` would be the natural home for a Rust-source
 * rule, but nothing in Clippy can see a Markdown file, and putting the check in
 * `build.rs` would mean writing a build script for a crate that deliberately
 * has a plain `tauri_build::build()` (ADR-0039's premise) and would only run
 * when the crate rebuilds. `pretest` was the other candidate and is taken:
 * `check-fixture-content.js` owns it, and `tests/unit/setup05-lifecycle.test.js`
 * pins that hook's exact string, so chaining onto it would have meant editing a
 * test to accommodate a guard — the wrong way round. `postbuild` and
 * `pretypecheck` both hang off artefacts this check does not need.
 *
 * Like `postlint`, this runs even when the thing it guards is untouched, which
 * is the point: the failure it looks for is a command that was added and never
 * written down, and that is invisible in every other gate.
 *
 * Deliberately thin — walk, read, reconcile, print, exit. Every decision (what
 * counts as a definition, what a discrepancy means, when an empty run is a
 * failure) lives in `command-inventory-guard.js` so it can be unit-tested
 * without a Rust toolchain.
 */

import { readdir, readFile } from 'node:fs/promises'
import { join, relative, sep } from 'node:path'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

import {
  findCommandDefinitions,
  findHandlerRegistrations,
  modulePathForSource,
  parseInventory,
  reconcileCommands,
  summariseInventoryRun,
} from './command-inventory-guard.js'

// Resolved from this module's own location rather than the working directory,
// so the check behaves the same whether npm, a CI runner or a human started it.
const repoRoot = fileURLToPath(new URL('../', import.meta.url))
const sourceDir = join(repoRoot, 'src-tauri', 'src')
const inventoryPath = join(repoRoot, 'src-tauri', 'commands.inventory.md')

/** For diagnostics: the inventory named the way a reader would type it. */
const INVENTORY_NAME = 'src-tauri/commands.inventory.md'

/**
 * Reads every `.rs` file under `dir`, keyed by its path relative to `dir` with
 * `/` separators.
 *
 * The whole tree, not just `commands/`: a `#[tauri::command]` in `services/`
 * would be just as reachable and just as unlisted, and a guard that only looked
 * where the convention says to look would be blind to the one case where the
 * convention was broken.
 *
 * @param {string} dir
 * @returns {Promise<Map<string, string>>}
 */
async function collectSources(dir) {
  /** @type {Map<string, string>} */
  const files = new Map()

  /** @type {import('node:fs').Dirent[]} */
  let entries
  try {
    entries = await readdir(dir, { recursive: true, withFileTypes: true })
  } catch {
    // Reported as an empty walk, which `summariseInventoryRun` treats as a
    // failure — never as "no commands found, all clean".
    return files
  }

  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith('.rs')) continue
    const absolute = join(entry.parentPath, entry.name)
    files.set(
      relative(dir, absolute).split(sep).join('/'),
      await readFile(absolute, 'utf8'),
    )
  }

  return files
}

/** @returns {Promise<number>} Process exit code. */
async function main() {
  const sources = await collectSources(sourceDir)

  /** @type {string[]} */
  const defined = []
  /** @type {string[]} */
  const registered = []
  /** @type {string[]} */
  const problems = []
  let handlerInvocations = 0

  for (const [name, source] of [...sources].sort(([a], [b]) => a.localeCompare(b))) {
    const modulePath = modulePathForSource(name)
    // `collectSources` already filtered to `.rs`, so this cannot be `null`; the
    // guard is here because a silently skipped file is a silently unwatched
    // command.
    if (modulePath === null) continue

    const definitions = findCommandDefinitions(modulePath, source)
    defined.push(...definitions.commands)
    problems.push(
      ...definitions.problems.map((problem) => `src-tauri/src/${name}: ${problem}`),
    )

    const registrations = findHandlerRegistrations(source)
    registered.push(...registrations.commands)
    handlerInvocations += registrations.invocations
    problems.push(
      ...registrations.problems.map((problem) => `src-tauri/src/${name}: ${problem}`),
    )
  }

  /** @type {string[]} */
  let inventoried = []
  try {
    const parsed = parseInventory(await readFile(inventoryPath, 'utf8'))
    inventoried = parsed.commands
    problems.push(...parsed.problems.map((problem) => `${INVENTORY_NAME}: ${problem}`))
  } catch (error) {
    problems.push(
      `${INVENTORY_NAME} could not be read, so nothing was verified against it: ` +
        String(error),
    )
  }

  const reconciliation = reconcileCommands({ defined, registered, inventoried })
  const summary = summariseInventoryRun({
    reconciliation,
    problems,
    filesScanned: sources.size,
    handlerInvocations,
  })

  if (summary.scannedNothing) {
    console.error(
      `command inventory guard: no .rs files were found under ${sourceDir}, so the IPC ` +
        'surface was not examined at all. Confirm this is being run inside the ' +
        'AeroWorship working tree.',
    )
  }

  if (summary.foundNoHandler) {
    console.error(
      'command inventory guard: no `generate_handler!` invocation was found anywhere ' +
        'under src-tauri/src/. Either the builder in lib.rs lost its `invoke_handler` — ' +
        'in which case the app has no IPC surface at all — or the call was written in a ' +
        'shape this guard does not recognise, in which case it is watching nothing. ' +
        'Neither is a clean run.',
    )
  }

  for (const problem of problems) {
    console.error(`command inventory guard: ${problem}`)
  }

  for (const discrepancy of reconciliation.discrepancies) {
    console.error(
      `command inventory guard: \`${discrepancy.name}\` ${discrepancy.reason}`,
    )
    console.error(
      `    defined: ${yesNo(discrepancy.defined)}  ` +
        `registered in generate_handler!: ${yesNo(discrepancy.registered)}  ` +
        `in ${INVENTORY_NAME}: ${yesNo(discrepancy.inventoried)}`,
    )
  }

  if (summary.exitCode !== 0) {
    console.error(
      `\ncommand inventory guard: the IPC surface is not what all three lists say it is.\n` +
        '`generate_handler!` in src-tauri/src/lib.rs is the only true list of what the ' +
        'webview can call: app-defined commands are not ACL-gated, so they appear in no ' +
        `capabilities/*.json and in nothing under gen/schemas/ (ADR-0039). ${INVENTORY_NAME} ` +
        'is what gives an NFR-14 release review one file to read instead of a macro call ' +
        'in a builder chain — which is why an unlisted command is a failure here and not ' +
        'a note.',
    )
    return summary.exitCode
  }

  console.log(
    `command inventory guard: ${summary.checked} command(s) defined, registered and ` +
      `inventoried (${reconciliation.agreed.join(', ')}).`,
  )
  return 0
}

/**
 * @param {boolean} value
 * @returns {string}
 */
function yesNo(value) {
  return value ? 'yes' : 'no'
}

process.exitCode = await main()

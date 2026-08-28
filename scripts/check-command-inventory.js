// @ts-check

/**
 * `prelint` hook: fails when the lists that describe this app's IPC surface
 * disagree — the `#[tauri::command]` definitions under `src-tauri/src/`, the
 * `generate_handler!` registrations, the committed inventory
 * `src-tauri/commands.inventory.md`, and, since SEC-01, the commands
 * `src-tauri/capabilities/*.json` actually grant through the permissions in
 * `src-tauri/permissions/` (ADR-0039, ADR-0041, NFR-14).
 *
 * The fourth list is the one with a runtime consequence. `src-tauri/permissions/`
 * existing is what turns the app ACL manifest on, and from that moment an
 * app-defined command no capability names is rejected at invoke time with
 * `Command {} not allowed by ACL` — with no other gate red, and in practice no
 * way back but a rebuild, because `dynamic-acl` is off (ADR-0013).
 *
 * "App-defined command" is the honest scope and the reason this file says it
 * that way. The gate in `tauri-2.11.5/src/webview/mod.rs:1823-1826` ends
 * `&& request.cmd != FETCH_CHANNEL_DATA_COMMAND && invoke.acl.is_none()`, so
 * `plugin:__TAURI_CHANNEL__|fetch` is exempt from the ACL by construction — the
 * one command no capability has to name. See `src-tauri/src/commands/mod.rs`
 * for what that costs and who owns it; nothing this guard reconciles can see it,
 * which is exactly why it is written down rather than implied.
 *
 * Its mirror image is just as quiet: a permission left behind by a deleted
 * command idles for ever. Both are set comparisons, which is why they belong
 * here rather than in a reviewer's head.
 *
 * Why `prelint` and not one of the other five gates. This guard reads Rust
 * source as text, a Markdown file and a handful of JSON: no toolchain, no build
 * output, no git index — so it belongs on the cheapest gate that always runs,
 * and `npm run lint` is exactly the gate whose job is "the source obeys rules
 * the compiler does not enforce". `cargo clippy` would be the natural home for
 * a Rust-source
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
  findAmbiguousCommandNames,
  findCommandDefinitions,
  findHandlerRegistrations,
  findRuntimeAclEscapes,
  modulePathForSource,
  parseAppPermissionFile,
  parseCapabilityFile,
  parseInventory,
  reconcileCommands,
  resolveAppCommandGrants,
  summariseInventoryRun,
} from './command-inventory-guard.js'

// Resolved from this module's own location rather than the working directory,
// so the check behaves the same whether npm, a CI runner or a human started it.
const repoRoot = fileURLToPath(new URL('../', import.meta.url))
const sourceDir = join(repoRoot, 'src-tauri', 'src')
const inventoryPath = join(repoRoot, 'src-tauri', 'commands.inventory.md')
const permissionsDir = join(repoRoot, 'src-tauri', 'permissions')
const capabilitiesDir = join(repoRoot, 'src-tauri', 'capabilities')

/** For diagnostics: the inventory named the way a reader would type it. */
const INVENTORY_NAME = 'src-tauri/commands.inventory.md'

/**
 * Extensions `tauri-build` accepts for a permission or a capability file.
 *
 * All of them are listed, not just the one this repository writes, because the
 * point of reading them is to see every grant: a `.toml` permission this guard
 * skipped would be a grant nobody reviewed, which is the exact shape of the
 * failure it exists to prevent. Only JSON is parsed; the others are reported.
 */
const ACL_FILE_EXTENSIONS = ['.json', '.toml', '.json5']

/**
 * Reads every `.rs` file under `dir`, keyed by its path relative to `dir` with
 * `/` separators.
 *
 * The whole tree, not just `commands/`: a `#[tauri::command]` in `services/`
 * would be just as reachable and just as unlisted, and a guard that only looked
 * where the convention says to look would be blind to the one case where the
 * convention was broken.
 *
 * ## Symbolic links are refused, here and in `collectAclFiles`
 *
 * A link is neither followed nor ignored: it comes back in `links`, and the
 * caller turns it into a problem. The rule is the same in both walkers because
 * the reason is the same in both, and the one place it was NOT applied is the
 * one place it was proved to matter: a junction at `src-tauri/src/linked`
 * pointing at a directory holding a `runtime_authority_mut` call made this
 * guard exit 0, with its reassuring summary line, over source `rustc` compiles
 * perfectly well.
 *
 * The reason is that this walk's behaviour is not a property of links, it is a
 * property of which node API you happened to call. Measured on node 22.16.0,
 * Windows 11, one process, one directory, one junction:
 *
 *   | call                                                  | descends? |
 *   | ----------------------------------------------------- | --------- |
 *   | `fs.readdirSync(dir, {recursive, withFileTypes})`      | yes       |
 *   | `fs/promises.readdir(dir, {recursive, withFileTypes})` | **no**    |
 *   | `fs/promises.readdir(dir, {recursive})` (names only)   | yes       |
 *
 * Identical on `C:` and on `D:`, and identical for a junction whose target is
 * inside the walked tree, outside it, or on the other volume. This module calls
 * the middle row, so everything under a directory link is invisible to it while
 * being ordinary source to the compiler. A file link is worse and simpler: its
 * `Dirent` is `isFile() === false` in every one of the three, so it is dropped
 * by the extension test below without ever being counted.
 *
 * Do not narrow this to "refuse file links only". That is the one edit the
 * table above forbids, and the reason it looks safe is that a reader who
 * measured the first row would conclude directory links are harmless. Four
 * people measured this and got three different answers; the guard depends on
 * none of them by refusing both.
 *
 * @param {string} dir
 * @returns {Promise<{ files: Map<string, string>, links: string[] }>}
 */
async function collectSources(dir) {
  /** @type {Map<string, string>} */
  const files = new Map()
  /** @type {string[]} */
  const links = []

  /** @type {import('node:fs').Dirent[]} */
  let entries
  try {
    entries = await readdir(dir, { recursive: true, withFileTypes: true })
  } catch {
    // Reported as an empty walk, which `summariseInventoryRun` treats as a
    // failure — never as "no commands found, all clean".
    return { files, links }
  }

  for (const entry of entries) {
    const absolute = join(entry.parentPath, entry.name)
    const name = relative(dir, absolute).split(sep).join('/')

    if (entry.isSymbolicLink()) {
      links.push(name)
      continue
    }

    if (!entry.isFile() || !entry.name.endsWith('.rs')) continue
    files.set(name, await readFile(absolute, 'utf8'))
  }

  return { files, links }
}

/**
 * Reads every ACL file under `dir`, keyed by its path relative to `dir` with
 * `/` separators.
 *
 * A file whose IMMEDIATE parent is named `schemas` is skipped, because what
 * that directory holds is the JSON Schema a permission file is written against,
 * not a permission.
 *
 * "Immediate parent" is the whole rule, and it is copied from
 * `tauri-utils-2.9.3/src/acl/build.rs` rather than approximated: both
 * `define_permissions` (`:88`) and `parse_capabilities` (`:217`) filter on
 * `p.parent().unwrap().file_name().unwrap() != "schemas"`.
 *
 * One rule here, two constants there, and they are only equal by coincidence:
 * `:88` compares against `PERMISSION_SCHEMAS_FOLDER_NAME` (`acl/mod.rs:46`) and
 * `:217` against `CAPABILITIES_SCHEMA_FOLDER_NAME` (`acl/build.rs:50`). Both
 * spell `"schemas"` today. `:216` also carries tauri's own
 * `// TODO: remove this before stable`. So this is a rule to re-read on a tauri
 * upgrade, not one to assume stable — if those two constants ever diverge, or
 * the capability filter goes away, one directory of this repository stops being
 * read the way tauri reads it and nothing here would say so. Both are compared
 * as `OsStr`, so `Schemas/` is not skipped by tauri and is not skipped here
 * either; that much is consistent rather than divergent.
 *
 * Skipping a file
 * because *any* ancestor is called `schemas` would be a broader rule than
 * tauri's, and broader in the one direction that matters: `permissions/schemas/
 * nested/reach.json` is a file `tauri_build` reads and grants from, so a guard
 * that skipped it would print a clean run over a permission nobody reviewed.
 * That is not a hypothetical — it is how a redeclared identifier slips past the
 * "two permissions call themselves the same thing" rule, which has unit tests
 * that would simply never run.
 *
 * A symbolic link is refused, exactly as in `collectSources` and for the reason
 * written out there. The half of it specific to this directory: `tauri_utils`
 * reaches these files with `glob::glob(...).flat_map(|p| p.canonicalize())`, and
 * `canonicalize` resolves links, so tauri grants from a linked permission file
 * whatever this walk decides to do about it. Refusing costs nothing — no ACL
 * file in this repository is a link.
 *
 * Nothing else is skipped — a missing directory comes back empty, which the
 * caller turns into "no ACL manifest" rather than into a crash, and every other
 * file becomes either a parse or a problem.
 *
 * @param {string} dir
 * @returns {Promise<{ files: Map<string, string>, links: string[] }>}
 */
async function collectAclFiles(dir) {
  /** @type {Map<string, string>} */
  const files = new Map()
  /** @type {string[]} */
  const links = []

  /** @type {import('node:fs').Dirent[]} */
  let entries
  try {
    entries = await readdir(dir, { recursive: true, withFileTypes: true })
  } catch {
    return { files, links }
  }

  for (const entry of entries) {
    const absolute = join(entry.parentPath, entry.name)
    const name = relative(dir, absolute).split(sep).join('/')
    const segments = name.split('/')
    if (segments.at(-2) === 'schemas') continue

    if (entry.isSymbolicLink()) {
      links.push(name)
      continue
    }

    if (!entry.isFile()) continue
    if (!ACL_FILE_EXTENSIONS.some((extension) => entry.name.endsWith(extension))) continue
    files.set(name, await readFile(absolute, 'utf8'))
  }

  return { files, links }
}

/** @returns {Promise<number>} Process exit code. */
async function main() {
  const sourceTree = await collectSources(sourceDir)
  const sources = sourceTree.files

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

    problems.push(
      ...findRuntimeAclEscapes(source).map(
        (problem) => `src-tauri/src/${name}: ${problem}`,
      ),
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

  const permissionTree = await collectAclFiles(permissionsDir)
  const capabilityTree = await collectAclFiles(capabilitiesDir)
  const permissionFiles = permissionTree.files
  const capabilityFiles = capabilityTree.files

  for (const [directory, tree] of /** @type {const} */ ([
    ['src-tauri/src', sourceTree],
    ['src-tauri/permissions', permissionTree],
    ['src-tauri/capabilities', capabilityTree],
  ])) {
    for (const link of tree.links) {
      problems.push(
        `${directory}/${link} is a symbolic link, and this guard will not walk one. ` +
          'rustc compiles through a linked directory and tauri-build canonicalises the ' +
          'paths it globs, so both read what is behind it; whether THIS walk does depends ' +
          'on which node readdir overload is called, and the one used here does not. ' +
          'Everything under the link would therefore be compiled, and granted, and never ' +
          'examined by any list this guard reconciles. Put the files in the tree instead',
      )
    }
  }

  /** @type {import('./command-inventory-guard.js').AppPermission[]} */
  const permissions = []
  for (const [name, text] of [...permissionFiles].sort(([a], [b]) =>
    a.localeCompare(b),
  )) {
    const where = `src-tauri/permissions/${name}`
    if (!name.endsWith('.json')) {
      problems.push(
        `${where}: tauri-build reads this file and this guard cannot. Permissions in this ` +
          'repository are written in JSON, the same language as the capabilities; a ' +
          'permission the guard cannot read is a grant nobody reviews',
      )
      continue
    }
    const parsed = parseAppPermissionFile(text)
    permissions.push(...parsed.permissions)
    problems.push(...parsed.problems.map((problem) => `${where}: ${problem}`))
  }

  /** @type {import('./command-inventory-guard.js').ParsedCapability[]} */
  const capabilities = []
  for (const [name, text] of [...capabilityFiles].sort(([a], [b]) =>
    a.localeCompare(b),
  )) {
    const where = `src-tauri/capabilities/${name}`
    if (!name.endsWith('.json')) {
      problems.push(
        `${where}: tauri-build reads this file and this guard cannot. Capabilities in ` +
          'this repository are written in JSON; one the guard cannot read is a window ' +
          'whose grants nobody reviews',
      )
      continue
    }
    const parsed = parseCapabilityFile(text)
    capabilities.push(...parsed.capabilities)
    problems.push(...parsed.problems.map((problem) => `${where}: ${problem}`))
  }

  const grants = resolveAppCommandGrants({ permissions, capabilities })
  problems.push(...grants.problems)
  problems.push(...findAmbiguousCommandNames(registered))

  const reconciliation = reconcileCommands({
    defined,
    registered,
    inventoried,
    granted: grants.granted,
    aclEnabled: grants.aclEnabled,
  })
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
        `in ${INVENTORY_NAME}: ${yesNo(discrepancy.inventoried)}` +
        // The fourth column appears only when there is an ACL manifest to read
        // it from. On a tree with no src-tauri/permissions/, no capability can
        // grant an app-defined command at all, and printing "no" there would
        // report an absence as a finding.
        (grants.aclEnabled
          ? `  named by a capability: ${yesNo(discrepancy.granted)}`
          : ''),
    )
  }

  if (summary.exitCode !== 0) {
    console.error(
      grants.aclEnabled
        ? `\ncommand inventory guard: the IPC surface is not what all four lists say it is.\n` +
            '`generate_handler!` in src-tauri/src/lib.rs is the only true list of what the ' +
            'webview can call, and since src-tauri/permissions/ exists it is no longer the ' +
            'only list that binds: this application ships an ACL manifest, so a command no ' +
            'capability names is rejected at invoke time and only a rebuild fixes it ' +
            `(ADR-0013, ADR-0041). ${INVENTORY_NAME} is what gives an NFR-14 release ` +
            'review one file to read instead of a macro call in a builder chain, and the ' +
            'capabilities are what decide which window may call what — which is why a ' +
            'disagreement between any two of the four is a failure here and not a note.'
        : `\ncommand inventory guard: the IPC surface is not what all three lists say it is.\n` +
            '`generate_handler!` in src-tauri/src/lib.rs is the only true list of what the ' +
            'webview can call: with no src-tauri/permissions/ in this tree, app-defined ' +
            'commands are not ACL-gated, so they appear in no capabilities/*.json and in ' +
            `nothing under gen/schemas/ (ADR-0039). ${INVENTORY_NAME} is what gives an ` +
            'NFR-14 release review one file to read instead of a macro call in a builder ' +
            'chain — which is why an unlisted command is a failure here and not a note.',
    )
    return summary.exitCode
  }

  console.log(
    `command inventory guard: ${summary.checked} command(s) defined, registered and ` +
      `inventoried (${reconciliation.agreed.join(', ')}).`,
  )
  if (grants.aclEnabled) {
    console.log(
      `command inventory guard: the app ACL manifest is ON (${permissions.length} ` +
        'permission(s) under src-tauri/permissions/); every command above is named by a ' +
        'capability, no permission is left over, and no capability that grants an ' +
        'app-defined command names a glob or the `output` window.',
    )
  }
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

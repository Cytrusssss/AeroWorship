// @ts-check

/**
 * Proves `check-command-inventory.js` — the actual `prelint` entry point, not a
 * reassembly of its pure functions — goes red in each of the three directions
 * an IPC surface can drift, and green only when all three lists agree
 * (ADR-0039, NFR-14).
 *
 * Why a spawned copy rather than only unit tests. `check-command-inventory.js`
 * resolves the tree it inspects from its own file location
 * (`fileURLToPath(new URL('../', import.meta.url))`), by design, so it can never
 * be pointed at another tree by an argument or an environment variable. Copying
 * it and `command-inventory-guard.js` into a throwaway directory makes that
 * directory the repository root it examines — the same technique, and the same
 * reasoning, as `tests/unit/check-fixture-content.e2e.test.js`.
 *
 * What that buys over `scripts/command-inventory-guard.test.js`, which already
 * drives every decision with literal strings: four things live in the runner and
 * in nothing the pure functions can see, and three of them are decisions the
 * implementer recorded as deliberate.
 *
 *   1. The walk covers **all** of `src-tauri/src/`, not just `commands/`.
 *   2. `generate_handler!` is read from **every** file, not just `lib.rs` — a
 *      second `invoke_handler` on a `WebviewWindowBuilder` is an equally
 *      reachable surface.
 *   3. A missing `src-tauri/src/` is reported as an empty walk and fails, rather
 *      than throwing or reading as clean.
 *   4. A missing or unreadable inventory becomes a problem, not a crash.
 *   5. The fourth list — the commands `src-tauri/capabilities/*.json` actually
 *      grant through the permissions in `src-tauri/permissions/` (SEC-01) — is
 *      not merely computed but *used*. Every case above writes a tree with no
 *      `permissions/` directory, which is precisely the tree on which the
 *      fourth list is switched off, so not one of them touches the wiring
 *      between `resolveAppCommandGrants` and the exit code. That wiring is two
 *      lines of the runner, and no pure function can see either: delete
 *      `problems.push(...grants.problems)`, or pass `aclEnabled: false` into
 *      `reconcileCommands`, and every unit test in
 *      `scripts/command-inventory-guard.test.js` stays green while the guard
 *      prints a clean run over a tree whose only command is granted to nobody,
 *      or granted to the output window. `the fourth list, end to end` below is
 *      what makes each of those two edits red.
 *
 * No git, unlike the fixture-content suite: this guard reads a directory tree
 * and a Markdown file and nothing else, so each case costs one `node` spawn and
 * a handful of `writeFileSync` calls. The suite-level timeout below is sized for
 * process start-up on this machine with a wide margin, and no `beforeAll` does
 * any work, so the separate `hookTimeout` budget is untouched.
 */

import { execFileSync } from 'node:child_process'
import {
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')
const GUARD_SRC = join(ROOT, 'scripts/command-inventory-guard.js')
const CHECK_SRC = join(ROOT, 'scripts/check-command-inventory.js')

/** One `node` spawn per case; generous, since a cold spawn on Windows is not fast. */
const SPAWN_TIMEOUT_MS = 30_000

/** The real command in this tree, so the fixtures read true. */
const LIST_MONITORS = `#[tauri::command]
pub fn list_monitors<R: Runtime>(app: AppHandle<R>) -> Vec<Monitor> {
    vec![]
}
`

/**
 * An inventory entry in the committed file's own grammar.
 *
 * @param {string} path
 * @returns {string}
 */
function entry(path) {
  return `- \`${path}\` — synthetic justification written for this test only.`
}

/**
 * An inventory file with the given entries, wrapped in prose of the same shape
 * the committed one carries — headings and paragraphs, which must be ignored,
 * and no list items other than the entries themselves.
 *
 * @param {readonly string[]} paths
 * @returns {string}
 */
function inventoryOf(paths) {
  return [
    '# IPC command inventory',
    '',
    'Synthetic fixture for check-command-inventory.e2e.test.js. Prose here uses',
    'paragraphs, because nothing else in this file may start a list item.',
    '',
    '## Commands',
    '',
    ...paths.map((path) => entry(path)),
    '',
  ].join('\n')
}

/**
 * Writes `files`, keyed by path relative to `directory`, creating parents as it
 * goes. Nothing is written for an empty map — which is how a case says "this
 * directory does not exist in my tree" rather than "it exists and is empty";
 * for `src-tauri/permissions/` those are different trees.
 *
 * @param {string} directory
 * @param {Record<string, string>} files
 */
function writeTree(directory, files) {
  for (const [relative, content] of Object.entries(files)) {
    const absolute = join(directory, relative)
    mkdirSync(dirname(absolute), { recursive: true })
    writeFileSync(absolute, content)
  }
}

/**
 * Builds a throwaway tree holding a real copy of the guard plus the Rust
 * sources and inventory a case wants it to read.
 *
 * `permissions` defaulting to `{}` is not a convenience, it is the switch under
 * test: `src-tauri/permissions/` existing is exactly what turns the app ACL
 * manifest on, for `tauri_build` and for this guard alike, so a case that omits
 * it gets a tree on which the fourth list does not apply — which is the tree
 * every case written before SEC-01 wanted.
 *
 * @param {string} label
 * @param {object} tree
 * @param {Record<string, string>} tree.sources Paths relative to `src-tauri/src/`.
 * @param {string | null} tree.inventory `null` writes no inventory file at all.
 * @param {Record<string, string>} [tree.permissions] Paths relative to
 *   `src-tauri/permissions/`. Empty writes no directory at all, so the ACL
 *   manifest is off.
 * @param {Record<string, string>} [tree.capabilities] Paths relative to
 *   `src-tauri/capabilities/`. Empty writes no directory at all.
 * @param {Record<string, string>} [tree.links] Symbolic links to create, keyed
 *   by path relative to the tree root and valued by the directory they point
 *   at, also relative to the tree root. Created with type `'junction'`, which
 *   is the only kind Windows makes without a privilege a developer machine may
 *   not have; node ignores the type on every other platform.
 * @returns {string} absolute path to the new tree
 */
function createTree(
  label,
  { sources, inventory, permissions = {}, capabilities = {}, links = {} },
) {
  const dir = mkdtempSync(join(tmpdir(), `aeroworship-command-inventory-${label}-`))
  mkdirSync(join(dir, 'scripts'), { recursive: true })
  copyFileSync(GUARD_SRC, join(dir, 'scripts/command-inventory-guard.js'))
  copyFileSync(CHECK_SRC, join(dir, 'scripts/check-command-inventory.js'))

  writeTree(join(dir, 'src-tauri', 'src'), sources)

  writeTree(join(dir, 'src-tauri', 'permissions'), permissions)
  writeTree(join(dir, 'src-tauri', 'capabilities'), capabilities)

  for (const [linkPath, target] of Object.entries(links)) {
    const absolute = join(dir, linkPath)
    mkdirSync(dirname(absolute), { recursive: true })
    symlinkSync(join(dir, target), absolute, 'junction')
  }

  if (inventory !== null) {
    mkdirSync(join(dir, 'src-tauri'), { recursive: true })
    writeFileSync(join(dir, 'src-tauri', 'commands.inventory.md'), inventory)
  }

  return dir
}

/**
 * Runs the real, copied `check-command-inventory.js`. Never throws on a
 * non-zero exit — most cases below expect one.
 *
 * @param {string} dir
 * @returns {{ status: number, stdout: string, stderr: string }}
 */
function runGuard(dir) {
  try {
    const stdout = execFileSync(
      process.execPath,
      ['scripts/check-command-inventory.js'],
      { cwd: dir, encoding: 'utf8' },
    )
    return { status: 0, stdout, stderr: '' }
  } catch (error) {
    const failure =
      /** @type {NodeJS.ErrnoException & { status?: number, stdout?: string, stderr?: string }} */ (
        error
      )
    if (typeof failure.status !== 'number') throw error
    return {
      status: failure.status,
      stdout: typeof failure.stdout === 'string' ? failure.stdout : '',
      stderr: typeof failure.stderr === 'string' ? failure.stderr : '',
    }
  }
}

/**
 * Builds a tree, runs the guard against it, and removes the tree.
 *
 * @param {string} label
 * @param {object} tree
 * @param {Record<string, string>} tree.sources
 * @param {string | null} tree.inventory
 * @param {Record<string, string>} [tree.permissions]
 * @param {Record<string, string>} [tree.capabilities]
 * @param {Record<string, string>} [tree.links]
 */
function check(label, tree) {
  const dir = createTree(label, tree)
  try {
    return runGuard(dir)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

/**
 * The presence line the guard prints under each discrepancy, as one string, so
 * a case can assert *which* of the three lists was missing the command rather
 * than that some complaint was made about it.
 *
 * @param {string} stderr
 * @param {string} command
 * @returns {string}
 */
function presenceLineFor(stderr, command) {
  const lines = stderr.split(/\r?\n/)
  const header = lines.findIndex((line) => line.includes(`\`${command}\``))
  if (header === -1) throw new Error(`the guard never reported \`${command}\``)
  return lines[header + 1] ?? ''
}

/** A tree that is correct in every respect, as the baseline each case perturbs. */
const HEALTHY = {
  sources: {
    'lib.rs':
      'pub mod commands;\n' +
      'pub fn run() {\n' +
      '    tauri::Builder::default()\n' +
      '        .invoke_handler(tauri::generate_handler![commands::display::list_monitors])\n' +
      '        .run(tauri::generate_context!());\n' +
      '}\n',
    'main.rs': 'fn main() {\n    aeroworship_lib::run()\n}\n',
    'commands/mod.rs': 'pub mod display;\n',
    'commands/display.rs': LIST_MONITORS,
  },
  inventory: inventoryOf(['commands::display::list_monitors']),
}

/**
 * A permission file in the grammar `src-tauri/permissions/list_monitors.json`
 * uses, reduced to the keys the guard models — anything else it carries is a
 * problem in its own right and would make these cases red for the wrong reason.
 *
 * @param {string} identifier
 * @param {readonly string[]} allow Commands, named the flat way the ACL names
 *   them, which is how a real permission file names them too.
 * @returns {string}
 */
function permissionAllowing(identifier, allow) {
  return `${JSON.stringify(
    {
      permission: [
        {
          identifier,
          description: 'Synthetic permission written for this test only.',
          commands: { allow: [...allow] },
        },
      ],
    },
    null,
    2,
  )}
`
}

/**
 * A capability file in the grammar `src-tauri/capabilities/*.json` use.
 *
 * @param {string} identifier
 * @param {readonly string[] | null} windows `null` omits the key entirely,
 *   which is a different tree from `[]` only in intent: both leave the
 *   capability naming no target at all.
 * @param {readonly string[]} permissions
 * @returns {string}
 */
function capabilityGranting(identifier, windows, permissions) {
  return `${JSON.stringify(
    {
      identifier,
      description: 'Synthetic capability written for this test only.',
      ...(windows === null ? {} : { windows: [...windows] }),
      permissions: [...permissions],
    },
    null,
    2,
  )}
`
}

/**
 * The same healthy tree, plus the ACL manifest this repository really ships:
 * one permission for the one command, granted to `main` alone, and a second
 * capability that names the output window and grants it nothing.
 *
 * `output-window.json` carrying an empty `permissions` list is load-bearing in
 * both directions. It is what the repository does, and it must NOT be reported
 * as "grants app-defined commands to the `output` window" — a capability that
 * grants no app permission is never examined for its targets, which is the only
 * reason a file whose whole purpose is to name that window can exist at all.
 */
const ACL_HEALTHY = {
  sources: HEALTHY.sources,
  inventory: HEALTHY.inventory,
  permissions: {
    'list_monitors.json': permissionAllowing('allow-list-monitors', ['list_monitors']),
  },
  capabilities: {
    'main-window.json': capabilityGranting(
      'main-window',
      ['main'],
      ['core:event:default', 'allow-list-monitors'],
    ),
    'output-window.json': capabilityGranting('output-window', ['output'], []),
  },
}

describe('the three directions, end to end', { timeout: SPAWN_TIMEOUT_MS }, () => {
  it('passes when definitions, generate_handler! and the inventory name the same set', () => {
    const result = check('healthy', HEALTHY)
    expect(result.status).toBe(0)
    expect(result.stdout).toContain(
      '1 command(s) defined, registered and inventoried (commands::display::list_monitors).',
    )
    expect(result.stderr).toBe('')
  })

  it('DIRECTION 2 (the silent one): fails a command that is defined AND registered but not inventoried', () => {
    // The case this guard exists for. Definition and registration were added
    // in the same change, so they agree — a two-way comparison of the two code
    // sites is green here, and only the third list catches it.
    const result = check('unlisted', {
      sources: {
        ...HEALTHY.sources,
        'lib.rs': HEALTHY.sources['lib.rs'].replace(
          'generate_handler![commands::display::list_monitors]',
          'generate_handler![commands::display::list_monitors, commands::output::ping]',
        ),
        'commands/mod.rs': 'pub mod display;\npub mod output;\n',
        'commands/output.rs': '#[tauri::command]\npub fn ping() -> u8 {\n    0\n}\n',
      },
      inventory: HEALTHY.inventory,
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/`commands::output::ping` .*absent from the inventory/)
    // Asserted as the presence triple, not as "some message appeared": the
    // point is that the two code sites agreed and the inventory did not.
    expect(presenceLineFor(result.stderr, 'commands::output::ping')).toBe(
      '    defined: yes  registered in generate_handler!: yes  ' +
        'in src-tauri/commands.inventory.md: no',
    )
    // The command that IS in all three is not dragged down with it.
    expect(result.stderr).not.toContain('`commands::display::list_monitors`')
    expect(result.stdout).not.toMatch(/defined, registered and inventoried/)
  })

  it('DIRECTION 1: fails a command that is defined and inventoried but never registered', () => {
    const result = check('unregistered', {
      sources: {
        ...HEALTHY.sources,
        'commands/mod.rs': 'pub mod display;\npub mod output;\n',
        'commands/output.rs': '#[tauri::command]\npub fn ping() -> u8 {\n    0\n}\n',
      },
      inventory: inventoryOf([
        'commands::display::list_monitors',
        'commands::output::ping',
      ]),
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/`commands::output::ping` .*unreachable/)
    expect(presenceLineFor(result.stderr, 'commands::output::ping')).toBe(
      '    defined: yes  registered in generate_handler!: no  ' +
        'in src-tauri/commands.inventory.md: yes',
    )
  })

  it('DIRECTION 3: fails a stale inventory entry whose command is gone from the code', () => {
    const result = check('stale', {
      sources: HEALTHY.sources,
      inventory: inventoryOf([
        'commands::display::list_monitors',
        'commands::legacy::removed_last_week',
      ]),
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/`commands::legacy::removed_last_week` .*stale entry/)
    expect(presenceLineFor(result.stderr, 'commands::legacy::removed_last_week')).toBe(
      '    defined: no  registered in generate_handler!: no  ' +
        'in src-tauri/commands.inventory.md: yes',
    )
  })
})

describe('what the walk has to reach', { timeout: SPAWN_TIMEOUT_MS }, () => {
  it('finds a command defined outside commands/, in a nested module, via its mod.rs', () => {
    // Green is the assertion that matters: had the walk stopped at
    // `commands/`, this command would be registered and inventoried but not
    // defined, and the run would be red for a reason that has nothing to do
    // with the tree being wrong.
    const result = check('whole-tree', {
      sources: {
        'lib.rs':
          'pub fn run() {\n' +
          '    tauri::Builder::default().invoke_handler(tauri::generate_handler![\n' +
          '        commands::display::list_monitors,\n' +
          '        services::telemetry::sink::report,\n' +
          '    ]);\n' +
          '}\n',
        'commands/display.rs': LIST_MONITORS,
        'services/telemetry/mod.rs': 'pub mod sink;\n',
        'services/telemetry/sink.rs':
          '#[tauri::command]\npub async fn report(payload: String) {}\n',
      },
      inventory: inventoryOf([
        'commands::display::list_monitors',
        'services::telemetry::sink::report',
      ]),
    })

    expect(result.status).toBe(0)
    expect(result.stdout).toContain('2 command(s) defined, registered and inventoried')
    expect(result.stdout).toContain('services::telemetry::sink::report')
  })

  it('reads a SECOND generate_handler! that is not in lib.rs', () => {
    // The `WebviewWindowBuilder` case. Read only from `lib.rs`, `ping` would
    // be defined and inventoried but not registered, and this run would be
    // red — so a green run is the proof that the second handler was read.
    const result = check('second-handler', {
      sources: {
        'lib.rs':
          'pub fn run() {\n' +
          '    tauri::Builder::default()\n' +
          '        .invoke_handler(tauri::generate_handler![commands::display::list_monitors]);\n' +
          '}\n',
        'commands/display.rs': LIST_MONITORS,
        'services/output_window.rs':
          'pub fn open(app: &AppHandle) {\n' +
          '    WebviewWindowBuilder::new(app, "output", url)\n' +
          '        .invoke_handler(tauri::generate_handler![commands::output::ping])\n' +
          '        .build();\n' +
          '}\n',
        'commands/output.rs': '#[tauri::command]\npub fn ping() -> u8 {\n    0\n}\n',
      },
      inventory: inventoryOf([
        'commands::display::list_monitors',
        'commands::output::ping',
      ]),
    })

    expect(result.status).toBe(0)
    expect(result.stdout).toContain('2 command(s) defined, registered and inventoried')
  })

  it('is not fooled by module docs that quote the attribute and the macro in prose', () => {
    // `commands/mod.rs` in this repository really does quote both. Counting
    // either would invent a command that does not exist, and the guard would
    // be red on a tree that is correct.
    const result = check('prose', {
      sources: {
        ...HEALTHY.sources,
        'commands/mod.rs':
          '//! The IPC surface: one module per command group.\n' +
          '//!\n' +
          '//! Every command added here carries #[tauri::command] and must also be listed\n' +
          '//! in the `generate_handler!` call in [`crate::run`], and in\n' +
          '//! `src-tauri/commands.inventory.md`.\n' +
          '//! pub fn not_a_command() {}\n' +
          'pub mod display;\n',
      },
      inventory: HEALTHY.inventory,
    })

    expect(result.status).toBe(0)
    expect(result.stdout).toContain('1 command(s) defined, registered and inventoried')
  })
})

describe('runs that verified nothing', { timeout: SPAWN_TIMEOUT_MS }, () => {
  it('fails, naming the directory, when there is no Rust source to scan at all', () => {
    // A walk pointed at a tree that is not this one. Three empty lists agree
    // perfectly, so without this clause the guard would print "in sync" about
    // a surface nobody examined.
    const result = check('no-sources', {
      sources: {},
      inventory: inventoryOf([]),
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/no \.rs files were found under/)
    expect(result.stderr).toMatch(/the IPC surface was not examined at all/)
    expect(result.stdout).not.toMatch(/defined, registered and inventoried/)
  })

  it('fails when there is Rust source but no generate_handler! anywhere', () => {
    const result = check('no-handler', {
      sources: { 'lib.rs': 'pub fn run() {\n    tauri::Builder::default();\n}\n' },
      inventory: inventoryOf([]),
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/no `generate_handler!` invocation was found/)
    expect(result.stderr).toMatch(/Neither is a clean run/)
  })

  it('fails when the handler exists but registers nothing and no command is defined', () => {
    // The third vacuous shape: everything present, everything empty. Nothing
    // disagrees, and nothing was verified.
    const result = check('empty-handler', {
      sources: {
        'lib.rs':
          'pub fn run() {\n' +
          '    tauri::Builder::default().invoke_handler(tauri::generate_handler![]);\n' +
          '}\n',
      },
      inventory: inventoryOf([]),
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/the IPC surface is not what all three lists say it is/)
    expect(result.stdout).not.toMatch(/defined, registered and inventoried/)
  })
})

describe('files the guard could not read', { timeout: SPAWN_TIMEOUT_MS }, () => {
  it('fails with a named problem, not a crash, when the inventory file is absent', () => {
    const result = check('no-inventory', { sources: HEALTHY.sources, inventory: null })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain(
      'src-tauri/commands.inventory.md could not be read, so nothing was verified against it',
    )
    // A crash would print a stack trace instead of a diagnostic.
    expect(result.stderr).not.toMatch(/at \w+ \(/)
  })

  it('fails on a mistyped inventory line even when all three sets otherwise agree', () => {
    // The locked decision, proved through the runner: a line whose trimmed
    // form starts with `- ` and is not a valid entry is a problem. Here the
    // command sets match perfectly, so nothing but that clause makes this red
    // — and a mistyped entry that parsed as nothing would leave the file
    // looking complete while a command silently left the inventory.
    const result = check('mistyped-entry', {
      sources: HEALTHY.sources,
      inventory: `${HEALTHY.inventory}\n- \`commands::output::ping\` - hyphen, not an em dash\n`,
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(/starts a list item but is not an entry/)
    expect(result.stderr).toContain('src-tauri/commands.inventory.md:')
    expect(result.stdout).not.toMatch(/defined, registered and inventoried/)
  })

  it('names the file when a #[tauri::command] cannot be tied to a function', () => {
    const result = check('dangling-attribute', {
      sources: {
        ...HEALTHY.sources,
        'commands/output.rs': '#[tauri::command]\nstruct NotAFunction;\n',
      },
      inventory: HEALTHY.inventory,
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain('src-tauri/src/commands/output.rs:')
    expect(result.stderr).toMatch(/cannot be named/)
  })
})

describe('the fourth list, end to end', { timeout: SPAWN_TIMEOUT_MS }, () => {
  it('passes, and says the manifest is ON, when the capabilities grant exactly what is registered', () => {
    // The baseline the cases below perturb one field at a time. It is also the
    // only case in this file that proves the guard reads src-tauri/permissions/
    // and src-tauri/capabilities/ at all: on every tree above there is nothing
    // there to read.
    const result = check('acl-healthy', ACL_HEALTHY)

    expect(result.status).toBe(0)
    expect(result.stdout).toContain(
      '1 command(s) defined, registered and inventoried (commands::display::list_monitors).',
    )
    expect(result.stdout).toContain(
      'the app ACL manifest is ON (1 permission(s) under src-tauri/permissions/)',
    )
    // The capability that names `output` and grants it nothing is the shape
    // this repository ships. It must not be mistaken for a grant.
    expect(result.stderr).toBe('')
  })

  it('fails when a capability that grants an app-defined command also names the `output` window', () => {
    // MUTATION KILLER. Delete `problems.push(...grants.problems)` from
    // check-command-inventory.js and every unit test stays green, because the
    // three code lists still agree perfectly here: the ONLY thing wrong with
    // this tree is one extra label, and the only thing that can report it is
    // the line that carries the grant problems into the exit code.
    const result = check('acl-output-window', {
      ...ACL_HEALTHY,
      capabilities: {
        ...ACL_HEALTHY.capabilities,
        'main-window.json': capabilityGranting(
          'main-window',
          ['main', 'output'],
          ['core:event:default', 'allow-list-monitors'],
        ),
      },
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain(
      'capability `main-window` grants app-defined commands to the `output` window',
    )
    // Nothing else was wrong, so nothing else may be reported: this is what
    // makes the assertion above the reason for the exit code rather than a
    // coincidence standing beside it.
    expect(result.stderr).not.toContain('`commands::display::list_monitors`')
    expect(result.stdout).not.toContain(
      'no capability that grants an app-defined command names a glob',
    )
  })

  it('fails when a capability that grants an app-defined command names a glob instead of a label', () => {
    // The one-character version of the same reach: `*` matches `output` too,
    // and a diff showing it says nothing about the window it re-arms.
    const result = check('acl-glob', {
      ...ACL_HEALTHY,
      capabilities: {
        ...ACL_HEALTHY.capabilities,
        'main-window.json': capabilityGranting(
          'main-window',
          ['*'],
          ['core:event:default', 'allow-list-monitors'],
        ),
      },
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain(
      'capability `main-window` grants app-defined commands to the pattern "*"',
    )
    expect(result.stderr).not.toContain('`commands::display::list_monitors`')
  })

  it('fails a registered command that the permission files grant to nobody', () => {
    // MUTATION KILLER. Pin `aclEnabled: false` in the `reconcileCommands` call
    // and this tree reads as three lists in perfect agreement — which it is.
    // The command this application registers is granted zero permissions and
    // would be rejected at invoke time with "Command not allowed by ACL", with
    // no other gate red and no runtime escape (ADR-0013); and the permission
    // that IS defined names a command that does not exist.
    const result = check('acl-ungranted', {
      ...ACL_HEALTHY,
      permissions: {
        'list_monitors.json': permissionAllowing('allow-list-monitors', [
          'not_list_monitors',
        ]),
      },
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toMatch(
      /`commands::display::list_monitors` is registered in `generate_handler!` and named by no capability/,
    )
    // Asserted as the presence quadruple: the fourth column is the whole point,
    // and it is printed only when the runner believes the manifest is on.
    expect(presenceLineFor(result.stderr, 'commands::display::list_monitors')).toBe(
      '    defined: yes  registered in generate_handler!: yes  ' +
        'in src-tauri/commands.inventory.md: yes  named by a capability: no',
    )
    // The mirror image, reported under the only name anyone wrote it as.
    expect(result.stderr).toMatch(/`not_list_monitors` is named by a capability/)
    expect(result.stderr).toContain(
      'the IPC surface is not what all four lists say it is',
    )
    expect(result.stdout).not.toMatch(/defined, registered and inventoried/)
  })

  it('fails a capability that grants app-defined commands to no window and no webview', () => {
    // An inert grant reads exactly like a live one in review, and the command
    // it was meant to reach is rejected at invoke time all the same.
    const result = check('acl-no-target', {
      ...ACL_HEALTHY,
      capabilities: {
        ...ACL_HEALTHY.capabilities,
        'main-window.json': capabilityGranting('main-window', null, [
          'allow-list-monitors',
        ]),
      },
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain(
      'capability `main-window` grants app-defined commands but names no window and no ' +
        'webview, so it grants them to nothing',
    )
  })

  it('fails, naming both files, on a TOML permission and a TOML capability it cannot read', () => {
    // tauri-build reads these; this guard does not, and adding a TOML parser
    // would be a dependency against the NFR-16 budget. The JSON files beside
    // them are the healthy ones, so the sets still agree and these two
    // unreadable files are the only thing making the run red.
    const result = check('acl-toml', {
      ...ACL_HEALTHY,
      permissions: {
        ...ACL_HEALTHY.permissions,
        'extra.toml': '[[permission]]\nidentifier = "allow-anything"\n',
      },
      capabilities: {
        ...ACL_HEALTHY.capabilities,
        'extra.toml': 'identifier = "extra"\nwindows = ["output"]\n',
      },
    })

    expect(result.status).not.toBe(0)
    expect(result.stderr).toContain(
      'src-tauri/permissions/extra.toml: tauri-build reads this file and this guard cannot',
    )
    expect(result.stderr).toContain(
      'src-tauri/capabilities/extra.toml: tauri-build reads this file and this guard cannot',
    )
    // Reported, never guessed at: an unread TOML must not be silently treated
    // as a permission or as a capability.
    expect(result.stderr).not.toContain('allow-anything')
  })

  it('skips a schemas/ directory, as tauri-build does, instead of reading it as a grant', () => {
    // Green is the assertion. Both files below would be problems if read: the
    // first defines no `permission` array, the second is an object with no
    // `permissions` key. tauri-build writes exactly such a directory into
    // src-tauri/capabilities/, so a guard that read it would be red on a tree
    // that is correct.
    const result = check('acl-schemas', {
      ...ACL_HEALTHY,
      permissions: {
        ...ACL_HEALTHY.permissions,
        'schemas/schema.json': '{ "$id": "https://example.invalid/permission" }\n',
      },
      capabilities: {
        ...ACL_HEALTHY.capabilities,
        'schemas/windows-schema.json': '{ "$id": "https://example.invalid/windows" }\n',
      },
    })

    expect(result.status).toBe(0)
    expect(result.stderr).toBe('')
    expect(result.stdout).toContain('the app ACL manifest is ON')
  })
})

describe(
  'the fourth list, where the guard has to match tauri exactly',
  {
    timeout: SPAWN_TIMEOUT_MS,
  },
  () => {
    it('reads a permission nested BELOW schemas/, because tauri-build reads it', () => {
      // MUTATION KILLER, and the finding an auditor and a tester reached from
      // opposite directions on the same day.
      //
      // `tauri-utils-2.9.3/src/acl/build.rs:88` skips a file only when its
      // IMMEDIATE parent is `schemas`
      // (`p.parent().unwrap().file_name().unwrap() != "schemas"`), and `:217`
      // does the same for capabilities. A guard that skips whenever ANY ancestor
      // segment is `schemas` is blind to exactly this file — which tauri-build
      // parses, and whose redeclared identifier then silently replaces the one
      // that was reviewed. The "two permissions call themselves the same thing"
      // rule has unit tests of its own; on a tree shaped like this one they never
      // ran.
      const result = check('acl-schemas-nested', {
        ...ACL_HEALTHY,
        permissions: {
          ...ACL_HEALTHY.permissions,
          'schemas/nested/reach.json': permissionAllowing('allow-list-monitors', [
            'shutdown_everything',
          ]),
        },
      })

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(
        'two permissions under src-tauri/permissions/ both call themselves `allow-list-monitors`',
      )
      expect(result.stdout).not.toMatch(/the app ACL manifest is ON/)
    })

    it('still skips a file whose immediate parent is schemas/, at any depth of the tree', () => {
      // The other half of the same rule, so tightening it does not turn into
      // "read everything". `deep/schemas/x.json` has `schemas` as its immediate
      // parent and is skipped; the case above has `nested` and is not.
      const result = check('acl-schemas-deep', {
        ...ACL_HEALTHY,
        permissions: {
          ...ACL_HEALTHY.permissions,
          'deep/schemas/schema.json': '{ "$id": "https://example.invalid/deep" }\n',
        },
        capabilities: {
          ...ACL_HEALTHY.capabilities,
          'deep/schemas/windows-schema.json': '{ "$id": "https://example.invalid/w" }\n',
        },
      })

      expect(result.status).toBe(0)
      expect(result.stderr).toBe('')
    })

    it('refuses a symbolic link in the ACL tree instead of walking past it', () => {
      // tauri reaches its files with `glob(...).flat_map(|p| p.canonicalize())`,
      // which resolves links. What this walk does is platform-dependent, so the
      // guard refuses rather than guesses. The junction below points at a
      // directory holding no ACL file at all, which makes the link itself the
      // only thing wrong with the tree.
      const result = check('acl-symlink', {
        ...ACL_HEALTHY,
        sources: { ...HEALTHY.sources, 'notes/README.md': 'not an ACL file\n' },
        links: { 'src-tauri/permissions/linked': 'src-tauri/src/notes' },
      })

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain('src-tauri/permissions/linked is a symbolic link')
      expect(result.stderr).toContain('tauri-build canonicalises the paths it globs')
    })

    it('refuses a symbolic link in src-tauri/src/, the walk that feeds the escape rule', () => {
      // MUTATION KILLER, and the finding that held this item: the link rule was
      // written into `collectAclFiles` and not into `collectSources` — and
      // `collectSources` is the walk `findRuntimeAclEscapes` reads from. Verified
      // on the real repository before this case existed: a junction at
      // `src-tauri/src/linked` over a directory holding a `runtime_authority_mut`
      // call left the guard at exit 0 with its reassuring summary line, over
      // source rustc compiles.
      //
      // The link points at a directory whose file WOULD be red if it were read,
      // so this case also pins which way the failure goes: refused, not
      // swallowed. Under the promises `readdir` this module calls, nothing under
      // the junction is walked at all.
      const result = check('src-symlink', {
        ...ACL_HEALTHY,
        sources: { ...HEALTHY.sources, 'hidden/reopen.rs': 'x.__allow_command(a, b);\n' },
        links: { 'src-tauri/src/linked': 'src-tauri/src/hidden' },
      })

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain('src-tauri/src/linked is a symbolic link')
      expect(result.stderr).toContain('this guard will not walk one')
    })

    it('fails a call to __allow_command that a block comment closes in front of', () => {
      // MUTATION KILLER for the comment rule. The old rule skipped any line
      // matching `/^\s*(\/\/|\*|\/\*)/`, and a Rust block comment may close
      // mid-line with code after it, so this line was silent while granting a
      // command to `windows: ["*"]` — recorded in no capability file and in
      // nothing under gen/schemas/.
      const result = check('acl-escape-inline-block', {
        ...ACL_HEALTHY,
        sources: {
          ...HEALTHY.sources,
          'services/hack.rs':
            'pub fn widen(context: &mut tauri::Context) {\n' +
            '    /* TODO(FR-503): temporary */ context.runtime_authority_mut()\n' +
            '        .__allow_command("open_file".into(), ExecutionContext::Local);\n' +
            '}\n',
        },
      })

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain('src-tauri/src/services/hack.rs:')
      expect(result.stderr).toContain('calls `runtime_authority_mut`')
    })

    it('fails the deref-assign form, which replaces the whole authority at once', () => {
      // MUTATION KILLER. `runtime_authority_mut` returns `&mut RuntimeAuthority`,
      // so this is the idiomatic use of it rather than a contrivance, and it is
      // wider than `__allow_command`. Its leading `*` was read as the
      // continuation of a block comment.
      const result = check('acl-escape-deref', {
        ...ACL_HEALTHY,
        sources: {
          ...HEALTHY.sources,
          'services/replace.rs':
            'pub fn swap(context: &mut tauri::Context, authority: RuntimeAuthority) {\n' +
            '    *context.runtime_authority_mut() = authority;\n' +
            '}\n',
        },
      })

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain('src-tauri/src/services/replace.rs:')
      expect(result.stderr).toContain('calls `runtime_authority_mut`')
    })

    it('fails two registered commands that collide on one flat IPC name', () => {
      // MUTATION KILLER. `findAmbiguousCommandNames` has unit tests; delete
      // `problems.push(...findAmbiguousCommandNames(registered))` from the runner
      // and they all stay green while this tree — two `open`s and one permission
      // that cannot mean either of them alone — passes.
      const collide = {
        sources: {
          'lib.rs':
            'pub fn run() {\n' +
            '    tauri::Builder::default().invoke_handler(tauri::generate_handler![\n' +
            '        commands::media::open,\n' +
            '        commands::song::open,\n' +
            '    ]);\n' +
            '}\n',
          'commands/media.rs': '#[tauri::command]\npub fn open() -> u8 {\n    0\n}\n',
          'commands/song.rs': '#[tauri::command]\npub fn open() -> u8 {\n    1\n}\n',
        },
        inventory: inventoryOf(['commands::media::open', 'commands::song::open']),
        permissions: { 'open.json': permissionAllowing('allow-open', ['open']) },
        capabilities: {
          'main-window.json': capabilityGranting('main-window', ['main'], ['allow-open']),
        },
      }

      const result = check('acl-ambiguous', collide)

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(
        '`commands::media::open` and `commands::song::open` are both registered and both ' +
          'answer to the single IPC command name `open`',
      )
      // Every other list agrees: both are defined, registered, inventoried and
      // granted. The collision is the only thing this run can be red about.
      expect(result.stderr).not.toMatch(
        /absent from the inventory|unreachable|stale entry/,
      )
    })

    it('fails a window pattern for every glob metacharacter, not only for `*`', () => {
      // MUTATION KILLER. Narrow `GLOB_METACHARACTERS` to `/[*]/` and every test
      // that existed before this one stays green, because every one of them used
      // `*`. `glob::Pattern` — the type tauri parses these with — reads `?`, a
      // character class and a brace alternation just as happily, and each of the
      // spellings below matches `output`.
      for (const pattern of ['outpu?', '[o]utput', 'outpu[t]', '{output,main}']) {
        const result = check('acl-glob-class', {
          ...ACL_HEALTHY,
          capabilities: {
            ...ACL_HEALTHY.capabilities,
            'main-window.json': capabilityGranting(
              'main-window',
              [pattern],
              ['core:event:default', 'allow-list-monitors'],
            ),
          },
        })

        expect(result.status, `pattern ${pattern} was accepted`).not.toBe(0)
        expect(result.stderr).toContain(
          `capability \`main-window\` grants app-defined commands to the pattern "${pattern}"`,
        )
      }
    })

    it('fails a source file that reopens the ACL from inside the process', () => {
      // MUTATION KILLER for the third piece of runner wiring. The claim "there is
      // no runtime escape" is written in four places in this repository and is
      // not quite what tauri guarantees: `runtime_authority_mut` and
      // `__allow_command` are `pub` and are not behind `dynamic-acl`. What IS
      // true is that reaching them takes Rust code, so it takes a diff — and this
      // is what makes that diff red rather than leaving it to a reviewer.
      const result = check('acl-runtime-escape', {
        ...ACL_HEALTHY,
        sources: {
          ...HEALTHY.sources,
          'services/reopen.rs':
            'pub fn widen(context: &mut tauri::Context) {\n' +
            '    context.runtime_authority_mut();\n' +
            '}\n',
        },
      })

      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain('src-tauri/src/services/reopen.rs:')
      expect(result.stderr).toContain(
        'calls `runtime_authority_mut`, which reopens the ACL from inside the process',
      )
    })

    it('does not mistake the doc comments that describe that rule for a call', () => {
      // Three files in this repository name both APIs in prose, and one of them
      // is the guard's own source. A rule that went red on its own documentation
      // would be deleted within a week.
      const result = check('acl-escape-prose', {
        ...ACL_HEALTHY,
        sources: {
          ...HEALTHY.sources,
          'commands/notes.rs':
            '//! `dynamic-acl` is off, but `runtime_authority_mut` and `__allow_command`\n' +
            '//! stay `pub`, so "no runtime escape" is too strong a claim.\n' +
            '\n' +
            '/// See `__allow_command` for why this is not granted at runtime.\n' +
            'pub fn nothing() {}\n',
        },
      })

      expect(result.status).toBe(0)
      expect(result.stderr).toBe('')
    })
  },
)

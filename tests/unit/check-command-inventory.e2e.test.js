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
 *
 * No git, unlike the fixture-content suite: this guard reads a directory tree
 * and a Markdown file and nothing else, so each case costs one `node` spawn and
 * a handful of `writeFileSync` calls. The suite-level timeout below is sized for
 * process start-up on this machine with a wide margin, and no `beforeAll` does
 * any work, so the separate `hookTimeout` budget is untouched.
 */

import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
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
 * Builds a throwaway tree holding a real copy of the guard plus the Rust
 * sources and inventory a case wants it to read.
 *
 * @param {string} label
 * @param {object} tree
 * @param {Record<string, string>} tree.sources Paths relative to `src-tauri/src/`.
 * @param {string | null} tree.inventory `null` writes no inventory file at all.
 * @returns {string} absolute path to the new tree
 */
function createTree(label, { sources, inventory }) {
  const dir = mkdtempSync(join(tmpdir(), `aeroworship-command-inventory-${label}-`))
  mkdirSync(join(dir, 'scripts'), { recursive: true })
  copyFileSync(GUARD_SRC, join(dir, 'scripts/command-inventory-guard.js'))
  copyFileSync(CHECK_SRC, join(dir, 'scripts/check-command-inventory.js'))

  for (const [relative, content] of Object.entries(sources)) {
    const absolute = join(dir, 'src-tauri', 'src', relative)
    mkdirSync(dirname(absolute), { recursive: true })
    writeFileSync(absolute, content)
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

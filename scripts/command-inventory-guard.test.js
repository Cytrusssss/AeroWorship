// @ts-check

/**
 * Pure-function coverage for `command-inventory-guard.js` (ADR-0039, NFR-14),
 * which landed with none. `check-command-inventory.js` — the walk, the reads,
 * the printing and the exit code — is proved separately and end-to-end in
 * `tests/unit/check-command-inventory.e2e.test.js`, the same division
 * `bindings-guard.test.js` draws against `check-bindings.js`.
 *
 * What gets the most cases, and why. The guard reconciles three sets, and only
 * one of the three directions is silent: a command that is **defined and
 * registered but never inventoried**. The other two police themselves — a
 * command that is registered but not defined does not compile, and one that is
 * defined but not registered fails the first time the frontend invokes it. A
 * guard that only compared the `#[tauri::command]` definitions against
 * `generate_handler!` would be green for exactly the case it exists to catch,
 * because those two sites are edited in the same commit and therefore agree.
 * So every combination of the three sets is asserted below, individually and by
 * name, rather than "some discrepancy was reported".
 *
 * The other cases that earn their place are the ones where a parser could
 * quietly find nothing: a mistyped inventory line, a `#[tauri::command]` whose
 * function this parser cannot name, an unclosed `generate_handler!`, and three
 * flavours of vacuous run. Each of those is a **problem**, never a skip — a
 * command the guard cannot see is a command the guard is not guarding, and
 * reporting that as clean is the ADR-0020 shape.
 */

import { describe, expect, it } from 'vitest'

import {
  findCommandDefinitions,
  findHandlerRegistrations,
  modulePathForSource,
  parseInventory,
  reconcileCommands,
  summariseInventoryRun,
} from './command-inventory-guard.js'

/**
 * @param {readonly string[]} defined
 * @param {readonly string[]} registered
 * @param {readonly string[]} inventoried
 */
function reconcile(defined, registered, inventoried) {
  return reconcileCommands({ defined, registered, inventoried })
}

/**
 * Looks a discrepancy up by command name, so an assertion never depends on
 * where in the sorted list it landed.
 *
 * @param {ReturnType<typeof reconcileCommands>} reconciliation
 * @param {string} name
 */
function discrepancyFor(reconciliation, name) {
  const found = reconciliation.discrepancies.find((entry) => entry.name === name)
  if (!found) throw new Error(`no discrepancy was reported for \`${name}\``)
  return found
}

/**
 * A run whose non-vacuity is beyond doubt, so each case below varies exactly
 * one thing. Defaults chosen to be a healthy run: files were scanned, a handler
 * was found, nothing failed to parse.
 *
 * @param {object} run
 * @param {ReturnType<typeof reconcileCommands>} run.reconciliation
 * @param {readonly string[]} [run.problems]
 * @param {number} [run.filesScanned]
 * @param {number} [run.handlerInvocations]
 */
function summarise({
  reconciliation,
  problems = [],
  filesScanned = 4,
  handlerInvocations = 1,
}) {
  return summariseInventoryRun({
    reconciliation,
    problems,
    filesScanned,
    handlerInvocations,
  })
}

describe('modulePathForSource', () => {
  it('names a module from a plain file and from its mod.rs spelling identically', () => {
    // The two spellings are the same module to Rust, so they must be the same
    // module here: a command that moved from one to the other would otherwise
    // read as a rename and go red for no reason.
    expect(modulePathForSource('commands/display.rs')).toBe('commands::display')
    expect(modulePathForSource('commands/display/mod.rs')).toBe('commands::display')
  })

  it('puts lib.rs and main.rs at the crate root, where a command is named bare', () => {
    // `generate_handler!` would have to spell such a command as just its
    // function name, so anything else here is a discrepancy invented by the
    // guard itself.
    expect(modulePathForSource('lib.rs')).toBe('')
    expect(modulePathForSource('main.rs')).toBe('')
  })

  it('keeps a top-level module and nests deeper ones', () => {
    expect(modulePathForSource('db.rs')).toBe('db')
    expect(modulePathForSource('services/display/output.rs')).toBe(
      'services::display::output',
    )
    expect(modulePathForSource('services/display/mod.rs')).toBe('services::display')
  })

  it('accepts Windows separators and a leading ./, since the caller passes a relative path', () => {
    expect(modulePathForSource('commands\\display.rs')).toBe('commands::display')
    expect(modulePathForSource('./commands/display.rs')).toBe('commands::display')
    expect(modulePathForSource('.\\commands\\display\\mod.rs')).toBe('commands::display')
  })

  it('strips only the LAST segment, so a directory named lib or mod survives', () => {
    // `lib/mod.rs` is the module `lib`, not the crate root. Popping more than
    // the final segment would silently merge two different modules.
    expect(modulePathForSource('lib/mod.rs')).toBe('lib')
    expect(modulePathForSource('mod/display.rs')).toBe('mod::display')
    expect(modulePathForSource('main/window.rs')).toBe('main::window')
  })

  it('answers null for anything that is not a Rust file', () => {
    expect(modulePathForSource('commands/display.txt')).toBeNull()
    expect(modulePathForSource('README.md')).toBeNull()
    expect(modulePathForSource('commands/display.rs.bak')).toBeNull()
    expect(modulePathForSource('')).toBeNull()
  })
})

describe('findCommandDefinitions', () => {
  it('names a command by module and captures it past generics, as display.rs writes it', () => {
    // The real shape in this tree: `pub fn list_monitors<R: Runtime>(app: …)`.
    // A capture that stopped at the `<` would produce a name no
    // `generate_handler!` could ever match.
    const source = [
      '#[tauri::command]',
      'pub fn list_monitors<R: Runtime>(app: AppHandle<R>) -> Vec<Monitor> {',
      '    vec![]',
      '}',
    ].join('\n')

    expect(findCommandDefinitions('commands::display', source)).toEqual({
      commands: ['commands::display::list_monitors'],
      problems: [],
    })
  })

  it('names a crate-root command bare, with no leading ::', () => {
    const result = findCommandDefinitions('', '#[tauri::command]\npub fn ping() {}\n')
    expect(result.commands).toEqual(['ping'])
    expect(result.problems).toEqual([])
  })

  it('accepts the bare #[command] spelling and irregular spacing inside the attribute', () => {
    const source = [
      '#[command]',
      'pub fn a() {}',
      '#[ tauri :: command ]',
      'pub fn b() {}',
      '  #[tauri::command]',
      '  pub fn c() {}',
      '#[tauri::command(rename_all = "snake_case")]',
      'pub fn d() {}',
    ].join('\n')

    const result = findCommandDefinitions('m', source)
    expect(result.commands).toEqual(['m::a', 'm::b', 'm::c', 'm::d'])
    expect(result.problems).toEqual([])
  })

  it('reads every qualifier that may sit between the attribute and fn', () => {
    const source = [
      '#[tauri::command]',
      'fn private_one() {}',
      '#[tauri::command]',
      'pub(crate) fn crate_one() {}',
      '#[tauri::command]',
      'pub(in crate::commands) fn scoped_one() {}',
      '#[tauri::command]',
      'pub async fn async_one() {}',
      '#[tauri::command]',
      'pub const fn const_one() {}',
      '#[tauri::command]',
      'pub unsafe fn unsafe_one() {}',
      '#[tauri::command]',
      'pub extern "C" fn extern_one() {}',
    ].join('\n')

    const result = findCommandDefinitions('m', source)
    expect(result.commands).toEqual([
      'm::private_one',
      'm::crate_one',
      'm::scoped_one',
      'm::async_one',
      'm::const_one',
      'm::unsafe_one',
      'm::extern_one',
    ])
    expect(result.problems).toEqual([])
  })

  it('steps over doc comments and other attributes between the attribute and fn', () => {
    const source = [
      '#[tauri::command]',
      '/// Doc comment sitting under the attribute.',
      '#[allow(clippy::needless_pass_by_value)]',
      '#[serde(rename_all = "camelCase")]',
      'pub fn set_output_monitor(monitor_id: String) {}',
    ].join('\n')

    const result = findCommandDefinitions('commands::display', source)
    expect(result.commands).toEqual(['commands::display::set_output_monitor'])
    expect(result.problems).toEqual([])
  })

  it('does NOT count an attribute quoted in prose — the anchor is the whole point', () => {
    // `commands/mod.rs` and this repository's docs both quote the attribute
    // inside `//!`/`///` comments. Counting those would invent commands that
    // do not exist and turn the guard into noise nobody reads.
    const source = [
      '//! Every command added here carries #[tauri::command] and must also be',
      '//! listed in `generate_handler!`.',
      '/// See #[tauri::command] for the attribute this uses.',
      '// #[tauri::command]',
      '//! pub fn documented_but_not_real() {}',
      'pub fn ordinary_helper() {}',
    ].join('\n')

    expect(findCommandDefinitions('commands', source)).toEqual({
      commands: [],
      problems: [],
    })
  })

  it('DOES count an attribute commented out with a /* … */ block — the deliberate over-report', () => {
    // Locked direction. A command bulk-commented-out still reads as a
    // definition, so the guard demands a registration for a function that does
    // not exist and goes red. Noisy, never silent; if this ever flips to
    // "skipped", a real command hidden behind an unbalanced block comment
    // becomes invisible.
    const source = [
      '/*',
      '#[tauri::command]',
      'pub fn temporarily_disabled() {}',
      '*/',
    ].join('\n')

    const result = findCommandDefinitions('commands::display', source)
    expect(result.commands).toEqual(['commands::display::temporarily_disabled'])
    expect(result.problems).toEqual([])
  })

  it('reports an attribute with no function after it as a PROBLEM, not a silent skip', () => {
    const source = ['#[tauri::command]', '// nothing that looks like a signature'].join(
      '\n',
    )

    const result = findCommandDefinitions('commands::display', source)
    expect(result.commands).toEqual([])
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toMatch(/line 1/)
    expect(result.problems[0]).toMatch(/cannot be named/)
  })

  it('does not let one function be claimed by two attributes, and reports the orphan', () => {
    // The scan stops at the next attribute rather than running on to the first
    // `fn` it can find, so a dangling attribute cannot borrow the name of the
    // command below it and read as a second, non-existent command.
    const source = [
      '#[tauri::command]',
      '#[tauri::command]',
      'pub fn only_one_here() {}',
    ].join('\n')

    const result = findCommandDefinitions('m', source)
    expect(result.commands).toEqual(['m::only_one_here'])
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toMatch(/line 1/)
  })

  it('keeps source order and finds every command in a multi-command file', () => {
    const source = [
      '#[tauri::command]',
      'pub fn zebra() {}',
      '',
      '#[tauri::command]',
      'pub fn alpha() {}',
    ].join('\n')

    expect(findCommandDefinitions('m', source).commands).toEqual(['m::zebra', 'm::alpha'])
  })

  it('parses a CRLF file exactly as it parses an LF one', () => {
    // This repository is developed on Windows; a checkout with CRLF endings
    // must not silently produce zero commands.
    const lines = ['#[tauri::command]', 'pub fn list_monitors() {}']
    expect(findCommandDefinitions('commands::display', lines.join('\r\n'))).toEqual(
      findCommandDefinitions('commands::display', lines.join('\n')),
    )
    expect(
      findCommandDefinitions('commands::display', lines.join('\r\n')).commands,
    ).toEqual(['commands::display::list_monitors'])
  })

  it('attributes a command inside an inline mod block to the FILE module — the documented red', () => {
    // Inline modules are not modelled, deliberately. The command below is
    // really `m::inner::nested`, so naming it `m::nested` makes it disagree
    // with whatever `generate_handler!` says and the run goes red. That is the
    // safe direction, and it is asserted so a change to it has to be deliberate.
    const source = [
      'mod inner {',
      '    #[tauri::command]',
      '    pub fn nested() {}',
      '}',
    ].join('\n')

    expect(findCommandDefinitions('m', source).commands).toEqual(['m::nested'])
  })

  it('finds nothing, and no problem, in a file with no commands at all', () => {
    expect(findCommandDefinitions('db', 'pub fn init() {}\n')).toEqual({
      commands: [],
      problems: [],
    })
  })

  it('returns the same answer when called twice on the same source', () => {
    // Guards the module-level regexes against ever acquiring sticky/global
    // state, which would make the second file in a walk parse differently
    // from the first.
    const source = '#[tauri::command]\npub fn a() {}\n'
    expect(findCommandDefinitions('m', source)).toEqual(
      findCommandDefinitions('m', source),
    )
  })
})

describe('findHandlerRegistrations', () => {
  it('reads the registrations and counts the invocation, in the shape lib.rs uses', () => {
    const source = [
      'pub fn run() {',
      '    tauri::Builder::default()',
      '        .invoke_handler(tauri::generate_handler![commands::display::list_monitors])',
      '        .run(tauri::generate_context!());',
      '}',
    ].join('\n')

    expect(findHandlerRegistrations(source)).toEqual({
      commands: ['commands::display::list_monitors'],
      problems: [],
      invocations: 1,
    })
  })

  it('counts a SECOND invocation anywhere in the file and reads its commands too', () => {
    // An `invoke_handler` on a `WebviewWindowBuilder` is a second, equally
    // reachable surface. A guard that read only the first would leave it
    // unwatched — the exact hole this guard exists to close.
    const source = [
      '.invoke_handler(tauri::generate_handler![commands::display::list_monitors])',
      '.invoke_handler(tauri::generate_handler![commands::output::ping])',
    ].join('\n')

    const result = findHandlerRegistrations(source)
    expect(result.invocations).toBe(2)
    expect(result.commands).toEqual([
      'commands::display::list_monitors',
      'commands::output::ping',
    ])
    expect(result.problems).toEqual([])
  })

  it('accepts all three macro delimiters', () => {
    for (const [open, close] of [
      ['[', ']'],
      ['(', ')'],
      ['{', '}'],
    ]) {
      const result = findHandlerRegistrations(`generate_handler!${open}a::b${close}`)
      expect(result.commands, `delimiter ${open}${close}`).toEqual(['a::b'])
      expect(result.invocations).toBe(1)
    }
  })

  it('reads a multi-line list, tolerates a trailing comma, and strips both comment forms', () => {
    const source = [
      'tauri::generate_handler![',
      '    commands::display::list_monitors,',
      '    // commands::display::set_output_monitor, — lands with FR-102',
      '    commands::output::ping, /* inline note */',
      ']',
    ].join('\n')

    expect(findHandlerRegistrations(source)).toEqual({
      commands: ['commands::display::list_monitors', 'commands::output::ping'],
      problems: [],
      invocations: 1,
    })
  })

  it('strips a crate:: or self:: prefix, so the same command is one name', () => {
    const result = findHandlerRegistrations(
      'generate_handler![crate::commands::display::list_monitors, self::db::ping]',
    )
    expect(result.commands).toEqual(['commands::display::list_monitors', 'db::ping'])
  })

  it('does NOT match the macro name mentioned in prose', () => {
    // `commands/mod.rs` writes ``the `generate_handler!` call``; the delimiter
    // must be on the same line as the macro name, so a backtick or a space
    // after the `!` ends the match.
    const source = [
      '//! Every command added here must also be listed in the `generate_handler!`',
      '//! call in [`crate::run`].',
      '//! Nothing is registered by this sentence about generate_handler!',
      '[commands::display::list_monitors]',
    ].join('\n')

    expect(findHandlerRegistrations(source)).toEqual({
      commands: [],
      problems: [],
      invocations: 0,
    })
  })

  it('reports an unclosed invocation as a problem and registers nothing from it', () => {
    const result = findHandlerRegistrations(
      'tauri::generate_handler![commands::display::list_monitors\n',
    )
    expect(result.commands).toEqual([])
    expect(result.invocations).toBe(1)
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toMatch(/never closed/)
  })

  it('reports an entry that is not a plain Rust path as a problem, not a skip', () => {
    // A shape this parser cannot resolve to a command name must fail loudly:
    // silently dropping it would remove a reachable command from the
    // registered set and, with it, from everything this guard compares.
    const result = findHandlerRegistrations(
      'generate_handler![commands::display::list_monitors, make_command!(x)]',
    )
    expect(result.commands).toEqual(['commands::display::list_monitors'])
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toMatch(/not a plain Rust path/)
  })

  it('reports an empty handler as zero commands but ONE invocation', () => {
    // The distinction `summariseInventoryRun` needs: this tree has a handler,
    // it just registers nothing — which is a different failure from a tree
    // whose handler has vanished.
    expect(findHandlerRegistrations('generate_handler![]')).toEqual({
      commands: [],
      problems: [],
      invocations: 1,
    })
  })

  it('reports no invocation for a file that has none', () => {
    expect(findHandlerRegistrations('pub fn run() {}\n')).toEqual({
      commands: [],
      problems: [],
      invocations: 0,
    })
  })

  it('returns the same answer when called twice — the macro regex is global', () => {
    // `HANDLER_MACRO` carries the `g` flag and is a module-level constant. If
    // it were ever consumed with `exec`/`test` instead of `matchAll`, its
    // `lastIndex` would persist and every second file in the walk would parse
    // differently. Pinned rather than assumed.
    const source = 'generate_handler![a::b]'
    expect(findHandlerRegistrations(source)).toEqual(findHandlerRegistrations(source))
  })

  it('parses a CRLF file exactly as it parses an LF one', () => {
    const lines = [
      'tauri::generate_handler![',
      '    commands::display::list_monitors,',
      ']',
    ]
    expect(findHandlerRegistrations(lines.join('\r\n'))).toEqual(
      findHandlerRegistrations(lines.join('\n')),
    )
    expect(findHandlerRegistrations(lines.join('\r\n')).commands).toEqual([
      'commands::display::list_monitors',
    ])
  })
})

describe('parseInventory', () => {
  const REAL_ENTRY =
    '- `commands::display::list_monitors` — enumerates the displays the OS reports (FR-101).'

  it('reads the entry shape the committed inventory itself uses', () => {
    expect(parseInventory(`# IPC command inventory\n\n${REAL_ENTRY}\n`)).toEqual({
      commands: ['commands::display::list_monitors'],
      problems: [],
    })
  })

  it('ignores prose, headings and a thematic break', () => {
    const text = [
      '# IPC command inventory',
      '',
      'Every command this application exposes to its webview, in one place.',
      '',
      '---',
      '',
      'Text that merely mentions - a hyphen mid-sentence.',
      '',
      REAL_ENTRY,
    ].join('\n')

    expect(parseInventory(text)).toEqual({
      commands: ['commands::display::list_monitors'],
      problems: [],
    })
  })

  it('treats a MISTYPED list item as a problem, never as a line to skip', () => {
    // The locked decision. An entry that silently parsed as nothing would drop
    // a command out of the inventory while leaving the file looking complete —
    // precisely the failure this guard exists to prevent.
    const text = [
      REAL_ENTRY,
      '- `commands::output::ping` - a hyphen where the em dash belongs',
    ].join('\n')

    const result = parseInventory(text)
    expect(result.commands).toEqual(['commands::display::list_monitors'])
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toMatch(/line 2/)
    expect(result.problems[0]).toMatch(/starts a list item but is not an entry/)
  })

  it('rejects every other near-miss spelling of an entry, one problem each', () => {
    const nearMisses = [
      '- commands::output::ping — the path is not in backticks',
      '- `commands::output::ping` —',
      '- `commands::output::ping`',
      '- `commands::output::ping` – an en dash, not an em dash',
      '- ``commands::output::ping`` — doubled backticks',
      '- `commands::output::ping()` — parentheses are not part of a path',
    ]

    for (const line of nearMisses) {
      const result = parseInventory(`${line}\n`)
      expect(result.commands, line).toEqual([])
      expect(result.problems, line).toHaveLength(1)
    }
  })

  it('accepts an indented list item, since the line is trimmed before it is read', () => {
    const result = parseInventory(`  ${REAL_ENTRY}\n`)
    expect(result.commands).toEqual(['commands::display::list_monitors'])
    expect(result.problems).toEqual([])
  })

  it('reports a duplicated command once, as a problem, and keeps one entry', () => {
    const result = parseInventory([REAL_ENTRY, REAL_ENTRY].join('\n'))
    expect(result.commands).toEqual(['commands::display::list_monitors'])
    expect(result.problems).toHaveLength(1)
    expect(result.problems[0]).toMatch(/more than once/)
    expect(result.problems[0]).toMatch(/line 2/)
  })

  it('reads a bare crate-root command name', () => {
    const result = parseInventory('- `ping` — a command defined in lib.rs itself.\n')
    expect(result.commands).toEqual(['ping'])
    expect(result.problems).toEqual([])
  })

  it('finds nothing in an inventory whose Commands section was emptied', () => {
    // Not a problem in itself — `summariseInventoryRun` is what refuses to
    // call a run with zero agreed commands a clean one.
    expect(parseInventory('# IPC command inventory\n\n## Commands\n')).toEqual({
      commands: [],
      problems: [],
    })
  })

  it('parses a CRLF inventory exactly as it parses an LF one', () => {
    const lines = ['## Commands', '', REAL_ENTRY]
    expect(parseInventory(lines.join('\r\n'))).toEqual(parseInventory(lines.join('\n')))
    expect(parseInventory(lines.join('\r\n')).commands).toEqual([
      'commands::display::list_monitors',
    ])
  })
})

describe('reconcileCommands — the three directions, each named separately', () => {
  it('agrees when all three lists name the same command', () => {
    const name = 'commands::display::list_monitors'
    expect(reconcile([name], [name], [name])).toEqual({
      agreed: [name],
      discrepancies: [],
    })
  })

  it('DR-: registered and reachable but NOT inventoried — the silent direction', () => {
    // The case a two-way comparison is green for. The definition and the
    // registration were added in the same commit, so they agree; nothing but
    // the third list says the reachable surface just grew.
    const name = 'commands::output::ping'
    const result = reconcile([name], [name], [])

    expect(result.agreed).toEqual([])
    const entry = discrepancyFor(result, name)
    expect(entry.defined).toBe(true)
    expect(entry.registered).toBe(true)
    expect(entry.inventoried).toBe(false)
    expect(entry.reason).toMatch(/absent from the inventory/)
    expect(entry.reason).toMatch(/silent/)
    expect(entry.reason).toMatch(/ADR-0039/)
    expect(entry.reason).toMatch(/commands\.inventory\.md/)
  })

  it('D-I: defined and inventoried but never registered — unreachable', () => {
    const name = 'commands::output::ping'
    const entry = discrepancyFor(reconcile([name], [], [name]), name)
    expect([entry.defined, entry.registered, entry.inventoried]).toEqual([
      true,
      false,
      true,
    ])
    expect(entry.reason).toMatch(/unreachable/)
    expect(entry.reason).toMatch(/generate_handler!/)
  })

  it('--I: inventoried but gone from the code — a stale entry', () => {
    const name = 'commands::legacy::removed'
    const entry = discrepancyFor(reconcile([], [], [name]), name)
    expect([entry.defined, entry.registered, entry.inventoried]).toEqual([
      false,
      false,
      true,
    ])
    expect(entry.reason).toMatch(/stale entry/)
  })

  it('D--: defined and nothing else', () => {
    const name = 'commands::output::ping'
    const entry = discrepancyFor(reconcile([name], [], []), name)
    expect(entry.reason).toMatch(/neither registered nor inventoried/)
  })

  it('-RI: registered and inventoried but not defined', () => {
    const name = 'commands::output::ping'
    const entry = discrepancyFor(reconcile([], [name], [name]), name)
    expect(entry.reason).toMatch(/does not compile/)
  })

  it('-R-: registered and nothing else', () => {
    const name = 'commands::output::ping'
    const entry = discrepancyFor(reconcile([], [name], []), name)
    expect(entry.reason).toMatch(/does not compile/)
  })

  it('gives each of the seven disagreeing combinations its own reason', () => {
    // No two combinations share a sentence, and none falls through to the
    // "is inconsistent across the three lists" fallback — which would mean a
    // reader is told a code rather than what to do.
    const combinations = [
      [1, 1, 0],
      [1, 0, 1],
      [1, 0, 0],
      [0, 1, 1],
      [0, 1, 0],
      [0, 0, 1],
    ]
    const reasons = combinations.map(([d, r, i]) => {
      const name = `m::c${d}${r}${i}`
      const result = reconcile(d ? [name] : [], r ? [name] : [], i ? [name] : [])
      return discrepancyFor(result, name).reason
    })

    expect(new Set(reasons).size).toBe(reasons.length)
    for (const reason of reasons) {
      expect(reason).not.toMatch(/is inconsistent across the three lists/)
    }
  })

  it('sorts both lists, so a diagnostic reads the same on two machines', () => {
    const result = reconcile(
      ['m::zulu', 'm::alpha', 'm::only_defined_z', 'm::only_defined_a'],
      ['m::zulu', 'm::alpha'],
      ['m::zulu', 'm::alpha'],
    )
    expect(result.agreed).toEqual(['m::alpha', 'm::zulu'])
    expect(result.discrepancies.map((entry) => entry.name)).toEqual([
      'm::only_defined_a',
      'm::only_defined_z',
    ])
  })

  it('counts a command named twice in one list as one command', () => {
    const name = 'commands::display::list_monitors'
    expect(reconcile([name], [name, name], [name])).toEqual({
      agreed: [name],
      discrepancies: [],
    })
  })

  it('reports nothing at all when all three lists are empty', () => {
    // Three empty lists agree perfectly, which is why this cannot be where the
    // vacuity check lives — see `summariseInventoryRun` below.
    expect(reconcile([], [], [])).toEqual({ agreed: [], discrepancies: [] })
  })
})

describe('summariseInventoryRun — a vacuous run is a failure, not a clean one', () => {
  const name = 'commands::display::list_monitors'
  const healthy = reconcile([name], [name], [name])

  it('passes only when all three agree on a non-empty set with nothing unparsed', () => {
    const summary = summarise({ reconciliation: healthy })
    expect(summary).toEqual({
      exitCode: 0,
      checked: 1,
      scannedNothing: false,
      foundNoHandler: false,
    })
  })

  it('FAILS when the three lists agree perfectly on NOTHING', () => {
    // The load-bearing clause. An inventory that was emptied, a walk pointed
    // somewhere wrong, or a `generate_handler!` refactored into a shape this
    // guard cannot read all arrive here with three empty sets and no
    // disagreement whatsoever. "In sync" would be true and useless.
    const summary = summarise({ reconciliation: reconcile([], [], []) })
    expect(summary.exitCode).toBe(1)
    expect(summary.checked).toBe(0)
  })

  it('FAILS when no .rs file was scanned, and says so', () => {
    const summary = summarise({ reconciliation: healthy, filesScanned: 0 })
    expect(summary.exitCode).toBe(1)
    expect(summary.scannedNothing).toBe(true)
  })

  it('FAILS when no generate_handler! invocation was found anywhere, and says so', () => {
    const summary = summarise({ reconciliation: healthy, handlerInvocations: 0 })
    expect(summary.exitCode).toBe(1)
    expect(summary.foundNoHandler).toBe(true)
  })

  it('does not report either vacuity flag on a healthy run of exactly one file and one handler', () => {
    // The boundary: one file and one invocation is a real run.
    const summary = summarise({
      reconciliation: healthy,
      filesScanned: 1,
      handlerInvocations: 1,
    })
    expect(summary).toEqual({
      exitCode: 0,
      checked: 1,
      scannedNothing: false,
      foundNoHandler: false,
    })
  })

  it('FAILS on a parse problem even when all three lists agree', () => {
    // A mistyped inventory line alongside a correct one, or a
    // `#[tauri::command]` this parser could not name: the sets can still
    // match, and the run must still be red, because something in the surface
    // was not read.
    const summary = summarise({
      reconciliation: healthy,
      problems: [
        'src-tauri/commands.inventory.md: line 61 starts a list item but is not an entry',
      ],
    })
    expect(summary.exitCode).toBe(1)
    expect(summary.checked).toBe(1)
  })

  it('FAILS on a discrepancy, and still counts the commands that do agree', () => {
    const summary = summarise({
      reconciliation: reconcile([name, 'm::extra'], [name, 'm::extra'], [name]),
    })
    expect(summary.exitCode).toBe(1)
    expect(summary.checked).toBe(1)
  })
})

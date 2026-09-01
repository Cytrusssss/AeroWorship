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
 * The fourth list, added by SEC-01, has its own pair of silent directions and
 * they are covered the same way: a command registered with no permission
 * written for it is rejected at runtime with no gate red anywhere, and a
 * permission left behind by a deleted command idles for ever. Both are asserted
 * by name below, as is the case that decides whether the fourth list applies at
 * all — a tree with no `src-tauri/permissions/` is reconciled as three lists,
 * because for such a tree no capability *could* grant an app-defined command.
 * The window-pattern cases belong to the same group: a `"windows": ["*"]` on a
 * capability that grants an app command hands the projector window every
 * command back, and nothing else in the tree changes.
 *
 * `findRuntimeAclEscapes` is covered here for a reason worth stating: it was
 * the one exported decision in the module with no unit test at all, which is
 * precisely the shape ADR-0020 and the module doc it lives under both forbid,
 * and it is the reason two real ways of reopening the ACL survived a mutation of
 * its comment rule with the whole suite green. Its cases are written as the four
 * spellings that must be caught, the four that must not be, and the two false
 * positives that are deliberate — the last group named so that a later reader
 * removes the rule on purpose rather than by tidying.
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

describe('parseAppPermissionFile', () => {
  it('reads the shape the committed permission file uses', () => {
    const parsed = parseAppPermissionFile(
      JSON.stringify({
        permission: [
          {
            identifier: 'allow-list-monitors',
            description: 'why it exists',
            commands: { allow: ['list_monitors'] },
          },
        ],
      }),
    )

    expect(parsed.problems).toEqual([])
    expect(parsed.permissions).toEqual([
      { identifier: 'allow-list-monitors', allow: ['list_monitors'], deny: [] },
    ])
  })

  it('reports `default` and `set`, which it does not resolve, instead of skipping them', () => {
    // A permission set this guard passed over would drop commands out of the
    // fourth list while leaving the file looking complete — the same silent
    // shape the inventory grammar refuses.
    const parsed = parseAppPermissionFile(
      JSON.stringify({
        default: { permissions: ['allow-list-monitors'] },
        set: [],
        permission: [{ identifier: 'allow-x', commands: { allow: ['x'] } }],
      }),
    )

    expect(parsed.permissions).toHaveLength(1)
    expect(parsed.problems).toHaveLength(2)
    expect(parsed.problems.join(' ')).toMatch(/`default`/)
    expect(parsed.problems.join(' ')).toMatch(/`set`/)
  })

  it('reports `platforms` and `scope`, which would make the answer depend on the target', () => {
    const parsed = parseAppPermissionFile(
      JSON.stringify({
        permission: [
          {
            identifier: 'allow-x',
            commands: { allow: ['x'] },
            platforms: ['windows'],
          },
        ],
      }),
    )

    expect(parsed.problems).toHaveLength(1)
    expect(parsed.problems[0]).toMatch(/`platforms`/)
  })

  it('reports a permission that names no command at all, and keeps it out of the list', () => {
    const parsed = parseAppPermissionFile(
      JSON.stringify({ permission: [{ identifier: 'allow-nothing', commands: {} }] }),
    )

    expect(parsed.permissions).toEqual([])
    expect(parsed.problems[0]).toMatch(/names no command at all/)
  })

  it('keeps `commands.deny`, because denial wins in tauri', () => {
    const parsed = parseAppPermissionFile(
      JSON.stringify({
        permission: [{ identifier: 'deny-x', commands: { deny: ['x'] } }],
      }),
    )

    expect(parsed.permissions).toEqual([{ identifier: 'deny-x', allow: [], deny: ['x'] }])
  })

  it('reports invalid JSON as a problem, not as an empty file', () => {
    const parsed = parseAppPermissionFile('{ not json')

    expect(parsed.permissions).toEqual([])
    expect(parsed.problems[0]).toMatch(/is not valid JSON/)
  })
})

describe('parseCapabilityFile', () => {
  it('reads the shape the committed capabilities use', () => {
    const parsed = parseCapabilityFile(
      JSON.stringify({
        $schema: '../gen/schemas/windows-schema.json',
        identifier: 'main-window',
        description: 'why',
        windows: ['main'],
        permissions: ['core:event:default', 'allow-list-monitors'],
      }),
    )

    expect(parsed.problems).toEqual([])
    expect(parsed.capabilities).toEqual([
      {
        identifier: 'main-window',
        windows: ['main'],
        webviews: [],
        permissions: ['core:event:default', 'allow-list-monitors'],
      },
    ])
  })

  it('reads a capability that grants nothing, which is a statement and not an absence', () => {
    const parsed = parseCapabilityFile(
      JSON.stringify({
        identifier: 'output-window',
        windows: ['output'],
        permissions: [],
      }),
    )

    expect(parsed.problems).toEqual([])
    expect(parsed.capabilities[0]?.permissions).toEqual([])
  })

  it('reads all three file shapes tauri accepts, so none parses as no grants', () => {
    const one = { identifier: 'c', windows: ['main'], permissions: ['allow-x'] }

    expect(parseCapabilityFile(JSON.stringify([one])).capabilities).toHaveLength(1)
    expect(
      parseCapabilityFile(JSON.stringify({ capabilities: [one] })).capabilities,
    ).toHaveLength(1)
    expect(parseCapabilityFile(JSON.stringify(one)).capabilities).toHaveLength(1)
  })

  it('takes the identifier out of the object form of a permission entry', () => {
    // The object form carries a scope. Scope narrows what a command may touch,
    // never which commands are reachable, so only the identifier is read.
    const parsed = parseCapabilityFile(
      JSON.stringify({
        identifier: 'c',
        windows: ['main'],
        permissions: [{ identifier: 'allow-x', allow: [{ path: '$APPDATA' }] }],
      }),
    )

    expect(parsed.problems).toEqual([])
    expect(parsed.capabilities[0]?.permissions).toEqual(['allow-x'])
  })

  it('reports `remote` and `platforms` rather than reading past them', () => {
    const parsed = parseCapabilityFile(
      JSON.stringify({
        identifier: 'c',
        windows: ['main'],
        permissions: [],
        remote: { urls: ['https://example.test'] },
        platforms: ['windows'],
      }),
    )

    expect(parsed.problems).toHaveLength(2)
    expect(parsed.problems.join(' ')).toMatch(/`remote`/)
    expect(parsed.problems.join(' ')).toMatch(/`platforms`/)
  })
})

describe('resolveAppCommandGrants — the fourth list, and when it applies at all', () => {
  /**
   * @param {string[]} windows
   * @param {string[]} permissions
   */
  function capability(windows, permissions) {
    return { identifier: 'main-window', windows, webviews: [], permissions }
  }

  const allowListMonitors = {
    identifier: 'allow-list-monitors',
    allow: ['list_monitors'],
    deny: [],
  }

  it('reports the ACL as OFF when no permission file exists, which is the tauri_build rule', () => {
    // Not leniency: with no permissions/ directory there is no app permission
    // to name, so no capability could grant an app-defined command and
    // requiring a grant would be requiring something that cannot exist.
    const resolution = resolveAppCommandGrants({ permissions: [], capabilities: [] })

    expect(resolution.aclEnabled).toBe(false)
    expect(resolution.granted).toEqual([])
    expect(resolution.problems).toEqual([])
  })

  it('grants the commands a referenced permission allows, named as the ACL names them', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [capability(['main'], ['core:event:default', 'allow-list-monitors'])],
    })

    expect(resolution.aclEnabled).toBe(true)
    expect(resolution.granted).toEqual(['list_monitors'])
    expect(resolution.problems).toEqual([])
  })

  it('subtracts a denial, because denial wins in tauri', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [
        allowListMonitors,
        { identifier: 'no', allow: [], deny: ['list_monitors'] },
      ],
      capabilities: [capability(['main'], ['allow-list-monitors', 'no'])],
    })

    expect(resolution.granted).toEqual([])
  })

  it('passes over prefixed identifiers, which were ACL-gated all along', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [],
      capabilities: [capability(['main'], ['core:event:default'])],
    })

    expect(resolution.aclEnabled).toBe(false)
    expect(resolution.problems).toEqual([])
  })

  it('reports a capability naming a permission no file defines', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [capability(['main'], ['allow-list-monitors', 'allow-typo'])],
    })

    expect(resolution.problems).toHaveLength(1)
    expect(resolution.problems[0]).toMatch(/`allow-typo`, which no permission file/)
  })

  it('reports a permission no capability names — inert, in the direction that reads as safe', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [],
    })

    expect(resolution.problems).toHaveLength(1)
    expect(resolution.problems[0]).toMatch(/no capability names it/)
  })

  it('REJECTS a glob on a capability that grants an app-defined command', () => {
    // The whole point of SEC-01 in one character: "*" hands the output window
    // every command back, and nothing else in the tree changes.
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [capability(['*'], ['allow-list-monitors'])],
    })

    expect(resolution.problems).toHaveLength(1)
    expect(resolution.problems[0]).toMatch(/may name literal labels only/)
  })

  it('REJECTS the output window by name, and says which capability did it', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [capability(['main', 'output'], ['allow-list-monitors'])],
    })

    expect(resolution.problems).toHaveLength(1)
    expect(resolution.problems[0]).toMatch(
      /`main-window` grants app-defined commands to the `output` window/,
    )
  })

  it('rejects a glob in `webviews` too, not only in `windows`', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [
        {
          identifier: 'c',
          windows: [],
          webviews: ['out*'],
          permissions: ['allow-list-monitors'],
        },
      ],
    })

    expect(resolution.problems).toHaveLength(1)
    expect(resolution.problems[0]).toMatch(/may name literal labels only/)
  })

  it('leaves a capability that grants no app command alone, whatever windows it names', () => {
    // core:* permissions have always been scoped by this field and are not this
    // guard's business; a rule that fired on them would be noise.
    const resolution = resolveAppCommandGrants({
      permissions: [allowListMonitors],
      capabilities: [
        capability(['main'], ['allow-list-monitors']),
        {
          identifier: 'other',
          windows: ['*'],
          webviews: [],
          permissions: ['core:event:default'],
        },
      ],
    })

    expect(resolution.problems).toEqual([])
  })

  it('reports two permissions sharing an identifier, where the later silently wins', () => {
    const resolution = resolveAppCommandGrants({
      permissions: [
        allowListMonitors,
        { ...allowListMonitors, allow: ['something_else'] },
      ],
      capabilities: [capability(['main'], ['allow-list-monitors'])],
    })

    expect(resolution.problems.join(' ')).toMatch(/both call themselves/)
    expect(resolution.granted).toEqual(['list_monitors'])
  })
})

describe('findAmbiguousCommandNames — the IPC namespace is flat', () => {
  it('says nothing about distinct command names', () => {
    expect(
      findAmbiguousCommandNames([
        'commands::display::list_monitors',
        'commands::output::ping',
      ]),
    ).toEqual([])
  })

  it('reports two modules whose commands answer to the same IPC name', () => {
    // tauri dispatches on the bare name and a permission allows the bare name,
    // so one permission would grant both and the webview could not say which
    // it called. There is no correct resolution — one has to be renamed.
    const problems = findAmbiguousCommandNames(['a::open', 'b::open'])

    expect(problems).toHaveLength(1)
    expect(problems[0]).toMatch(/`a::open` and `b::open`/)
    expect(problems[0]).toMatch(/single IPC command name `open`/)
  })

  it('does not report the same command listed twice', () => {
    expect(findAmbiguousCommandNames(['a::open', 'a::open'])).toEqual([])
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

describe('reconcileCommands — the fourth list, and the two silent ACL directions', () => {
  const PATH = 'commands::display::list_monitors'
  const NAME = 'list_monitors'

  /**
   * @param {object} sets
   * @param {readonly string[]} [sets.defined]
   * @param {readonly string[]} [sets.registered]
   * @param {readonly string[]} [sets.inventoried]
   * @param {readonly string[]} [sets.granted]
   * @param {boolean} [sets.aclEnabled]
   */
  function reconcileAcl({
    defined = [PATH],
    registered = [PATH],
    inventoried = [PATH],
    granted = [NAME],
    aclEnabled = true,
  }) {
    return reconcileCommands({ defined, registered, inventoried, granted, aclEnabled })
  }

  it('agrees when the capability grants the command the other three name', () => {
    // The bare name and the Rust path are two namespaces, not two spellings:
    // the webview sends `list_monitors` as `cmd` and the permission allows that
    // string, while every other list here uses the module path.
    expect(reconcileAcl({})).toEqual({ agreed: [PATH], discrepancies: [] })
  })

  it('DIRECTION 4a: registered, inventoried, defined — and named by NO capability', () => {
    // Nothing else in this repository goes red for this. It compiles, it is
    // written down, and it is rejected the first time the frontend calls it.
    const { agreed, discrepancies } = reconcileAcl({ granted: [] })

    expect(agreed).toEqual([])
    expect(discrepancies).toHaveLength(1)
    expect(discrepancies[0]?.granted).toBe(false)
    expect(discrepancies[0]?.reason).toMatch(/named by no capability/)
    expect(discrepancies[0]?.reason).toMatch(/Command \{\} not allowed by ACL/)
  })

  it('DIRECTION 4b: a grant left behind by a command that is gone, reported under its bare name', () => {
    const { discrepancies } = reconcileAcl({
      defined: [],
      registered: [],
      inventoried: [],
      granted: ['removed_last_week'],
    })

    expect(discrepancies).toHaveLength(1)
    expect(discrepancies[0]?.name).toBe('removed_last_week')
    expect(discrepancies[0]?.reason).toMatch(/the grant is inert/)
    // The three code lists have never seen this name, so they have no complaint
    // to add; only the ACL clause is printed.
    expect(discrepancies[0]?.reason).not.toMatch(/inconsistent across the three lists/)
  })

  it('says both things about one command when both are wrong', () => {
    const { discrepancies } = reconcileAcl({ inventoried: [], granted: [] })

    expect(discrepancies[0]?.reason).toMatch(/absent from the inventory/)
    expect(discrepancies[0]?.reason).toMatch(/named by no capability/)
  })

  it('does not fault a command that is merely not registered for also not being granted', () => {
    // `D-I` is already the whole story: it is unreachable because no handler
    // names it, and a capability for a command nothing dispatches would be the
    // wrong fix.
    const { discrepancies } = reconcileAcl({ registered: [], granted: [] })

    expect(discrepancies).toHaveLength(1)
    expect(discrepancies[0]?.reason).toMatch(/unreachable/)
    expect(discrepancies[0]?.reason).not.toMatch(/named by no capability/)
  })

  it('matches a crate-root command, whose path is already bare', () => {
    expect(
      reconcileCommands({
        defined: ['ping'],
        registered: ['ping'],
        inventoried: ['ping'],
        granted: ['ping'],
        aclEnabled: true,
      }).agreed,
    ).toEqual(['ping'])
  })

  it('ignores the fourth list entirely when the ACL manifest is off', () => {
    // A tree with no permissions/ is reconciled as three lists, because for
    // such a tree the fourth is not a weaker rule — it is a rule about nothing.
    expect(
      reconcileCommands({
        defined: [PATH],
        registered: [PATH],
        inventoried: [PATH],
        granted: ['whatever'],
        aclEnabled: false,
      }),
    ).toEqual({ agreed: [PATH], discrepancies: [] })
  })

  it('defaults to the ACL being off, so the three-list callers are unchanged', () => {
    expect(
      reconcileCommands({ defined: [PATH], registered: [PATH], inventoried: [PATH] }),
    ).toEqual({ agreed: [PATH], discrepancies: [] })
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

describe('findRuntimeAclEscapes — the ACL may not be reopened from inside', () => {
  // Both names are `pub` on tauri 2.11.5 and neither is behind `dynamic-acl`
  // (`tauri/src/lib.rs:478-482`, `tauri/src/ipc/authority.rs:136-146`), so
  // "there is no runtime escape" is a claim about this repository's source,
  // not about the framework. This function is what makes it one.

  it('reports a plain call to __allow_command, with the line number', () => {
    const source =
      'fn a() {}\nfn b(x: &mut RuntimeAuthority) {\n    x.__allow_command(c, d);\n}\n'
    const problems = findRuntimeAclEscapes(source)
    expect(problems).toHaveLength(1)
    expect(problems[0]).toContain('line 3')
    expect(problems[0]).toContain('`__allow_command`')
    expect(problems[0]).toContain('reopens the ACL from inside the process')
  })

  it('reports a call hidden behind a block comment that closes on the same line', () => {
    // The accident case. A Rust block comment may end mid-line with code after
    // it, so "this line starts with a comment" says nothing about the rest of
    // it — and the developer marking a temporary hack inline is exactly the one
    // who writes this line.
    const source =
      '/* TODO(FR-503): temporary */ context.runtime_authority_mut().__allow_command(o, l);\n'
    expect(findRuntimeAclEscapes(source)).toHaveLength(2)
  })

  it('reports the deref-assign form, which replaces the whole authority', () => {
    // `runtime_authority_mut` returns `&mut RuntimeAuthority`, so this is the
    // idiomatic use of it and it is wider than `__allow_command`: not one
    // command granted to every window, but every rule replaced at once. The
    // leading `*` used to be read as the continuation of a block comment.
    const problems = findRuntimeAclEscapes(
      '*context.runtime_authority_mut() = authority;\n',
    )
    expect(problems).toHaveLength(1)
    expect(problems[0]).toContain('`runtime_authority_mut`')
  })

  it('reports a call that carries an explaining line comment after it', () => {
    // Only the comment is removed, not the line.
    expect(
      findRuntimeAclEscapes('    ctx.__allow_command(a, b); // just for the demo\n'),
    ).toHaveLength(1)
  })

  it('says nothing about the prose that documents the rule', () => {
    // Three files in this repository name both APIs in comments, one of them
    // being the module this function lives in. A rule that went red on its own
    // documentation would be deleted rather than obeyed.
    const source =
      '//! `dynamic-acl` is off, but `runtime_authority_mut` and `__allow_command`\n' +
      '//! stay `pub`, so "no runtime escape" is too strong.\n' +
      '\n' +
      '/// See `__allow_command` for why this is not granted at runtime.\n' +
      '// __allow_command is deliberately not called here.\n' +
      'pub fn nothing() {}\n'
    expect(findRuntimeAclEscapes(source)).toEqual([])
  })

  it('says nothing when the name appears only after `//` on a line of real code', () => {
    expect(findRuntimeAclEscapes('let x = 1; // unlike __allow_command\n')).toEqual([])
  })

  it('reports every occurrence separately, in source order', () => {
    const source =
      'a.__allow_command(x);\n' +
      '// nothing here\n' +
      '*b.runtime_authority_mut() = y;\n'
    const problems = findRuntimeAclEscapes(source)
    expect(problems).toHaveLength(2)
    expect(problems[0]).toContain('line 1')
    expect(problems[1]).toContain('line 3')
  })

  it('DELIBERATELY reports a name inside a string literal', () => {
    // A false positive, and the safe direction for a gate: answering it costs
    // one reworded string, while the alternative is a lexer that could be wrong
    // in the other direction. Asserted so that removing it is a decision.
    expect(findRuntimeAclEscapes('let s = "__allow_command";\n')).toHaveLength(1)
  })

  it('DELIBERATELY reports a middle line of a block comment', () => {
    // Same trade. `stripComments` would remove this, and cannot be used here:
    // it collapses a multi-line block into one space, which loses the line
    // number, and its documented precondition is a fragment with no string
    // literals — which Rust source is not.
    expect(
      findRuntimeAclEscapes(' * __allow_command grants to every window\n'),
    ).toHaveLength(1)
  })

  it('reports `RuntimeAuthority::new`, the only way to build what `Context::new` takes', () => {
    // The path neither of the two names above can see. `Context::new` is `pub`,
    // is not `#[doc(hidden)]` and takes a `RuntimeAuthority` as a plain
    // parameter (`tauri/src/lib.rs:485-497`), so a hand-assembled `Context`
    // installs any ACL at all without ever calling `__allow_command` or
    // `runtime_authority_mut`. `RuntimeAuthority`'s fields are private or
    // `pub(crate)` and it has no `Default` and no `From`, so this constructor
    // and the macro below are the whole of how one can be obtained outside
    // `tauri` — which is why they are on the list while `Context::new`, a
    // substring that would also match `RenderContext::new`, is not.
    const problems = findRuntimeAclEscapes(
      'let context = Context::new(config, assets, icon, None, info, pattern,\n' +
        '    RuntimeAuthority::new(acl, resolved), None);\n',
    )
    expect(problems).toHaveLength(1)
    expect(problems[0]).toContain('line 2')
    expect(problems[0]).toContain('`RuntimeAuthority::new`')
  })

  it('reports the `runtime_authority!` macro, which expands to that constructor', () => {
    // `tauri/src/ipc/authority.rs:80-99` defines it twice, once per
    // `dynamic-acl` arm, and both arms call `RuntimeAuthority::new`. Neither is
    // behind the feature, so turning `dynamic-acl` off (ADR-0013) removes
    // `add_capability` and leaves this one untouched. Listed separately because
    // the macro spelling carries no `::` and would not match the constructor.
    const problems = findRuntimeAclEscapes(
      'let authority = tauri::runtime_authority!(acl, Resolved::default());\n',
    )
    expect(problems).toHaveLength(1)
    expect(problems[0]).toContain('`runtime_authority!`')
  })

  it('says nothing about a file that mentions neither name', () => {
    // The baseline, and only that: it would stay green if this function always
    // answered `[]`, so it pins nothing on its own. The cases above are what
    // hold the positives; this one is here so that a file with no escape in it
    // is stated to be clean rather than left merely untested.
    expect(findRuntimeAclEscapes('pub fn run() {\n    build();\n}\n')).toEqual([])
  })
})

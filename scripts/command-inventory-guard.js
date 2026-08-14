// @ts-check

/**
 * Anti-drift guard for the IPC surface (ADR-0039, NFR-14): everything it
 * decides, in pure functions.
 *
 * The problem it exists for. `generate_handler!` in `src-tauri/src/lib.rs` is
 * the only true list of what the webview can call. App-defined commands are not
 * ACL-gated for a local origin, so they appear in no `capabilities/*.json` and
 * in nothing under `gen/schemas/` — an NFR-14 review of the capability
 * manifests reports a smaller surface than the one that exists. `commands/mod.rs`
 * states the rule that keeps the two halves together, and until this guard
 * existed the rule was enforced by nothing at all.
 *
 * Which direction actually needs a guard. "Defined but not registered" fails
 * loudly the first time the frontend invokes it, and "registered but not
 * defined" does not compile; both police themselves. The direction that is
 * silent is **registered and never reviewed** — a command added to the handler
 * in the same commit that defines it, widening the reachable surface with
 * nothing anywhere saying so. A guard that only compared the two code sites
 * would be green for exactly that case, because the two code sites agree: they
 * were edited together.
 *
 * Hence three sets, not two. Alongside the `#[tauri::command]` definitions and
 * the `generate_handler!` registrations there is a committed inventory,
 * `src-tauri/commands.inventory.md`, one Markdown list item per command:
 *
 *     - `commands::display::list_monitors` — enumerates displays for FR-101.
 *
 * All three must name the same set. The inventory adds no information a parser
 * could not derive; what it adds is a **review artefact** — a file that has to
 * be edited, in prose, in the same diff, and one file for a reviewer to read
 * instead of a macro call buried in a builder chain. That is the whole of its
 * value, and it is why the plain two-way comparison was not enough.
 *
 * Kept free of I/O so every decision can be unit-tested without a Rust
 * toolchain — the same split `dist-html-guard.js` and `bindings-guard.js` make,
 * and for the same reason: a decision that lives only in the script is a
 * decision no test can reach (ADR-0020).
 */

/**
 * A `#[tauri::command]` attribute, anchored to the start of its line.
 *
 * The anchor is what keeps prose from registering commands. Both `mod.rs` and
 * this repository's docs quote the attribute inside `//!`/`///` comments, and a
 * comment line begins with `/`, never with `#`. The bare `#[command]` spelling
 * is accepted too, since `use tauri::command;` makes it legal and no other
 * `command` attribute macro exists in this tree.
 *
 * What it over-reports: an attribute at the start of a line inside a `/* … *\/`
 * block, i.e. a command commented out in bulk. That reads as a definition, so
 * the guard demands a registration and an inventory entry for a function that
 * does not exist and goes red. Noisy, not silent — the right direction for a
 * guard to be wrong in.
 */
const COMMAND_ATTRIBUTE = /^\s*#\[\s*(?:tauri\s*::\s*)?command\b/

/**
 * A function signature, far enough into it to capture the name. Covers the
 * qualifiers that may sit between `#[tauri::command]` and `fn`: any visibility,
 * `async`, `unsafe`, `const`, an ABI string. Anchored for the same reason as
 * `COMMAND_ATTRIBUTE` — `/// fn foo` is documentation, not a definition.
 */
const FUNCTION_SIGNATURE =
  /^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?(?:extern\s+"[^"]*"\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/

/** A Rust path, optionally rooted, as `generate_handler!` and the inventory spell it. */
const RUST_PATH =
  /^(?:crate::|self::)?[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*$/

/**
 * A `generate_handler!` invocation and its opening delimiter.
 *
 * `[^\S\n]*` rather than `\s*`: the delimiter has to be on the same line as the
 * macro name. Prose that mentions ``generate_handler!`` mid-sentence is followed
 * by a backtick or a space, so it never matches; requiring the same line also
 * stops a sentence that happens to end on the macro name from swallowing
 * whatever the next line starts with.
 */
const HANDLER_MACRO = /generate_handler[^\S\n]*![^\S\n]*([[({])/g

/** @type {Record<string, string>} */
const CLOSING_DELIMITER = { '[': ']', '(': ')', '{': '}' }

/**
 * The module path a file under `src-tauri/src/` contributes.
 *
 * `commands/display.rs` and `commands/display/mod.rs` both answer
 * `commands::display`; `lib.rs` and `main.rs` answer the empty string, the crate
 * root, so a command defined there is named by its bare function name — which is
 * how `generate_handler!` would have to spell it.
 *
 * Inline `mod` blocks are not modelled. A command inside one would be attributed
 * to the file's module instead of the nested one, disagree with the path
 * `generate_handler!` needs, and go red. That is the safe direction, and there
 * are no inline modules in this tree today.
 *
 * @param {string} relativePath Path relative to `src-tauri/src/`, either separator.
 * @returns {string | null} The module path, or `null` if this is not a Rust file.
 */
export function modulePathForSource(relativePath) {
  const normalised = relativePath.replaceAll('\\', '/')
  if (!normalised.endsWith('.rs')) return null

  const segments = normalised
    .slice(0, -'.rs'.length)
    .split('/')
    .filter((segment) => segment.length > 0 && segment !== '.')

  const last = segments.at(-1)
  if (last === 'mod' || last === 'lib' || last === 'main') segments.pop()

  return segments.join('::')
}

/**
 * @typedef {object} ParseResult
 * @property {string[]} commands Fully qualified command paths, in source order.
 * @property {string[]} problems Human-readable descriptions of things that could
 *   not be parsed. A problem is a failure, never a reason to skip: a file this
 *   guard cannot read is a file whose commands it cannot see.
 */

/**
 * Every command defined in one Rust source file.
 *
 * The scan is line-based and deliberately shallow — it does not know what a
 * string literal is and does not need to, because both patterns it uses are
 * anchored to the start of a line.
 *
 * An attribute with no function after it is a problem rather than a silent skip.
 * It means either a shape this parser does not understand or a genuinely
 * dangling attribute, and the first of those must not be able to hide a command.
 *
 * @param {string} modulePath As returned by `modulePathForSource`.
 * @param {string} source Contents of the file.
 * @returns {ParseResult}
 */
export function findCommandDefinitions(modulePath, source) {
  /** @type {string[]} */
  const commands = []
  /** @type {string[]} */
  const problems = []

  const lines = source.split('\n')

  for (let i = 0; i < lines.length; i += 1) {
    if (!COMMAND_ATTRIBUTE.test(lines[i] ?? '')) continue

    /** @type {string | null} */
    let name = null
    for (let j = i + 1; j < lines.length; j += 1) {
      const line = lines[j] ?? ''
      // Another attribute first means the previous one never reached a
      // function; stop rather than attributing this one's name to both.
      if (COMMAND_ATTRIBUTE.test(line)) break
      const match = FUNCTION_SIGNATURE.exec(line)
      if (match) {
        name = match[1] ?? null
        break
      }
    }

    if (name === null) {
      problems.push(
        `the \`#[tauri::command]\` on line ${i + 1} is not followed by a function ` +
          'signature this guard recognises, so the command it defines cannot be named',
      )
      continue
    }

    commands.push(modulePath === '' ? name : `${modulePath}::${name}`)
  }

  return { commands, problems }
}

/**
 * @typedef {ParseResult & { invocations: number }} HandlerParseResult
 * @property {number} invocations How many `generate_handler!` calls were found.
 *   Counted so the caller can tell "this file registers nothing" from "this file
 *   has no handler", and so a tree with no handler at all cannot read as clean.
 */

/**
 * Every command registered by the `generate_handler!` calls in one file.
 *
 * More than one invocation is handled because more than one is possible — an
 * `invoke_handler` may also be set on a `WebviewWindowBuilder` — and a second
 * handler that this guard did not read would be an unwatched surface of exactly
 * the kind it exists to close.
 *
 * @param {string} source Contents of the file.
 * @returns {HandlerParseResult}
 */
export function findHandlerRegistrations(source) {
  /** @type {string[]} */
  const commands = []
  /** @type {string[]} */
  const problems = []
  let invocations = 0

  for (const match of source.matchAll(HANDLER_MACRO)) {
    invocations += 1

    const open = match[1] ?? '['
    const close = CLOSING_DELIMITER[open] ?? ']'
    const start = match.index + match[0].length

    let depth = 1
    let end = -1
    for (let i = start; i < source.length; i += 1) {
      const char = source[i]
      if (char === open) depth += 1
      else if (char === close) {
        depth -= 1
        if (depth === 0) {
          end = i
          break
        }
      }
    }

    if (end === -1) {
      problems.push(
        `a \`generate_handler!\` invocation is never closed with \`${close}\`, so the ` +
          'commands it registers could not be read',
      )
      continue
    }

    for (const entry of stripComments(source.slice(start, end)).split(',')) {
      const trimmed = entry.trim()
      if (trimmed === '') continue
      if (!RUST_PATH.test(trimmed)) {
        problems.push(
          `\`generate_handler!\` names \`${trimmed}\`, which is not a plain Rust path; ` +
            'this guard cannot tell which command that is',
        )
        continue
      }
      commands.push(trimmed.replace(/^(?:crate|self)::/, ''))
    }
  }

  return { commands, problems, invocations }
}

/**
 * Removes Rust comments from a fragment known to contain no string literals —
 * the inside of a `generate_handler!` list, which is paths and commas.
 *
 * @param {string} fragment
 * @returns {string}
 */
function stripComments(fragment) {
  return fragment.replace(/\/\*[\s\S]*?\*\//g, ' ').replace(/\/\/[^\n]*/g, '')
}

/**
 * One inventory entry: a Markdown list item naming a command and saying, in one
 * line, what it is for. See `parseInventory` for the exact grammar.
 */
const INVENTORY_ENTRY =
  /^- `([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)`\s+—\s+(\S.*)$/

/**
 * Every command named by the committed inventory.
 *
 * The grammar is one Markdown list item per command: `- `, the full Rust path in
 * backticks, an em dash, then a non-empty justification. Any line whose trimmed
 * form starts with `- ` and does not match is a **problem**, not a line to skip
 * — a mistyped entry that silently parsed as nothing would drop a command out of
 * the inventory while leaving the file looking complete, which is the failure
 * this guard exists to prevent. The cost is that the inventory's prose cannot
 * use bullet lists; that is stated in the file itself.
 *
 * @param {string} text Contents of `src-tauri/commands.inventory.md`.
 * @returns {ParseResult}
 */
export function parseInventory(text) {
  /** @type {string[]} */
  const commands = []
  /** @type {string[]} */
  const problems = []

  const lines = text.split('\n')
  for (let i = 0; i < lines.length; i += 1) {
    const line = (lines[i] ?? '').trim()
    if (!line.startsWith('- ')) continue

    const match = INVENTORY_ENTRY.exec(line)
    if (!match) {
      problems.push(
        `inventory line ${i + 1} starts a list item but is not an entry: expected ` +
          '``- `module::command` — why it exists`` (backticked path, em dash, one ' +
          `non-empty line). Got: ${JSON.stringify(line)}`,
      )
      continue
    }

    const name = match[1] ?? ''
    if (commands.includes(name)) {
      problems.push(`the inventory names \`${name}\` more than once (line ${i + 1})`)
      continue
    }

    commands.push(name)
  }

  return { commands, problems }
}

/**
 * Why a command that is not in all three places is not in all three places.
 *
 * Keyed by presence: `D` defined, `R` registered, `I` inventoried; `-` absent.
 * `DRI` is agreement and has no entry here.
 *
 * @type {Record<string, string>}
 */
const DISCREPANCY_REASONS = {
  'DR-': [
    'reachable from the webview but absent from the inventory. This is the silent',
    'direction: app-defined commands are not ACL-gated, so this command appears in no',
    'capabilities/*.json either and an NFR-14 review of the manifests would not show it',
    '(ADR-0039). Add an entry to src-tauri/commands.inventory.md in the same change that',
    'registers it, or take the registration out.',
  ].join(' '),
  'D-I': [
    'defined and inventoried but never named in `generate_handler!`. It is unreachable:',
    'the frontend gets "Command not found" the first time it invokes it. Add it to the',
    'handler in src-tauri/src/lib.rs.',
  ].join(' '),
  'D--': [
    'defined with `#[tauri::command]` but neither registered nor inventoried. Either a',
    'registration was forgotten (it fails at invoke time, not at compile time) or this is',
    'dead code that should go.',
  ].join(' '),
  '-RI': [
    'registered and inventoried, but no `#[tauri::command]` under src-tauri/src/ defines',
    'it. That does not compile — check the spelling of the path in `generate_handler!`.',
  ].join(' '),
  '-R-': [
    'registered but neither defined nor inventoried. That does not compile; if the command',
    'was renamed, the handler still names the old path.',
  ].join(' '),
  '--I': [
    'inventoried but neither defined nor registered — a stale entry left behind by a',
    'command that was removed or renamed. Delete it, so the inventory stays a list of what',
    'is reachable rather than of what once was.',
  ].join(' '),
}

/**
 * @typedef {object} Discrepancy
 * @property {string} name Fully qualified command path.
 * @property {boolean} defined Whether a `#[tauri::command]` defines it.
 * @property {boolean} registered Whether `generate_handler!` names it.
 * @property {boolean} inventoried Whether the committed inventory names it.
 * @property {string} reason What that combination means and what to do about it.
 */

/**
 * @typedef {object} Reconciliation
 * @property {string[]} agreed Commands present in all three places, sorted.
 * @property {Discrepancy[]} discrepancies Everything else, sorted by name.
 */

/**
 * Compares the three lists.
 *
 * Sorted output, so a diagnostic reads the same on two machines whose directory
 * listings came back in different orders.
 *
 * @param {object} sets
 * @param {readonly string[]} sets.defined
 * @param {readonly string[]} sets.registered
 * @param {readonly string[]} sets.inventoried
 * @returns {Reconciliation}
 */
export function reconcileCommands({ defined, registered, inventoried }) {
  const definedSet = new Set(defined)
  const registeredSet = new Set(registered)
  const inventoriedSet = new Set(inventoried)

  const names = [...new Set([...definedSet, ...registeredSet, ...inventoriedSet])].sort()

  /** @type {string[]} */
  const agreed = []
  /** @type {Discrepancy[]} */
  const discrepancies = []

  for (const name of names) {
    const isDefined = definedSet.has(name)
    const isRegistered = registeredSet.has(name)
    const isInventoried = inventoriedSet.has(name)

    const key = `${isDefined ? 'D' : '-'}${isRegistered ? 'R' : '-'}${isInventoried ? 'I' : '-'}`

    if (key === 'DRI') {
      agreed.push(name)
      continue
    }

    discrepancies.push({
      name,
      defined: isDefined,
      registered: isRegistered,
      inventoried: isInventoried,
      // The fallback is unreachable — eight combinations, one agreed and seven
      // listed — but a missing key must not produce `undefined` in a diagnostic.
      reason:
        DISCREPANCY_REASONS[key] ?? `is inconsistent across the three lists (${key})`,
    })
  }

  return { agreed, discrepancies }
}

/**
 * @typedef {object} InventorySummary
 * @property {number} exitCode 0 only when all three lists name the same
 *   non-empty set and nothing failed to parse.
 * @property {number} checked How many commands are accounted for in all three.
 * @property {boolean} scannedNothing Whether the walk found no Rust source at
 *   all, which is a failure of this guard's premise rather than a clean run.
 * @property {boolean} foundNoHandler Whether the tree contains no
 *   `generate_handler!` invocation at all.
 */

/**
 * Turns the reconciliation into counts and an exit code.
 *
 * The three clauses that earn their place are the vacuous ones. Every other
 * check here asks whether the lists disagree, and three empty lists agree
 * perfectly — so a walk pointed at the wrong directory, a `generate_handler!`
 * that was refactored into a shape this guard does not recognise, or an
 * inventory that was emptied, would all print "in sync" about a surface nobody
 * examined. An empty run is therefore a failure, and it is the same reasoning
 * `summariseComparison` in `bindings-guard.js` applies to an empty generator.
 *
 * @param {object} run
 * @param {Reconciliation} run.reconciliation
 * @param {readonly string[]} run.problems
 * @param {number} run.filesScanned
 * @param {number} run.handlerInvocations
 * @returns {InventorySummary}
 */
export function summariseInventoryRun({
  reconciliation,
  problems,
  filesScanned,
  handlerInvocations,
}) {
  const scannedNothing = filesScanned === 0
  const foundNoHandler = handlerInvocations === 0

  const failed =
    scannedNothing ||
    foundNoHandler ||
    problems.length > 0 ||
    reconciliation.discrepancies.length > 0 ||
    reconciliation.agreed.length === 0

  return {
    exitCode: failed ? 1 : 0,
    checked: reconciliation.agreed.length,
    scannedNothing,
    foundNoHandler,
  }
}

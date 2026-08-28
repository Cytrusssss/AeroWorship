// @ts-check

/**
 * Anti-drift guard for the IPC surface (ADR-0039, NFR-14): everything it
 * decides, in pure functions.
 *
 * The problem it exists for. `generate_handler!` in `src-tauri/src/lib.rs` is
 * the only true list of what the webview can call. For as long as this
 * application shipped no ACL manifest of its own, app-defined commands were not
 * ACL-gated for a local origin at all, so they appeared in no
 * `capabilities/*.json` and in nothing under `gen/schemas/` — an NFR-14 review
 * of the capability manifests reported a smaller surface than the one that
 * existed. `commands/mod.rs` states the rule that keeps the halves together, and
 * until this guard existed the rule was enforced by nothing at all.
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
 * Four sets since SEC-01, and the fourth one has teeth the other three do not.
 * `src-tauri/permissions/` now exists, which is the whole switch: `tauri_build`
 * globs `permissions/**` and, from the first permission it finds, the app ships
 * an ACL manifest and `tauri` rejects every **app-defined** command that no
 * capability names, at invoke time, with `Command {} not allowed by ACL`.
 * Not *every* command: `plugin:__TAURI_CHANNEL__|fetch` is excluded inside the
 * gate itself (`tauri-2.11.5/src/webview/mod.rs:1823-1826`), which is a hole
 * these four lists cannot see and `src-tauri/src/commands/mod.rs` owns. So the
 * fourth
 * list is the commands granted by `src-tauri/capabilities/*.json` through those
 * permissions, and it closes two failures that are silent in opposite
 * directions:
 *
 *   - a command registered with no permission written for it is dead at
 *     runtime, with **nothing** red before it — it compiles, it is inventoried,
 *     and `dynamic-acl` is off (ADR-0013) so no restart helps and no
 *     configuration file reopens it; only new Rust code, and therefore a
 *     rebuild;
 *   - a permission left behind by a command that was deleted idles for ever,
 *     because nothing anywhere rejects a grant for a command that does not
 *     exist.
 *
 * The fourth list is only *required* when the ACL manifest is on, and that is
 * modelled rather than assumed: `resolveAppCommandGrants` reports `aclEnabled`
 * from whether any permission file was found, exactly as `tauri_build` decides
 * it. A tree with no `permissions/` directory is reconciled as three lists,
 * because for such a tree the fourth is not a weaker rule — it is a rule about
 * nothing.
 *
 * The window patterns are checked too, and that is not scope creep: the point
 * of SEC-01 is that the `output` window gets zero app-defined commands, and one
 * `"windows": ["*"]` in a capability hands them all back with no other symptom.
 * A capability that grants an app command may therefore name only literal
 * window and webview labels, and never the output window's.
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
 * The label the Display Service gives the Projector Output window
 * (`OUTPUT_WINDOW_LABEL` in `src-tauri/src/services/display.rs`).
 *
 * Written out here rather than parsed out of the Rust, because the check it
 * serves is not "is this string equal to that string" — it is "a capability that
 * grants an app-defined command may name literal labels only, and `output` is
 * not one of them".
 *
 * Note the shape of that rule, because it is weaker than an allow-list and the
 * difference is deliberate. A third literal label — `preview`, say — passes.
 * The guard is not a list of windows that may be granted things; it forbids the
 * two spellings that hand a grant to a window nobody named: a glob, and this
 * label. An allow-list was rejected because the set of windows is not fixed
 * (PRD §6.3 creates them from Rust), so it would have to be edited by every
 * item that adds one, and a rule edited that often stops being read. What keeps
 * a third window honest is that its label appears in a diff; what a glob and
 * `output` have in common is that neither does.
 */
const OUTPUT_WINDOW_LABEL = 'output'

/**
 * Names whose appearance in this crate would mean the ACL is edited, or built,
 * outside the manifest. See `findRuntimeAclEscapes`.
 *
 * The first two edit an authority that already exists. The second two build a
 * new one, which is a route that needs no obfuscation at all: `Context::new`
 * (`tauri/src/lib.rs:485-497`) is `pub`, is not `#[doc(hidden)]`, and takes a
 * `RuntimeAuthority` as an ordinary parameter, so a hand-assembled `Context`
 * handed to `Builder::build` installs any ACL it likes without either of the
 * first two names appearing anywhere.
 *
 * `Context::new` itself is deliberately NOT on this list, and the reason is a
 * chokepoint rather than a judgement call. `RuntimeAuthority`'s fields are all
 * private or `pub(crate)`, it has no `Default` and no `From`, so outside `tauri`
 * the only ways to obtain one are `RuntimeAuthority::new` and the
 * `runtime_authority!` macro that expands to it. Covering the two constructors
 * covers every value `Context::new` could ever be handed, and it avoids a
 * substring that would also match `RenderContext::new` or `SlideContext::new` —
 * names this codebase has no reason not to grow. Measured on the tree as it
 * stands: `Context::new` matches 0 lines, but so do the bare words `Context`
 * and `Authority`, so today's count cannot rule that risk out.
 */
const RUNTIME_ACL_ESCAPES = [
  '__allow_command',
  'runtime_authority_mut',
  'RuntimeAuthority::new',
  'runtime_authority!',
]

/** Characters that make a capability's window/webview entry a pattern rather than a label. */
const GLOB_METACHARACTERS = /[*?[\]{}]/

/**
 * Top-level keys an app permission file may carry.
 *
 * `default` and `set` are legal in Tauri's format and are deliberately absent:
 * this guard does not resolve permission sets, and a set it silently ignored
 * would drop commands out of the fourth list while leaving the file looking
 * complete. Meeting one is a problem, which is the loud direction.
 */
const PERMISSION_FILE_KEYS = new Set(['$schema', 'permission'])

/** Keys of a single inlined permission this guard understands. */
const PERMISSION_KEYS = new Set(['identifier', 'description', 'commands'])

/** Keys of a permission's `commands` object. */
const PERMISSION_COMMANDS_KEYS = new Set(['allow', 'deny'])

/**
 * Top-level keys a capability file may carry.
 *
 * `remote` and `platforms` are legal and deliberately absent for the same
 * reason as `set` above, but the stakes are higher: `remote` would extend a
 * grant to content this application did not author (NFR-13), and `platforms`
 * would make the answer depend on a build target this guard cannot see. Both
 * are problems, not fields to skip.
 */
const CAPABILITY_KEYS = new Set([
  '$schema',
  'identifier',
  'description',
  'local',
  'windows',
  'webviews',
  'permissions',
])

/**
 * @param {unknown} value
 * @returns {value is Record<string, unknown>}
 */
function isPlainObject(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/**
 * Reads an optional array-of-strings field, reporting anything else.
 *
 * @param {Record<string, unknown>} owner
 * @param {string} key
 * @param {string} where Human-readable owner, for the diagnostic.
 * @param {string[]} problems Appended to in place.
 * @returns {string[]}
 */
function readStringArray(owner, key, where, problems) {
  const value = owner[key]
  if (value === undefined) return []
  if (!Array.isArray(value) || value.some((item) => typeof item !== 'string')) {
    problems.push(`${where}: \`${key}\` must be an array of strings`)
    return []
  }
  return /** @type {string[]} */ (value)
}

/**
 * @typedef {object} AppPermission
 * @property {string} identifier As a capability spells it, with no prefix.
 * @property {string[]} allow Commands this permission allows, as
 *   `generate_handler!` spells them.
 * @property {string[]} deny Commands it denies. Denial wins in `tauri`, so this
 *   is subtracted from the grant rather than ignored.
 */

/**
 * Every permission defined by one file under `src-tauri/permissions/`.
 *
 * JSON only. Tauri also accepts TOML here; the caller reports a TOML file as a
 * problem rather than parsing it, because a permission format this guard cannot
 * read is a grant it cannot see, and adding a TOML parser would be a dependency
 * against the NFR-16 budget for a file this repository has decided to write in
 * the same language as its capabilities.
 *
 * @param {string} text Contents of the permission file.
 * @returns {{ permissions: AppPermission[], problems: string[] }}
 */
export function parseAppPermissionFile(text) {
  /** @type {AppPermission[]} */
  const permissions = []
  /** @type {string[]} */
  const problems = []

  /** @type {unknown} */
  let parsed
  try {
    parsed = JSON.parse(text)
  } catch (error) {
    problems.push(
      `is not valid JSON, so the permissions it defines cannot be read: ${error}`,
    )
    return { permissions, problems }
  }

  if (!isPlainObject(parsed)) {
    problems.push('must be a JSON object with a `permission` array')
    return { permissions, problems }
  }

  for (const key of Object.keys(parsed)) {
    if (!PERMISSION_FILE_KEYS.has(key)) {
      problems.push(
        `carries the top-level key \`${key}\`, which this guard does not model. Only ` +
          '`permission` (an array of inlined permissions) and `$schema` are understood; ' +
          '`default` and `set` would hide commands from the fourth list',
      )
    }
  }

  const entries = parsed.permission
  if (entries === undefined) {
    problems.push(
      'defines no `permission` array, so it grants nothing and should not exist',
    )
    return { permissions, problems }
  }
  if (!Array.isArray(entries)) {
    problems.push('has a `permission` key that is not an array')
    return { permissions, problems }
  }

  for (let i = 0; i < entries.length; i += 1) {
    const entry = entries[i]
    const where = `permission ${i + 1}`

    if (!isPlainObject(entry)) {
      problems.push(`${where} is not an object`)
      continue
    }

    for (const key of Object.keys(entry)) {
      if (!PERMISSION_KEYS.has(key)) {
        problems.push(
          `${where} carries \`${key}\`, which this guard does not model — a permission ` +
            'narrowed by `platforms` or `scope` would make this answer wrong on some ' +
            'build, and being wrong quietly is what this guard exists to stop',
        )
      }
    }

    const identifier = entry.identifier
    if (typeof identifier !== 'string' || identifier.trim() === '') {
      problems.push(`${where} has no \`identifier\`, so no capability could name it`)
      continue
    }

    const commands = entry.commands
    if (!isPlainObject(commands)) {
      problems.push(`\`${identifier}\` has no \`commands\` object, so it allows nothing`)
      continue
    }
    for (const key of Object.keys(commands)) {
      if (!PERMISSION_COMMANDS_KEYS.has(key)) {
        problems.push(
          `\`${identifier}\` carries \`commands.${key}\`, which is not a thing`,
        )
      }
    }

    const allow = readStringArray(commands, 'allow', `\`${identifier}\``, problems)
    const deny = readStringArray(commands, 'deny', `\`${identifier}\``, problems)

    if (allow.length === 0 && deny.length === 0) {
      problems.push(
        `\`${identifier}\` names no command at all, so granting it does nothing. Give it ` +
          'a `commands.allow`, or delete it',
      )
      continue
    }

    permissions.push({ identifier, allow, deny })
  }

  return { permissions, problems }
}

/**
 * @typedef {object} ParsedCapability
 * @property {string} identifier
 * @property {string[]} windows Window labels or patterns, verbatim.
 * @property {string[]} webviews Webview labels or patterns, verbatim.
 * @property {string[]} permissions Permission identifiers, verbatim, `core:`
 *   prefixes and all.
 */

/**
 * Every capability declared by one file under `src-tauri/capabilities/`.
 *
 * A capability file may hold one capability, a list of them, or an object with
 * a `capabilities` key; all three are Tauri's format and all three are read,
 * because a shape that parsed as nothing would report a window as granted
 * nothing when it is granted everything.
 *
 * @param {string} text Contents of the capability file.
 * @returns {{ capabilities: ParsedCapability[], problems: string[] }}
 */
export function parseCapabilityFile(text) {
  /** @type {ParsedCapability[]} */
  const capabilities = []
  /** @type {string[]} */
  const problems = []

  /** @type {unknown} */
  let parsed
  try {
    parsed = JSON.parse(text)
  } catch (error) {
    problems.push(`is not valid JSON, so the grants it makes cannot be read: ${error}`)
    return { capabilities, problems }
  }

  /** @type {unknown[]} */
  let raw
  if (Array.isArray(parsed)) {
    raw = parsed
  } else if (isPlainObject(parsed) && Array.isArray(parsed.capabilities)) {
    raw = parsed.capabilities
  } else if (isPlainObject(parsed)) {
    raw = [parsed]
  } else {
    problems.push('is neither a capability, a list of capabilities, nor a named list')
    return { capabilities, problems }
  }

  for (let i = 0; i < raw.length; i += 1) {
    const entry = raw[i]
    if (!isPlainObject(entry)) {
      problems.push(`capability ${i + 1} is not an object`)
      continue
    }

    const identifier =
      typeof entry.identifier === 'string' && entry.identifier.trim() !== ''
        ? entry.identifier
        : `capability ${i + 1}`
    if (typeof entry.identifier !== 'string') {
      problems.push(`capability ${i + 1} has no \`identifier\``)
    }

    for (const key of Object.keys(entry)) {
      if (!CAPABILITY_KEYS.has(key)) {
        problems.push(
          `\`${identifier}\` carries \`${key}\`, which this guard does not model. ` +
            '`remote` would extend a grant to content this application did not author ' +
            '(NFR-13) and `platforms` would make the grant depend on a build target ' +
            'nothing here can see; neither may pass unread',
        )
      }
    }

    /** @type {string[]} */
    const identifiers = []
    const entries = entry.permissions
    if (entries === undefined) {
      problems.push(`\`${identifier}\` has no \`permissions\` key`)
    } else if (!Array.isArray(entries)) {
      problems.push(`\`${identifier}\` has a \`permissions\` key that is not an array`)
    } else {
      for (const permission of entries) {
        // The object form carries a scope alongside the identifier. Scope
        // narrows what a command may touch, never which commands are reachable,
        // so only the identifier matters here.
        const name = isPlainObject(permission) ? permission.identifier : permission
        if (typeof name !== 'string' || name.trim() === '') {
          problems.push(
            `\`${identifier}\` lists a permission this guard cannot name: ` +
              `${JSON.stringify(permission)}`,
          )
          continue
        }
        identifiers.push(name)
      }
    }

    capabilities.push({
      identifier,
      windows: readStringArray(entry, 'windows', `\`${identifier}\``, problems),
      webviews: readStringArray(entry, 'webviews', `\`${identifier}\``, problems),
      permissions: identifiers,
    })
  }

  return { capabilities, problems }
}

/**
 * @typedef {object} GrantResolution
 * @property {boolean} aclEnabled Whether any app permission exists — which is
 *   exactly how `tauri_build` decides whether the app ships an ACL manifest, and
 *   therefore whether an ungranted command is rejected at runtime.
 * @property {string[]} granted App-defined commands some capability makes
 *   reachable, sorted, and named the way the ACL names them: bare, with no
 *   module path, because that is the string the webview sends as `cmd` and the
 *   string `commands.allow` matches. `reconcileCommands` is what translates
 *   between that flat namespace and the Rust paths the other three lists use.
 * @property {string[]} problems Everything wrong with the permission and
 *   capability files themselves, as opposed to with the sets they produce.
 */

/**
 * Turns the permission files and the capability files into the fourth list.
 *
 * Three things are checked here that the set comparison downstream cannot see,
 * because each of them is a way for a grant to be wrong while every list still
 * agrees:
 *
 *   - a capability naming a permission no file defines — the build fails on
 *     this too, but `npm run lint` is minutes earlier and says which capability;
 *   - a permission no capability names — inert, and inert in the direction that
 *     reads as safe, so nothing would ever report it;
 *   - a capability that grants an app-defined command to a glob, to no window at
 *     all, or to the output window. The first is how the `output` window gets
 *     its whole reach back in one character, which is the thing SEC-01 exists to
 *     prevent.
 *
 * Permissions whose identifier carries a prefix (`core:event:default`,
 * `plugin:x|y`) belong to Tauri or to a plugin, not to this application, and are
 * passed over: they were ACL-gated all along and are not what this guard tracks.
 *
 * @param {object} acl
 * @param {readonly AppPermission[]} acl.permissions Every app permission found.
 * @param {readonly ParsedCapability[]} acl.capabilities Every capability found.
 * @returns {GrantResolution}
 */
export function resolveAppCommandGrants({ permissions, capabilities }) {
  /** @type {string[]} */
  const problems = []

  /** @type {Map<string, AppPermission>} */
  const byIdentifier = new Map()
  for (const permission of permissions) {
    if (byIdentifier.has(permission.identifier)) {
      problems.push(
        `two permissions under src-tauri/permissions/ both call themselves ` +
          `\`${permission.identifier}\`; the later one silently replaces the earlier`,
      )
      continue
    }
    byIdentifier.set(permission.identifier, permission)
  }

  /** @type {Set<string>} */
  const referenced = new Set()
  /** @type {Set<string>} */
  const allowed = new Set()
  /** @type {Set<string>} */
  const denied = new Set()

  for (const capability of capabilities) {
    const appPermissions = capability.permissions.filter((name) => !name.includes(':'))
    if (appPermissions.length === 0) continue

    for (const name of appPermissions) {
      const permission = byIdentifier.get(name)
      if (!permission) {
        problems.push(
          `capability \`${capability.identifier}\` names \`${name}\`, which no permission ` +
            'file under src-tauri/permissions/ defines. An identifier with no prefix is an ' +
            'app permission; if a core one was meant, it needs its `core:` prefix. ' +
            'tauri-build fails on this too, at the next build',
        )
        continue
      }
      referenced.add(name)
      for (const command of permission.allow) allowed.add(command)
      for (const command of permission.deny) denied.add(command)
    }

    const targets = [...capability.windows, ...capability.webviews]
    if (targets.length === 0) {
      problems.push(
        `capability \`${capability.identifier}\` grants app-defined commands but names ` +
          'no window and no webview, so it grants them to nothing. Name `main`, or take ' +
          'the permissions out',
      )
    }
    for (const target of targets) {
      if (GLOB_METACHARACTERS.test(target)) {
        problems.push(
          `capability \`${capability.identifier}\` grants app-defined commands to the ` +
            `pattern "${target}". A capability that grants an app-defined command may ` +
            'name literal labels only: a glob is how the output window gets every ' +
            'command back in one character, with nothing else changing (SEC-01)',
        )
      } else if (target === OUTPUT_WINDOW_LABEL) {
        problems.push(
          `capability \`${capability.identifier}\` grants app-defined commands to the ` +
            `\`${OUTPUT_WINDOW_LABEL}\` window. It is granted none, deliberately — see ` +
            'capabilities/output-window.json, which exists to say so (SEC-01, NFR-13)',
        )
      }
    }
  }

  for (const identifier of byIdentifier.keys()) {
    if (referenced.has(identifier)) continue
    problems.push(
      `the permission \`${identifier}\` is defined under src-tauri/permissions/ but no ` +
        'capability names it, so it grants nothing to anyone. Name it in a capability, or ' +
        'delete it — an unreferenced permission reads as protection that is not there',
    )
  }

  return {
    aclEnabled: byIdentifier.size > 0,
    granted: [...allowed].filter((command) => !denied.has(command)).sort(),
    problems,
  }
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
    'direction: the definition and the registration are edited in the same change, so the',
    'two code sites agree and neither reports anything — the inventory is the only list',
    'that can, and it is the one file an NFR-14 review reads instead of a macro call in a',
    'builder chain (ADR-0039). Add an entry to src-tauri/commands.inventory.md in the same',
    'change that registers it, or take the registration out.',
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
 * The two ACL directions, kept as separate sentences rather than folded into
 * `DISCREPANCY_REASONS`.
 *
 * Sixteen canned strings for four flags would be sixteen strings nobody reads.
 * The ACL dimension is orthogonal to the definition/registration/inventory one
 * — a command can be missing from the inventory and from the capabilities for
 * unrelated reasons — so it is reported as its own clause appended to whatever
 * the other three lists had to say.
 *
 * Written as predicates, like `DISCREPANCY_REASONS`, because either may be the
 * first thing said about a command; the caller re-attaches a subject when one
 * follows another.
 *
 * @type {Record<'ungranted' | 'stale', string>}
 */
const ACL_REASONS = {
  ungranted: [
    'is registered in `generate_handler!` and named by no capability. src-tauri/permissions/',
    'exists, so this application ships an ACL manifest and app-defined commands are gated',
    'exactly like plugin: ones: tauri rejects this command at invoke time with',
    '"Command {} not allowed by ACL", and nothing else anywhere goes red first. There is no',
    'runtime escape — dynamic-acl is off (ADR-0013), so only a rebuild fixes it. Write an',
    'allow-… permission under src-tauri/permissions/ and name it in',
    'capabilities/main-window.json, in the same change that registers the command.',
  ].join(' '),
  stale: [
    'is named by a capability but is not registered in `generate_handler!`, so the grant',
    'is inert — and stays inert, because nothing rejects a permission for a command that does',
    'not exist. Delete the entry under src-tauri/permissions/ and the line in the capability',
    'that names it.',
  ].join(' '),
}

/**
 * The name the ACL knows a command by: its bare function name.
 *
 * This is not a formatting detail, it is the reason the fourth list needs a
 * translation at all. `generate_handler!`, the inventory and the source all
 * spell a command as a Rust path; the webview passes `list_monitors` as
 * `request.cmd` and a permission's `commands.allow` names it the same way, so
 * the ACL namespace is flat. Two commands whose paths differ only in their
 * module are one command to it — which is why `findAmbiguousCommandNames`
 * exists.
 *
 * @param {string} path
 * @returns {string}
 */
function commandName(path) {
  const separator = path.lastIndexOf('::')
  // A command defined in `lib.rs` or `main.rs` is already bare — `modulePathForSource`
  // answers the empty string for the crate root — so "no separator" is a real
  // case here and not a defensive branch.
  return separator === -1 ? path : path.slice(separator + '::'.length)
}

/**
 * Registered commands whose bare names collide.
 *
 * The ACL namespace is flat and so is the IPC one: `tauri` dispatches on the
 * `cmd` string the webview sends, and a permission allows that same string. Two
 * commands called `open` in different modules are therefore indistinguishable
 * to both, and a permission written for one grants the other. Reported rather
 * than resolved, because there is no correct resolution — one of them has to be
 * renamed.
 *
 * @param {readonly string[]} registered
 * @returns {string[]} One problem per colliding name, sorted.
 */
export function findAmbiguousCommandNames(registered) {
  /** @type {Map<string, Set<string>>} */
  const byName = new Map()
  for (const path of new Set(registered)) {
    const name = commandName(path)
    const paths = byName.get(name) ?? new Set()
    paths.add(path)
    byName.set(name, paths)
  }

  /** @type {string[]} */
  const problems = []
  for (const [name, paths] of [...byName].sort(([a], [b]) => a.localeCompare(b))) {
    if (paths.size < 2) continue
    problems.push(
      `\`${[...paths].sort().join('` and `')}\` are both registered and both answer to the ` +
        `single IPC command name \`${name}\`. tauri dispatches on that name and a ` +
        'permission allows that name, so one permission grants both and the webview ' +
        'cannot say which it called. Rename one of them',
    )
  }
  return problems
}

/**
 * Tauri APIs that reopen the ACL from inside the running process.
 *
 * `dynamic-acl` being off (ADR-0013) removes `Manager::add_capability`, and it
 * is tempting to write "there is no runtime escape" and stop there. That is
 * broader than what tauri guarantees. On 2.11.5 both
 * `Context::runtime_authority_mut` (`tauri/src/lib.rs:478-482`) and
 * `RuntimeAuthority::__allow_command` (`tauri/src/ipc/authority.rs:136-146`)
 * are `pub` and neither is behind that feature; the second inserts
 * `windows: vec!["*"]`, which is precisely the reach `capabilities/*.json` are
 * written to withhold, granted to every window at once and visible in no
 * manifest, no capability file and nothing under `gen/schemas/`.
 *
 * What stays true is the consequence, not the absolute: reopening the ACL takes
 * Rust code, so it takes a rebuild and it takes a diff. This makes that diff
 * impossible to land quietly, which is the only part a guard can enforce. Zero
 * occurrences today; if an item ever needs one, it deletes this rule in the
 * same change and says why.
 *
 * `#[doc(hidden)]` on both is not a defence. It hides them from rustdoc and
 * from nothing else.
 *
 * ## What counts as a comment here, and why it is only `//`
 *
 * These names have to be sayable in prose: three files in this repository
 * document both of them, one of which is this module. So `//`, `///` and `//!`
 * are removed from each line before it is searched — a line comment eats the
 * rest of its line, which makes that removal exact.
 *
 * Nothing else is removed, and that is the correction this function exists in
 * its current form to record. It used to SKIP any line matching
 * `/^\s*(\/\/|\*|\/\*)/`, and two real ways of reopening the ACL walked
 * straight past it:
 *
 *   - `/* dev only *``/ context.runtime_authority_mut().__allow_command(...)` —
 *     a Rust block comment may close mid-line with code after it on the same
 *     line, so "starts with `/*`" says nothing about the rest of the line. This
 *     is the accident case, not the attacker case: the developer who marks a
 *     temporary hack with an inline comment is exactly the one who writes it.
 *   - `*context.runtime_authority_mut() = authority;` — the signature is
 *     `-> &mut RuntimeAuthority` (`tauri/src/lib.rs:480`), so deref-assign is
 *     the idiomatic use of it, and it replaces the WHOLE authority, which is
 *     wider than `__allow_command`. The leading `*` was being read as the
 *     continuation of a block comment.
 *
 * `stripComments` in this module was the obvious material and does not fit.
 * Two reasons, both disqualifying: it collapses a multi-line `/* *``/` into one
 * space, so the line number in the diagnostic below would be wrong; and its
 * documented precondition is a fragment with no string literals, which Rust
 * source is not — a `"/*"` inside a string would blank out the real code after
 * it, and a false NEGATIVE is the one direction this rule may not have.
 *
 * The deliberate false positives, which are not to be "fixed": a name inside a
 * string literal is reported, and so is a middle line of a `/* *``/ block that
 * does not begin with `//`. Both make a gate noisy in the safe direction, and
 * both are cheap to answer — reword the prose, or say why the call is there.
 * The only false negative left is a `//` appearing inside a string literal
 * before one of these names on the same physical line, as in
 * `log(&format!("see http://x"), ctx.runtime_authority_mut());`. Measured:
 * `rustfmt --check` exits 0 on that line and this function returns `[]`. Do not
 * write "formatting closes this" — it closes the two-statement spelling, where
 * rustfmt breaks the line, and it does not close the single-expression one. The
 * residue is stated rather than mitigated: this rule calls itself a grep, not a
 * compiler, and that is the shape of a grep.
 *
 * @param {string} source Contents of one `.rs` file.
 * @returns {string[]} One problem per occurrence, in source order.
 */
export function findRuntimeAclEscapes(source) {
  /** @type {string[]} */
  const problems = []
  const lines = source.split(/\r?\n/)

  for (let i = 0; i < lines.length; i += 1) {
    // Only `//` to end of line, so `///` and `//!` prose is dropped while a
    // closed block comment leaves the code that follows it on the line.
    const line = (lines[i] ?? '').replace(/\/\/.*$/, '')

    for (const escape of RUNTIME_ACL_ESCAPES) {
      if (!line.includes(escape)) continue
      problems.push(
        `line ${i + 1} calls \`${escape}\`, which reopens the ACL from inside the process. ` +
          '`dynamic-acl` is off (ADR-0013), but that removes `add_capability` only: this ' +
          'API is `pub` and not behind the feature, and `__allow_command` grants to ' +
          '`windows: ["*"]` — every window, recorded in no capability file and in nothing ' +
          'under gen/schemas/. If an item genuinely needs it, it says so in the same change ' +
          'and this rule goes with it',
      )
    }
  }

  return problems
}

/**
 * @typedef {object} Discrepancy
 * @property {string} name Fully qualified command path.
 * @property {boolean} defined Whether a `#[tauri::command]` defines it.
 * @property {boolean} registered Whether `generate_handler!` names it.
 * @property {boolean} inventoried Whether the committed inventory names it.
 * @property {boolean} granted Whether a capability makes it reachable. Always
 *   `false`, and meaningless, when `aclEnabled` is `false`.
 * @property {string} reason What that combination means and what to do about it.
 */

/**
 * @typedef {object} Reconciliation
 * @property {string[]} agreed Commands present in every list that applies, sorted.
 * @property {Discrepancy[]} discrepancies Everything else, sorted by name.
 */

/**
 * Compares the lists.
 *
 * Four of them when the app ships an ACL manifest, three when it does not — and
 * that is not leniency. Without a `permissions/` directory there are no app
 * permissions to name, no capability can grant an app-defined command, and
 * requiring a grant would be requiring something that cannot exist. The flag is
 * therefore taken from the tree (`resolveAppCommandGrants`) rather than assumed,
 * and it is the same condition `tauri_build` uses.
 *
 * Sorted output, so a diagnostic reads the same on two machines whose directory
 * listings came back in different orders.
 *
 * @param {object} sets
 * @param {readonly string[]} sets.defined
 * @param {readonly string[]} sets.registered
 * @param {readonly string[]} sets.inventoried
 * @param {readonly string[]} [sets.granted] Commands a capability makes
 *   reachable, named as the ACL names them — bare, with no module path. Empty,
 *   and unread, when `aclEnabled` is false.
 * @param {boolean} [sets.aclEnabled] Whether an app ACL manifest is shipped.
 * @returns {Reconciliation}
 */
export function reconcileCommands({
  defined,
  registered,
  inventoried,
  granted = [],
  aclEnabled = false,
}) {
  const definedSet = new Set(defined)
  const registeredSet = new Set(registered)
  const inventoriedSet = new Set(inventoried)
  const grantedSet = new Set(aclEnabled ? granted : [])

  const paths = new Set([...definedSet, ...registeredSet, ...inventoriedSet])
  // A grant for a command no list mentions has no path to be reported under, so
  // it is reported under the only name anyone wrote it down as: the bare one the
  // permission file uses.
  const orphanGrants = [...grantedSet].filter(
    (name) => ![...paths].some((path) => commandName(path) === name),
  )

  const names = [...new Set([...paths, ...orphanGrants])].sort()

  /** @type {string[]} */
  const agreed = []
  /** @type {Discrepancy[]} */
  const discrepancies = []

  for (const name of names) {
    const isDefined = definedSet.has(name)
    const isRegistered = registeredSet.has(name)
    const isInventoried = inventoriedSet.has(name)
    const isGranted = grantedSet.has(commandName(name))

    const key = `${isDefined ? 'D' : '-'}${isRegistered ? 'R' : '-'}${isInventoried ? 'I' : '-'}`

    /** @type {string[]} */
    const reasons = []
    // `---` reaches here only for an orphan grant, and the ACL clause below says
    // everything there is to say about one; the three code lists have no
    // complaint to make about a name none of them has ever seen.
    if (key !== 'DRI' && key !== '---') {
      // The fallback is unreachable — eight combinations, one agreed, one
      // deferred and six listed — but a missing key must not produce
      // `undefined` in a diagnostic.
      reasons.push(
        DISCREPANCY_REASONS[key] ?? `is inconsistent across the three lists (${key}).`,
      )
    }
    if (aclEnabled) {
      // A subject is re-attached when a clause is not the first thing said, so
      // both orders read as sentences rather than as two predicates run together.
      if (isRegistered && !isGranted) {
        reasons.push(
          reasons.length === 0 ? ACL_REASONS.ungranted : `It ${ACL_REASONS.ungranted}`,
        )
      }
      if (isGranted && !isRegistered) {
        reasons.push(reasons.length === 0 ? ACL_REASONS.stale : `It ${ACL_REASONS.stale}`)
      }
    }

    if (reasons.length === 0) {
      agreed.push(name)
      continue
    }

    discrepancies.push({
      name,
      defined: isDefined,
      registered: isRegistered,
      inventoried: isInventoried,
      granted: isGranted,
      reason: reasons.join(' '),
    })
  }

  // No `aclEnabled` in the result: the caller passed it in and already has it,
  // and adding it here would change the shape of a value three existing tests
  // compare whole. Whether the fourth column is worth printing is the caller's
  // question anyway.
  return { agreed, discrepancies }
}

/**
 * @typedef {object} InventorySummary
 * @property {number} exitCode 0 only when every list that applies names the same
 *   non-empty set and nothing failed to parse.
 * @property {number} checked How many commands are accounted for in all of them.
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

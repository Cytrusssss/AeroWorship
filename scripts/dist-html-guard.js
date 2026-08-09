// @ts-check

/**
 * Build-time guard for the production CSP's `style-src 'self'` (ADR-0015):
 * everything it decides, in pure functions. Four stages — choosing which
 * documents of a `dist/` listing have to be opened (`selectDocuments`),
 * scanning one for markup the CSP forbids (`findCspViolations`), asking whether
 * it loads anything at all (`hasModuleScript`), and turning the per-document
 * results into counts and an exit code (`summariseDocuments`).
 *
 * The CSP in `src-tauri/tauri.conf.json` is only applied to documents served
 * over `tauri://localhost`, so it is never enforced during `tauri dev`, and
 * `server.headers` in `vite.config.ts` cannot stand in for it either — the Vite
 * client injects `<style>` elements for CSS hot-update, so a dev-time
 * `style-src` header would have to be permissive to be usable. That leaves
 * exactly one place where a `style-src` regression is observable before a user
 * sees a blank projector: the built HTML. Hence this check, which
 * `npm run build` runs through the `postbuild` lifecycle and which therefore
 * also covers `npm run tauri build` (`beforeBuildCommand: "npm run build"`).
 *
 * Nothing here parses HTML properly on purpose. A tolerant scanner that
 * over-reports is the right shape for the CSP half: the three constructs
 * `findCspViolations` looks for are things the build must never emit at all, so
 * a false positive is a five-minute conversation while a false negative ships a
 * window that renders unstyled. `hasModuleScript` errs the other way round for
 * the same reason — a start tag it fails to recognise reads as "this document
 * loads nothing", which is the noisy answer, not the quiet one.
 *
 * Kept free of I/O so every decision this guard makes can be unit-tested;
 * `check-dist-html.js` is the thin script that reads files, prints and exits.
 * That split is not tidiness. A decision that lives only in the script is a
 * decision no test can reach, so deleting it leaves all six gate commands green
 * — the ADR-0020 failure class, which is what this guard exists to close.
 */

/**
 * Chooses which entries of a directory listing this guard has to open, and
 * reports which of the documents the build was supposed to emit are absent.
 *
 * A recursive `readdir` returns paths joined with the platform separator, so on
 * Windows a nested hit arrives as `pages\x.html`. Measured on Node 22.16:
 * `new URL()` does resolve that correctly, because WHATWG treats `\` as `/` for
 * special schemes and `file:` is one — so the caller's read would work either
 * way. Normalising is still not cosmetic. It is what makes the `expected`
 * membership test below separator-independent, and what keeps a name in a
 * diagnostic in the `dist/pages/x.html` form the reader will paste back into a
 * command.
 *
 * `expected` names are compared against whole relative paths, so a name without
 * a directory part is a claim that the document sits at the root of the scanned
 * directory. See `EXPECTED_DOCUMENTS` in `check-dist-html.js`.
 *
 * Note that a *directory* whose name ends in `.html` is selected too: the raw
 * listing is strings, and nothing here touches the file system to find out. The
 * caller is what turns the failed read into a named diagnostic.
 *
 * @param {readonly string[]} entries Raw result of `readdir(dir, { recursive: true })`.
 * @param {readonly string[]} expected Documents the build must have emitted.
 * @returns {{ documents: string[], missing: string[] }} Every `.html` entry,
 *   separator-normalised and sorted, and the `expected` names not among them.
 */
export function selectDocuments(entries, expected) {
  const documents = entries
    .map((entry) => entry.replaceAll('\\', '/'))
    .filter((name) => name.toLowerCase().endsWith('.html'))
    .sort()

  return {
    documents,
    missing: expected.filter((name) => !documents.includes(name)),
  }
}

/**
 * @typedef {'style-element' | 'style-attribute' | 'inline-script'} ViolationKind
 */

/**
 * @typedef {object} CspViolation
 * @property {ViolationKind} kind What was found.
 * @property {string} reason Why the production CSP would reject it.
 * @property {number} index Zero-based offset of the match in the source.
 * @property {number} line One-based line number, for a clickable message.
 * @property {number} column One-based column number.
 * @property {string} excerpt A short, single-line quote of the offending text.
 */

/** Longest excerpt echoed back; enough to identify, short enough to read. */
const EXCERPT_LENGTH = 72

/**
 * An inline `style` attribute, in all three value syntaxes HTML allows:
 * double-quoted, single-quoted and unquoted. Vite emits none of them today, but
 * that is a property of this version of Vite rather than one we control, and
 * `dist/*.html` will carry the output of whatever plugin is added later — a
 * guard that catches one form out of three is not a guard.
 *
 * The lookbehind is the load-bearing part: the attribute name has to stand on
 * its own, so `data-style="x"`, `xstyle="x"` and `-style="x"` are left alone.
 * Whitespace before the name is what separates attributes in a tag; `^` covers
 * a fragment that begins with the attribute itself. `<` is deliberately absent,
 * which is what keeps `<style>` from being counted twice.
 *
 * An unquoted value cannot contain whitespace, quotes, a backtick, `=`, `<` or
 * `>` (HTML tokenizer, "attribute value (unquoted) state"), and must be at
 * least one character long — so `style=` alone is not a match.
 */
const STYLE_ATTRIBUTE = /(?<=^|\s)style\s*=\s*(?:"[^"]*"|'[^']*'|[^\s"'`=<>]+)/gi

/**
 * Turns a byte offset into a one-based line/column pair.
 *
 * @param {string} source
 * @param {number} index
 * @returns {{ line: number, column: number }}
 */
function locate(source, index) {
  const before = source.slice(0, index)
  const line = before.split('\n').length
  const lastBreak = before.lastIndexOf('\n')
  return { line, column: index - lastBreak }
}

/**
 * Collapses whitespace and truncates, so a violation message stays one line
 * even when the offending construct is a 400-line inline script.
 *
 * @param {string} text
 * @returns {string}
 */
function excerpt(text) {
  const flattened = text.replace(/\s+/g, ' ').trim()
  return flattened.length > EXCERPT_LENGTH
    ? `${flattened.slice(0, EXCERPT_LENGTH)}…`
    : flattened
}

/**
 * Scans one HTML document for constructs the production CSP forbids.
 *
 * Three checks, all of them things Vite does not emit today:
 *
 * 1. `<style` — an inline stylesheet block; blocked by `style-src 'self'`.
 * 2. A `style` attribute in any of its three value syntaxes; also `style-src`,
 *    and unlike a `<style>` block it fails silently as "the layout looks
 *    wrong". See `STYLE_ATTRIBUTE`.
 * 3. A `<script>` element with a non-whitespace body — blocked by
 *    `script-src 'self'`. `<script type="module" src="…"></script>` has an
 *    empty body and passes; that is the shape Vite produces.
 *
 * `csp_hashes.styles` is declared by the Tauri runtime but never populated
 * (`tauri-codegen` only hashes `.js`/`.mjs`), so there is no automatic rescue
 * for the first two.
 *
 * @param {string} html Contents of a built HTML document.
 * @returns {CspViolation[]} Every violation found, ordered by position.
 */
export function findCspViolations(html) {
  /** @type {CspViolation[]} */
  const violations = []

  /**
   * @param {ViolationKind} kind
   * @param {string} reason
   * @param {number} index
   * @param {string} text
   */
  const record = (kind, reason, index, text) => {
    violations.push({
      kind,
      reason,
      index,
      ...locate(html, index),
      excerpt: excerpt(text),
    })
  }

  for (const match of html.matchAll(/<style\b[^>]*>?/gi)) {
    record(
      'style-element',
      "inline <style> block; blocked by style-src 'self'",
      match.index,
      match[0],
    )
  }

  for (const match of html.matchAll(STYLE_ATTRIBUTE)) {
    // The match spans name and value, which is what makes the excerpt legible;
    // long values are truncated by `excerpt`.
    record(
      'style-attribute',
      "inline style attribute; blocked by style-src 'self'",
      match.index,
      match[0],
    )
  }

  // A `<script>` whose body is empty or whitespace-only is a reference to an
  // external file and is fine. Anything else is executable text in the
  // document, which `script-src 'self'` refuses.
  for (const match of html.matchAll(/<script\b[^>]*>([\s\S]*?)<\/script\s*>/gi)) {
    // Group 1 always participates in a successful match; the fallback only
    // exists because `noUncheckedIndexedAccess` cannot know that.
    const body = match[1] ?? ''
    if (body.trim() === '') continue
    record(
      'inline-script',
      "inline <script> body; blocked by script-src 'self'",
      match.index,
      body,
    )
  }

  return violations.sort((a, b) => a.index - b.index)
}

/**
 * Renders one violation as a single `file:line:col` diagnostic line.
 *
 * @param {string} file Label for the document — usually its path.
 * @param {CspViolation} violation
 * @returns {string}
 */
export function formatViolation(file, violation) {
  return `${file}:${violation.line}:${violation.column}  ${violation.kind}  ${violation.reason}\n    ${violation.excerpt}`
}

/**
 * A `<script …>` start tag and everything between its name and the `>`. Void of
 * any attempt to handle a `>` inside an attribute value, which the HTML
 * tokenizer allows: a start tag written that way would be cut short here and
 * the scan would move on. Vite emits nothing of the sort, and the direction of
 * the error is the safe one — a missed script reads as "this document loads
 * nothing", not as "this document is fine".
 */
const SCRIPT_START_TAG = /<script\b([^>]*)>/gi

/**
 * One attribute of a start tag, in all three value syntaxes plus the valueless
 * form. Group 1 is the name; groups 2–4 are the double-quoted, single-quoted
 * and unquoted values.
 */
const TAG_ATTRIBUTE =
  /(?<=^|\s)([^\s"'>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'`=<>]+)))?/g

/**
 * Whether a built document loads at least one external ES module — that is, a
 * `<script>` start tag carrying both `type="module"` and a non-empty `src`, in
 * any attribute order and any case. The shape `vite build` emits today is
 * `<script type="module" crossorigin src="/assets/main-<hash>.js"></script>`,
 * read off `dist/index.html` and `dist/output.html` rather than recalled.
 *
 * This is the one assertion in this file that is positive rather than a denial.
 * Everything else here answers "does the document contain something forbidden",
 * and a zero-byte `dist/output.html` answers no to all of it — so without this,
 * a build that emitted an empty document counted as clean, and the first
 * symptom would have been a blank projector during a service.
 *
 * What it does not check: that the `src` resolves to a file that exists, that
 * the file parses, or that the module is the right one. All it separates is
 * "this document names an entry point" from "this document names none" — a
 * document in the second group can still be full of markup.
 *
 * @param {string} html Contents of a built HTML document.
 * @returns {boolean}
 */
export function hasModuleScript(html) {
  for (const tag of html.matchAll(SCRIPT_START_TAG)) {
    /** @type {Map<string, string>} */
    const attributes = new Map()
    for (const attribute of (tag[1] ?? '').matchAll(TAG_ATTRIBUTE)) {
      // First occurrence wins, which is what the HTML parser does with a
      // duplicated attribute name.
      const name = (attribute[1] ?? '').toLowerCase()
      if (!attributes.has(name)) {
        attributes.set(name, attribute[2] ?? attribute[3] ?? attribute[4] ?? '')
      }
    }

    if (attributes.get('type')?.trim().toLowerCase() !== 'module') continue
    if ((attributes.get('src') ?? '').trim() !== '') return true
  }

  return false
}

/**
 * @typedef {object} DocumentReport
 * @property {string} name Path of the document, relative to the scanned directory.
 * @property {readonly CspViolation[] | null} violations What `findCspViolations`
 *   returned, or `null` when the document could not be opened at all.
 * @property {boolean} loadsModule What `hasModuleScript` returned. Only read
 *   when `violations` is not `null`.
 */

/**
 * Folds the per-document results into the three counts the caller prints and
 * the exit code it returns.
 *
 * This lives here rather than in `check-dist-html.js` because when the guard is
 * allowed to return 0 is the single most consequential line in it, and a line
 * that only exists inside a script no test can import is a line nothing
 * defends — the ADR-0020 shape, in the guard written to close it.
 *
 * An unreadable document contributes to neither of the other counts: nothing
 * was read, so nothing is known about its markup or its scripts, and counting
 * it as blank would send the reader looking for an empty file that may be fine.
 *
 * An empty `reports` is a non-zero exit with all three counts at zero. Nothing
 * was examined, and "nothing was examined" is the one answer this guard must
 * never dress up as "clean" — its caller checks for that case first and prints
 * a better diagnostic, but the caller losing that check is exactly the kind of
 * edit this function exists to survive.
 *
 * @param {readonly DocumentReport[]} reports One entry per document opened or attempted.
 * @returns {{ violations: number, unreadable: number, blank: number, exitCode: number }}
 *   `violations` totals hits across documents; `unreadable` and `blank` count
 *   documents. A document can be counted both as carrying violations and as
 *   blank — an inline `<script>` body is a violation and does not load a module.
 */
export function summariseDocuments(reports) {
  let violations = 0
  let unreadable = 0
  let blank = 0

  for (const report of reports) {
    if (report.violations === null) {
      unreadable += 1
      continue
    }
    violations += report.violations.length
    if (!report.loadsModule) blank += 1
  }

  const clean = reports.length > 0 && violations === 0 && unreadable === 0 && blank === 0

  return {
    violations,
    unreadable,
    blank,
    exitCode: clean ? 0 : 1,
  }
}

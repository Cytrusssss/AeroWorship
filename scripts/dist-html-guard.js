// @ts-check

/**
 * Build-time guard for the production CSP's `style-src 'self'` (ADR-0015).
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
 * over-reports is the right shape for a guard: the three constructs below are
 * things the build must never emit at all, so a false positive is a five-minute
 * conversation while a false negative ships a window that renders unstyled.
 *
 * Kept free of I/O so it can be unit-tested; `check-dist-html.js` is the thin
 * script that reads files and exits.
 */

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

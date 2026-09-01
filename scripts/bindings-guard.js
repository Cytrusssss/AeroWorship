// @ts-check

/**
 * Anti-drift guard for the generated Rust→TypeScript contract (NFR-33):
 * everything it decides, in pure functions.
 *
 * The problem it exists for. `ts-rs` writes `src/shared/bindings/*.ts` from the
 * Rust types, and those files are committed so the frontend can import them
 * without a Rust toolchain in the loop. A committed generated file is a copy,
 * and a copy that nothing compares is a copy that rots: change a field in Rust,
 * forget to regenerate, and every gate in this repository stays green while the
 * frontend type-checks against a contract the backend no longer speaks. That is
 * precisely the silent-pass class ADR-0020 and ADR-0021 were written about.
 *
 * The shape of the answer. `check-bindings.js` re-runs the generator into a
 * throwaway directory and hands both sides here; this module decides whether
 * they agree. Comparison is byte-for-byte over the file contents, because the
 * question is not "are these two contracts compatible" — it is "is the
 * committed file the file the generator would produce right now". Anything
 * short of equality has to be a failure, or the guard starts having opinions
 * about which differences matter and stops being mechanical.
 *
 * Kept free of I/O so every decision can be unit-tested without a Rust
 * toolchain or a temporary directory; `check-bindings.js` is the thin script
 * that spawns cargo, reads files, prints and exits. That split is the same one
 * `dist-html-guard.js` makes and for the same reason: a decision that lives
 * only in the script is a decision no test can reach.
 */

/**
 * @typedef {object} BindingsComparison
 * @property {string[]} missing Files the generator produced that are not
 *   committed — a new or renamed type whose binding was never checked in.
 * @property {string[]} unexpected Files that are committed but the generator
 *   does not produce — a deleted type's leftover binding, or a hand-written
 *   file in a directory PRD §6.13 marks "do not edit".
 * @property {string[]} differing Files present on both sides whose contents are
 *   not identical — the ordinary drift case, a Rust type edited without
 *   regenerating.
 * @property {string[]} matching Files present on both sides and identical.
 */

/**
 * Compares a freshly generated set of bindings against the committed set.
 *
 * Both sides are keyed by path relative to the bindings directory, so a
 * `#[ts(export_to = "session/Slide.ts")]` compares against
 * `src/shared/bindings/session/Slide.ts` and not against a bare `Slide.ts`.
 * The caller is responsible for normalising separators; see `collectBindings`
 * in `check-bindings.js`.
 *
 * Every result list is sorted, so a diagnostic reads the same on two machines
 * whose directory listings came back in different orders.
 *
 * Note what is *not* here: no allowance for a file the generator "might not"
 * emit, and no notion of an ignorable difference. Both would have to be a list
 * of names, and a list of names in a guard is a list of things it no longer
 * checks.
 *
 * @param {ReadonlyMap<string, string>} generated Freshly generated contents.
 * @param {ReadonlyMap<string, string>} committed Contents currently in the tree.
 * @returns {BindingsComparison}
 */
export function compareBindings(generated, committed) {
  /** @type {string[]} */
  const missing = []
  /** @type {string[]} */
  const differing = []
  /** @type {string[]} */
  const matching = []

  for (const [name, content] of generated) {
    const other = committed.get(name)
    if (other === undefined) {
      missing.push(name)
    } else if (other === content) {
      matching.push(name)
    } else {
      differing.push(name)
    }
  }

  const unexpected = [...committed.keys()].filter((name) => !generated.has(name))

  return {
    missing: missing.sort(),
    unexpected: unexpected.sort(),
    differing: differing.sort(),
    matching: matching.sort(),
  }
}

/**
 * @typedef {object} LineDifference
 * @property {number} line 1-based number of the first line that differs — or,
 *   when the two differ in nothing but their line endings, one past the last
 *   line they share, which names no line in either file (see
 *   `describeFirstDifference`).
 * @property {string | null} generated That line as generated, or `null` if the
 *   generated file has no such line (it is the shorter of the two).
 * @property {string | null} committed That line as committed, or `null` if the
 *   committed file has no such line.
 */

/**
 * Locates the first line at which two versions of a binding diverge.
 *
 * This is for the diagnostic, not for the verdict — `compareBindings` has
 * already decided. Printing whole files would bury the one line that matters
 * under thirty identical ones, and printing nothing leaves the reader running
 * the generator by hand to find out what changed.
 *
 * Split on `\n` with a trailing `\r` stripped, so a file that arrived with CRLF
 * is reported by content rather than by an invisible difference on every line.
 * That is presentation only: `compareBindings` compares raw bytes and a
 * line-ending change is still drift. It should not arise here — `.gitattributes`
 * pins the working tree to LF — but if it ever does, the reader deserves a
 * message that says so instead of a diff where both lines look the same.
 *
 * When the line endings are the *only* difference, the strip leaves the two
 * line arrays identical, so there is no line to point at: the fallback below
 * returns `null` on both sides, with a `line` one past the last line either
 * file has. That pair of `null`s is the signature of "identical line for line,
 * different bytes", and it is what `check-bindings.js` recognises to print the
 * line-ending message the paragraph above promises — rather than
 * `first difference at line N` quoting `(no such line)` twice, which is what
 * naming a line here would come to.
 *
 * @param {string} generated
 * @param {string} committed
 * @returns {LineDifference | null} `null` when the two are identical.
 */
export function describeFirstDifference(generated, committed) {
  if (generated === committed) return null

  const left = generated.split('\n').map(stripCarriageReturn)
  const right = committed.split('\n').map(stripCarriageReturn)

  const shared = Math.min(left.length, right.length)
  for (let i = 0; i < shared; i += 1) {
    if (left[i] !== right[i]) {
      return { line: i + 1, generated: left[i] ?? null, committed: right[i] ?? null }
    }
  }

  // One file is a prefix of the other: the first difference is the first line
  // only the longer one has. Reached when a field or a whole type was added or
  // removed at the end of the file.
  const line = shared + 1
  return {
    line,
    generated: left[shared] ?? null,
    committed: right[shared] ?? null,
  }
}

/**
 * @param {string} line
 * @returns {string}
 */
function stripCarriageReturn(line) {
  return line.endsWith('\r') ? line.slice(0, -1) : line
}

/**
 * @typedef {object} BindingsSummary
 * @property {number} exitCode 0 only when the committed bindings are exactly
 *   what the generator produces, and there is at least one of them.
 * @property {number} inspected How many files the generator produced.
 * @property {boolean} generatedNothing Whether the generator emitted no files
 *   at all, which is a failure of this guard's premise rather than a clean run.
 */

/**
 * Turns a comparison into counts and an exit code.
 *
 * The clause that earns its place is `generatedNothing`. Every other check here
 * asks whether the two sides disagree, and two empty sides agree perfectly — so
 * deleting `#[ts(export)]` from the last exported type, or filtering the cargo
 * invocation down to a test name that no longer exists, would leave this guard
 * printing "in sync" about a contract nobody generated. `cargo test <filter>`
 * exits 0 when the filter matches nothing, so that is not a hypothetical: it is
 * the most likely way this whole mechanism gets switched off without anyone
 * noticing. An empty run is therefore a failure.
 *
 * @param {BindingsComparison} comparison
 * @returns {BindingsSummary}
 */
export function summariseComparison(comparison) {
  const inspected =
    comparison.matching.length + comparison.missing.length + comparison.differing.length

  const generatedNothing = inspected === 0

  const drifted =
    comparison.missing.length > 0 ||
    comparison.unexpected.length > 0 ||
    comparison.differing.length > 0

  return {
    exitCode: generatedNothing || drifted ? 1 : 0,
    inspected,
    generatedNothing,
  }
}

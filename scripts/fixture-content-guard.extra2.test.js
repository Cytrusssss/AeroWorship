// @ts-check

/**
 * Further pure-function coverage for `fixture-content-guard.js`, written by
 * `tester` for SETUP-05 siklus 2, on top of the implementer's own
 * `fixture-content-guard.test.js` (which siklus 2 extended with
 * `parseLsFilesEntry` / `isGitlinkMode` / `gitlinkVerdict` / the `unsafe`
 * bucket) and `fixture-content-guard.extra.test.js` (siklus 1, `tester`).
 *
 * Three things earned their own file rather than an edit to either of those:
 *
 *   - `parseLsFilesEntry` against index-mode and stage shapes the
 *     implementer's own cases do not exercise: `100755` (executable) and
 *     `120000` (symlink) full parses (not just `isGitlinkMode`), a path
 *     carrying an embedded tab or space, and — the case the coordinator brief
 *     explicitly asked to be verified rather than assumed — the three-stage
 *     record shape `git ls-files -z -s` emits for an unresolved merge
 *     conflict (stage 1/2/3, no stage 0). The implementer states this shape
 *     is not specially handled; this file pins down what the existing code
 *     actually does with it (a duplicate-path listing that
 *     `identifyCandidates` still fails closed on when the path's content
 *     cannot be confirmed at stage 0 — see the `identifyCandidates` block
 *     below), rather than trusting that statement.
 *   - `breaksBatchLineProtocol`'s negative space: control bytes the doc
 *     comment says do *not* need screening, confirmed one at a time so a
 *     future edit that narrows or widens the byte set is a visible diff.
 *   - `classifyIndexedPaths` bucket precedence, independently re-derived
 *     rather than trusted from the implementer's own test of the same claim
 *     (`fixture-content-guard.test.js`, "sends an unsafe path to unsafe even
 *     when it also matches an allowlisted prefix") — see
 *     `tests/unit/check-fixture-content.e2e.test.js` for the same claim
 *     proven a third way, against the real CLI.
 */

import { describe, expect, it } from 'vitest'

import {
  ALLOWLISTED_PREFIXES,
  breaksBatchLineProtocol,
  classifyIndexedPaths,
  identifyCandidates,
  isAllowlisted,
  isGitlinkMode,
  parseLsFilesEntry,
  summariseRun,
} from './fixture-content-guard.js'

describe('parseLsFilesEntry — mode variety', () => {
  it('parses an executable blob (100755)', () => {
    expect(
      parseLsFilesEntry(`100755 ${'c'.repeat(40)} 0\ttests/integration/fixtures/run.sh`),
    ).toEqual({ mode: '100755', path: 'tests/integration/fixtures/run.sh' })
  })

  it('parses a symlink (120000)', () => {
    expect(
      parseLsFilesEntry(
        `120000 ${'d'.repeat(40)} 0\ttests/integration/fixtures/link-to-real`,
      ),
    ).toEqual({ mode: '120000', path: 'tests/integration/fixtures/link-to-real' })
  })

  it('does not classify an executable blob or a symlink as a gitlink', () => {
    expect(isGitlinkMode('100755')).toBe(false)
    expect(isGitlinkMode('120000')).toBe(false)
  })
})

describe('parseLsFilesEntry — unusual but legal path bytes', () => {
  it('preserves an embedded tab as part of the path, not as a second field separator', () => {
    // Only the *first* tab after the stage digits is the record's structural
    // separator; a tab appearing later belongs to the path.
    const entry = parseLsFilesEntry(`100644 ${'a'.repeat(40)} 0\tweird\tpath.aero`)
    expect(entry).toEqual({ mode: '100644', path: 'weird\tpath.aero' })
  })

  it('preserves spaces in the path', () => {
    const entry = parseLsFilesEntry(
      `100644 ${'a'.repeat(40)} 0\ttests/integration/fixtures/my service copy.aero`,
    )
    expect(entry).toEqual({
      mode: '100644',
      path: 'tests/integration/fixtures/my service copy.aero',
    })
  })
})

describe('parseLsFilesEntry — merge-conflict stages (1/2/3, no stage 0)', () => {
  // `git ls-files -z -s` emits one record per stage for an unresolved
  // conflict, all sharing the same path. The coordinator brief asked this
  // to be verified rather than assumed "handled" or "not handled" — this is
  // the verification of the parser's own behaviour: it parses every stage
  // record structurally the same way an ordinary stage-0 record parses,
  // dropping the stage number (the return type carries only `mode`/`path`).
  // What `check-fixture-content.js` does with three identical *paths* in a
  // row as a result is covered separately below, at `identifyCandidates`.
  it('parses each stage of a conflicted entry, extracting mode and path identically', () => {
    const path = 'tests/integration/fixtures/conflict-case/disputed.aero'
    const stage1 = parseLsFilesEntry(`100644 ${'1'.repeat(40)} 1\t${path}`)
    const stage2 = parseLsFilesEntry(`100644 ${'2'.repeat(40)} 2\t${path}`)
    const stage3 = parseLsFilesEntry(`100644 ${'3'.repeat(40)} 3\t${path}`)
    expect(stage1).toEqual({ mode: '100644', path })
    expect(stage2).toEqual({ mode: '100644', path })
    expect(stage3).toEqual({ mode: '100644', path })
  })
})

describe('identifyCandidates — a path listed more than once with unresolved (missing) content', () => {
  // Simulates, at the pure-function boundary, what `check-fixture-content.js`
  // hands `identifyCandidates` for a path git can only place at stage 1/2/3:
  // `git cat-file --batch -c` on `:path` resolves stage 0, which does not
  // exist for a conflicted path, so every query for it comes back `missing`
  // — and because `classifyIndexedPaths` never deduplicates, the path
  // appears in `paths` once per stage record.
  it('fails closed (unreadable) for a duplicated, declared-extension path whose content is missing at every occurrence', () => {
    const path = 'tests/integration/fixtures/conflict-case/disputed.aero'
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([[path, { status: 'missing' }]])
    const { candidates, unreadable } = identifyCandidates(
      [path, path, path],
      contentByPath,
    )
    // Every occurrence is still flagged — none of the three is silently
    // dropped just because its neighbours already reported the same path.
    expect(candidates).toEqual([
      { path, reason: 'extension' },
      { path, reason: 'extension' },
      { path, reason: 'extension' },
    ])
    expect(unreadable).toEqual([path, path, path])
  })

  it('W-new-1 CLOSED: an UNDECLARED (non-extension) path in the same missing-content shape is never silent anymore', () => {
    // This test used to be named "is a real, named gap ... — documented, not
    // fixed" and asserted `unreadable` was `[]` — i.e. it locked in the bug
    // (auditor W-new-1, found independently by this file's own author too):
    // a path with no declared extension whose content could not be confirmed
    // present was folded into *neither* `candidates` nor `unreadable`, so it
    // vanished from the run's accounting entirely. `identifyCandidates` now
    // pushes every such path into `unreadable` unconditionally — see its own
    // doc comment in `fixture-content-guard.js` — so this test is the canary
    // that was supposed to start failing the moment that landed, and it did.
    //
    // What is asserted now is not merely "unreadable is non-empty" but the
    // specific shape the fix promises: still not a `candidate` (this
    // function has no bytes to sniff for a `missing` entry, so claiming the
    // path *is* media would be the same overclaim ADR-0020/0021 already
    // showed expensive — see `identifyCandidates`'s doc, "no basis to claim
    // the path *is* media requiring a manifest declaration"), but present in
    // `unreadable` for every occurrence, exactly mirroring the declared-
    // extension case in the test above. `summariseRun` is what turns that
    // into a hard failure — proven separately, immediately below, without
    // any help from `candidates` — so a caller cannot read "empty
    // candidates" as "nothing to worry about" here.
    //
    // Still not exercised end-to-end (an active, uncommitted merge conflict
    // is not a state `pretest` is realistically run against — see
    // PROGRESS.md SETUP-05 tester report), but the function-level behaviour
    // is real and is what this test pins.
    const path = 'tests/integration/fixtures/conflict-case/disputed.bin'
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([[path, { status: 'missing' }]])
    const { candidates, unreadable } = identifyCandidates(
      [path, path, path],
      contentByPath,
    )
    expect(candidates).toEqual([])
    expect(unreadable).toEqual([path, path, path])
  })

  it('W-new-1 CLOSED, verified at the summariseRun boundary too: unreadable alone fails the run, with zero candidateVerdicts to lean on', () => {
    // The doc comment on `identifyCandidates` claims `summariseRun` "fails
    // the run on any non-empty `unreadable`, independent of `candidates`".
    // The implementer's own `summariseRun` describe block in
    // `fixture-content-guard.test.js` ("fails on an unreadable candidate
    // even when every checked verdict passed") always pairs `unreadable`
    // with a non-empty, passing `candidateVerdicts` array — so it proves
    // "unreadable fails even when candidates also passed", not "unreadable
    // fails on its own, unaided". This closes that gap: `candidateVerdicts`
    // is `[]` here, exactly the shape `identifyCandidates` now produces for
    // the case above (an undeclared, unreadable path contributes nothing to
    // `candidates`, hence nothing to `candidateVerdicts`), so this is the
    // real caller shape, not a hypothetical one.
    const summary = summariseRun({
      indexedCount: 3,
      allowlistedCount: 0,
      candidateVerdicts: [],
      unreadable: ['tests/integration/fixtures/conflict-case/disputed.bin'],
    })
    expect(summary.exitCode).toBe(1)
    expect(summary.unreadableCount).toBe(1)
    expect(summary.violationCount).toBe(0)
  })
})

describe('parseLsFilesEntry — malformed records', () => {
  it('rejects a record missing the structural tab entirely', () => {
    expect(parseLsFilesEntry(`100644 ${'a'.repeat(40)} 0 no-tab-here.aero`)).toBeNull()
  })

  it('rejects a record with a non-octal mode digit', () => {
    expect(parseLsFilesEntry(`999999 ${'a'.repeat(40)} 0\tfixtures/x.aero`)).toBeNull()
  })

  it('rejects a record with a sha shorter than 4 hex characters', () => {
    expect(parseLsFilesEntry(`100644 abc 0\tfixtures/x.aero`)).toBeNull()
  })

  it('rejects an empty string', () => {
    expect(parseLsFilesEntry('')).toBeNull()
  })
})

describe('breaksBatchLineProtocol — the negative space (bytes deliberately NOT screened)', () => {
  it('does not flag a horizontal tab', () => {
    expect(breaksBatchLineProtocol('a\tb.aero')).toBe(false)
  })

  it('does not flag a vertical tab', () => {
    expect(breaksBatchLineProtocol('a\vb.aero')).toBe(false)
  })

  it('does not flag a form feed', () => {
    expect(breaksBatchLineProtocol('a\fb.aero')).toBe(false)
  })

  it('does not flag a NUL byte', () => {
    // Not reachable via a real `git ls-files -z` path (NUL is the record
    // separator, so git itself can never hand this function a path
    // containing one) — asserted anyway so the function's own stated
    // contract ("only \n and \r break the wire format this guards")
    // matches its code, independent of what git happens to make reachable.
    expect(breaksBatchLineProtocol('a\0b.aero')).toBe(false)
  })

  it('flags \\r\\n as a single carrier of both forbidden bytes, at any position', () => {
    expect(breaksBatchLineProtocol('\r\nleading.aero')).toBe(true)
    expect(breaksBatchLineProtocol('trailing.aero\r\n')).toBe(true)
    expect(breaksBatchLineProtocol('mid\r\ndle.aero')).toBe(true)
  })
})

describe('classifyIndexedPaths — bucket precedence, independently re-derived', () => {
  it('unsafe is decided before the allowlist check ever runs, for a path that would otherwise match it', () => {
    const allowlistedUnsafe = `${ALLOWLISTED_PREFIXES[0]}payload\n.ico`
    expect(isAllowlisted(allowlistedUnsafe)).toBe(true) // prefix matches, on its own
    expect(breaksBatchLineProtocol(allowlistedUnsafe)).toBe(true) // and so does unsafe

    const { allowlisted, toInspect, unsafe } = classifyIndexedPaths([allowlistedUnsafe])
    expect(unsafe).toEqual([allowlistedUnsafe])
    expect(allowlisted).toEqual([])
    expect(toInspect).toEqual([])
  })

  it('an ordinary allowlisted path (no unsafe byte) is still skipped, unaffected by the check above', () => {
    const ordinary = `${ALLOWLISTED_PREFIXES[0]}icon.png`
    const { allowlisted, toInspect, unsafe } = classifyIndexedPaths([ordinary])
    expect(allowlisted).toEqual([ordinary])
    expect(toInspect).toEqual([])
    expect(unsafe).toEqual([])
  })
})

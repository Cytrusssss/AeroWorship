// @ts-check

/**
 * Coverage for the functions siklus 4 added to `fixture-content-guard.js`,
 * which landed with none: `looksLikeAeroSession` (ADR-0030), `isUnmergedStage`
 * and `unmergedPathVerdicts` (the merge-conflict split), and
 * `manifestPathsForCandidates` (the allowlisted-manifest fetch). The
 * `{ status: 'unparsed' }` branch of `parseCatFileBatch` is the fourth thing
 * that arrived untested; it is covered in `fixture-content-guard.test.js`
 * instead, next to the throw it replaced, so the two can be read against each
 * other.
 *
 * `looksLikeAeroSession` gets each of its three filters exercised on its own
 * rather than only end-to-end. The function's whole design argument is that the
 * layers are independently load-bearing — filter 1 is what keeps `docs/PRD.md`
 * (which contains the literal `aeroworship.session` twice) from becoming a
 * candidate and turning `npm test` red on a clean checkout, filter 3 is what
 * keeps every other JSON file in the repository out — so a test that only ever
 * fed it whole documents would pass identically if two of the three were
 * deleted.
 *
 * Two behaviours below were pinned as *observed*, not endorsed: a BOM-prefixed
 * session document and a `.`-escaped `kind` both came back `false`. They ended
 * differently, and both endings are asserted here rather than described.
 *
 *   - The BOM one was a defect and is fixed: filter 3 now decodes from the same
 *     offset filter 1 skips the BOM at, so a BOM-prefixed session document is
 *     detected. The test below asserts that *and* its neighbour — a BOM that
 *     follows whitespace is still not a session document — because a fix that
 *     widened past filter 1's own rule would pass the first assertion alone.
 *   - The escaped `kind` is not a defect to fix but a gap to know about, and
 *     `looksLikeAeroSession`'s doc now names it with the same weight as the
 *     encrypted/compressed one. The test still pins the behaviour, because an
 *     unpinned gap is one nobody can see change.
 */

import { readFileSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

import {
  AERO_SESSION_KIND,
  evaluateCandidate,
  findFixturesRoot,
  hasDeclaredExtension,
  identifyCandidates,
  isUnmergedStage,
  looksLikeAeroSession,
  manifestPathFor,
  manifestPathsForCandidates,
  summariseRun,
  unmergedPathVerdicts,
} from './fixture-content-guard.js'

const ROOT = resolve(import.meta.dirname, '..')

/**
 * A minimal Appendix C session document. Synthetic throughout — field values
 * are placeholders invented for this file, never real order-of-service content.
 *
 * @param {Record<string, unknown>} [extra]
 * @returns {string}
 */
function sessionJson(extra) {
  return JSON.stringify({
    kind: AERO_SESSION_KIND,
    schema_version: 1,
    title: 'SYNTHETIC — invented for fixture-content-guard.extra3.test.js only',
    ...extra,
  })
}

describe('looksLikeAeroSession — filter 1: first non-whitespace byte must be `{`', () => {
  it('rejects a markdown document that contains the literal twice — the docs/PRD.md shape', () => {
    // Stated in `looksLikeAeroSession`'s own doc as the case that would
    // otherwise "turn `npm test` red on a clean checkout": the PRD is tracked,
    // is not under any fixtures/ directory, and mentions the sentinel in
    // Appendix C's schema and again in its worked example.
    const markdown = Buffer.from(
      `# AeroWorship PRD\n\nAppendix C: every document carries \`kind: "${AERO_SESSION_KIND}"\`.\n\n` +
        `C.3 worked example:\n\n    { "kind": "${AERO_SESSION_KIND}" }\n`,
      'utf8',
    )
    expect(markdown.includes(AERO_SESSION_KIND)).toBe(true) // filter 2 would pass
    expect(looksLikeAeroSession(markdown)).toBe(false) // filter 1 stops it first
  })

  it('rejects the real docs/PRD.md, which really does carry the sentinel', () => {
    // The claim above, against the actual file rather than a reconstruction of
    // it. The first assertion is what keeps this test from going vacuous if
    // the PRD is ever reworded: without it, a PRD that no longer mentions the
    // sentinel would still "pass" while proving nothing.
    const prd = readFileSync(join(ROOT, 'docs/PRD.md'))
    expect(prd.includes(AERO_SESSION_KIND)).toBe(true)
    expect(looksLikeAeroSession(prd)).toBe(false)
  })

  it('rejects a JS source file whose first byte is not `{`', () => {
    expect(
      looksLikeAeroSession(
        Buffer.from(`// kind: ${AERO_SESSION_KIND}\nexport const x = 1\n`, 'utf8'),
      ),
    ).toBe(false)
  })

  it('rejects a top-level JSON array even when it holds a session object', () => {
    const array = Buffer.from(JSON.stringify([JSON.parse(sessionJson())]), 'utf8')
    expect(array[0]).toBe(0x5b /* [ */)
    expect(looksLikeAeroSession(array)).toBe(false)
  })

  it('skips leading whitespace of every kind the filter names (space, tab, LF, CR)', () => {
    for (const lead of [' ', '\t', '\n', '\r', ' \r\n\t  ', '\n\n\n']) {
      expect(looksLikeAeroSession(Buffer.from(lead + sessionJson(), 'utf8')), lead).toBe(
        true,
      )
    }
  })

  it('does not skip a leading byte that is merely blank-looking but not in the named set', () => {
    // A vertical tab and a form feed are whitespace to a human and to some
    // parsers, but not to JSON and not to this filter. Pinned so the byte set
    // cannot quietly widen: each of these is also invalid JSON at the top
    // level, so accepting them here would only move the rejection later.
    expect(looksLikeAeroSession(Buffer.from(`\v${sessionJson()}`, 'utf8'))).toBe(false)
    expect(looksLikeAeroSession(Buffer.from(`\f${sessionJson()}`, 'utf8'))).toBe(false)
  })

  it('does not throw on an empty buffer, a whitespace-only buffer, or a bare BOM', () => {
    expect(looksLikeAeroSession(Buffer.alloc(0))).toBe(false)
    expect(looksLikeAeroSession(Buffer.from('   \n\t  ', 'utf8'))).toBe(false)
    expect(looksLikeAeroSession(Buffer.from([0xef, 0xbb, 0xbf]))).toBe(false)
  })

  it('does not throw on binary bytes that are not valid UTF-8 at all', () => {
    expect(looksLikeAeroSession(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0xff, 0xfe]))).toBe(
      false,
    )
    // Starts with `{`, so it reaches past filter 1 and is stopped by 2 or 3.
    expect(looksLikeAeroSession(Buffer.from([0x7b, 0xff, 0xfe, 0xfd, 0x00]))).toBe(false)
  })

  it('detects a BOM-prefixed session document — all three filters agree about the BOM', () => {
    // Filter 1 advances past a UTF-8 BOM ("first non-whitespace byte (after an
    // optional UTF-8 BOM) is `{`"), so a document written by an editor that
    // emits one — Windows Notepad among them, on the OS this product targets —
    // clears filters 1 and 2. Filter 3 used to lose it again: it decoded from
    // byte 0, so the BOM survived as a leading U+FEFF, `JSON.parse` refused it
    // and the `catch` returned `false` — cancelling filter 1's own allowance,
    // and letting a real renamed `.aero` saved with a BOM through the guard.
    // Filter 3 now decodes from the same offset filter 1 started at.
    const withBom = Buffer.concat([
      Buffer.from([0xef, 0xbb, 0xbf]),
      Buffer.from(sessionJson(), 'utf8'),
    ])
    expect(withBom[3]).toBe(0x7b) // filter 1 is satisfied
    expect(withBom.includes(AERO_SESSION_KIND)).toBe(true) // filter 2 is satisfied
    expect(looksLikeAeroSession(withBom)).toBe(true) // …and filter 3 now agrees
  })

  it('does NOT accept a BOM that follows whitespace — the same rule filter 1 applies', () => {
    // The other half of the pair, and the one that proves the fix did not
    // widen past what filter 1 allows. Filter 1's allowance is for a BOM at
    // byte 0 only: it looks for the BOM *before* skipping whitespace, so a
    // space followed by a BOM stops at the 0xEF byte and never reaches `{`.
    // A fix that
    // stripped a BOM wherever it appeared would make the three filters
    // disagree again, in the opposite direction — and `JSON.parse` rejects
    // that document anyway, so accepting it here would only move the failure.
    const bomAfterSpace = Buffer.concat([
      Buffer.from(' ', 'utf8'),
      Buffer.from([0xef, 0xbb, 0xbf]),
      Buffer.from(sessionJson(), 'utf8'),
    ])
    expect(bomAfterSpace.includes(AERO_SESSION_KIND)).toBe(true) // filter 2 would pass
    expect(looksLikeAeroSession(bomAfterSpace)).toBe(false) // filter 1 stops it
  })
})

describe('looksLikeAeroSession — filter 2: the literal must appear in the raw bytes', () => {
  it('rejects ordinary JSON config files that clear filter 1', () => {
    for (const json of [
      '{"name":"aeroworship","version":"0.1.0"}',
      '{"compilerOptions":{"strict":true}}',
      '{"entries":{"sample.aero":{"synthetic":true,"purpose":"x".repeat}}}',
    ]) {
      expect(looksLikeAeroSession(Buffer.from(json, 'utf8')), json).toBe(false)
    }
  })

  it('rejects a real fixtures.manifest.json, which is JSON about fixtures but is not one', () => {
    const manifest = JSON.stringify({
      entries: {
        'service.aero': {
          synthetic: true,
          purpose: 'Synthetic fixture authored for this test, no real service content.',
        },
      },
    })
    expect(looksLikeAeroSession(Buffer.from(manifest, 'utf8'))).toBe(false)
  })

  it('ACCEPTED GAP: a `\\u002e`-escaped kind parses to the right value but is rejected, because filter 2 searches raw bytes', () => {
    // This is filter 2 acting *independently* of filter 3 — the one input
    // where the two disagree, and therefore the only way to prove filter 2 is
    // load-bearing rather than a pure optimisation. `JSON.parse` yields
    // `kind === 'aeroworship.session'`, so filter 3 alone would return `true`;
    // the raw bytes never contain the literal, so filter 2 returns `false`
    // first. A `.aero` rewritten with escaped punctuation is therefore not
    // detected. That is now named in `looksLikeAeroSession`'s own doc as the
    // second known and accepted gap, alongside the encrypted/compressed one,
    // and kept deliberately: filter 2 is what stops this function parsing
    // every JSON file in the index on every run, and nothing AeroWorship ships
    // emits an escaped `kind`. Pinned rather than reported — if the trade is
    // ever revisited, this test is what says so out loud.
    // The backslash is built from its code point so the intent survives every
    // layer of quoting between here and the bytes: the document on the wire is
    // `{"kind":"aeroworship.session"}`, six characters where a `.` would
    // otherwise be.
    const backslash = String.fromCharCode(0x5c)
    const escaped = Buffer.from(`{"kind":"aeroworship${backslash}u002esession"}`, 'utf8')
    expect(JSON.parse(escaped.toString('utf8')).kind).toBe(AERO_SESSION_KIND)
    expect(escaped.includes(AERO_SESSION_KIND)).toBe(false)
    expect(looksLikeAeroSession(escaped)).toBe(false)
  })
})

describe('looksLikeAeroSession — filter 3: parse, with `kind` at the TOP level', () => {
  it('accepts a minimal Appendix C session document', () => {
    expect(looksLikeAeroSession(Buffer.from(sessionJson(), 'utf8'))).toBe(true)
  })

  it('accepts one whose `kind` is not the first key', () => {
    const reordered = JSON.stringify({
      schema_version: 1,
      items: [],
      kind: AERO_SESSION_KIND,
    })
    expect(looksLikeAeroSession(Buffer.from(reordered, 'utf8'))).toBe(true)
  })

  it('rejects a NESTED kind — the sentinel has to sit at the top level', () => {
    // The distinction filter 3 exists for: any document may quote the
    // sentinel somewhere inside itself (a test fixture manifest describing an
    // .aero, a config recording a MIME type). Only a document whose own root
    // object declares it is a session document.
    for (const nested of [
      { meta: { kind: AERO_SESSION_KIND } },
      { items: [{ kind: AERO_SESSION_KIND }] },
      { document: { inner: { deeper: { kind: AERO_SESSION_KIND } } } },
    ]) {
      const buffer = Buffer.from(JSON.stringify(nested), 'utf8')
      expect(buffer.includes(AERO_SESSION_KIND)).toBe(true) // filter 2 passes
      expect(looksLikeAeroSession(buffer), JSON.stringify(nested)).toBe(false)
    }
  })

  it('rejects a top-level `kind` that is some other value, with the literal elsewhere', () => {
    const other = JSON.stringify({
      kind: 'aeroworship.template',
      note: `not a ${AERO_SESSION_KIND} document`,
    })
    expect(looksLikeAeroSession(Buffer.from(other, 'utf8'))).toBe(false)
  })

  it('rejects a top-level `kind` whose value merely contains the sentinel as a substring', () => {
    const superstring = JSON.stringify({ kind: `${AERO_SESSION_KIND}.backup` })
    expect(looksLikeAeroSession(Buffer.from(superstring, 'utf8'))).toBe(false)
  })

  it('rejects a truncated document that clears filters 1 and 2 but cannot be parsed', () => {
    const truncated = Buffer.from(sessionJson()).subarray(0, 40)
    expect(truncated[0]).toBe(0x7b)
    expect(truncated.includes(AERO_SESSION_KIND)).toBe(true)
    expect(looksLikeAeroSession(truncated)).toBe(false)
  })

  it('rejects a `kind` that is not a string', () => {
    for (const value of [1, true, null, [AERO_SESSION_KIND], { v: AERO_SESSION_KIND }]) {
      const buffer = Buffer.from(
        JSON.stringify({ kind: value, note: AERO_SESSION_KIND }),
        'utf8',
      )
      expect(looksLikeAeroSession(buffer), JSON.stringify(value)).toBe(false)
    }
  })
})

describe('looksLikeAeroSession — what it buys the guard (ADR-0030: a renamed .aero)', () => {
  it('makes a session document staged as `service.dat` a candidate by content', () => {
    const path = 'tests/integration/fixtures/renamed/service.dat'
    expect(hasDeclaredExtension(path)).toBe(false) // the evasion: no .aero suffix
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([
      [path, { status: 'ok', content: Buffer.from(sessionJson(), 'utf8') }],
    ])
    const { candidates, unreadable } = identifyCandidates([path], contentByPath)
    expect(candidates).toEqual([{ path, reason: 'content:aero-session' }])
    expect(unreadable).toEqual([])
  })

  it('and that candidate is then rejected for having no manifest, like any other', () => {
    const path = 'tests/integration/fixtures/renamed/service.dat'
    const verdict = evaluateCandidate({ path, reason: 'content:aero-session' }, new Map())
    expect(verdict.ok).toBe(false)
    expect(summariseRun({ candidateVerdicts: [verdict], unreadable: [] }).exitCode).toBe(
      1,
    )
  })

  it('does not make an ordinary tracked JSON file a candidate', () => {
    const path = 'tsconfig.json'
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([
      [path, { status: 'ok', content: Buffer.from('{"compilerOptions":{}}', 'utf8') }],
    ])
    const { candidates, unreadable } = identifyCandidates([path], contentByPath)
    expect(candidates).toEqual([])
    expect(unreadable).toEqual([])
  })
})

describe('isUnmergedStage', () => {
  it('treats stage 0 — and only stage 0 — as resolved', () => {
    expect(isUnmergedStage('0')).toBe(false)
    expect(isUnmergedStage('1')).toBe(true)
    expect(isUnmergedStage('2')).toBe(true)
    expect(isUnmergedStage('3')).toBe(true)
  })
})

describe('unmergedPathVerdicts', () => {
  const conflicted = 'tests/integration/fixtures/merge/disputed.aero'

  it('returns nothing for an empty list', () => {
    expect(unmergedPathVerdicts([])).toEqual([])
  })

  it('emits ONE rejection for a conflicted .aero listed once per stage', () => {
    // git lists a conflicted path three times (stages 1/2/3). Three identical
    // verdicts for one path would triple the violation count and print the
    // same paragraph three times.
    const verdicts = unmergedPathVerdicts([conflicted, conflicted, conflicted])
    expect(verdicts).toHaveLength(1)
    expect(verdicts[0]?.path).toBe(conflicted)
    expect(verdicts[0]?.ok).toBe(false)
    expect(verdicts[0]?.reason).toMatch(/unresolved merge conflict/)
    expect(verdicts[0]?.reason).toMatch(/stages 1\/2\/3, no stage 0/)
  })

  it('says how to clear it, and that it does not block other conflicted paths', () => {
    // The reason text is the whole point of this function: the behaviour it
    // replaced blocked every `npm test` for the duration of a conflict while
    // blaming "index changed mid-scan?", a cause with nothing to do with it.
    const reason = unmergedPathVerdicts([conflicted])[0]?.reason ?? ''
    expect(reason).toMatch(/Resolve the conflict and `git add`/)
    expect(reason).toMatch(/does not block other conflicted/)
    expect(reason).not.toMatch(/mid-scan/)
  })

  it('flags .aerotpl too, and matches the extension case-insensitively', () => {
    expect(unmergedPathVerdicts(['fixtures/t.aerotpl'])).toHaveLength(1)
    expect(unmergedPathVerdicts(['fixtures/SERVICE.AERO'])).toHaveLength(1)
  })

  it('stays SILENT about a conflicted path with no declared extension — the over-blocking this exists to stop', () => {
    // The deliberate line in this function's doc: an unresolved conflict on
    // README.md or a .png is ordinary work in progress. git refuses to commit
    // while anything is unmerged, so that content cannot reach a commit
    // without first passing stage 0, where the guard reads it in full.
    expect(
      unmergedPathVerdicts([
        'README.md',
        'src/main/main.ts',
        'tests/integration/fixtures/photo.png',
        'tests/integration/fixtures/render.webp',
        'tests/integration/fixtures/fixtures.manifest.json',
      ]),
    ).toEqual([])
  })

  it('judges by name only — a conflicted .aero OUTSIDE any fixtures/ directory is flagged the same way', () => {
    const stray = 'smuggled.aero'
    expect(findFixturesRoot(stray)).toBeNull()
    expect(unmergedPathVerdicts([stray])).toHaveLength(1)
  })

  it('preserves listing order and deduplicates across a mixed listing', () => {
    const verdicts = unmergedPathVerdicts([
      'README.md',
      'fixtures/b.aero',
      'fixtures/photo.png',
      'fixtures/b.aero',
      'fixtures/a.aerotpl',
      'fixtures/b.aero',
      'fixtures/a.aerotpl',
    ])
    expect(verdicts.map((v) => v.path)).toEqual(['fixtures/b.aero', 'fixtures/a.aerotpl'])
    expect(verdicts.every((v) => !v.ok)).toBe(true)
  })

  it('every verdict it emits counts as a violation in summariseRun', () => {
    const summary = summariseRun({
      candidateVerdicts: unmergedPathVerdicts([conflicted, conflicted]),
      unreadable: [],
    })
    expect(summary.exitCode).toBe(1)
    expect(summary.violationCount).toBe(1)
    expect(summary.checkedCount).toBe(1)
  })
})

describe('manifestPathsForCandidates', () => {
  it('returns nothing for no candidates', () => {
    expect(manifestPathsForCandidates([])).toEqual([])
  })

  it('names the manifest at the TOPMOST fixtures/ root, matching findFixturesRoot', () => {
    expect(
      manifestPathsForCandidates([
        { path: 'a/fixtures/b/fixtures/c.aero', reason: 'extension' },
      ]),
    ).toEqual(['a/fixtures/fixtures.manifest.json'])
  })

  it('deduplicates candidates sharing one root, in first-seen order', () => {
    expect(
      manifestPathsForCandidates([
        { path: 'tests/fixtures/one.aero', reason: 'extension' },
        { path: 'tests/fixtures/deep/two.aerotpl', reason: 'extension' },
        { path: 'tests/fixtures/three.aero', reason: 'extension' },
      ]),
    ).toEqual(['tests/fixtures/fixtures.manifest.json'])
  })

  it('keeps distinct roots distinct, in first-seen order (not sorted)', () => {
    expect(
      manifestPathsForCandidates([
        { path: 'z/fixtures/one.aero', reason: 'extension' },
        { path: 'a/fixtures/two.aero', reason: 'extension' },
        { path: 'z/fixtures/three.aero', reason: 'extension' },
      ]),
    ).toEqual(['z/fixtures/fixtures.manifest.json', 'a/fixtures/fixtures.manifest.json'])
  })

  it('skips a candidate with no fixtures/ ancestor instead of inventing a manifest for it', () => {
    // `evaluateCandidate` rejects that candidate outright without consulting
    // any manifest, so fetching one would be a wasted `cat-file` round-trip
    // against a path that cannot exist.
    expect(
      manifestPathsForCandidates([
        { path: 'src-tauri/icons/service.aero', reason: 'extension' },
        { path: 'smuggled.aero', reason: 'extension' },
      ]),
    ).toEqual([])
  })

  it('closes the unescapable loop it was written for: an allowlisted candidate WITH a fixtures/ root', () => {
    // The bug: a candidate found by `identifyAllowlistedExtensionCandidates`
    // has its manifest under the allowlisted prefix too, so the manifest was
    // absent from `toInspect` and every lookup missed — telling the developer
    // to create a file they had already created, with no way out.
    const candidate = /** @type {import('./fixture-content-guard.js').Candidate} */ ({
      path: 'src-tauri/icons/fixtures/sample.aero',
      reason: 'extension',
    })
    const manifestPath = 'src-tauri/icons/fixtures/fixtures.manifest.json'
    expect(manifestPathsForCandidates([candidate])).toEqual([manifestPath])
    expect(
      manifestPathFor(/** @type {string} */ (findFixturesRoot(candidate.path))),
    ).toBe(manifestPath)

    // Without the manifest fetched, the verdict is the misleading one…
    expect(evaluateCandidate(candidate, new Map()).reason).toMatch(
      /is not in the index — create it/,
    )

    // …and with it fetched (which is what this function makes the caller do),
    // a correct declaration finally passes.
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([
      [
        manifestPath,
        {
          status: 'ok',
          content: Buffer.from(
            JSON.stringify({
              entries: {
                'sample.aero': {
                  synthetic: true,
                  purpose:
                    'Synthetic fixture authored for this test; no real service content.',
                },
              },
            }),
            'utf8',
          ),
        },
      ],
    ])
    expect(evaluateCandidate(candidate, contentByPath)).toEqual({
      path: candidate.path,
      ok: true,
    })
  })

  it('a manifest that genuinely is absent still produces the "not in the index" verdict, which is then true', () => {
    const candidate = /** @type {import('./fixture-content-guard.js').Candidate} */ ({
      path: 'src-tauri/icons/fixtures/sample.aero',
      reason: 'extension',
    })
    // The caller asked for it; git answered `missing`.
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([
      ['src-tauri/icons/fixtures/fixtures.manifest.json', { status: 'missing' }],
    ])
    const verdict = evaluateCandidate(candidate, contentByPath)
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/is not in the index — create it/)
  })
})

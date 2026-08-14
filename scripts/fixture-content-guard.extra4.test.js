// @ts-check

/**
 * Coverage for the two things siklus 6 changed in `fixture-content-guard.js`
 * that nothing pinned: the object-spec prefix (`BATCH_SPEC_PREFIX`, `:0:`) and
 * `evaluateCandidate`'s optional third argument.
 *
 * The prefix is the one fail-*open* path this guard has had. `gitrevisions`
 * defines `:<n>:<path>` for n ∈ 0..3 alongside `:<path>`, so under the old
 * short form a tracked path whose own name begins with `0:`…`3:` was read by
 * git as "stage n of the rest of the name" — a different file. Measured on
 * this machine (git 2.47.1.windows.1), in a repository whose index holds
 * `notes.dat` and nothing named `0:notes.dat`:
 *
 *     :notes.dat      → d41b9ff… blob 19   "plain sibling text"
 *     :0:notes.dat    → d41b9ff… blob 19   the same blob
 *     :0:0:notes.dat  → :0:0:notes.dat missing
 *     :1:notes.dat    → :1:notes.dat missing
 *
 * The third line is the load-bearing one: git consumed `:0:` as a stage
 * selector and went looking for a *literal path* `0:notes.dat`, which that
 * index does not have. So `:0:${path}` addresses `path` itself, and `:${path}`
 * does not when `path` starts with a stage digit — under the old form the
 * guard sniffed `0:notes.dat`'s neighbour and passed a real `.aero` clean.
 *
 * Those four lines are what `resolveObjectSpec` below models, and the first
 * test asserts the model reproduces all four before anything is built on it.
 * The model exists because the index entry that makes the bug bite cannot be
 * created on this platform at all: Git for Windows refuses `0:notes.dat`
 * through every route into the index — `update-index --cacheinfo`
 * (`error: Invalid path '0:notes.dat'`), `--index-info` (`Ignoring path
 * 0:notes.dat`), and `read-tree` over a `mktree`-built tree — and
 * `core.protectNTFS=false` does not open it, because `has_dos_drive_prefix`
 * in `compat/mingw.h` reads an alphanumeric run followed by `:` as a drive
 * prefix and `verify_path` refuses before the protectNTFS-gated check is
 * reached (re-measured here, not taken on trust). A test that reached for a
 * Linux git (WSL) to build such an index would be a test that only runs where
 * WSL happens to exist — the machine-dependent gate ADR-0020 already made
 * this repository pay for once. So the proof here is a pure one over
 * `buildBatchRequest`/`parseCatFileBatch`, with a control (the last test in
 * the first suite) that reproduces the old fail-open through the same model:
 * without it, every assertion below would pass just as happily with the old
 * prefix restored, and would be proving nothing.
 */

import { describe, expect, it } from 'vitest'

import {
  buildBatchRequest,
  evaluateCandidate,
  identifyCandidates,
  parseCatFileBatch,
  summariseRun,
} from './fixture-content-guard.js'

/** A synthetic Appendix C session document — invented here, never real. */
const SESSION_DOC =
  '{"kind":"aeroworship.session","schema_version":1,' +
  '"title":"SYNTHETIC — invented for fixture-content-guard.extra4.test.js"}'

/** Ordinary text, so a guard reading *this* blob would see nothing to declare. */
const PLAIN_SIBLING = 'plain sibling text, nothing to declare here\n'

describe('the batch object spec addresses a literal path, not a revision (siklus 6)', () => {
  /**
   * Models the slice of gitrevisions' object-spec grammar `git cat-file
   * --batch` applies to a `:`-leading query, resolved against a fake stage-0
   * index. Faithfulness to real git is asserted, not assumed — see the first
   * test below and the four measured lines in this file's doc.
   *
   * @param {string} spec One query line, exactly as written to git's stdin.
   * @param {ReadonlyMap<string, string>} index Path → stage-0 content.
   * @returns {{ status: 'ok', content: string } | { status: 'missing' }}
   */
  function resolveObjectSpec(spec, index) {
    if (!spec.startsWith(':')) throw new Error(`not an index spec: ${spec}`)
    const rest = spec.slice(1)
    // `:<n>:<path>`, n ∈ 0..3 — a single stage digit followed by `:` is
    // consumed as a selector before the path begins. Anything else is
    // `:<path>`, i.e. stage 0 of the whole remainder.
    const staged = /^([0-3]):([\s\S]*)$/.exec(rest)
    const stage = staged ? Number(staged[1]) : 0
    const path = staged ? staged[2] : rest
    if (stage !== 0 || path === undefined) return { status: 'missing' }
    const content = index.get(path)
    return content === undefined ? { status: 'missing' } : { status: 'ok', content }
  }

  /**
   * Emits the bytes `git cat-file --batch` answers a whole request with,
   * including the verbatim query echo on a `missing` line.
   *
   * @param {string} request Output of a request builder.
   * @param {ReadonlyMap<string, string>} index
   * @returns {Buffer}
   */
  function fakeCatFileBatch(request, index) {
    const lines = request.split('\n').slice(0, -1)
    return Buffer.concat(
      lines.map((line) => {
        const resolved = resolveObjectSpec(line, index)
        if (resolved.status === 'missing') {
          return Buffer.from(`${line} missing\n`, 'utf8')
        }
        const content = Buffer.from(resolved.content, 'utf8')
        return Buffer.concat([
          Buffer.from(`${'a'.repeat(40)} blob ${content.length}\n`, 'ascii'),
          content,
          Buffer.from('\n', 'ascii'),
        ])
      }),
    )
  }

  /** The short form this guard used before siklus 6 — the control, not production. */
  const buildShortFormRequest = (/** @type {readonly string[]} */ paths) =>
    paths.map((path) => `:${path}\n`).join('')

  it('the git model reproduces the four measured lines, or nothing below proves anything', () => {
    const index = new Map([['notes.dat', PLAIN_SIBLING]])
    expect(resolveObjectSpec(':notes.dat', index)).toEqual({
      status: 'ok',
      content: PLAIN_SIBLING,
    })
    expect(resolveObjectSpec(':0:notes.dat', index)).toEqual({
      status: 'ok',
      content: PLAIN_SIBLING,
    })
    expect(resolveObjectSpec(':0:0:notes.dat', index)).toEqual({ status: 'missing' })
    expect(resolveObjectSpec(':1:notes.dat', index)).toEqual({ status: 'missing' })

    // …and once the hostile path really is in the index, the two forms name
    // two different files. This is the whole bug, in one pair of lines.
    const both = new Map([
      ['notes.dat', PLAIN_SIBLING],
      ['0:notes.dat', SESSION_DOC],
    ])
    expect(resolveObjectSpec(':0:notes.dat', both)).toEqual({
      status: 'ok',
      content: PLAIN_SIBLING,
    })
    expect(resolveObjectSpec(':0:0:notes.dat', both)).toEqual({
      status: 'ok',
      content: SESSION_DOC,
    })
  })

  it('every line buildBatchRequest writes resolves back to the path it was built from', () => {
    // The round-trip that `buildBatchRequest`'s contract with
    // `parseCatFileBatch` now depends on: spec → path is the identity, for
    // every name a stage selector could otherwise eat.
    const paths = [
      '0:notes.dat',
      '1:notes.dat',
      '2:service.aero',
      '3:template.aerotpl',
      '0:0:doubly.dat',
      ':leading-colon.dat',
      '4:not-a-stage.dat',
      'notes.dat',
      'tests/integration/fixtures/0:inside.aero',
      'lagu-kéç.aero',
    ]
    const index = new Map(paths.map((path) => [path, `content of ${path}`]))
    const lines = buildBatchRequest(paths).split('\n').slice(0, -1)
    expect(lines).toHaveLength(paths.length)
    for (const [i, path] of paths.entries()) {
      expect(resolveObjectSpec(/** @type {string} */ (lines[i]), index), path).toEqual({
        status: 'ok',
        content: `content of ${path}`,
      })
    }
  })

  it('writes one line per path, each the path verbatim behind a single `:0:`', () => {
    for (const path of ['0:notes.dat', 'notes.dat', 'a/0:b.aero', ':x']) {
      const request = buildBatchRequest([path])
      expect(request.endsWith('\n'), path).toBe(true)
      expect(request.split('\n').slice(0, -1), path).toHaveLength(1)
      expect(request, path).toBe(`:0:${path}\n`)
    }
  })

  it('never maps two distinct paths onto one spec — the collision the short form allowed', () => {
    const paths = ['notes.dat', '0:notes.dat', '1:notes.dat', '0:0:notes.dat']
    const specs = paths.map((path) => buildBatchRequest([path]))
    expect(new Set(specs).size).toBe(paths.length)

    // Distinct *text* is only half of it; the halves that matter are what the
    // two texts resolve to. Under the short form, the spec built for
    // `0:notes.dat` resolves to the neighbour's blob — the collision, stated
    // as a resolution rather than as a string comparison.
    const both = new Map([
      ['notes.dat', PLAIN_SIBLING],
      ['0:notes.dat', SESSION_DOC],
    ])
    const specOf = (/** @type {string} */ request) => request.slice(0, -1)
    expect(
      resolveObjectSpec(specOf(buildShortFormRequest(['0:notes.dat'])), both),
    ).toEqual({ status: 'ok', content: PLAIN_SIBLING })
    expect(resolveObjectSpec(specOf(buildBatchRequest(['0:notes.dat'])), both)).toEqual({
      status: 'ok',
      content: SESSION_DOC,
    })
  })

  it('reads a `missing` echo only for the key that produced it, never a neighbour’s', () => {
    // git quotes the query back verbatim, so the echo for `0:notes.dat` is
    // `:0:0:notes.dat missing`. The echo one line above it in the doc —
    // `:0:notes.dat missing` — belongs to a *different* path, and accepting it
    // for this key would be the same misattribution in the response direction.
    const own = Buffer.from(':0:0:notes.dat missing\n', 'utf8')
    expect(parseCatFileBatch(own, ['0:notes.dat']).get('0:notes.dat')).toEqual({
      status: 'missing',
    })

    const neighbour = Buffer.from(':0:notes.dat missing\n', 'utf8')
    expect(parseCatFileBatch(neighbour, ['0:notes.dat']).get('0:notes.dat')).toEqual({
      status: 'unparsed',
      header: ':0:notes.dat missing',
    })

    // And `unparsed` is fail-closed, not silence: the path lands in
    // `unreadable` and the run exits 1 (see `summariseRun`).
    const { candidates, unreadable } = identifyCandidates(
      ['0:notes.dat'],
      parseCatFileBatch(neighbour, ['0:notes.dat']),
    )
    expect(candidates).toEqual([])
    expect(unreadable).toEqual(['0:notes.dat'])
    expect(summariseRun({ candidateVerdicts: [], unreadable }).exitCode).toBe(1)
  })

  it('sniffs a real .aero staged as `0:notes.dat` on ITS OWN bytes, and rejects it', () => {
    // The end-to-end shape of the fail-open, driven through the real request
    // builder and the real parser against the modelled git above.
    const index = new Map([
      ['notes.dat', PLAIN_SIBLING],
      ['0:notes.dat', SESSION_DOC],
    ])
    const keys = ['notes.dat', '0:notes.dat']
    const contentByPath = parseCatFileBatch(
      fakeCatFileBatch(buildBatchRequest(keys), index),
      keys,
    )
    expect(contentByPath.get('0:notes.dat')).toEqual({
      status: 'ok',
      content: Buffer.from(SESSION_DOC, 'utf8'),
    })

    const { candidates, unreadable } = identifyCandidates(keys, contentByPath)
    expect(unreadable).toEqual([])
    expect(candidates).toEqual([{ path: '0:notes.dat', reason: 'content:aero-session' }])

    const verdicts = candidates.map((c) => evaluateCandidate(c, contentByPath))
    expect(verdicts[0]?.ok).toBe(false)
    expect(verdicts[0]?.reason).toMatch(
      /not under any directory literally named "fixtures"/,
    )
    expect(summariseRun({ candidateVerdicts: verdicts, unreadable }).exitCode).toBe(1)
  })

  it('CONTROL: the same pipeline with the old `:` prefix passes that file clean', () => {
    // Not an endorsement — a demonstration that the model has teeth. If this
    // test also found the candidate, the one above would pass with the bug
    // restored and would be worthless. The bytes read for `0:notes.dat` here
    // are `notes.dat`'s, and nothing in the guard ever notices.
    const index = new Map([
      ['notes.dat', PLAIN_SIBLING],
      ['0:notes.dat', SESSION_DOC],
    ])
    const keys = ['notes.dat', '0:notes.dat']
    const contentByPath = parseCatFileBatch(
      fakeCatFileBatch(buildShortFormRequest(keys), index),
      keys,
    )
    expect(contentByPath.get('0:notes.dat')).toEqual({
      status: 'ok',
      content: Buffer.from(PLAIN_SIBLING, 'utf8'), // the neighbour's blob
    })

    const { candidates, unreadable } = identifyCandidates(keys, contentByPath)
    expect(candidates).toEqual([])
    expect(unreadable).toEqual([])
    expect(summariseRun({ candidateVerdicts: [], unreadable }).exitCode).toBe(0)
  })
})

describe('evaluateCandidate — the optional `unmergedPaths` argument (siklus 6, W3)', () => {
  const root = 'tests/integration/fixtures'
  const manifestPath = `${root}/fixtures.manifest.json`
  /** @type {import('./fixture-content-guard.js').Candidate} */
  const candidate = { path: `${root}/disputed.aero`, reason: 'extension' }

  /** @returns {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
  const noManifest = () => new Map()

  it('names the conflict when the manifest itself is the unmerged path', () => {
    const verdict = evaluateCandidate(candidate, noManifest(), new Set([manifestPath]))
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/is itself in an unresolved merge conflict/)
    expect(verdict.reason).toMatch(/stages 1\/2\/3, no stage 0/)
    expect(verdict.reason).toMatch(/resolve the conflict and `git add` the manifest/)
    // The sentence that was wrong for this case must be gone, not merely
    // accompanied: the manifest *is* in the index, at stages 1/2/3.
    expect(verdict.reason).not.toMatch(/is not in the index/)
  })

  it('falls back to the older sentence when the argument is omitted — and still never passes', () => {
    // The argument is optional, so calling without it stays legal and must
    // stay fail-closed. Both call shapes are pinned because only one of them
    // is exercised by `check-fixture-content.js`, and the other is what every
    // other test in this repository uses.
    for (const verdict of [
      evaluateCandidate(candidate, noManifest()),
      evaluateCandidate(candidate, noManifest(), undefined),
      evaluateCandidate(candidate, noManifest(), new Set()),
      evaluateCandidate(candidate, noManifest(), new Set(['some/other/path.aero'])),
    ]) {
      expect(verdict.ok).toBe(false)
      expect(verdict.reason).toMatch(/is not in the index — create it/)
      expect(verdict.reason).not.toMatch(/unresolved merge conflict/)
      expect(
        summariseRun({ candidateVerdicts: [verdict], unreadable: [] }).exitCode,
      ).toBe(1)
    }
  })

  it('rejects under either sentence — the conflict branch changes the reason, never the verdict', () => {
    const conflicted = evaluateCandidate(candidate, noManifest(), new Set([manifestPath]))
    expect(conflicted.ok).toBe(false)
    expect(conflicted.path).toBe(candidate.path)
    expect(
      summariseRun({ candidateVerdicts: [conflicted], unreadable: [] }),
    ).toMatchObject({ exitCode: 1, violationCount: 1, checkedCount: 1 })
  })

  it('applies to a `missing` manifest entry too, not only an absent map key', () => {
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const answered = new Map([[manifestPath, { status: 'missing' }]])
    expect(
      evaluateCandidate(candidate, answered, new Set([manifestPath])).reason,
    ).toMatch(/is itself in an unresolved merge conflict/)
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const unparsed = new Map([[manifestPath, { status: 'unparsed', header: 'weird' }]])
    expect(
      evaluateCandidate(candidate, unparsed, new Set([manifestPath])).reason,
    ).toMatch(/is itself in an unresolved merge conflict/)
  })

  it('does not fire when the manifest was readable after all — a readable declaration is judged normally', () => {
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const readable = new Map([
      [
        manifestPath,
        {
          status: 'ok',
          content: Buffer.from(
            JSON.stringify({
              entries: {
                'disputed.aero': {
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
    expect(evaluateCandidate(candidate, readable, new Set([manifestPath]))).toEqual({
      path: candidate.path,
      ok: true,
    })
  })

  it('cannot rescue a candidate with no fixtures/ ancestor at all', () => {
    // That branch is decided before any manifest is looked for, so the extra
    // argument must not reach it.
    const stray = /** @type {import('./fixture-content-guard.js').Candidate} */ ({
      path: 'smuggled.aero',
      reason: 'extension',
    })
    const verdict = evaluateCandidate(stray, noManifest(), new Set(['smuggled.aero']))
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/not under any directory literally named "fixtures"/)
  })
})

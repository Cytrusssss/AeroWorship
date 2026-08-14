// @ts-check

/**
 * Pure-function coverage for `fixture-content-guard.js`. `check-fixture-content.js`
 * itself — the part that actually spawns git and reads the real index — is
 * exercised by hand against this repository (see the implementer report for
 * SETUP-05 for the transcript of a real staged rejection and a real staged
 * pass), the same division `dist-html-guard.test.js` draws against
 * `check-dist-html.js`.
 *
 * The rejection paths get more cases than the acceptance path on purpose:
 * ADR-0023's whole premise is that this guard must default to rejecting, so
 * every way a candidate could slip through as "declared" earns its own case.
 */

import { describe, expect, it } from 'vitest'

import {
  ALLOWLISTED_PREFIXES,
  breaksBatchLineProtocol,
  buildBatchRequest,
  classifyIndexedPaths,
  evaluateCandidate,
  evaluateManifestEntry,
  findFixturesRoot,
  gitlinkVerdict,
  hasDeclaredExtension,
  identifyAllowlistedExtensionCandidates,
  identifyCandidates,
  isAllowlisted,
  isGitlinkMode,
  manifestPathFor,
  parseCatFileBatch,
  parseLsFilesEntry,
  relativeToFixturesRoot,
  sniffMediaKind,
  summariseRun,
  unsafePathVerdict,
} from './fixture-content-guard.js'

describe('isAllowlisted / hasDeclaredExtension', () => {
  it('allowlists exactly the declared prefixes — the whole array, not just membership', () => {
    // `toEqual` on the full array, not `toContain` on one entry: `toContain`
    // passes identically whether `ALLOWLISTED_PREFIXES` holds one entry or
    // ten, so widening it to something as broad as `'src-tauri/'` (which
    // would swallow every tracked Rust source file) would not fail a single
    // test. Locking the whole array makes any widening a visible diff here,
    // the same discipline `EXPECTED_DOCUMENTS` gets in `check-dist-html.js`.
    expect(ALLOWLISTED_PREFIXES).toEqual(['src-tauri/icons/'])
    expect(isAllowlisted('src-tauri/icons/icon.png')).toBe(true)
    expect(isAllowlisted('src-tauri/icons-extra/icon.png')).toBe(false)
    expect(isAllowlisted('src/assets/media/welcome.jpg')).toBe(false)
  })

  it('matches both declared extensions, case-insensitively', () => {
    expect(hasDeclaredExtension('service.aero')).toBe(true)
    expect(hasDeclaredExtension('SERVICE.AERO')).toBe(true)
    expect(hasDeclaredExtension('template.aerotpl')).toBe(true)
    expect(hasDeclaredExtension('notes.txt')).toBe(false)
    expect(hasDeclaredExtension('service.aero.bak')).toBe(false)
  })
})

describe('classifyIndexedPaths', () => {
  it('splits allowlisted paths out from everything else', () => {
    const { allowlisted, toInspect } = classifyIndexedPaths([
      'src-tauri/icons/icon.png',
      'tests/integration/fixtures/sample.aero',
      'package.json',
    ])
    expect(allowlisted).toEqual(['src-tauri/icons/icon.png'])
    expect(toInspect).toEqual(['tests/integration/fixtures/sample.aero', 'package.json'])
  })

  it('routes a path carrying a raw \\n or \\r to `unsafe`, never to `toInspect` or `allowlisted`', () => {
    const { allowlisted, toInspect, unsafe } = classifyIndexedPaths([
      'tests/integration/fixtures/evil\nsmuggled.aero',
      'tests/integration/fixtures/evil\rcarriage.aero',
      'tests/integration/fixtures/sample.aero',
    ])
    expect(unsafe).toEqual([
      'tests/integration/fixtures/evil\nsmuggled.aero',
      'tests/integration/fixtures/evil\rcarriage.aero',
    ])
    expect(toInspect).toEqual(['tests/integration/fixtures/sample.aero'])
    expect(allowlisted).toEqual([])
  })

  it('sends an unsafe path to `unsafe` even when it also matches an allowlisted prefix', () => {
    const { allowlisted, unsafe } = classifyIndexedPaths(['src-tauri/icons/evil\n.png'])
    expect(unsafe).toEqual(['src-tauri/icons/evil\n.png'])
    expect(allowlisted).toEqual([])
  })
})

describe('breaksBatchLineProtocol', () => {
  it('flags a path containing a raw newline or carriage return', () => {
    expect(breaksBatchLineProtocol('a\nb.aero')).toBe(true)
    expect(breaksBatchLineProtocol('a\rb.aero')).toBe(true)
    expect(breaksBatchLineProtocol('a\r\nb.aero')).toBe(true)
  })

  it('does not flag an ordinary path', () => {
    expect(breaksBatchLineProtocol('tests/integration/fixtures/sample.aero')).toBe(false)
  })
})

describe('unsafePathVerdict', () => {
  it('is always a rejection, naming the path', () => {
    const verdict = unsafePathVerdict('a\nb.aero')
    expect(verdict).toEqual({
      path: 'a\nb.aero',
      ok: false,
      reason: expect.stringMatching(/raw \\n or \\r byte/),
    })
  })
})

describe('parseLsFilesEntry / isGitlinkMode / gitlinkVerdict', () => {
  it('parses a plain blob record', () => {
    expect(
      parseLsFilesEntry(
        `100644 ${'a'.repeat(40)} 0\ttests/integration/fixtures/sample.aero`,
      ),
    ).toEqual({
      mode: '100644',
      stage: '0',
      path: 'tests/integration/fixtures/sample.aero',
    })
  })

  it('parses a gitlink record and identifies it as one', () => {
    const entry = parseLsFilesEntry(`160000 ${'b'.repeat(40)} 0\tfixtures/church-repo`)
    expect(entry).toEqual({ mode: '160000', stage: '0', path: 'fixtures/church-repo' })
    expect(isGitlinkMode(/** @type {{mode: string}} */ (entry).mode)).toBe(true)
  })

  it('does not treat an ordinary file mode as a gitlink', () => {
    expect(isGitlinkMode('100644')).toBe(false)
    expect(isGitlinkMode('100755')).toBe(false)
    expect(isGitlinkMode('120000')).toBe(false)
  })

  it('preserves a path containing a raw newline verbatim (NUL is the record separator, not \\n)', () => {
    const entry = parseLsFilesEntry(`100644 ${'a'.repeat(40)} 0\tweird\npath.aero`)
    expect(entry).toEqual({ mode: '100644', stage: '0', path: 'weird\npath.aero' })
  })

  it('returns null for a record that does not match the expected shape', () => {
    expect(parseLsFilesEntry('not a valid record')).toBeNull()
  })

  it('gitlinkVerdict is always a rejection, naming the path', () => {
    const verdict = gitlinkVerdict('fixtures/church-repo')
    expect(verdict).toEqual({
      path: 'fixtures/church-repo',
      ok: false,
      reason: expect.stringMatching(/git submodule/),
    })
  })
})

describe('findFixturesRoot / manifestPathFor / relativeToFixturesRoot', () => {
  it('finds a top-level fixtures/ ancestor', () => {
    expect(findFixturesRoot('tests/integration/fixtures/sample.aero')).toBe(
      'tests/integration/fixtures',
    )
  })

  it('finds a nested fixtures/ ancestor from the Cargo-convention location', () => {
    expect(
      findFixturesRoot('src-tauri/crates/core/tests/fixtures/hostile/traversal.aero'),
    ).toBe('src-tauri/crates/core/tests/fixtures')
  })

  it('returns null when no segment is literally "fixtures"', () => {
    expect(findFixturesRoot('stray-outside-fixtures.aero')).toBeNull()
    expect(findFixturesRoot('tests/integration/fixture/sample.aero')).toBeNull()
  })

  it('picks the topmost fixtures/ segment when more than one exists', () => {
    expect(findFixturesRoot('a/fixtures/b/fixtures/c.aero')).toBe('a/fixtures')
  })

  it('names the manifest at the root, not beside the file', () => {
    const root = findFixturesRoot('tests/integration/fixtures/relink-a/media/photo.webp')
    expect(root).toBe('tests/integration/fixtures')
    expect(manifestPathFor(/** @type {string} */ (root))).toBe(
      'tests/integration/fixtures/fixtures.manifest.json',
    )
    expect(
      relativeToFixturesRoot(
        'tests/integration/fixtures/relink-a/media/photo.webp',
        /** @type {string} */ (root),
      ),
    ).toBe('relink-a/media/photo.webp')
  })
})

describe('sniffMediaKind', () => {
  it('recognises every documented signature', () => {
    /** @type {Array<[string, Buffer]>} */
    const cases = [
      ['png', Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0])],
      ['jpeg', Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0, 0])],
      ['gif', Buffer.from('GIF89a...', 'ascii')],
      ['bmp', Buffer.from('BM....', 'ascii')],
      ['ico', Buffer.from([0x00, 0x00, 0x01, 0x00, 0, 0])],
      ['tiff', Buffer.from('II*\0....', 'ascii')],
      [
        'webp',
        Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('WEBP'),
        ]),
      ],
      [
        'wav',
        Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('WAVE'),
        ]),
      ],
      [
        'avi',
        Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('AVI '),
        ]),
      ],
      ['mp4', Buffer.concat([Buffer.from([0, 0, 0, 0x20]), Buffer.from('ftypisom')])],
      ['flac', Buffer.from('fLaC....', 'ascii')],
      ['ogg', Buffer.from('OggS....', 'ascii')],
      ['mp3', Buffer.from('ID3....', 'ascii')],
    ]
    for (const [kind, buffer] of cases) {
      expect(sniffMediaKind(buffer), kind).toBe(kind)
    }
  })

  it('does not classify ordinary text source as media', () => {
    expect(sniffMediaKind(Buffer.from('{"schema_version":1}', 'utf8'))).toBeNull()
    expect(sniffMediaKind(Buffer.from('export function foo() {}', 'utf8'))).toBeNull()
  })

  it('does not throw on a buffer shorter than any signature', () => {
    expect(sniffMediaKind(Buffer.alloc(0))).toBeNull()
    expect(sniffMediaKind(Buffer.from([0xff]))).toBeNull()
  })

  it('rejects a RIFF container that is neither WEBP, WAVE nor AVI', () => {
    const buffer = Buffer.concat([
      Buffer.from('RIFF'),
      Buffer.from([0, 0, 0, 0]),
      Buffer.from('XXXX'),
    ])
    expect(sniffMediaKind(buffer)).toBeNull()
  })
})

describe('buildBatchRequest / parseCatFileBatch — the git wire format', () => {
  /**
   * Hand-builds the exact bytes `git cat-file --batch` (git 2.47.1, default
   * `--batch-format`) emits for a fixed list of present/missing keys, so the
   * parser can be tested without spawning git at all.
   *
   * @param {ReadonlyArray<{ key: string, content: Buffer | null }>} entries
   * @returns {Buffer}
   */
  function buildBatchBuffer(entries) {
    const chunks = entries.map(({ key, content }) => {
      // git echoes the query line back **verbatim** before ` missing`, so the
      // echo carries whatever prefix `buildBatchRequest` wrote — `:0:`, the
      // explicit stage-0 form (see `BATCH_SPEC_PREFIX`). Hard-coded rather
      // than derived from the production function on purpose: this helper is
      // the model of git's side of the wire, and a model that borrowed the
      // implementation's own string would agree with it by construction.
      if (content === null) return Buffer.from(`:0:${key} missing\n`, 'ascii')
      const fakeSha = 'a'.repeat(40)
      const header = Buffer.from(`${fakeSha} blob ${content.length}\n`, 'ascii')
      return Buffer.concat([header, content, Buffer.from('\n', 'ascii')])
    })
    return Buffer.concat(chunks)
  }

  it('round-trips present and missing entries, in request order', () => {
    const keys = ['a.aero', 'b.png', 'c.txt']
    const buffer = buildBatchBuffer([
      { key: 'a.aero', content: Buffer.from('{}', 'utf8') },
      { key: 'b.png', content: null },
      { key: 'c.txt', content: Buffer.from('hello', 'utf8') },
    ])
    const result = parseCatFileBatch(buffer, keys)
    expect(result.get('a.aero')).toEqual({
      status: 'ok',
      content: Buffer.from('{}', 'utf8'),
    })
    expect(result.get('b.png')).toEqual({ status: 'missing' })
    expect(result.get('c.txt')).toEqual({
      status: 'ok',
      content: Buffer.from('hello', 'utf8'),
    })
  })

  it('is binary-safe: content bytes containing a raw newline are not truncated', () => {
    const content = Buffer.from([0x89, 0x0a, 0x00, 0x0a, 0xff])
    const buffer = buildBatchBuffer([{ key: 'x.png', content }])
    const result = parseCatFileBatch(buffer, ['x.png'])
    expect(result.get('x.png')).toEqual({ status: 'ok', content })
  })

  it('throws rather than silently misreading a truncated response', () => {
    const buffer = Buffer.from(`${'a'.repeat(40)} blob 10\nshort`, 'ascii')
    expect(() => parseCatFileBatch(buffer, ['x'])).toThrow(
      /shorter than the declared size/,
    )
  })

  // A response line that is neither an object header nor this key's `missing`
  // echo used to throw here, which cost the repository its verdict on *every
  // other* path (the caller returns 1 before `identifyCandidates` runs once).
  // It now yields `{ status: 'unparsed', header }` instead. That replacement is
  // only sound if the guarantee the throw provided survives: the odd path must
  // still be reported and must still fail the run. `toEqual` on the returned
  // entry alone would drop that guarantee silently, so the three cases below
  // assert the whole chain — the entry itself, that it reaches `unreadable`,
  // and that `summariseRun` exits 1 on it — plus the claim that motivated not
  // throwing at all (`offset` still points at the next entry).
  it('yields an `unparsed` entry, not a throw, when a header line does not parse', () => {
    const buffer = Buffer.from('not a valid header\n', 'ascii')
    const result = parseCatFileBatch(buffer, ['x'])
    expect(result.get('x')).toEqual({ status: 'unparsed', header: 'not a valid header' })
  })

  it('keeps judging every remaining key after an unparsed line — that is the whole reason it no longer throws', () => {
    // The unparsed line is one entry on the wire (`--batch` answers a query
    // line with either a header plus content, or a single line), so `offset`
    // must already sit at the next entry. If it drifted, `b.aero`'s content
    // would come back wrong or the parse would throw here instead.
    const buffer = Buffer.concat([
      Buffer.from('this line is not a header at all\n', 'ascii'),
      buildBatchBuffer([
        { key: 'b.aero', content: Buffer.from('{"schema_version":1}', 'utf8') },
        { key: 'c.png', content: null },
      ]),
    ])
    const result = parseCatFileBatch(buffer, ['a.weird', 'b.aero', 'c.png'])
    expect(result.get('a.weird')).toEqual({
      status: 'unparsed',
      header: 'this line is not a header at all',
    })
    expect(result.get('b.aero')).toEqual({
      status: 'ok',
      content: Buffer.from('{"schema_version":1}', 'utf8'),
    })
    expect(result.get('c.png')).toEqual({ status: 'missing' })
  })

  it('an unparsed entry still reaches `unreadable` and still fails the run — the guarantee the removed throw carried', () => {
    const buffer = Buffer.from('not a valid header\n', 'ascii')
    const contentByPath = parseCatFileBatch(buffer, [
      'tests/integration/fixtures/odd.bin',
    ])

    // Not a candidate: there are no bytes to sniff, so claiming it is media
    // would be an overclaim (`identifyCandidates`'s own doc). But it must not
    // be silent either — `unreadable` is what says "unverified, not clean".
    const { candidates, unreadable } = identifyCandidates(
      ['tests/integration/fixtures/odd.bin'],
      contentByPath,
    )
    expect(candidates).toEqual([])
    expect(unreadable).toEqual(['tests/integration/fixtures/odd.bin'])

    // …and `unreadable` alone, with no violations at all, is exit 1.
    const summary = summariseRun({ candidateVerdicts: [], unreadable })
    expect(summary.exitCode).toBe(1)
    expect(summary.unreadableCount).toBe(1)
    expect(summary.violationCount).toBe(0)
  })

  it('still throws on framing damage that genuinely desynchronises the stream', () => {
    // The line above the `unparsed` branch: a declared size the response
    // cannot satisfy means `offset` is no longer trustworthy for any later
    // key, so continuing would attribute the wrong bytes to the wrong path.
    // That distinction is what makes the `unparsed` branch a narrowing rather
    // than a softening, so it is asserted right next to it.
    const truncated = Buffer.from(`${'a'.repeat(40)} blob 99\nonly-a-few\n`, 'ascii')
    expect(() => parseCatFileBatch(truncated, ['x'])).toThrow(
      /shorter than the declared size/,
    )

    const noTrailingNewline = Buffer.concat([
      Buffer.from(`${'a'.repeat(40)} blob 2\nab`, 'ascii'),
      Buffer.from('NOT-A-NEWLINE', 'ascii'),
    ])
    expect(() => parseCatFileBatch(noTrailingNewline, ['x'])).toThrow(
      /missing the trailing newline/,
    )
  })

  it('throws when the stream ends before a header is found', () => {
    const buffer = Buffer.from('no newline anywhere in here', 'ascii')
    expect(() => parseCatFileBatch(buffer, ['x'])).toThrow(/ended before a header/)
  })

  it('buildBatchRequest is the exact inverse assumption parseCatFileBatch relies on', () => {
    expect(buildBatchRequest(['a.aero', 'b/c.aerotpl'])).toBe(
      ':0:a.aero\n:0:b/c.aerotpl\n',
    )
    expect(buildBatchRequest([])).toBe('')
  })

  it('buildBatchRequest throws rather than silently corrupting the request for a path carrying \\n or \\r', () => {
    // Regression coverage for the C1 correlation break: a path like this
    // would otherwise split the one line written for it — `` `:${path}\n` ``
    // at the time of that break, `` `:0:${path}\n` `` since siklus 6 spelled
    // the stage digit out (`BATCH_SPEC_PREFIX`) — into two stdin lines and
    // desync every entry requested after it, whichever prefix is in front.
    // This is the belt-and-suspenders check
    // — `check-fixture-content.js` is expected to have already routed such a
    // path to `classifyIndexedPaths`'s `unsafe` bucket and never call this
    // function with it at all; see `check-fixture-content.e2e.test.js` /
    // the implementer report for the end-to-end proof against real git.
    expect(() => buildBatchRequest(['a.aero', 'evil\nsmuggled.aero'])).toThrow(
      /raw \\n or \\r byte/,
    )
    expect(() => buildBatchRequest(['evil\rcarriage.aero'])).toThrow(
      /raw \\n or \\r byte/,
    )
  })
})

describe('identifyCandidates', () => {
  /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
  const content = new Map([
    [
      'tests/integration/fixtures/sample.aero',
      { status: 'ok', content: Buffer.from('{}') },
    ],
    [
      'tests/integration/fixtures/render/thumb.webp',
      {
        status: 'ok',
        content: Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('WEBP'),
        ]),
      },
    ],
    ['src/main/main.ts', { status: 'ok', content: Buffer.from('export {}') }],
    ['tests/integration/fixtures/broken.aero', { status: 'missing' }],
  ])

  it('flags a declared extension without reading its content', () => {
    const { candidates } = identifyCandidates(
      ['tests/integration/fixtures/sample.aero'],
      content,
    )
    expect(candidates).toEqual([
      { path: 'tests/integration/fixtures/sample.aero', reason: 'extension' },
    ])
  })

  it('flags a sniffed media file that carries no declared extension', () => {
    const { candidates } = identifyCandidates(
      ['tests/integration/fixtures/render/thumb.webp'],
      content,
    )
    expect(candidates).toEqual([
      { path: 'tests/integration/fixtures/render/thumb.webp', reason: 'content:webp' },
    ])
  })

  it('leaves ordinary source untouched', () => {
    const { candidates, unreadable } = identifyCandidates(['src/main/main.ts'], content)
    expect(candidates).toEqual([])
    expect(unreadable).toEqual([])
  })

  it('still surfaces an extension-matched candidate whose content could not be confirmed, and marks it unreadable', () => {
    const { candidates, unreadable } = identifyCandidates(
      ['tests/integration/fixtures/broken.aero'],
      content,
    )
    expect(candidates).toEqual([
      { path: 'tests/integration/fixtures/broken.aero', reason: 'extension' },
    ])
    expect(unreadable).toEqual(['tests/integration/fixtures/broken.aero'])
  })

  it('never treats an unreadable non-extension path as a media candidate, but still fails closed as unreadable (W-new-1)', () => {
    // Before the W-new-1 fix, this path landed in neither `candidates` nor
    // `unreadable` — a binary file whose content genuinely could not be
    // fetched read as "clean" by omission, the same shape C1 was (a guard
    // reporting clean over bytes it never read). It must never be a
    // *candidate* (there is no byte here to sniff, so there is no basis to
    // claim it is media), but it must always land in `unreadable` so
    // `summariseRun` fails the run instead of staying silent about it.
    const { candidates, unreadable } = identifyCandidates(['nowhere.jpg'], new Map())
    expect(candidates).toEqual([])
    expect(unreadable).toEqual(['nowhere.jpg'])
  })

  it('marks a missing (not just absent-from-map) non-extension path unreadable too', () => {
    /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
    const contentByPath = new Map([['orphan.bin', { status: 'missing' }]])
    const { candidates, unreadable } = identifyCandidates(['orphan.bin'], contentByPath)
    expect(candidates).toEqual([])
    expect(unreadable).toEqual(['orphan.bin'])
  })
})

describe('identifyAllowlistedExtensionCandidates (W-new-2)', () => {
  it('flags a declared-extension path even though it sits under an allowlisted prefix', () => {
    // This is the exact shape W-new-2 named: a real `.aero` staged at
    // `src-tauri/icons/service.aero` must not be exempted from the
    // extension check just because that directory is trusted for ordinary
    // binary icons.
    expect(isAllowlisted('src-tauri/icons/service.aero')).toBe(true)
    const candidates = identifyAllowlistedExtensionCandidates([
      'src-tauri/icons/service.aero',
    ])
    expect(candidates).toEqual([
      { path: 'src-tauri/icons/service.aero', reason: 'extension' },
    ])
  })

  it('flags an .aerotpl the same way as .aero', () => {
    const candidates = identifyAllowlistedExtensionCandidates([
      'src-tauri/icons/order.aerotpl',
    ])
    expect(candidates).toEqual([
      { path: 'src-tauri/icons/order.aerotpl', reason: 'extension' },
    ])
  })

  it('leaves an ordinary allowlisted binary icon alone — no content sniffing happens here', () => {
    const candidates = identifyAllowlistedExtensionCandidates([
      'src-tauri/icons/icon.png',
    ])
    expect(candidates).toEqual([])
  })

  it('returns nothing for an empty allowlisted bucket', () => {
    expect(identifyAllowlistedExtensionCandidates([])).toEqual([])
  })
})

describe('classifyIndexedPaths — allowlist no longer hides a declared extension end-to-end (W-new-2)', () => {
  it('routes an allowlisted .aero to `allowlisted`, not `toInspect`, but identifyAllowlistedExtensionCandidates still catches it', () => {
    const { allowlisted, toInspect } = classifyIndexedPaths([
      'src-tauri/icons/service.aero',
    ])
    expect(allowlisted).toEqual(['src-tauri/icons/service.aero'])
    expect(toInspect).toEqual([])
    expect(identifyAllowlistedExtensionCandidates(allowlisted)).toEqual([
      { path: 'src-tauri/icons/service.aero', reason: 'extension' },
    ])
  })

  it('evaluateCandidate rejects it unconditionally: src-tauri/icons/ has no fixtures/ ancestor', () => {
    const candidates = identifyAllowlistedExtensionCandidates([
      'src-tauri/icons/service.aero',
    ])
    expect(candidates).toHaveLength(1)
    const verdict = evaluateCandidate(
      /** @type {import('./fixture-content-guard.js').Candidate} */ (candidates[0]),
      new Map(),
    )
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/not under any directory literally named "fixtures"/)
  })
})

describe('evaluateManifestEntry', () => {
  const manifest = JSON.stringify({
    entries: {
      'sample.aero': {
        synthetic: true,
        purpose: 'Hand-written NFR-15 hostile-path fixture, no real service content.',
      },
      'placeholder.aero': { synthetic: true, purpose: 'too short' },
      'lying.aero': {
        synthetic: false,
        purpose: 'a purpose long enough to pass the length check',
      },
    },
  })

  it('accepts a well-formed synthetic declaration', () => {
    expect(evaluateManifestEntry(manifest, 'sample.aero')).toEqual({ ok: true })
  })

  it('rejects a missing entry', () => {
    const verdict = evaluateManifestEntry(manifest, 'never-declared.aero')
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/no entry for/)
  })

  it('rejects synthetic: false even with a long purpose', () => {
    const verdict = evaluateManifestEntry(manifest, 'lying.aero')
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/synthetic is not literally `true`/)
  })

  it('rejects a purpose under the 20-character floor', () => {
    const verdict = evaluateManifestEntry(manifest, 'placeholder.aero')
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/too short/)
  })

  it('rejects invalid JSON', () => {
    const verdict = evaluateManifestEntry('not json', 'sample.aero')
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/not valid JSON/)
  })

  it('rejects a manifest with no entries object', () => {
    const verdict = evaluateManifestEntry(
      JSON.stringify({ schemaVersion: 1 }),
      'sample.aero',
    )
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/no "entries" object/)
  })

  it('rejects an array masquerading as the manifest root', () => {
    const verdict = evaluateManifestEntry('[]', 'sample.aero')
    expect(verdict.ok).toBe(false)
  })
})

describe('evaluateCandidate', () => {
  it('rejects unconditionally when there is no fixtures/ ancestor at all — the git add -f case', () => {
    const verdict = evaluateCandidate(
      { path: 'stray-outside-fixtures.aero', reason: 'extension' },
      new Map(),
    )
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/not under any directory literally named "fixtures"/)
  })

  it('rejects when the manifest itself is not in the index', () => {
    const verdict = evaluateCandidate(
      { path: 'tests/integration/fixtures/sample.aero', reason: 'extension' },
      new Map(),
    )
    expect(verdict.ok).toBe(false)
    expect(verdict.reason).toMatch(/fixtures.manifest.json" is not in the index/)
  })

  it('accepts a candidate properly declared in its manifest', () => {
    const manifestText = JSON.stringify({
      entries: {
        'sample.aero': {
          synthetic: true,
          purpose: 'Hand-written placeholder, produced only to exercise this test.',
        },
      },
    })
    const contentByPath = new Map(
      /** @type {const} */ ([
        [
          'tests/integration/fixtures/fixtures.manifest.json',
          { status: 'ok', content: Buffer.from(manifestText, 'utf8') },
        ],
      ]),
    )
    const verdict = evaluateCandidate(
      { path: 'tests/integration/fixtures/sample.aero', reason: 'extension' },
      contentByPath,
    )
    expect(verdict).toEqual({ path: 'tests/integration/fixtures/sample.aero', ok: true })
  })
})

describe('summariseRun', () => {
  it('is clean (exit 0) with zero candidates and reports how many were checked', () => {
    const summary = summariseRun({
      candidateVerdicts: [],
      unreadable: [],
    })
    expect(summary).toEqual({
      exitCode: 0,
      checkedCount: 0,
      violationCount: 0,
      unreadableCount: 0,
    })
  })

  it('is clean (exit 0) when every candidate passed', () => {
    const summary = summariseRun({
      candidateVerdicts: [{ path: 'a.aero', ok: true }],
      unreadable: [],
    })
    expect(summary.exitCode).toBe(0)
    expect(summary.checkedCount).toBe(1)
  })

  it('fails on any violation, regardless of how many candidates passed', () => {
    const summary = summariseRun({
      candidateVerdicts: [
        { path: 'a.aero', ok: true },
        { path: 'b.aero', ok: false, reason: 'no manifest' },
      ],
      unreadable: [],
    })
    expect(summary.exitCode).toBe(1)
    expect(summary.violationCount).toBe(1)
  })

  it('fails on an unreadable candidate even when every checked verdict passed', () => {
    const summary = summariseRun({
      candidateVerdicts: [{ path: 'a.aero', ok: true }],
      unreadable: ['b.aero'],
    })
    expect(summary.exitCode).toBe(1)
    expect(summary.unreadableCount).toBe(1)
  })
})

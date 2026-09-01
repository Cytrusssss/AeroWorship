// @ts-check

/**
 * Additional pure-function coverage for `fixture-content-guard.js`, written by
 * `tester` alongside the implementer's own `fixture-content-guard.test.js`.
 * Three things earned their own file rather than an edit to the implementer's:
 *
 *   - `sniffMediaKind` boundary and false-positive behaviour — how many bytes
 *     each signature actually reads, what happens one byte short of it, and
 *     what a short *text* signature (`BM`, 2 ASCII bytes) does to an ordinary
 *     source file that happens to start with those two letters. That last
 *     case is not a bug this file fixes; ADR-0023's guard is deliberately
 *     fail-closed, so an over-eager match costs a manifest entry, not a
 *     security hole. It is asserted here so the behaviour is visible and
 *     cannot regress into something worse (e.g. a crash) without a test
 *     noticing.
 *   - `parseCatFileBatch` edge bytes the implementer's round-trip case does
 *     not exercise on their own: a zero-length blob, content carrying a raw
 *     NUL byte, a zero-length entry immediately followed by a present one
 *     (offset drift is the classic bug class here), and content whose own
 *     final byte is `\n` sitting right up against git's framing `\n`.
 *   - `isAllowlisted` precision: a prefix match has to be anchored at the
 *     start of the path, not merely present somewhere in it, and a
 *     similarly-named sibling directory must not slip through.
 */

import { describe, expect, it } from 'vitest'

import {
  buildBatchRequest,
  isAllowlisted,
  parseCatFileBatch,
  sniffMediaKind,
} from './fixture-content-guard.js'

describe('sniffMediaKind — signature length and boundary behaviour', () => {
  it('matches a signature that is exactly as long as the magic number, no trailing bytes', () => {
    expect(sniffMediaKind(Buffer.from([0x00, 0x00, 0x01, 0x00]))).toBe('ico')
    expect(sniffMediaKind(Buffer.from('GIF89a', 'ascii'))).toBe('gif')
    expect(
      sniffMediaKind(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])),
    ).toBe('png')
    // RIFF(4) + size(4) + "WEBP"(4) = 12 bytes is the minimum WEBP needs.
    expect(
      sniffMediaKind(
        Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('WEBP'),
        ]),
      ),
    ).toBe('webp')
  })

  it('returns null when the buffer is exactly one byte short of a signature', () => {
    expect(sniffMediaKind(Buffer.from([0x00, 0x00, 0x01]))).toBeNull() // ico, 3 of 4
    expect(sniffMediaKind(Buffer.from('GIF89', 'ascii'))).toBeNull() // gif, 5 of 6
    expect(
      sniffMediaKind(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a])),
    ).toBeNull() // png, 7 of 8
    expect(
      sniffMediaKind(
        Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('WEB'),
        ]),
      ),
    ).toBeNull() // webp, 11 of 12
  })

  it('the BMP signature is two ASCII bytes and matches ordinary text that happens to start "BM" — documented, not fixed', () => {
    // This is the negative case ADR-0023's brief for the tester specifically
    // asks for: "text prefixed with bytes that look like a magic number".
    // `sniffMediaKind` reports this as bmp; the guard then demands a manifest
    // declaration for what is really a prose file. Fail-closed by design
    // (ADR-0023's whole premise), so the cost is an extra manifest entry, not
    // a security gap — but it is a real false positive and worth pinning so a
    // future change to this signature list is a deliberate edit, not a
    // regression nobody notices.
    expect(
      sniffMediaKind(Buffer.from('BM: budget migration notes for June', 'utf8')),
    ).toBe('bmp')
  })

  it('does not match a RIFF container one byte short of declaring its kind', () => {
    // RIFF + 4-byte size only, no kind tag at all.
    expect(
      sniffMediaKind(Buffer.concat([Buffer.from('RIFF'), Buffer.from([0, 0, 0, 0])])),
    ).toBeNull()
  })
})

describe('isAllowlisted — anchored prefix, not substring', () => {
  it('does not allowlist a sibling directory whose name merely starts the same way', () => {
    expect(isAllowlisted('src-tauri/icons-evil/payload.png')).toBe(false)
    expect(isAllowlisted('src-tauri/icons2/payload.png')).toBe(false)
  })

  it('does not allowlist the prefix when it appears mid-path rather than at the root', () => {
    expect(isAllowlisted('vendor/src-tauri/icons/logo.png')).toBe(false)
    expect(isAllowlisted('tests/integration/fixtures/src-tauri/icons/logo.png')).toBe(
      false,
    )
  })

  it('does allowlist a real subdirectory beneath the declared prefix', () => {
    expect(isAllowlisted('src-tauri/icons/windows/icon.ico')).toBe(true)
  })
})

describe('parseCatFileBatch — additional wire-format bytes', () => {
  /**
   * @param {ReadonlyArray<{ key: string, content: Buffer | null }>} entries
   * @returns {Buffer}
   */
  function buildBatchBuffer(entries) {
    const chunks = entries.map(({ key, content }) => {
      // `:0:`, not `:` — git quotes the query line back verbatim, and the
      // query is written with the explicit stage-0 prefix (`BATCH_SPEC_PREFIX`).
      if (content === null) return Buffer.from(`:0:${key} missing\n`, 'ascii')
      const fakeSha = 'b'.repeat(40)
      const header = Buffer.from(`${fakeSha} blob ${content.length}\n`, 'ascii')
      return Buffer.concat([header, content, Buffer.from('\n', 'ascii')])
    })
    return Buffer.concat(chunks)
  }

  it('reads a zero-length blob as present with empty content, not as missing', () => {
    const buffer = buildBatchBuffer([{ key: 'empty.aero', content: Buffer.alloc(0) }])
    const result = parseCatFileBatch(buffer, ['empty.aero'])
    expect(result.get('empty.aero')).toEqual({ status: 'ok', content: Buffer.alloc(0) })
  })

  it('does not let a zero-length entry drift the offset for the entry after it', () => {
    const buffer = buildBatchBuffer([
      { key: 'empty.aero', content: Buffer.alloc(0) },
      { key: 'next.aero', content: Buffer.from('{"schema_version":1}', 'utf8') },
    ])
    const result = parseCatFileBatch(buffer, ['empty.aero', 'next.aero'])
    expect(result.get('empty.aero')).toEqual({ status: 'ok', content: Buffer.alloc(0) })
    expect(result.get('next.aero')).toEqual({
      status: 'ok',
      content: Buffer.from('{"schema_version":1}', 'utf8'),
    })
  })

  it('extracts content containing a raw NUL byte intact', () => {
    const content = Buffer.from([0x00, 0x01, 0x00, 0xff, 0x00])
    const buffer = buildBatchBuffer([{ key: 'weird.bin', content }])
    const result = parseCatFileBatch(buffer, ['weird.bin'])
    expect(result.get('weird.bin')).toEqual({ status: 'ok', content })
  })

  it("does not confuse the content's own trailing newline with git's framing newline", () => {
    // The content's *last byte* is 0x0a, immediately followed by git's own
    // separator 0x0a. A parser that found "the next newline" instead of
    // trusting the declared byte length would either truncate this content by
    // one byte or misread the following entry's header.
    const content = Buffer.from('line one\nline two\n', 'utf8')
    const buffer = buildBatchBuffer([
      { key: 'trailing-newline.txt', content },
      { key: 'after.aero', content: Buffer.from('{}', 'utf8') },
    ])
    const result = parseCatFileBatch(buffer, ['trailing-newline.txt', 'after.aero'])
    expect(result.get('trailing-newline.txt')).toEqual({ status: 'ok', content })
    expect(result.get('after.aero')).toEqual({
      status: 'ok',
      content: Buffer.from('{}', 'utf8'),
    })
  })

  it('round-trips a mixed batch of missing / zero-length / present entries in one pass', () => {
    const keys = ['a.aero', 'gone.aero', 'b.aero']
    const buffer = buildBatchBuffer([
      { key: 'a.aero', content: Buffer.alloc(0) },
      { key: 'gone.aero', content: null },
      { key: 'b.aero', content: Buffer.from('{"x":1}', 'utf8') },
    ])
    const result = parseCatFileBatch(buffer, keys)
    expect(result.get('a.aero')).toEqual({ status: 'ok', content: Buffer.alloc(0) })
    expect(result.get('gone.aero')).toEqual({ status: 'missing' })
    expect(result.get('b.aero')).toEqual({
      status: 'ok',
      content: Buffer.from('{"x":1}', 'utf8'),
    })
  })

  it('buildBatchRequest / parseCatFileBatch compose for a realistic multi-file query', () => {
    const keys = [
      'tests/integration/fixtures/a.aero',
      'tests/integration/fixtures/b.aerotpl',
    ]
    expect(buildBatchRequest(keys)).toBe(
      ':0:tests/integration/fixtures/a.aero\n:0:tests/integration/fixtures/b.aerotpl\n',
    )
  })
})

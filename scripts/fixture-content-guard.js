// @ts-check

/**
 * Pure decisions behind `check-fixture-content.js` (ADR-0023). `.gitignore`
 * unignores `**\/fixtures/**\/*.aero` and `*.aerotpl` because NFR-15 and
 * NFR-28 need those fixtures in the repo — but that exception works by
 * removing the one signal that otherwise keeps a real order-of-service file
 * out of git entirely. Inside a directory named `fixtures/`, a real `.aero`
 * dropped there while debugging looks exactly like a legitimate one in
 * `git status`; nothing distinguishes them by inspection.
 *
 * The bar ADR-0023 sets is **positive**, not "doesn't look like a lyric":
 * a fixture has to *prove* it was authored as a fixture, or it is rejected by
 * default. Content sniffing for "this looks synthetic" was rejected on
 * purpose — a hand-written test lyric and a real congregation's lyric are
 * indistinguishable by pattern, and that is precisely the case that must not
 * slip through. So the proof this file asks for is a declaration: every
 * candidate path needs a named entry in a `fixtures.manifest.json` sitting at
 * the root of the `fixtures/` directory that contains it, carrying
 * `synthetic: true` and a real sentence explaining what the fixture is and
 * how it was produced. Writing that sentence is the deliberate, auditable act
 * that a person copying in a real file has to either perform truthfully or
 * skip — and skipping it is what leaves the file rejected.
 *
 * Two kinds of candidate, found two different ways:
 *
 *   - `.aero` / `.aerotpl` by extension, anywhere in the index — cheap, no
 *     content needed. ADR-0023 names both explicitly: FR-409 makes
 *     `.aerotpl` its own export format, and `template_media` means a real
 *     template drags along a church's own image references.
 *   - Render/media artefacts by content, also anywhere in the index. This is
 *     the harder half of ADR-0023's brief: "`/cache/` and `/media/` anchored
 *     at the root only catch the root-dump shape; a copy that lands as
 *     `AeroWorship/cache/decks/…`, `tmp/data/`, or a relink bundle under
 *     `tests/integration/fixtures/relink-a/media/` all pass, and no
 *     extension pattern in `.gitignore` touches `.webp`." A checker that
 *     judges *what* is staged rather than *where* has to actually look at
 *     bytes, not paths — `sniffMediaKind` below reads a magic-number header,
 *     not a filename.
 *
 * What this file does not decide: whether a path is even worth reading. That
 * is `ALLOWLISTED_PREFIXES` — a short, explicit list of directories that are
 * already known to hold legitimate tracked binaries (`src-tauri/icons/`
 * today), so the guard does not have to read every source file in the repo on
 * every run just to notice it is not a JPEG. A location judged "safe" here is
 * still judged by nothing else — it is skipped entirely, not exempted from a
 * check that ran.
 *
 * I/O — spawning git, reading `fixtures.manifest.json` off disk-via-git,
 * printing — all lives in `check-fixture-content.js`. Kept out of here so
 * every decision above can be driven with literal bytes in a test, the same
 * split `dist-html-guard.js` uses and for the same reason: a decision that
 * only exists inside the script that calls git is a decision no test can
 * reach.
 */

/** Directory name that flips `.gitignore`'s exception on (see file doc). */
export const FIXTURES_DIR_NAME = 'fixtures'

/**
 * Name of the sidecar manifest a `fixtures/` directory must carry once it
 * holds a candidate. One per `fixtures/` root, not one per file — so a
 * directory of twenty hostile-path fixtures declares itself once, and a
 * reviewer sees every declaration for a directory in one diff hunk instead of
 * scattered across twenty sidecar files.
 */
export const MANIFEST_FILE_NAME = 'fixtures.manifest.json'

/**
 * Extensions `.gitignore` unignores under `fixtures/`. Matched
 * case-insensitively because git's own `*.aero` glob is case-sensitive on
 * Linux but the filesystem that produced a fixture may not have been.
 */
export const DECLARED_EXTENSIONS = ['.aero', '.aerotpl']

/**
 * Paths (repo-root-relative, forward-slash, trailing `/`) this guard never
 * fetches content for or sniffs the magic bytes of. Small and explicit on
 * purpose: adding a directory here is a claim "everything under this path is
 * legitimate binary source (icons, and the like), so do not spend a
 * `cat-file` round-trip sniffing it as congregation media", and that claim
 * should be visible in a diff the same way adding a third entry point to
 * `EXPECTED_DOCUMENTS` in `check-dist-html.js` is.
 *
 * `src-tauri/icons/` holds the five tracked app-icon binaries `SETUP-01`
 * shipped. Nothing else in the repository is a tracked binary today.
 * ADR-0023's own prose mentions `src/assets/media/` as a directory the
 * guard must not swallow — that directory does not exist in this working
 * tree (verified: `git ls-files src/` and a filesystem walk both come back
 * empty), so it is deliberately **not** listed here. The item that creates it
 * decides then whether it needs an exception, the same way a new Vite entry
 * point earns a line in `EXPECTED_DOCUMENTS` rather than a wildcard.
 *
 * This is an exemption from **content sniffing only**, not from the
 * declared-extension check. `.aero`/`.aerotpl` are the two shapes ADR-0023
 * names explicitly, and `hasDeclaredExtension` costs nothing to evaluate — no
 * git call, no bytes read — so there is no reason `src-tauri/icons/` (or any
 * future allowlisted prefix) should also grant a real `.aero` cover: an icon
 * directory has no legitimate reason to hold one, and ADR-0023's own words
 * are "fixture dibuktikan sintetis **di mana pun ia berada**". See
 * `identifyAllowlistedExtensionCandidates`, which is what still runs the
 * extension check over these paths, and `classifyIndexedPaths`, which is what
 * this list actually excuses from `toInspect`.
 */
export const ALLOWLISTED_PREFIXES = ['src-tauri/icons/']

/**
 * @param {string} path Repo-root-relative path, forward-slash (as `git
 *   ls-files` always emits, on every OS — unlike `readdir`, there is no
 *   backslash-normalisation step needed here).
 * @returns {boolean}
 */
export function isAllowlisted(path) {
  return ALLOWLISTED_PREFIXES.some((prefix) => path.startsWith(prefix))
}

/**
 * @param {string} path
 * @returns {boolean} Whether `path` carries one of `DECLARED_EXTENSIONS`.
 */
export function hasDeclaredExtension(path) {
  const lower = path.toLowerCase()
  return DECLARED_EXTENSIONS.some((extension) => lower.endsWith(extension))
}

/**
 * Byte that would corrupt the line-delimited wire protocol `buildBatchRequest`
 * writes and `parseCatFileBatch` reads by position: a `:path\n` object spec
 * is one line, and `git cat-file --batch` reads stdin one line per query. Git
 * itself does not forbid `\n` inside a tracked path — the only bytes it
 * refuses are `/` and NUL — so a path can legitimately carry one, and when it
 * does, `` `:${path}\n` `` silently becomes *two* stdin lines: `` `:` `` plus
 * everything up to the embedded newline, then everything after it as an
 * unrelated second query. Every entry requested after that point shifts by
 * one, and `parseCatFileBatch`'s positional correlation — reading entry N of
 * the response as the content for key N of the request — reads the wrong
 * file's bytes as this path's content from then on, with no error raised.
 *
 * `\r` is refused for the same reason even though it does not itself split a
 * line: a path most tools would never intentionally produce is already
 * exotic enough, and to date nothing in this guard needs to allow it, so
 * failing closed here costs nothing real. Other control bytes are not
 * screened — none of them break the `\n`-delimited framing this function's
 * correctness actually depends on, and screening bytes this guard has no
 * concrete failure mode for would be a check nobody could explain a year
 * from now.
 *
 * @param {string} path
 * @returns {boolean}
 */
export function breaksBatchLineProtocol(path) {
  return path.includes('\n') || path.includes('\r')
}

/**
 * Splits an index listing into paths this guard never fetches the content
 * of (`allowlisted`), paths it has to inspect further (read content for, at
 * minimum to sniff — `toInspect`), and paths it refuses to ever send to
 * `git cat-file --batch` at all because `breaksBatchLineProtocol` flags them
 * (`unsafe`). The `unsafe` bucket is checked before the allowlist: a path
 * exotic enough to carry a raw `\n` or `\r` does not get to also claim the
 * benefit of the doubt `ALLOWLISTED_PREFIXES` grants ordinary tracked
 * binaries.
 *
 * `allowlisted` is **not** "skip this path entirely" — see
 * `ALLOWLISTED_PREFIXES`'s own doc. It only excuses a path from the
 * `git cat-file --batch` round-trip and `sniffMediaKind`; the caller is still
 * required to run `identifyAllowlistedExtensionCandidates` over this bucket,
 * which needs no content to flag a declared `.aero`/`.aerotpl` extension.
 *
 * This is where ADR-0023's fail-closed default is enforced for the `\n`/`\r`
 * class of path — every path landing in `unsafe` becomes an automatic
 * violation (see `unsafePathVerdict`) before its content is ever read, rather
 * than being fed to `buildBatchRequest` and trusted.
 *
 * @param {readonly string[]} paths Full `git ls-files` listing.
 * @returns {{ allowlisted: string[], toInspect: string[], unsafe: string[] }}
 */
export function classifyIndexedPaths(paths) {
  /** @type {string[]} */
  const allowlisted = []
  /** @type {string[]} */
  const toInspect = []
  /** @type {string[]} */
  const unsafe = []
  for (const path of paths) {
    if (breaksBatchLineProtocol(path)) {
      unsafe.push(path)
      continue
    }
    ;(isAllowlisted(path) ? allowlisted : toInspect).push(path)
  }
  return { allowlisted, toInspect, unsafe }
}

/**
 * Finds the nearest **topmost** ancestor directory literally named
 * `fixtures` in a path, and returns the path up to and including it.
 *
 * Topmost rather than nearest-to-the-file: a `fixtures/` root holds exactly
 * one manifest for everything beneath it, however deep, mirroring
 * `.gitignore`'s own `!**\/fixtures/**\/*.aero` — that pattern's `/**\/`
 * matches any depth under a single `fixtures/` directory, not a fresh
 * exception per nested directory.
 *
 * @param {string} path
 * @returns {string | null} The `fixtures/` root path, or `null` if no
 *   segment of `path` is literally `fixtures`.
 */
export function findFixturesRoot(path) {
  const segments = path.split('/')
  const index = segments.indexOf(FIXTURES_DIR_NAME)
  if (index === -1) return null
  return segments.slice(0, index + 1).join('/')
}

/**
 * @param {string} fixturesRoot Return value of `findFixturesRoot`.
 * @returns {string} Repo-root-relative path of that root's manifest.
 */
export function manifestPathFor(fixturesRoot) {
  return `${fixturesRoot}/${MANIFEST_FILE_NAME}`
}

/**
 * @param {string} path A path known to sit under `fixturesRoot`.
 * @param {string} fixturesRoot Return value of `findFixturesRoot(path)`.
 * @returns {string} `path` relative to `fixturesRoot`, the key a manifest
 *   entry is looked up by.
 */
export function relativeToFixturesRoot(path, fixturesRoot) {
  return path.slice(fixturesRoot.length + 1)
}

/**
 * Magic-number signatures for the binary media/render formats ADR-0023 names
 * (`.webp` explicitly) plus the other common still-image, video and audio
 * containers a church's media library realistically holds. Order matters
 * only where one signature is a prefix of another's search space; none of
 * these are, so the list is otherwise alphabetical by kind.
 *
 * Deliberately narrow to "binary media a congregation might have produced or
 * that a render cache emits" — not a general binary-file detector. A `.pptx`
 * or `.pdf` presentation-import source fixture (FR-501, FR-509 — see
 * `docs/PRD.md` §4.5) is a different risk this function does not cover, and
 * that is a named, accepted gap rather than an oversight: ADR-0023 scoped
 * this guard to the two shapes a real church document actually takes once a
 * fixture pollutes the repo — the two `.gitignore`-unignored extensions
 * (`.aero`/`.aerotpl`, `DECLARED_EXTENSIONS`) and rendered/media artefacts.
 * A `.pptx`/`.pdf` fixture is neither of those; it is a *source* file
 * FR-501/FR-509's import pipeline consumes, not something ADR-0023's brief
 * names, and its magic bytes (`PK\x03\x04` for `.pptx`, `%PDF` for `.pdf`)
 * are not in the signature list below — so today a real PowerPoint deck or
 * PDF order-of-service staged as a fixture would sniff as `null` and pass
 * through this function unflagged. The item that first needs a real
 * PPTX/PDF import fixture is the one that has to decide whether that risk
 * belongs in this function or needs its own guard; staying silent on that
 * day would be the actual gap, not staying silent today, before such a
 * fixture exists anywhere in this repository.
 *
 * @param {Buffer} buffer
 * @returns {string | null} A short kind label, or `null` if nothing matched.
 */
export function sniffMediaKind(buffer) {
  /**
   * @param {number} offset
   * @param {readonly number[]} expected
   * @returns {boolean}
   */
  const at = (offset, expected) =>
    buffer.length >= offset + expected.length &&
    expected.every((byte, i) => buffer[offset + i] === byte)
  /**
   * @param {number} offset
   * @param {string} text
   * @returns {boolean}
   */
  const ascii = (offset, text) =>
    at(
      offset,
      [...text].map((c) => c.charCodeAt(0)),
    )

  if (at(0, [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) return 'png'
  if (at(0, [0xff, 0xd8, 0xff])) return 'jpeg'
  if (ascii(0, 'GIF87a') || ascii(0, 'GIF89a')) return 'gif'
  if (ascii(0, 'BM')) return 'bmp'
  if (at(0, [0x00, 0x00, 0x01, 0x00])) return 'ico'
  if (ascii(0, 'II*\0') || ascii(0, 'MM\0*')) return 'tiff'
  if (ascii(0, 'RIFF') && ascii(8, 'WEBP')) return 'webp'
  if (ascii(0, 'RIFF') && ascii(8, 'WAVE')) return 'wav'
  if (ascii(0, 'RIFF') && ascii(8, 'AVI ')) return 'avi'
  if (ascii(4, 'ftyp')) return 'mp4' // isobmff: mp4, mov, m4a, heic, …
  if (ascii(0, 'fLaC')) return 'flac'
  if (ascii(0, 'OggS')) return 'ogg'
  if (
    ascii(0, 'ID3') ||
    at(0, [0xff, 0xfb]) ||
    at(0, [0xff, 0xf3]) ||
    at(0, [0xff, 0xf2])
  ) {
    return 'mp3'
  }
  return null
}

/**
 * Builds the stdin payload for `git cat-file --batch`, one `:path` object
 * spec per line — `:path` (no revision before the colon) addresses stage 0
 * of `path` in the **index**, i.e. what would be committed right now,
 * matching this guard's job of judging staged content rather than the
 * working tree. Verified against git 2.47.1: a query for a path absent from
 * the index echoes the literal query text back followed by ` missing`,
 * which is what makes per-line correlation in `parseCatFileBatch` reliable
 * even for paths that turn out not to exist.
 *
 * Callers are required to have already run every path through
 * `classifyIndexedPaths` and excluded whatever landed in its `unsafe`
 * bucket — a path `breaksBatchLineProtocol` flags would silently desync the
 * line-per-query correlation `parseCatFileBatch` relies on (see that
 * function's own doc). This function does not merely assume that happened:
 * it throws rather than emit a line that would corrupt every request after
 * it, so a caller that skips the classification step fails loudly here
 * instead of reading the wrong path's content later with no error at all.
 *
 * @param {readonly string[]} paths Must not contain any path
 *   `breaksBatchLineProtocol` would flag — see above.
 * @returns {string}
 */
export function buildBatchRequest(paths) {
  return paths
    .map((path) => {
      if (breaksBatchLineProtocol(path)) {
        throw new Error(
          `fixture content guard: refusing to query git for ${JSON.stringify(path)} — it ` +
            "contains a raw \\n or \\r byte, which would corrupt this batch request's " +
            'line-based framing. Callers must filter paths through `classifyIndexedPaths` ' +
            'first and treat its `unsafe` bucket as an automatic violation (see ' +
            '`unsafePathVerdict`), never pass them here.',
        )
      }
      return `:${path}\n`
    })
    .join('')
}

/**
 * @typedef {{ status: 'ok', content: Buffer } | { status: 'missing' }} BatchEntry
 */

/**
 * Parses the raw stdout of `git cat-file --batch` fed the output of
 * `buildBatchRequest(keys)`, back into one entry per key, in the order the
 * keys were requested.
 *
 * The wire format (git 2.47.1, `--batch` with no `--batch-format`, one
 * object per requested line): a present object is a header line
 * `<sha1> <type> <size>\n` followed by exactly `<size>` bytes of content and
 * a trailing `\n`; a missing one is a single line, the original query text
 * verbatim, followed by ` missing\n`. git answers one line of stdin with
 * exactly one entry of stdout, in order — so entries can be read
 * positionally against `keys` without git ever telling us which path an
 * entry belongs to. That is why `keys` has to be passed in as the same
 * ordered array `buildBatchRequest` built the request from.
 *
 * Positional correlation is only sound because every key was written as
 * exactly one `\n`-terminated line — a precondition this function has no way
 * to check from the *response* bytes alone, since a corrupted correlation
 * looks identical to a correct one from here: still one header per line,
 * still parseable, just attributed to the wrong key. That precondition is
 * `buildBatchRequest`'s job (see its own doc and `breaksBatchLineProtocol`):
 * it throws rather than emit a line for a path that would break it, so by
 * the time this function runs against real output, every key in `keys` is
 * already known to have produced exactly one line on the wire.
 *
 * Binary-safe throughout: content is sliced from the buffer by byte length,
 * never by scanning for a delimiter, so a `.png` whose bytes happen to
 * contain `\n` is read whole.
 *
 * @param {Buffer} buffer Raw stdout of `git cat-file --batch`.
 * @param {readonly string[]} keys The `:path` query strings, in request order
 *   (without the leading `:` — this function adds it back to compare against
 *   git's missing-entry echo). Every key must already have been proven safe
 *   by `buildBatchRequest` — see that function's doc.
 * @returns {Map<string, BatchEntry>} Keyed by the plain path (no leading `:`).
 * @see buildBatchRequest
 */
export function parseCatFileBatch(buffer, keys) {
  /** @type {Map<string, BatchEntry>} */
  const results = new Map()
  let offset = 0

  for (const key of keys) {
    const headerEnd = buffer.indexOf(0x0a, offset)
    if (headerEnd === -1) {
      throw new Error(
        `fixture content guard: cat-file batch output ended before a header for "${key}" ` +
          '— git exited early or the response was truncated.',
      )
    }
    const header = buffer.toString('latin1', offset, headerEnd)
    offset = headerEnd + 1

    if (header === `:${key} missing`) {
      results.set(key, { status: 'missing' })
      continue
    }

    const match = /^([0-9a-f]{4,64}) (\S+) (\d+)$/.exec(header)
    if (!match) {
      throw new Error(
        `fixture content guard: cat-file batch header for "${key}" did not parse: "${header}"`,
      )
    }
    const size = Number(match[3])
    const content = buffer.subarray(offset, offset + size)
    if (content.length !== size) {
      throw new Error(
        `fixture content guard: cat-file batch output for "${key}" was shorter than the ` +
          `declared size (${size} bytes) — truncated response.`,
      )
    }
    offset += size

    if (buffer[offset] !== 0x0a) {
      throw new Error(
        `fixture content guard: cat-file batch output for "${key}" is missing the trailing ` +
          'newline after its content — the response is malformed or offsets drifted.',
      )
    }
    offset += 1

    results.set(key, { status: 'ok', content: Buffer.from(content) })
  }

  return results
}

/**
 * @typedef {object} Candidate
 * @property {string} path
 * @property {string} reason `'extension'`, or `'content:' + sniffMediaKind()`.
 */

/**
 * Decides which of `paths` (already filtered to non-allowlisted, per
 * `classifyIndexedPaths`) require a manifest declaration, given their staged
 * content. A path with a declared extension is always a candidate — no
 * content needed. A path without one is a candidate only when its content
 * sniffs as media; a text source file, a JSON config, a markdown doc, none
 * of that.
 *
 * @param {readonly string[]} paths
 * @param {ReadonlyMap<string, BatchEntry>} contentByPath Keyed exactly as
 *   `paths` (no leading `:`); every entry in `paths` must have an entry here.
 * @returns {{ candidates: Candidate[], unreadable: string[] }} `unreadable`
 *   holds **every** path from `paths` whose content could not be confirmed
 *   present in the index at read time (see `check-fixture-content.js` for
 *   when that can legitimately happen) — regardless of whether it carries a
 *   declared extension. An extension-matched path is still a candidate too
 *   (an `.aero` path needs no content to be judged), but a path with no
 *   declared extension and unreadable content is deliberately **not** added
 *   to `candidates`: this function has no byte to sniff, so it has no basis
 *   to claim the path *is* media requiring a manifest declaration — only
 *   that it could not be ruled out, which is exactly what `unreadable`
 *   exists to say instead. Silently dropping that case (neither bucket) was
 *   the actual bug here: a binary file whose content genuinely could not be
 *   read is not the same claim as "this file is clean text", and folding it
 *   into neither list let it read as the latter. Either way, the caller must
 *   never let "unreadable" read as "clean" — `summariseRun` fails the run on
 *   any non-empty `unreadable`, independent of `candidates`.
 */
export function identifyCandidates(paths, contentByPath) {
  /** @type {Candidate[]} */
  const candidates = []
  /** @type {string[]} */
  const unreadable = []

  for (const path of paths) {
    const entry = contentByPath.get(path)
    const declared = hasDeclaredExtension(path)

    if (!entry || entry.status !== 'ok') {
      unreadable.push(path)
      if (declared) candidates.push({ path, reason: 'extension' })
      continue
    }

    if (declared) {
      candidates.push({ path, reason: 'extension' })
      continue
    }

    const kind = sniffMediaKind(entry.content)
    if (kind) candidates.push({ path, reason: `content:${kind}` })
  }

  return { candidates, unreadable }
}

/**
 * Extension-only candidacy for the `allowlisted` bucket `classifyIndexedPaths`
 * carves out of the index — the paths this guard deliberately never fetches
 * content for, so `sniffMediaKind` is never an option here (see
 * `ALLOWLISTED_PREFIXES`'s doc for what the allowlist does and does not
 * excuse). `hasDeclaredExtension` needs no bytes, only the path string, so it
 * costs nothing to run over every allowlisted path unconditionally.
 *
 * This function has no `unreadable` output: unlike `identifyCandidates`,
 * there is no content read attempted here to fail, so there is nothing to
 * report as unread. A path returned as a candidate here still goes through
 * `evaluateCandidate` exactly like any other — including the requirement
 * that it sit under a `fixtures/` directory with a manifest declaring it —
 * which is precisely what makes a real `.aero` staged at
 * `src-tauri/icons/service.aero` (no `fixtures/` ancestor at all) an
 * unconditional rejection: `findFixturesRoot` returns `null` for it.
 *
 * @param {readonly string[]} allowlistedPaths Return value of
 *   `classifyIndexedPaths(...).allowlisted`.
 * @returns {Candidate[]}
 */
export function identifyAllowlistedExtensionCandidates(allowlistedPaths) {
  /** @type {Candidate[]} */
  const candidates = []
  for (const path of allowlistedPaths) {
    if (hasDeclaredExtension(path)) candidates.push({ path, reason: 'extension' })
  }
  return candidates
}

/**
 * @typedef {object} ManifestVerdict
 * @property {boolean} ok
 * @property {string} [reason] Present when `ok` is `false`.
 */

/**
 * Judges one manifest's declaration for one fixture path. This is the
 * positive-proof check ADR-0023 asks for: `ok` only when the manifest
 * explicitly, non-trivially claims the file is synthetic — absence, a
 * malformed manifest, `synthetic: false` and a placeholder `purpose` are all
 * rejections, not passes. There is no code path here that returns `ok: true`
 * because something was merely absent or unreadable.
 *
 * The 20-character floor on `purpose` cannot prove the sentence is true —
 * nothing here can — but it does mean rubber-stamping a real file takes a
 * deliberate, readable lie in the same diff as the file itself, rather than
 * copying a one-word flag. That is the same trade `hasModuleScript` in
 * `dist-html-guard.js` makes when it accepts any non-empty `src`: it cannot
 * confirm the module is *correct*, only that a real claim was made.
 *
 * @param {string} manifestText Raw JSON text of a `fixtures.manifest.json`.
 * @param {string} relPath The candidate's path relative to the fixtures root
 *   the manifest belongs to.
 * @returns {ManifestVerdict}
 */
export function evaluateManifestEntry(manifestText, relPath) {
  /** @type {unknown} */
  let parsed
  try {
    parsed = JSON.parse(manifestText)
  } catch (error) {
    return { ok: false, reason: `manifest is not valid JSON (${String(error)})` }
  }

  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    return { ok: false, reason: 'manifest root is not a JSON object' }
  }
  const entries = /** @type {Record<string, unknown>} */ (parsed).entries
  if (typeof entries !== 'object' || entries === null || Array.isArray(entries)) {
    return { ok: false, reason: 'manifest has no "entries" object' }
  }

  const entry = /** @type {Record<string, unknown>} */ (entries)[relPath]
  if (typeof entry !== 'object' || entry === null || Array.isArray(entry)) {
    return {
      ok: false,
      reason:
        `no entry for "${relPath}" in manifest.entries — every fixture under this ` +
        'directory needs its own declaration',
    }
  }
  const { synthetic, purpose } = /** @type {Record<string, unknown>} */ (entry)
  if (synthetic !== true) {
    return {
      ok: false,
      reason: `manifest.entries["${relPath}"].synthetic is not literally \`true\``,
    }
  }
  const purposeText = typeof purpose === 'string' ? purpose.trim() : ''
  if (purposeText.length < 20) {
    return {
      ok: false,
      reason:
        `manifest.entries["${relPath}"].purpose is missing or too short (< 20 chars) — ` +
        'state which NFR/FR the fixture serves and how it was produced',
    }
  }

  return { ok: true }
}

/**
 * @typedef {object} CandidateVerdict
 * @property {string} path
 * @property {boolean} ok
 * @property {string} [reason]
 */

/**
 * Full verdict for one candidate: locate its `fixtures/` root, locate that
 * root's manifest in the already-fetched content map, and judge its entry.
 *
 * Every failure branch is a rejection — there is no branch that treats a
 * missing manifest, or a candidate with no `fixtures/` ancestor at all, as
 * anything other than a violation. A candidate outside any `fixtures/`
 * directory has no possible declaration site and is therefore rejected
 * unconditionally, which is what closes `git add -f` landing a real `.aero`
 * (or a real photo) anywhere else in the tree.
 *
 * @param {Candidate} candidate
 * @param {ReadonlyMap<string, BatchEntry>} contentByPath
 * @returns {CandidateVerdict}
 */
export function evaluateCandidate(candidate, contentByPath) {
  const fixturesRoot = findFixturesRoot(candidate.path)
  if (fixturesRoot === null) {
    return {
      path: candidate.path,
      ok: false,
      reason:
        `not under any directory literally named "${FIXTURES_DIR_NAME}", so there is no ` +
        'manifest it could be declared in. Move it under a fixtures/ tree, or do not stage it.',
    }
  }

  const manifestPath = manifestPathFor(fixturesRoot)
  const manifestEntry = contentByPath.get(manifestPath)
  if (!manifestEntry || manifestEntry.status !== 'ok') {
    return {
      path: candidate.path,
      ok: false,
      reason: `"${manifestPath}" is not in the index — create it and declare this file before staging`,
    }
  }

  const relPath = relativeToFixturesRoot(candidate.path, fixturesRoot)
  const verdict = evaluateManifestEntry(manifestEntry.content.toString('utf8'), relPath)
  if (!verdict.ok) {
    return {
      path: candidate.path,
      ok: false,
      reason: `${manifestPath}: ${verdict.reason}`,
    }
  }
  return { path: candidate.path, ok: true }
}

/**
 * Automatic-rejection verdict for a path `classifyIndexedPaths` put in its
 * `unsafe` bucket. Produced without ever calling `buildBatchRequest` for the
 * path, so the guard never has to trust content read under a correlation
 * `breaksBatchLineProtocol` has already said cannot be relied on.
 *
 * @param {string} path
 * @returns {CandidateVerdict}
 */
export function unsafePathVerdict(path) {
  return {
    path,
    ok: false,
    reason:
      "path contains a raw \\n or \\r byte. `git cat-file --batch`'s stdin protocol is " +
      'line-delimited, so a path like this would desynchronise the request/response ' +
      "correlation this guard's content sniffing depends on (see `buildBatchRequest` / " +
      '`parseCatFileBatch`) — rejected without its content ever being read, per ' +
      "ADR-0023's fail-closed default.",
  }
}

/**
 * @param {string} mode Git index mode, e.g. `'100644'`, `'120000'`,
 *   `'160000'` — the first field of a `git ls-files -z -s` record (see
 *   `parseLsFilesEntry`).
 * @returns {boolean} Whether `mode` marks a submodule (gitlink) entry rather
 *   than a blob or symlink.
 */
export function isGitlinkMode(mode) {
  return mode === '160000'
}

/**
 * Parses one NUL-delimited record from `git ls-files -z -s`:
 * `<mode> <sha1> <stage>\t<path>`. `check-fixture-content.js` reads the index
 * this way — rather than plain `git ls-files -z`, which drops the mode field
 * entirely — specifically so a submodule (gitlink, mode `160000`) can be
 * told apart from an ordinary tracked file before this guard decides how to
 * treat it (see `isGitlinkMode` and `gitlinkVerdict`, ADR-0023's W3
 * finding). `path` is taken verbatim to end-of-record: a record embedding a
 * raw `\n` or `\r` in its path is still parsed correctly here (NUL, not
 * newline, is the record separator `-z` gives us), and it is
 * `breaksBatchLineProtocol`/`classifyIndexedPaths` downstream, not this
 * function, whose job is to refuse to act on that later.
 *
 * @param {string} record One record, already split on NUL and non-empty.
 * @returns {{ mode: string, path: string } | null} `null` if `record` does
 *   not match the expected shape (defensive only — every record
 *   `git ls-files -z -s` emits matches it; a stray malformed record is
 *   treated as a hard failure by the caller, not silently dropped).
 */
export function parseLsFilesEntry(record) {
  const match = /^([0-7]+) [0-9a-f]{4,64} \d+\t([\s\S]*)$/.exec(record)
  if (!match) return null
  const [, mode, path] = match
  // Unreachable in practice — a successful match against this pattern always
  // populates both groups (the second may be an empty string, never
  // `undefined`) — but `noUncheckedIndexedAccess` cannot see that, and
  // narrowing explicitly here is cheaper than an `as` cast that would hide a
  // real regression if the pattern above ever changed.
  if (mode === undefined || path === undefined) return null
  return { mode, path }
}

/**
 * Automatic-rejection verdict for a submodule (gitlink) entry. A gitlink's
 * actual content lives in another repository's object database, not this
 * one — verified empirically (git 2.47.1, both a checked-out and an
 * uninitialised submodule): a query for the gitlink path via
 * `git cat-file --batch` comes back `missing` every time, because the
 * commit object it points at is never present in the superproject's own
 * `.git/objects`, only inside the submodule's own separate clone. Left
 * alone, that "missing, no declared extension" shape is exactly what
 * `identifyCandidates` silently drops — neither a candidate nor
 * `unreadable` — so a submodule would pass through this guard unnoticed by
 * omission, in a directory named `fixtures/` or anywhere else.
 * `git ls-files --recurse-submodules` was considered and rejected as the
 * fix: it only descends into a submodule that happens to be checked out on
 * the machine running this guard, so the same repository would be "clean"
 * on a developer's machine that ran `git submodule update --init` and
 * silently blind on a fresh clone or CI that did not — a machine-dependent
 * gate, the exact class ADR-0020 already proved expensive. Rejecting the
 * gitlink entry itself, unconditionally, is fail-closed and
 * machine-independent instead: AeroWorship does not use git submodules
 * today, and the day one is proposed it needs a real, reviewed answer for
 * how this guard treats it, not silent passage through a checker that
 * assumed submodules would never exist.
 *
 * @param {string} path
 * @returns {CandidateVerdict}
 */
export function gitlinkVerdict(path) {
  return {
    path,
    ok: false,
    reason:
      'this is a git submodule (mode 160000) — its content lives in a separate repository ' +
      'this guard cannot read, so it is rejected unconditionally rather than silently ' +
      'skipped. AeroWorship does not use git submodules; if one is genuinely needed, this ' +
      'guard needs a real mechanism for it before the submodule is committed.',
  }
}

/**
 * Folds a full run into counts and the exit code the wrapper returns —
 * lives here, not in the script, for the same ADR-0020 reason
 * `summariseDocuments` does in `dist-html-guard.js`: the line that decides
 * when this guard is allowed to say "clean" is the one line a deleted test
 * would never catch regressing.
 *
 * Three shapes, deliberately kept distinct rather than folded into a single
 * boolean, because they are different findings about *this run* and not just
 * about the fixtures it found:
 *
 *   - `indexedCount === 0` never happens in a git repository (`.git` itself
 *     is never tracked) and would mean git could not be asked at all — see
 *     the script for how that is turned into a hard failure before this
 *     function is ever reached.
 *   - Zero candidates is success **and** distinguishable from a run that
 *     never happened: `checkedCount` in the return value is the proof this
 *     scanned something, even when nothing needed a declaration. Compare
 *     `dist-html-guard.js`'s `documents.length === 0` guard, which exists for
 *     exactly the opposite reason (`dist/` must never be empty); `fixtures/`
 *     is allowed to be, today.
 *   - Any `unreadable` entry fails the run regardless of how many
 *     `violations` there are or are not — a candidate this guard could not
 *     actually verify must never be reported as clean by omission.
 *
 * @param {{ indexedCount: number, allowlistedCount: number,
 *   candidateVerdicts: readonly CandidateVerdict[], unreadable: readonly string[] }} run
 * @returns {{ exitCode: number, checkedCount: number, violationCount: number,
 *   unreadableCount: number }}
 */
export function summariseRun(run) {
  const violationCount = run.candidateVerdicts.filter((v) => !v.ok).length
  const unreadableCount = run.unreadable.length
  const exitCode = violationCount > 0 || unreadableCount > 0 ? 1 : 0
  return {
    exitCode,
    checkedCount: run.candidateVerdicts.length,
    violationCount,
    unreadableCount,
  }
}

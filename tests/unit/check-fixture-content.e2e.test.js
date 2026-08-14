// @ts-check

/**
 * Proves `check-fixture-content.js` — the actual `pretest` entry point, not a
 * reassembly of its pure functions — rejects real staged content, and
 * distinguishes "nothing to declare" from "something could not be verified".
 *
 * ADR-0023's brief for SETUP-05 (see PROGRESS.md, SETUP-05) records that the
 * six rejection scenarios were run once by hand against this repository's own
 * working tree and then cleaned up, leaving no reproducible proof. That is
 * exactly the failure this file exists to close: every scenario below is a
 * permanent, git-backed test, not a transcript.
 *
 * `check-fixture-content.js` resolves the repository root from its own file
 * location (`fileURLToPath(new URL('..', import.meta.url))`), by design — see
 * its own file doc — so it can never be pointed at a different repository via
 * an argument or environment variable. The only way to run the real,
 * unmodified entry point against content this suite controls is to copy both
 * `check-fixture-content.js` and `fixture-content-guard.js` into a throwaway
 * git repository and invoke the copy there; `import.meta.url` then resolves
 * to that copy's own location, so `root` becomes the throwaway repo. This is
 * not a workaround for a bug — it is the direct, documented consequence of a
 * design choice this file takes at face value — but it does mean a full
 * black-box run of the real CLI is only possible this way, and is the reason
 * this file exists at all rather than only extending the implementer's
 * pure-function suite in `scripts/fixture-content-guard.test.js`.
 *
 * Timing budget follows the precedent in `tests/unit/gitignore-guard.test.js`:
 * a per-`describe` `timeout`, not `vitest.config.ts`'s global default, sized
 * from measurements on this machine (see the constant below) rather than
 * assumed. Every case here spawns at least one `node` process (which itself
 * spawns one or two `git` processes internally) plus one or two `git`
 * processes of the harness's own — on this machine that is 1.3–1.5 s per
 * scenario purely in process-start overhead, before anything being tested
 * runs, matching the order of magnitude gitignore-guard.test.js's own doc
 * records for this environment.
 *
 * `describe(name, { timeout }, fn)` only sets the default `testTimeout` for
 * `it`s inside that suite — Vitest tracks `hookTimeout` as a wholly separate
 * budget, defaulting to 10 s, and nothing propagates the `describe`-level
 * option to it. Every `beforeAll` below does real work — `git init`, one or
 * more `git add`/`update-index`/`submodule add` calls, in one case cloning a
 * second throwaway repository — and on this machine the slowest of them (the
 * `git submodule add` pair) has been observed past 10 s on its own even
 * without another process competing for the disk, so the untouched 10 s
 * default was the actual cause of every `beforeAll` timeout this file's own
 * SETUP-05 history records (see PROGRESS.md, siklus 2 tester entry, "13
 * skip"). `GIT_TIMEOUT_MS` is therefore threaded as the explicit second
 * argument to every `beforeAll` call in this file too
 * (`beforeAll(fn, timeout)` — the documented signature,
 * `@vitest/runner`'s `tasks.d.ts`), not left to the suite-level `timeout`
 * option to (not) cover it. See `tests/unit/setup05-hook-timeout-control.test.js`
 * for the control experiment proving this actually takes effect rather than
 * being silently ignored the way an unpropagated option would be.
 */

import { execFileSync } from 'node:child_process'
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')
const GUARD_SRC = join(ROOT, 'scripts/fixture-content-guard.js')
const CHECK_SRC = join(ROOT, 'scripts/check-fixture-content.js')

/** See file doc: measured, not assumed, per ADR-0020's own standard. */
const GIT_TIMEOUT_MS = 90_000

/**
 * @param {string} cwd
 * @param {string[]} args
 * @param {string} [input]
 * @param {NodeJS.ProcessEnv} [env]
 * @returns {string}
 */
function git(cwd, args, input, env) {
  return execFileSync('git', args, {
    cwd,
    input,
    encoding: 'utf8',
    env: env ?? process.env,
  })
}

/**
 * Builds a throwaway git repository carrying a real copy of the guard, ready
 * to have candidate files staged into it.
 *
 * @param {string} label
 * @returns {string} absolute path to the new repo
 */
function createGuardRepo(label) {
  const repo = mkdtempSync(join(tmpdir(), `aeroworship-fixture-guard-${label}-`))
  mkdirSync(join(repo, 'scripts'), { recursive: true })
  copyFileSync(GUARD_SRC, join(repo, 'scripts/fixture-content-guard.js'))
  copyFileSync(CHECK_SRC, join(repo, 'scripts/check-fixture-content.js'))
  git(repo, ['-c', 'init.defaultBranch=main', 'init', '-q', '.'])
  // Silences the "LF will be replaced by CRLF" advisory noise on Windows so
  // stderr assertions below only ever see this guard's own output, and
  // removes a source of platform-dependent flake in what gets staged.
  git(repo, ['config', 'core.autocrlf', 'false'])
  git(repo, ['config', 'core.safecrlf', 'false'])
  return repo
}

/**
 * Writes `relPath` under `repo` (creating parent directories) and stages it.
 *
 * @param {string} repo
 * @param {string} relPath
 * @param {string | Buffer} content
 */
function stage(repo, relPath, content) {
  const abs = join(repo, relPath)
  mkdirSync(dirname(abs), { recursive: true })
  writeFileSync(abs, content)
  git(repo, ['add', '--', relPath])
}

/**
 * Runs the real, copied `check-fixture-content.js` and returns its verdict.
 * Never throws on a non-zero exit — a rejection is the expected outcome of
 * more than half the scenarios below, not a harness failure.
 *
 * @param {string} repo
 * @param {NodeJS.ProcessEnv} [env]
 * @returns {{ status: number, stdout: string, stderr: string }}
 */
function runGuard(repo, env) {
  try {
    const stdout = execFileSync(process.execPath, ['scripts/check-fixture-content.js'], {
      cwd: repo,
      encoding: 'utf8',
      env: env ?? process.env,
    })
    return { status: 0, stdout, stderr: '' }
  } catch (error) {
    const failure =
      /** @type {NodeJS.ErrnoException & { status?: number, stdout?: string, stderr?: string } } */ (
        error
      )
    if (typeof failure.status !== 'number') throw error
    return {
      status: failure.status,
      stdout: typeof failure.stdout === 'string' ? failure.stdout : '',
      stderr: typeof failure.stderr === 'string' ? failure.stderr : '',
    }
  }
}

/**
 * Groups the guard's stderr into the path → reason(s) pairing it actually
 * printed, so an assertion can name *which* path carried *which* reason
 * instead of counting phrases in one undifferentiated blob (a count is blind
 * to attribution: two mentions in one path's reason and none in the other's
 * totals the same as one each).
 *
 * The shape read here is the one `check-fixture-content.js` emits for every
 * failing path — one `console.error` per path, `fixture content guard: <path>`
 * then the reason on the next line indented by four spaces. Both a violation
 * verdict and an `unreadable` entry print in exactly that shape and are
 * deliberately not separated here: telling those two apart is the *reason
 * text's* job, and a caller that wants only one of them must say which by
 * asserting on the reason — precisely the distinction the gitlink suite below
 * depends on. The run's trailing summary paragraphs share the
 * `fixture content guard: ` prefix but have no indented continuation line, so
 * the indent requirement is what excludes them.
 *
 * @param {string} stderr
 * @returns {Map<string, string[]>} Reason text (trimmed) per path, in the
 *   order printed; a path is a key only if the guard actually reported it.
 */
function reasonsByPath(stderr) {
  /** @type {Map<string, string[]>} */
  const byPath = new Map()
  const lines = stderr.split(/\r?\n/)
  for (let i = 0; i < lines.length - 1; i += 1) {
    const header = /^fixture content guard: (.*)$/.exec(lines[i] ?? '')
    if (!header) continue
    const continuation = lines[i + 1] ?? ''
    if (!continuation.startsWith('    ')) continue
    const path = header[1] ?? ''
    byPath.set(path, [...(byPath.get(path) ?? []), continuation.trim()])
  }
  return byPath
}

/**
 * Restores `repo` to just the two script files: deletes every listed path
 * from disk and syncs the index to match, so the next scenario starts clean
 * without paying for a fresh `git init`.
 *
 * @param {string} repo
 * @param {string[]} relPaths
 */
function cleanup(repo, relPaths) {
  for (const relPath of relPaths) {
    rmSync(join(repo, relPath), { force: true })
  }
  git(repo, ['add', '-A', '.'])
}

/** A manifest entry long enough to clear the 20-character purpose floor. */
const SYNTHETIC_PURPOSE =
  'Hand-authored by the tester for SETUP-05, never real order-of-service content.'

/**
 * Writes `content` straight into the object database without ever touching
 * the working tree, and returns its blob SHA.
 *
 * @param {string} repo
 * @param {string | Buffer} content
 * @returns {string}
 */
function hashObject(repo, content) {
  return execFileSync('git', ['hash-object', '-w', '--stdin'], {
    cwd: repo,
    input: content,
    encoding: 'utf8',
  }).trim()
}

/**
 * Adds one index entry directly, bypassing the working tree entirely. This
 * is the only way to stage a path containing a raw `\n` or `\r` on Windows:
 * `writeFileSync` cannot create a file whose name embeds a control byte
 * (Win32 rejects it below the filesystem git itself would accept), so
 * `stage()` above is not an option for the C1 regression cases below. Git's
 * own path restriction is narrower — only `/` and NUL are structurally
 * forbidden — but `core.protectNTFS` / `core.protectHFS` default to `true`
 * on Windows and additionally refuse `update-index --cacheinfo` for a path
 * carrying `\n` or `\r`; both default `false` on Linux/macOS, which is where
 * CI runs. Passed unconditionally here (harmless where the defaults already
 * allow it) rather than branched on `process.platform`, so this helper does
 * the same thing everywhere it runs.
 *
 * @param {string} repo
 * @param {string} mode e.g. `'100644'`, `'160000'`
 * @param {string} sha
 * @param {string} path May contain `\n` / `\r` — passed as a single argv
 *   element via `execFileSync`, never through a shell, so no escaping hazard.
 */
function cacheAdd(repo, mode, sha, path) {
  git(repo, [
    '-c',
    'core.protectNTFS=false',
    '-c',
    'core.protectHFS=false',
    'update-index',
    '--add',
    '--cacheinfo',
    `${mode},${sha},${path}`,
  ])
}

/**
 * Removes an index-only entry added by `cacheAdd`, without touching the
 * working tree (there is nothing on disk to remove).
 *
 * @param {string} repo
 * @param {string} path
 */
function cacheRemove(repo, path) {
  git(repo, ['update-index', '--force-remove', '--', path])
}

describe('rejection scenarios (ADR-0023 P0)', { timeout: GIT_TIMEOUT_MS }, () => {
  /** @type {string} */
  let repo

  beforeAll(() => {
    repo = createGuardRepo('rejections')
  }, GIT_TIMEOUT_MS)

  afterAll(() => {
    rmSync(repo, { recursive: true, force: true })
  })

  it('rejects a .aero under fixtures/ with no manifest at all', () => {
    const path = 'tests/integration/fixtures/case1/service.aero'
    stage(repo, path, '{"schema_version":1}')
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(path)
      expect(result.stderr).toMatch(/fixtures\.manifest\.json" is not in the index/)
    } finally {
      cleanup(repo, [path])
    }
  })

  it('rejects a .aerotpl under fixtures/ with no manifest — the second declared extension', () => {
    // ADR-0023 calls out .aerotpl by name specifically because a guard that
    // only checks .aero was the exact failure it was written to prevent.
    const path = 'tests/integration/fixtures/case2/template.aerotpl'
    stage(repo, path, '{"schema_version":1}')
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(path)
      expect(result.stderr).toMatch(/fixtures\.manifest\.json" is not in the index/)
    } finally {
      cleanup(repo, [path])
    }
  })

  it('rejects a .aero outside any fixtures/ directory unconditionally — the git add -f shape', () => {
    const path = 'smuggled-case3.aero'
    stage(repo, path, '{"schema_version":1}')
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/not under any directory literally named "fixtures"/)
    } finally {
      cleanup(repo, [path])
    }
  })

  it('rejects a real PNG by its magic bytes even with an innocuous, unrelated extension', () => {
    // The extension is deliberately something nobody would flag by eye.
    const path = 'docs/notes-case4.txt'
    const pngBytes = Buffer.concat([
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
      Buffer.from('not really png data after the header, just bytes'),
    ])
    stage(repo, path, pngBytes)
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(path)
      // Outside fixtures/, so the rejection reason is the unconditional one —
      // proof the guard reached this file by content, since its extension
      // (.txt) is not one it scans for.
      expect(result.stderr).toMatch(/not under any directory literally named "fixtures"/)
    } finally {
      cleanup(repo, [path])
    }
  })

  it('rejects the same PNG-by-content candidate inside fixtures/ too, when undeclared', () => {
    const path = 'tests/integration/fixtures/case4b/render.dat'
    const pngBytes = Buffer.from([
      0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0,
    ])
    stage(repo, path, pngBytes)
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(path)
      expect(result.stderr).toMatch(/fixtures\.manifest\.json" is not in the index/)
    } finally {
      cleanup(repo, [path])
    }
  })

  // `findFixturesRoot` walks to the TOPMOST ancestor segment literally named
  // "fixtures" (see fixture-content-guard.js), so the manifest that governs a
  // file lives at the root of the nearest enclosing `fixtures/` directory —
  // not beside the file in some subdirectory under it. Each case below gives
  // itself its own `fixtures/` root (a distinct parent, e.g. `case5a/`, ahead
  // of the literal `fixtures` segment) precisely so its manifest is isolated
  // from every other case's, and stages files directly inside that root so
  // the manifest's `entries` keys are the plain filenames.

  it('rejects synthetic: false even alongside a long, well-formed purpose', () => {
    const root = 'tests/integration/case5a/fixtures'
    stage(repo, `${root}/lie.aero`, '{"schema_version":1}')
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({
        entries: { 'lie.aero': { synthetic: false, purpose: SYNTHETIC_PURPOSE } },
      }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/synthetic is not literally `true`/)
    } finally {
      cleanup(repo, [`${root}/lie.aero`, `${root}/fixtures.manifest.json`])
    }
  })

  it('rejects synthetic given as the string "true" rather than the boolean', () => {
    const root = 'tests/integration/case5b/fixtures'
    stage(repo, `${root}/stringy.aero`, '{"schema_version":1}')
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({
        entries: { 'stringy.aero': { synthetic: 'true', purpose: SYNTHETIC_PURPOSE } },
      }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/synthetic is not literally `true`/)
    } finally {
      cleanup(repo, [`${root}/stringy.aero`, `${root}/fixtures.manifest.json`])
    }
  })

  it('rejects a purpose under the 20-character floor', () => {
    const root = 'tests/integration/case5c/fixtures'
    stage(repo, `${root}/short.aero`, '{"schema_version":1}')
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({
        entries: { 'short.aero': { synthetic: true, purpose: 'too short' } },
      }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/purpose is missing or too short/)
    } finally {
      cleanup(repo, [`${root}/short.aero`, `${root}/fixtures.manifest.json`])
    }
  })

  it('rejects an empty-string purpose', () => {
    const root = 'tests/integration/case5d/fixtures'
    stage(repo, `${root}/empty.aero`, '{"schema_version":1}')
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({ entries: { 'empty.aero': { synthetic: true, purpose: '' } } }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/purpose is missing or too short/)
    } finally {
      cleanup(repo, [`${root}/empty.aero`, `${root}/fixtures.manifest.json`])
    }
  })

  it('rejects a whitespace-only purpose', () => {
    const root = 'tests/integration/case5e/fixtures'
    stage(repo, `${root}/blank.aero`, '{"schema_version":1}')
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({
        entries: { 'blank.aero': { synthetic: true, purpose: '                    ' } },
      }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/purpose is missing or too short/)
    } finally {
      cleanup(repo, [`${root}/blank.aero`, `${root}/fixtures.manifest.json`])
    }
  })

  it('rejects a candidate the manifest never mentions, even though the manifest exists and is otherwise well-formed', () => {
    const root = 'tests/integration/case5f/fixtures'
    stage(repo, `${root}/undeclared.aero`, '{"schema_version":1}')
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({
        entries: {
          'some-other-file.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE },
        },
      }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/no entry for "undeclared\.aero"/)
    } finally {
      cleanup(repo, [`${root}/undeclared.aero`, `${root}/fixtures.manifest.json`])
    }
  })
})

describe('the passing case (ADR-0023 P0.6)', { timeout: GIT_TIMEOUT_MS }, () => {
  /** @type {string} */
  let repo

  beforeAll(() => {
    repo = createGuardRepo('pass')
  }, GIT_TIMEOUT_MS)

  afterAll(() => {
    rmSync(repo, { recursive: true, force: true })
  })

  it('passes a .aero, a .aerotpl and a content-sniffed media file, all correctly declared in one manifest', () => {
    // Files staged directly inside the fixtures/ root itself (rather than a
    // subdirectory beneath it), so the manifest's entry keys are plain
    // filenames — see the note above the case5* block for why nesting a
    // manifest under a subdirectory of fixtures/ does not work.
    const root = 'tests/integration/case6/fixtures'
    const aeroPath = `${root}/service.aero`
    const aerotplPath = `${root}/template.aerotpl`
    const webpPath = `${root}/render.webp`
    const paths = [aeroPath, aerotplPath, webpPath]
    stage(repo, aeroPath, '{"schema_version":1}')
    stage(repo, aerotplPath, '{"schema_version":1}')
    stage(
      repo,
      webpPath,
      Buffer.concat([
        Buffer.from('RIFF'),
        Buffer.from([0, 0, 0, 0]),
        Buffer.from('WEBP'),
      ]),
    )
    stage(
      repo,
      `${root}/fixtures.manifest.json`,
      JSON.stringify({
        entries: {
          'service.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE },
          'template.aerotpl': { synthetic: true, purpose: SYNTHETIC_PURPOSE },
          'render.webp': { synthetic: true, purpose: SYNTHETIC_PURPOSE },
        },
      }),
    )
    try {
      const result = runGuard(repo)
      expect(result.status).toBe(0)
      expect(result.stdout).toMatch(/3 candidate\(s\) verified synthetic, clean\./)
    } finally {
      cleanup(repo, [...paths, `${root}/fixtures.manifest.json`])
    }
  })
})

describe(
  'each candidate is judged on ITS OWN staged bytes, for names that read like revisions (siklus 6)',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    // The clause siklus 6 added to `buildBatchRequest`'s contract: the object
    // spec on each line must address a literal *path*, not a revision. The
    // shape that broke it — a path beginning `0:`…`3:`, which `gitrevisions`
    // reads as a stage selector — cannot be staged by Git for Windows at all
    // (re-measured for this cycle: `update-index --cacheinfo` says
    // `error: Invalid path '0:notes.dat'`, `--index-info` says
    // `Ignoring path 0:notes.dat`, and `core.protectNTFS=false` does not open
    // either, because `has_dos_drive_prefix` treats an alphanumeric run before
    // `:` as a drive prefix), so it is proved as a pure property instead — see
    // `scripts/fixture-content-guard.extra4.test.js`.
    //
    // What *can* be staged here, on any platform, is the rest of the class:
    // names that look like `gitrevisions` syntax in every other way. They are
    // proved against real git rather than a model, and the assertion is per
    // path — `HEAD` is a candidate only if the guard sniffed the bytes staged
    // at the path `HEAD`, and the declared file passes only if its own
    // manifest key was matched.
    it('reads `HEAD`, `a@{0}.aero` and `c~1.aero` as paths, not as revisions', () => {
      const repo = createGuardRepo('rev-lookalike-names')
      const root = 'tests/integration/fixtures'
      const declared = `${root}/revnames/a@{0}.aero`
      const undeclared = `${root}/revnames/c~1.aero`
      // No declared extension and no magic number: this one is a candidate
      // only via `looksLikeAeroSession` (ADR-0030), i.e. only if the guard
      // really did read the blob staged at the path literally named `HEAD`.
      const head = `${root}/revnames/HEAD`
      try {
        stage(repo, declared, '{"schema_version":1,"note":"SYNTHETIC"}')
        stage(repo, undeclared, '{"schema_version":1,"note":"SYNTHETIC"}')
        stage(
          repo,
          head,
          '{"kind":"aeroworship.session","schema_version":1,' +
            '"title":"SYNTHETIC — invented for this test, never a real service"}',
        )
        stage(
          repo,
          `${root}/fixtures.manifest.json`,
          JSON.stringify({
            entries: {
              'revnames/a@{0}.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE },
            },
          }),
        )

        const result = runGuard(repo)
        expect(result.status).not.toBe(0)

        const reasons = reasonsByPath(result.stderr)
        expect([...reasons.keys()].sort()).toEqual([head, undeclared].sort())
        expect(reasons.get(head)?.[0]).toMatch(/no entry for "revnames\/HEAD"/)
        expect(reasons.get(undeclared)?.[0]).toMatch(/no entry for "revnames\/c~1\.aero"/)
        // The declared one is not among the rejections, and the count proves
        // it was judged rather than skipped: three candidates, two rejected.
        expect(result.stderr).toContain('2 of 3 candidate(s) rejected.')
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })
  },
)

describe('index-visibility failure modes (P1 #7)', { timeout: GIT_TIMEOUT_MS }, () => {
  it('treats a genuinely empty index as a hard failure, not as "0 candidates found"', () => {
    // A fresh repository with nothing ever staged — the guard must not
    // conflate "there is nothing to check" with "the index could not be
    // read", because both look identical from the caller's side unless the
    // guard actively distinguishes them.
    const repo = createGuardRepo('empty-index')
    try {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/nothing was verified/)
      expect(result.stdout).not.toMatch(/0 candidate\(s\) found/)
    } finally {
      rmSync(repo, { recursive: true, force: true })
    }
  })

  it('fails a candidate whose blob cannot be read from the object database, even with a correct manifest present', () => {
    // Simulates "the index changed mid-scan" (or a corrupted/incomplete
    // object store) the honest way: stage a real .aero, declare it correctly
    // in its manifest, confirm the baseline actually passes, then delete the
    // blob object git wrote for it and re-run. `git ls-files` still lists the
    // path (the index entry is untouched); `git cat-file --batch` can no
    // longer produce its content. The guard must not report this as clean by
    // omission — it is exactly the "unreadable" case
    // `fixture-content-guard.js`'s own doc says must never look like clean.
    const repo = createGuardRepo('unreadable-blob')
    try {
      const path = 'tests/integration/vanishing/fixtures/service.aero'
      stage(repo, path, '{"schema_version":1}')
      stage(
        repo,
        'tests/integration/vanishing/fixtures/fixtures.manifest.json',
        JSON.stringify({
          entries: { 'service.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE } },
        }),
      )

      const baseline = runGuard(repo)
      expect(baseline.status, 'baseline before corrupting the blob must pass').toBe(0)

      const sha = git(repo, ['rev-parse', `:${path}`]).trim()
      const objectPath = join(repo, '.git', 'objects', sha.slice(0, 2), sha.slice(2))
      rmSync(objectPath, { force: true })

      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toContain(path)
      expect(result.stderr).toMatch(/could not be confirmed present in the index/)
      expect(result.stdout).not.toMatch(/clean\./)
    } finally {
      rmSync(repo, { recursive: true, force: true })
    }
  })
})

describe(
  "C1 regression — an embedded \\n path never borrows another entry's content (ADR-0023 siklus 2, auditor C1)",
  { timeout: GIT_TIMEOUT_MS },
  () => {
    /**
     * Reproduces the exact adversarial shape the implementer used to prove C1
     * before/after (see PROGRESS.md, SETUP-05 siklus 2 entry): a single index
     * entry whose *path string* is the real manifest's own path, followed by a
     * raw `\n`, followed by an unrelated tail. Under the pre-fix guard,
     * `buildBatchRequest` wrote such a path onto its own stdin line unchecked
     * — as `` `:${path}\n` `` at the time, as `` `:0:${path}\n` `` since
     * siklus 6 spelled the stage digit out (`BATCH_SPEC_PREFIX`); the desync
     * does not depend on which prefix it is. Either way that one "line" is
     * really two, the first byte-for-byte identical to the query for the real
     * manifest entry sitting earlier in the same batch. `git cat-file
     * --batch` answers one line with one entry, so from that point on every
     * entry `parseCatFileBatch` read was shifted by one and attributed to the
     * wrong key. The fix (`classifyIndexedPaths`'s `unsafe` bucket) never lets
     * a path like this reach `buildBatchRequest` at all — this suite proves
     * that end-to-end, against the real CLI, not by re-deriving the pure
     * functions' behaviour in isolation (`fixture-content-guard.test.js`
     * already does that).
     *
     * Content used below is synthetic filler text invented for this test only
     * — never a real hymn, lyric, or order-of-service fragment — precisely
     * because ADR-0023 exists to keep real congregation content out of this
     * repository even inside a test that is *about* smuggling.
     */
    /** @type {string} */
    let repo
    const root = 'tests/integration/casec1/fixtures'
    const aeroPath = `${root}/service.aero`
    const manifestPath = `${root}/fixtures.manifest.json`
    const laterAeroPath = `${root}/zzz-after-the-malicious-entry.aero`
    // An UNdeclared content-sniffed candidate (webp magic bytes, no extension
    // that would flag it on its own) sorted right after the malicious entry.
    // This is the specific failure shape from the implementer's own before/
    // after transcript: a real media file's content getting silently
    // misattributed to something that does *not* sniff as media means it never
    // becomes a candidate at all — which reads as "clean" by omission, not as
    // a rejection. A structural exclusion (the `unsafe` bucket) has to keep
    // this file's positional slot untouched for it to still be caught.
    const undeclaredMediaPath = `${root}/render-case-never-declared.bin`
    // Lexicographically identical to `manifestPath` up to the embedded `\n`,
    // then a tail that never appears anywhere else in the index — the shape
    // that made the pre-fix guard misattribute the real manifest's content to
    // this path instead of ever reading its own.
    const maliciousPath = `${manifestPath}\nzzz-tail-never-inspected.dat`

    beforeAll(() => {
      repo = createGuardRepo('c1-regression')
      stage(repo, aeroPath, '{"schema_version":1}')
      stage(repo, laterAeroPath, '{"schema_version":1}')
      stage(
        repo,
        undeclaredMediaPath,
        Buffer.concat([
          Buffer.from('RIFF'),
          Buffer.from([0, 0, 0, 0]),
          Buffer.from('WEBP'),
        ]),
      )
      stage(
        repo,
        manifestPath,
        JSON.stringify({
          entries: {
            'service.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE },
            'zzz-after-the-malicious-entry.aero': {
              synthetic: true,
              purpose: SYNTHETIC_PURPOSE,
            },
            // Deliberately no entry for `render-case-never-declared.bin` — it
            // must still be found and rejected.
          },
        }),
      )
      const decoySha = hashObject(
        repo,
        'SYNTHETIC DECOY — invented for check-fixture-content.e2e.test.js only, ' +
          'not a real hymn or order-of-service fragment. Proves an embedded ' +
          'newline path is rejected without its content ever being read.',
      )
      cacheAdd(repo, '100644', decoySha, maliciousPath)
    }, GIT_TIMEOUT_MS)

    afterAll(() => {
      cacheRemove(repo, maliciousPath)
      rmSync(repo, { recursive: true, force: true })
    })

    it('rejects the \\n-carrying entry, still catches the undeclared media file next to it, and leaves the declared candidates unaffected', () => {
      const result = runGuard(repo)
      expect(result.status).not.toBe(0)
      // The malicious entry itself is rejected, and specifically for carrying
      // a raw newline — not for some unrelated reason.
      expect(result.stderr).toContain('zzz-tail-never-inspected.dat')
      expect(result.stderr).toMatch(/raw \\n or \\r byte/)
      // The undeclared media file sitting right next to the malicious entry in
      // index order is still found and rejected — a misattributed correlation
      // would instead have let its content silently read as something that
      // does not sniff as media, and it would never appear here at all.
      expect(result.stderr).toContain('render-case-never-declared.bin')
      expect(result.stderr).toMatch(/no entry for "render-case-never-declared\.bin"/)
      // Neither legitimate, correctly-declared candidate — the one whose path
      // the malicious entry's prefix impersonates, nor the one sitting after
      // it in index order — is reported as a violation (only failing verdicts
      // are ever printed). A shifted correlation would have corrupted one or
      // both of these into a spurious rejection or a spurious pass.
      expect(result.stderr).not.toContain('service.aero')
      expect(result.stderr).not.toContain('zzz-after-the-malicious-entry.aero')
      // The run must never claim success while a violation exists.
      expect(result.stdout).not.toMatch(/clean\./)
    })
  },
)

describe(
  'gitlink entries are rejected unconditionally (ADR-0023 siklus 2, auditor W3)',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    /** @type {string} */
    let outer
    /** @type {string} */
    let inner

    beforeAll(() => {
      inner = mkdtempSync(join(tmpdir(), 'aeroworship-fixture-guard-submodule-src-'))
      git(inner, ['-c', 'init.defaultBranch=main', 'init', '-q', '.'])
      git(inner, ['config', 'user.email', 'tester@example.invalid'])
      git(inner, ['config', 'user.name', 'AeroWorship tester'])
      writeFileSync(
        join(inner, 'church.txt'),
        'Synthetic placeholder committed only so the submodule has a commit to point at.',
      )
      git(inner, ['add', '.'])
      git(inner, ['commit', '-q', '-m', 'synthetic submodule content for the tester'])

      outer = createGuardRepo('gitlink')
      // `protocol.file.allow=always` is required from git 2.38 on to add a
      // submodule from a local filesystem path at all (CVE-2022-39253); it is
      // not a weakening of anything this guard is responsible for.
      git(outer, [
        '-c',
        'protocol.file.allow=always',
        'submodule',
        'add',
        '-q',
        inner,
        'tests/integration/fixtures/gitlink-case/church-repo',
      ])
      git(outer, [
        '-c',
        'protocol.file.allow=always',
        'submodule',
        'add',
        '-q',
        inner,
        'vendor/gitlink-outside-fixtures',
      ])
    }, GIT_TIMEOUT_MS)

    afterAll(() => {
      rmSync(outer, { recursive: true, force: true })
      rmSync(inner, { recursive: true, force: true })
    })

    it('`git cat-file --batch` on a gitlink path is `missing`, empirically, not a `commit` object', () => {
      // Verifies the premise `gitlinkVerdict`'s own doc states as measured
      // fact (and that the coordinator brief for this cycle explicitly asked
      // the tester to re-measure rather than trust): the gitlink's commit
      // object lives in the submodule's own `.git/modules/`, never in the
      // superproject's object database, so a batch query for it always comes
      // back missing — this is *why* `identifyCandidates` would otherwise
      // silently drop it (no extension match, "missing" content, no magic
      // bytes to sniff), not merely a curiosity.
      const output = git(
        outer,
        ['cat-file', '--batch'],
        ':tests/integration/fixtures/gitlink-case/church-repo\n',
      )
      expect(output.trim()).toBe(
        ':tests/integration/fixtures/gitlink-case/church-repo missing',
      )

      // The same measurement in the form the guard actually writes since
      // siklus 6 (`BATCH_SPEC_PREFIX`, `:0:`). `BATCH_SPEC_PREFIX`'s doc claims
      // the stage digit "widens nothing else" and cites a gitlink as one of the
      // three cases where the two forms agree; that claim is re-measured here
      // rather than trusted, because it is the form this guard's own behaviour
      // now depends on.
      const explicit = git(
        outer,
        ['cat-file', '--batch'],
        ':0:tests/integration/fixtures/gitlink-case/church-repo\n',
      )
      expect(explicit.trim()).toBe(
        ':0:tests/integration/fixtures/gitlink-case/church-repo missing',
      )
    })

    it('rejects a gitlink both inside and outside a fixtures/ directory, each for being a gitlink', () => {
      const result = runGuard(outer)
      expect(result.status).not.toBe(0)

      // Asserted per path, not by counting phrases across the whole of
      // stderr: merely appearing in the output proves nothing about *why* a
      // path was reported, and every other reason the guard can print would
      // also name these two paths and also exit non-zero. Measured: with
      // `isGitlinkMode` neutered, both paths are still printed and the run
      // still fails — as `unreadable` ("content could not be confirmed
      // present in the index"), because a gitlink's commit object is missing
      // from the superproject (see the `cat-file --batch` case above). That
      // is the collateral this pairing exists to tell apart.
      const reasons = reasonsByPath(result.stderr)
      expect([...reasons.keys()].sort()).toEqual([
        'tests/integration/fixtures/gitlink-case/church-repo',
        'vendor/gitlink-outside-fixtures',
      ])
      for (const path of reasons.keys()) {
        expect(reasons.get(path)).toHaveLength(1)
        expect(reasons.get(path)?.[0]).toMatch(/^this is a git submodule \(mode 160000\)/)
      }

      // Two *violations*, counted by the guard itself — not two entries it
      // merely failed to read, which is summarised separately ("could not be
      // verified") and would leave this line absent.
      expect(result.stderr).toContain('2 of 2 candidate(s) rejected.')
    })
  },
)

describe(
  'a CONFLICTED gitlink gets exactly one verdict, and it is the gitlink one (siklus 6, W2)',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    // The ordering `check-fixture-content.js` fixed: gitlinks are split out of
    // the **full** `ls-files` listing, before the unmerged split. Taken from
    // the stage-0 half instead, a submodule pointer that two branches moved to
    // two different commits (stages 2 and 3, no stage 0) reached neither
    // `gitlinkVerdict` nor `unmergedPathVerdicts` — the latter judges by name,
    // and `vendor/…` is no `.aero` — and so got no verdict at all. That is the
    // gap `gitlinkVerdict` exists to close, reopened by the order of two
    // filters, and it is invisible to any test that only stages a resolved
    // submodule.
    //
    // Built with `update-index --index-info` rather than by merging two
    // branches that each ran `git submodule add`: the shape under test is the
    // index shape (one path, mode 160000, stages 2 and 3, no stage 0), which
    // this produces exactly and in one call — no clone, no second repository,
    // no `protocol.file.allow` — and the harness asserts it got that shape from
    // git itself before running the guard. Neither commit object exists in this
    // repository's object database, which is true of a real gitlink too (see
    // the empirical `cat-file --batch` case in the suite above).
    //
    // No `beforeAll`: the repository is built and destroyed inside the `it`, so
    // this suite adds nothing to the file's hook-timeout budget.
    it('rejects it once as a submodule, and does not report it as a skipped merge conflict', () => {
      const repo = createGuardRepo('gitlink-conflict')
      const gitlink = 'vendor/gitlink-in-conflict'
      try {
        stage(
          repo,
          'docs/ordinary.txt',
          'Synthetic text so the index is not gitlink-only.\n',
        )
        git(
          repo,
          ['update-index', '--index-info'],
          `160000 ${'1'.repeat(40)} 2\t${gitlink}\n160000 ${'2'.repeat(40)} 3\t${gitlink}\n`,
        )
        // The harness proves it built the shape it claims: two unmerged
        // records, both mode 160000, before anything is asserted of the guard.
        const unmerged = git(repo, ['ls-files', '-u', '--', gitlink])
          .trimEnd()
          .split('\n')
        expect(unmerged).toHaveLength(2)
        expect(unmerged.every((line) => line.startsWith('160000 '))).toBe(true)

        const result = runGuard(repo)
        expect(result.status).not.toBe(0)

        const reasons = reasonsByPath(result.stderr)
        expect([...reasons.keys()]).toEqual([gitlink])
        // One verdict, not one per stage — a conflicted pointer is one thing.
        expect(reasons.get(gitlink)).toHaveLength(1)
        expect(reasons.get(gitlink)?.[0]).toMatch(
          /^this is a git submodule \(mode 160000\)/,
        )
        expect(result.stderr).toContain('1 of 1 candidate(s) rejected.')

        // It must not also have been counted among the paths the guard skips
        // for being unmerged: that is the bucket it fell into, unjudged, when
        // the two filters ran the other way round.
        expect(result.stdout).not.toMatch(/unresolved merge conflict/)
        expect(result.stderr).not.toMatch(/mid-scan/)
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })
  },
)

describe(
  'unsafe classification wins over the allowlist, end-to-end (P1 #8)',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    it('rejects a \\n-carrying path under the allowlisted src-tauri/icons/ prefix instead of skipping it', () => {
      const repo = createGuardRepo('unsafe-vs-allowlist')
      try {
        const path = 'src-tauri/icons/evil\nsmuggled.ico'
        const sha = hashObject(
          repo,
          'SYNTHETIC — invented for check-fixture-content.e2e.test.js only, not real icon bytes.',
        )
        cacheAdd(repo, '100644', sha, path)
        const result = runGuard(repo)
        expect(result.status).not.toBe(0)
        expect(result.stderr).toContain('smuggled.ico')
        expect(result.stderr).toMatch(/raw \\n or \\r byte/)
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })
  },
)

describe(
  'the allowlist exempts content-sniffing only, not the declared-extension check (ADR-0023 siklus 2, auditor W-new-2)',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    // Two directions, both required: a real `.aero` staged under the
    // allowlisted `src-tauri/icons/` prefix must still be rejected (the fix),
    // and an ordinary tracked icon under that same prefix must still pass
    // without ever needing a manifest entry (otherwise "rejected" could just
    // as easily mean the allowlist broke entirely, taking every real icon
    // build artefact down with it).
    it('rejects a real .aero staged under the allowlisted src-tauri/icons/ prefix, with no manifest to hide behind', () => {
      const repo = createGuardRepo('allowlist-extension-w-new-2-reject')
      try {
        const path = 'src-tauri/icons/service.aero'
        // Synthetic content only — this is not, and must never become,
        // congregation lyric text; it exists solely to prove the guard
        // reaches this path by its extension, not by content.
        stage(
          repo,
          path,
          JSON.stringify({
            schema_version: 1,
            note: 'SYNTHETIC — invented for check-fixture-content.e2e.test.js only.',
          }),
        )
        try {
          const result = runGuard(repo)
          expect(result.status).not.toBe(0)
          expect(result.stderr).toContain(path)
          // Outside any fixtures/ directory, so the unconditional reason
          // fires — proof the guard flagged it by its declared extension
          // (`.aero`), the one check `identifyAllowlistedExtensionCandidates`
          // still runs over the allowlisted bucket.
          expect(result.stderr).toMatch(
            /not under any directory literally named "fixtures"/,
          )
        } finally {
          cleanup(repo, [path])
        }
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })

    it('still passes a legitimate tracked icon under src-tauri/icons/ with no manifest at all', () => {
      const repo = createGuardRepo('allowlist-extension-w-new-2-pass')
      try {
        const path = 'src-tauri/icons/app-icon.ico'
        // Real ICO magic bytes (see sniffMediaKind) — if the allowlist were
        // broken in the other direction (exempting nothing, or newly
        // sniffing content it did not before), this file would either need a
        // manifest it does not have, or this assertion would catch a
        // regression that made the allowlist useless.
        const icoBytes = Buffer.concat([
          Buffer.from([0x00, 0x00, 0x01, 0x00]),
          Buffer.from('SYNTHETIC — invented for check-fixture-content.e2e.test.js only.'),
        ])
        stage(repo, path, icoBytes)
        try {
          const result = runGuard(repo)
          expect(result.status).toBe(0)
          expect(result.stdout).toMatch(/0 candidate\(s\) found/)
        } finally {
          cleanup(repo, [path])
        }
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })
  },
)

describe(
  'unresolved merge conflicts are split out before the batch request (siklus 4)',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    // Before siklus 4 every path in a conflict was queried as `:path`, which
    // resolves stage 0 — a stage a conflicted path does not have — so all three
    // of its records came back `missing` and the run failed, for the whole
    // duration of the conflict, blaming "index changed mid-scan?". Both halves
    // of the fix need proving against the real CLI: an ordinary conflict must
    // no longer block `npm test` at all, and a conflicted `.aero` must still be
    // reported, by name, with a reason that is true.
    //
    // No `beforeAll` here on purpose: each case builds and destroys its own
    // repository inside the `it`, so this suite adds no hook-timeout budget to
    // the file (see the file doc on `hookTimeout` being a separate budget from
    // the `describe`-level `timeout`).

    /**
     * Builds a repository holding a real, unresolved merge conflict on
     * `relPath` — two branches, two commits, one failed merge. Not simulated
     * with `update-index --cacheinfo`: the point is that git itself produced
     * the stage 1/2/3 shape this guard now has to recognise.
     *
     * @param {string} label
     * @param {string} relPath Path to put in conflict.
     * @param {(repo: string) => void} [seed] Extra staging for the base commit.
     * @returns {string} absolute path to the new repo
     */
    function createConflictedRepo(label, relPath, seed) {
      const repo = createGuardRepo(label)
      git(repo, ['config', 'user.email', 'tester@example.invalid'])
      git(repo, ['config', 'user.name', 'AeroWorship tester'])
      seed?.(repo)
      stage(repo, relPath, '{"schema_version":1,"note":"SYNTHETIC base revision"}')
      git(repo, ['add', '-A', '.'])
      git(repo, ['commit', '-q', '-m', 'synthetic base commit'])

      git(repo, ['checkout', '-q', '-b', 'incoming'])
      stage(repo, relPath, '{"schema_version":1,"note":"SYNTHETIC incoming revision"}')
      git(repo, ['commit', '-q', '-m', 'synthetic incoming revision'])

      git(repo, ['checkout', '-q', 'main'])
      stage(repo, relPath, '{"schema_version":1,"note":"SYNTHETIC local revision"}')
      git(repo, ['commit', '-q', '-m', 'synthetic local revision'])

      try {
        git(repo, ['merge', '-q', 'incoming'])
        throw new Error('harness: the merge was expected to conflict and did not')
      } catch (error) {
        // A conflicting merge exits non-zero; anything else is a harness bug.
        if (!(/** @type {{ status?: number }} */ (error).status)) throw error
      }
      // The harness proves it built what it claims before anything is asserted
      // about the guard: `ls-files -u` lists only unmerged entries.
      expect(git(repo, ['ls-files', '-u', '--', relPath]).trim()).not.toBe('')
      return repo
    }

    it('does not fail the run for a conflicted ordinary file, and says which paths it skipped', () => {
      const repo = createConflictedRepo('merge-plain', 'docs/notes.txt')
      try {
        const result = runGuard(repo)
        expect(result.status).toBe(0)
        expect(result.stdout).toMatch(/unresolved merge conflict/)
        expect(result.stdout).toMatch(/1 path\(s\) are in an unresolved merge conflict/)
        // The accusation the old behaviour printed, for a cause that had
        // nothing to do with it. It must be gone, not merely outweighed.
        expect(result.stderr).not.toMatch(/mid-scan/)
        expect(result.stderr).toBe('')
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })

    it('still rejects a conflicted .aero by name, with the conflict as the stated reason', () => {
      const root = 'tests/integration/merge-case/fixtures'
      const aeroPath = `${root}/disputed.aero`
      const repo = createConflictedRepo('merge-aero', aeroPath, (r) => {
        // Declared correctly and committed at stage 0 from the start, so the
        // rejection below cannot be an undeclared-fixture rejection wearing a
        // different hat.
        stage(
          r,
          `${root}/fixtures.manifest.json`,
          JSON.stringify({
            entries: { 'disputed.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE } },
          }),
        )
      })
      try {
        const result = runGuard(repo)
        expect(result.status).not.toBe(0)

        const reasons = reasonsByPath(result.stderr)
        expect([...reasons.keys()]).toEqual([aeroPath])
        expect(reasons.get(aeroPath)).toHaveLength(1)
        expect(reasons.get(aeroPath)?.[0]).toMatch(/^this path is in an unresolved merge/)
        // Not reported as unreadable — that was the old, wrong diagnosis, and
        // it would appear as a second reason for this same path.
        expect(result.stderr).not.toMatch(/mid-scan/)
        expect(result.stderr).toContain('1 of 1 candidate(s) rejected.')

        // The reason text promises the file "is then checked normally" once
        // resolved and `git add`ed. Proving that closes the loop: a developer
        // following the instruction must actually get past the guard.
        writeFileSync(
          join(repo, aeroPath),
          '{"schema_version":1,"note":"SYNTHETIC resolved revision"}',
        )
        git(repo, ['add', '--', aeroPath])
        const resolved = runGuard(repo)
        expect(resolved.status).toBe(0)
        expect(resolved.stdout).toMatch(/candidate\(s\) verified synthetic, clean\./)
        expect(resolved.stdout).not.toMatch(/unresolved merge conflict/)
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })

    it('blames the CONFLICTED MANIFEST, not the fixture, when the manifest is the unmerged one (siklus 6, W3)', () => {
      // The `unmergedPaths` argument `check-fixture-content.js` now threads
      // into `evaluateCandidate`, end to end. The fixture itself is untouched
      // by the merge and sits at stage 0, so it is still a candidate; its
      // manifest has no stage 0, is filtered out of the batch request with
      // every other unmerged path, and so comes back absent. The verdict was
      // always right — what was wrong was the sentence: "is not in the index —
      // create it" for a file sitting in the index at stages 1/2/3, the same
      // unescapable loop `manifestPathsForCandidates` had to remove once.
      const root = 'tests/integration/manifest-merge/fixtures'
      const manifestPath = `${root}/fixtures.manifest.json`
      const aeroPath = `${root}/declared.aero`
      const repo = createConflictedRepo('merge-manifest', manifestPath, (r) => {
        stage(r, aeroPath, '{"schema_version":1,"note":"SYNTHETIC, never edited"}')
      })
      try {
        // The conflict really is on the manifest alone: the fixture is at
        // stage 0, so it is judged as an ordinary candidate.
        expect(git(repo, ['ls-files', '-u', '--', manifestPath]).trim()).not.toBe('')
        expect(git(repo, ['ls-files', '-u', '--', aeroPath]).trim()).toBe('')

        const result = runGuard(repo)
        expect(result.status).not.toBe(0)

        const reasons = reasonsByPath(result.stderr)
        expect([...reasons.keys()]).toEqual([aeroPath])
        expect(reasons.get(aeroPath)).toHaveLength(1)
        const reason = reasons.get(aeroPath)?.[0] ?? ''
        expect(reason).toMatch(/is itself in an unresolved merge conflict/)
        expect(reason).toMatch(/resolve the conflict and `git add` the manifest/)
        // The sentence that sent a developer to create a file they already
        // had. It must be gone for this case, not merely accompanied.
        expect(reason).not.toMatch(/is not in the index/)

        // …and the promise it makes must hold: resolving the manifest clears
        // the run, so the instruction is a way out and not a loop.
        writeFileSync(
          join(repo, manifestPath),
          JSON.stringify({
            entries: { 'declared.aero': { synthetic: true, purpose: SYNTHETIC_PURPOSE } },
          }),
        )
        git(repo, ['add', '--', manifestPath])
        const resolved = runGuard(repo)
        expect(resolved.status).toBe(0)
        expect(resolved.stdout).toMatch(/candidate\(s\) verified synthetic, clean\./)
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })

    it('does not report a conflicted .png or manifest — only the declared extensions', () => {
      // The other half of the deliberate line in `unmergedPathVerdicts`: a
      // conflict on ordinary binary or JSON content is work in progress, and
      // blocking it would be exactly the over-blocking siklus 4 removed.
      const repo = createConflictedRepo(
        'merge-png',
        'tests/integration/fixtures/merge-media/thumb.png',
      )
      try {
        const result = runGuard(repo)
        expect(result.status).toBe(0)
        expect(reasonsByPath(result.stderr).size).toBe(0)
        expect(result.stdout).toMatch(/unresolved merge conflict/)
      } finally {
        rmSync(repo, { recursive: true, force: true })
      }
    })
  },
)

describe('running outside any git context (P2 #12)', { timeout: GIT_TIMEOUT_MS }, () => {
  it('fails hard, rather than passing silently, when there is no git repository at all', () => {
    const dir = mkdtempSync(join(tmpdir(), 'aeroworship-fixture-guard-norepo-'))
    try {
      mkdirSync(join(dir, 'scripts'), { recursive: true })
      copyFileSync(GUARD_SRC, join(dir, 'scripts/fixture-content-guard.js'))
      copyFileSync(CHECK_SRC, join(dir, 'scripts/check-fixture-content.js'))
      // Deliberately no `git init` — this directory is not a repository, and
      // has no repository above it either (the OS temp directory is not one).
      const result = runGuard(dir)
      expect(result.status).not.toBe(0)
      expect(result.stderr).toMatch(/could not list the git index/)
    } finally {
      rmSync(dir, { recursive: true, force: true })
    }
  })

  it('fails hard, with a message naming git, when git is not resolvable on PATH', () => {
    const repo = createGuardRepo('no-git-on-path')
    try {
      stage(repo, 'placeholder.txt', 'x')
      const withoutGit = Object.fromEntries(
        Object.entries(process.env).filter(([key]) => key.toLowerCase() !== 'path'),
      )
      // A directory that provably holds no `git`/`git.exe`, rather than an
      // empty string — an empty PATH can behave inconsistently in child
      // process creation across platforms, whereas a real, git-free directory
      // is unambiguous on every OS this guard runs on.
      const emptyBinDir = mkdtempSync(join(tmpdir(), 'aeroworship-empty-bin-'))
      withoutGit.PATH = emptyBinDir
      try {
        const result = runGuard(repo, withoutGit)
        expect(result.status).not.toBe(0)
        expect(result.stderr).toMatch(/git was not found on PATH/)
      } finally {
        rmSync(emptyBinDir, { recursive: true, force: true })
      }
    } finally {
      rmSync(repo, { recursive: true, force: true })
    }
  })
})

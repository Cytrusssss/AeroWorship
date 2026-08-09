// @ts-check

/**
 * `.gitignore` is the only thing standing between congregation data and the git
 * history. A `.aero` file is a real order of service and a `*.db` is real lyrics
 * (PRD §6.9); once either is committed, removing it costs a history rewrite on
 * every clone. Nothing else in the repository fails when a pattern here is
 * widened, narrowed or deleted — which is the ADR-0020 failure class, and the
 * reason this file exists rather than a comment saying the patterns were checked
 * once by hand.
 *
 * Two properties are guarded, and they pull in opposite directions:
 *
 *   - too narrow — a spelling of user data that is *not* ignored;
 *   - too wide — a fixture, or an already-tracked file, that *is*.
 *
 * The second is the one that fails silently. `git status` says nothing about an
 * ignored file, so a NFR-15 hostile-path fixture or a NFR-28 mutated-corpus file
 * that the exception stopped covering looks committed on the author's machine
 * and is simply absent on everybody else's.
 *
 * Ignore rules cannot be judged by reading them — the implementer of SETUP-06
 * first shipped a redundant second line here on a wrong belief about `/**\/`, and
 * found it only by running git. So this asks git, and asks it about the real
 * `.gitignore` bytes, copied into a throwaway repository under the OS temp
 * directory. Running in a scratch repository rather than this one is what lets
 * the test create files freely without a cleanup step that could delete
 * somebody's work, and what keeps the answers independent of whatever happens to
 * be lying around in the working tree.
 *
 * The one question that has to be asked of the *real* repository is whether a
 * tracked file has become ignored, so that check reads `git ls-files` here and
 * evaluates those paths against the copy.
 */

import { execFileSync } from 'node:child_process'
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')

/**
 * Per-case budget. It hangs on each `describe` below, which Vitest applies to
 * every `it` inside it, rather than on `testTimeout` in `vitest.config.ts`: a
 * global number would be spent by every future test in the suite, including
 * ones that should fail fast, and would attribute this file's cost to the
 * runner instead of to the thing that incurs it.
 *
 * That a suite-level option reaches the cases at all is the sort of thing that
 * fails silently — an ignored option leaves the 5 s default in place and looks
 * exactly like a working one until a slow machine turns up. It was checked by
 * running the file under `--testTimeout=1`, where all 17 cases pass because
 * this value overrides the flag, against a control file with no budget of its
 * own whose cases all time out at 1 ms under the same flag.
 *
 * Every case here answers its question by running git, and several build a
 * throwaway repository to do it — `git init`, a write per probe file, then
 * `git status`. Measured on this machine: 1.5 s for a bare init-plus-status,
 * 1.3–3.7 s for the `check-ignore` cases, and 3.1–8.7 s for the four that go
 * through `untrackedUnder`. Against Vitest's 5 s default that is not margin, it
 * is a coin toss — two cases passed in one run and timed out in the next
 * without a byte of `.gitignore` changing, which reports process spawn latency
 * as a defect in the ignore rules and trains the reader to re-run until green.
 *
 * A hang detector, then, and not a performance gate; the assertions are
 * untouched by it. A genuinely wedged git blocks forever and is still caught.
 */
const GIT_TIMEOUT_MS = 60_000

/** @type {string} */
let scratch
/** @type {string} */
let emptyExcludes

/**
 * @param {string} cwd
 * @param {string[]} args
 * @param {string} [input]
 * @returns {string} stdout, including the stdout of a non-zero exit.
 */
function git(cwd, args, input) {
  try {
    return execFileSync('git', args, { cwd, input, encoding: 'utf8' })
  } catch (error) {
    // `git check-ignore` exits 1 when nothing matched, which is an answer and
    // not a failure. A missing binary is a failure, and has to look like one:
    // skipping here would leave the suite green while this file checked nothing.
    const failure = /** @type {NodeJS.ErrnoException & { stdout?: string }} */ (error)
    if (failure.code === 'ENOENT') {
      throw new Error(
        'git was not found on PATH. This file verifies .gitignore by running git; ' +
          'without it the guard is unverified, so this is a failure rather than a skip.',
        { cause: error },
      )
    }
    if (typeof failure.stdout === 'string') return failure.stdout
    throw error
  }
}

/**
 * Every path's verdict, from a single `git check-ignore` process.
 *
 * `--no-index` matters more than it looks: without it, `check-ignore` consults
 * the index and reports *nothing at all* for a tracked path, so the too-wide
 * check below would pass by construction no matter what the patterns said.
 *
 * `-v -n` makes the output one line per input path, `source:line:pattern<TAB>path`
 * with an empty `::` prefix where no rule matched. The pattern is what
 * distinguishes the two ways of not being ignored: no rule matched at all, or a
 * negation rescued the file. A `!` pattern is a match that means "not ignored".
 *
 * @param {string[]} paths
 * @returns {Map<string, { ignored: boolean; rule: string }>}
 */
function verdicts(paths) {
  for (const path of paths) {
    // The output format is line- and tab-delimited, so a path containing either
    // would be silently misparsed into a wrong verdict.
    expect(path, 'test path must not contain a tab or newline').not.toMatch(/[\t\n\r]/)
  }

  const stdout = git(
    scratch,
    [
      '-c',
      `core.excludesFile=${emptyExcludes}`,
      'check-ignore',
      '--no-index',
      '-v',
      '-n',
      '--stdin',
    ],
    paths.join('\n') + '\n',
  )

  /** @type {Map<string, { ignored: boolean; rule: string }>} */
  const result = new Map()
  for (const line of stdout.split('\n')) {
    if (line.trim() === '') continue
    const tab = line.indexOf('\t')
    const prefix = line.slice(0, tab)
    const path = line.slice(tab + 1).replace(/\r$/, '')
    const firstColon = prefix.indexOf(':')
    const secondColon = prefix.indexOf(':', firstColon + 1)
    const rule = secondColon === -1 ? '' : prefix.slice(secondColon + 1)
    result.set(path, { ignored: rule !== '' && !rule.startsWith('!'), rule })
  }

  for (const path of paths) {
    expect(result.has(path), `git returned no verdict for ${path}`).toBe(true)
  }
  return result
}

/**
 * @param {string[]} paths
 * @returns {string[]} the subset git would ignore.
 */
function ignoredAmong(paths) {
  const answers = verdicts(paths)
  return paths.filter((path) => answers.get(path)?.ignored === true)
}

/**
 * @param {string[]} paths
 * @returns {string[]} the subset git would NOT ignore.
 */
function notIgnoredAmong(paths) {
  const answers = verdicts(paths)
  return paths.filter((path) => answers.get(path)?.ignored !== true)
}

/**
 * The real `.gitignore`, as text, so a test can hand git a *mutated* copy of it.
 *
 * `check-ignore` answers "does a rule match this path". `git status` answers the
 * question an author actually has, which is "is my file there" — and the two can
 * disagree, because git does not descend into an excluded directory and so
 * cannot re-include anything beneath one. Everything below therefore creates the
 * files and reads `git status`, in a repository that is not this one.
 */
const REAL_GITIGNORE = readFileSync(join(ROOT, '.gitignore'), 'utf8')

/**
 * Rewrite `.gitignore` line by line; `null` deletes the line. Line-based rather
 * than a regex over the whole text so a CRLF checkout cannot make the mutation
 * silently match nothing, which would leave the mutation tests below asserting
 * that the unmodified file behaves like itself.
 *
 * @param {(line: string) => string | null} fn
 * @returns {string}
 */
function mutateGitignore(fn) {
  return REAL_GITIGNORE.split('\n')
    .map((raw) => {
      const eol = raw.endsWith('\r') ? '\r' : ''
      const replacement = fn(eol === '' ? raw : raw.slice(0, -1))
      return replacement === null ? null : replacement + eol
    })
    .filter((line) => line !== null)
    .join('\n')
}

/**
 * Create the files for real under the given `.gitignore`, in a throwaway
 * repository of this call's own, and report what `git status` can see.
 *
 * @param {string} gitignoreText
 * @param {string[]} paths
 * @returns {Set<string>} the subset `git status --porcelain -uall` lists as untracked.
 */
function untrackedUnder(gitignoreText, paths) {
  const repo = mkdtempSync(join(tmpdir(), 'aeroworship-gitignore-status-'))
  try {
    git(repo, ['-c', 'init.defaultBranch=main', 'init', '-q', '.'])
    const excludes = join(repo, 'empty-excludes')
    writeFileSync(excludes, '')
    writeFileSync(join(repo, '.gitignore'), gitignoreText)
    for (const path of paths) {
      mkdirSync(dirname(join(repo, path)), { recursive: true })
      writeFileSync(join(repo, path), 'probe')
      // A path git refuses to write is not a verdict about ignore rules, and
      // would otherwise read as "correctly hidden".
      expect(existsSync(join(repo, path)), `probe file ${path} was not created`).toBe(
        true,
      )
    }
    const stdout = git(repo, [
      '-c',
      `core.excludesFile=${excludes}`,
      'status',
      '--porcelain',
      '-uall',
    ])
    const untracked = new Set(
      stdout
        .split('\n')
        .map((line) => line.replace(/\r$/, ''))
        .filter((line) => line.startsWith('?? '))
        .map((line) => line.slice(3)),
    )
    // A test whose probes are all *expected* to be hidden would pass on an empty
    // set for any reason at all, including git having reported nothing. Nothing
    // in the file ignores `.gitignore` itself, so its absence means the run
    // failed rather than that the patterns worked.
    expect(
      untracked.has('.gitignore'),
      'git status reported nothing; the run is void',
    ).toBe(true)
    return untracked
  } finally {
    rmSync(repo, { recursive: true, force: true })
  }
}

beforeAll(() => {
  scratch = mkdtempSync(join(tmpdir(), 'aeroworship-gitignore-'))
  emptyExcludes = join(scratch, 'empty-excludes')
  writeFileSync(emptyExcludes, '')
  git(scratch, ['-c', 'init.defaultBranch=main', 'init', '-q', '.'])
  copyFileSync(join(ROOT, '.gitignore'), join(scratch, '.gitignore'))
})

afterAll(() => {
  if (scratch !== undefined) rmSync(scratch, { recursive: true, force: true })
})

describe('the user-data patterns', { timeout: GIT_TIMEOUT_MS }, () => {
  // Realistic spellings, not minimal ones: `aeroworship.db-wal` is what SQLite
  // actually writes next to the database (PRD §6.9) and has no dot before `-wal`,
  // which is why `*-wal` and not `*.wal` is the pattern that has to catch it.
  const spellings = [
    '.env',
    '.env.local',
    '.env.production',
    'aeroworship.db',
    'aeroworship.db-wal',
    'aeroworship.db-shm',
    'aeroworship.sqlite',
    'aeroworship.sqlite3',
    'service.aero',
    'sunday.aerotpl',
    'aeroworship-2026-08-09.log',
    'session.autosave.json',
  ]

  it('ignores every spelling of user data at the repository root', () => {
    expect(notIgnoredAmong(spellings)).toEqual([])
  })

  it('ignores them at depth too, since a stray copy lands anywhere', () => {
    const deep = spellings.map((name) => `src-tauri/crates/core/${name}`)
    expect(notIgnoredAmong(deep)).toEqual([])
  })

  it('claims .env.local by .env* rather than leaving it to *.local', () => {
    // Both patterns match this name and git takes the last one. If `.env*` were
    // ever removed, `*.local` would keep `.env.local` ignored and hide the loss,
    // while `.env` and `.env.production` silently became committable.
    expect(verdicts(['.env.local']).get('.env.local')?.rule).toBe('.env*')
    expect(verdicts(['.env']).get('.env')?.rule).toBe('.env*')
  })

  it('catches the WAL siblings by their hyphen, not by a dotted extension', () => {
    expect(verdicts(['aeroworship.db-wal']).get('aeroworship.db-wal')?.rule).toBe('*-wal')
    expect(verdicts(['aeroworship.db-shm']).get('aeroworship.db-shm')?.rule).toBe('*-shm')
  })

  it('leaves near-miss names alone', () => {
    // The cost of a pattern that is one character too greedy is a source file
    // that never gets committed, and nothing says so.
    expect(
      ignoredAmong([
        'envelope.txt',
        'docs/environment.md',
        'walnut.txt',
        'src/shared/firmware.ts',
        'catalog.md',
        'src/main/views/Login.vue',
      ]),
    ).toEqual([])
  })
})

describe('the fixture exception', { timeout: GIT_TIMEOUT_MS }, () => {
  // NFR-15 asks for hostile path fixtures and NFR-28 for a mutated-file corpus.
  // Both are `.aero`/`.aerotpl` that have to be committable, and the marker is
  // the directory *name*, so all four homes below have to work: PRD §6.13 puts
  // fixtures under tests/{unit,integration,perf}/, and Cargo convention puts
  // them under a crate's own tests/.
  const fixtures = [
    'fixtures/hostile.aero',
    'fixtures/hostile.aerotpl',
    'fixtures/nested/sub/deep.aero',
    'fixtures/nested/sub/deep.aerotpl',
    'tests/unit/fixtures/dotdot-escape.aero',
    'tests/unit/fixtures/hostile/paths/absolute.aerotpl',
    'tests/integration/fixtures/roundtrip.aero',
    'tests/perf/fixtures/large-set.aero',
    'src-tauri/crates/core/tests/fixtures/truncated.aero',
    'src-tauri/crates/core/tests/fixtures/mutated/bitflip.aerotpl',
  ]

  it('un-ignores .aero and .aerotpl in a fixtures directory anywhere', () => {
    expect(ignoredAmong(fixtures)).toEqual([])
  })

  it('needs no separate rule for a file sitting directly in fixtures/', () => {
    // `**/fixtures/**/*.aero` matching `fixtures/hostile.aero` is the claim that
    // `/**/` also matches zero directories. A second, narrower line was written
    // here once on the belief that it did not; this is what made that line
    // redundant, and what would have to be restored if git's behaviour changed.
    expect(verdicts(['fixtures/hostile.aero']).get('fixtures/hostile.aero')?.rule).toBe(
      '!**/fixtures/**/*.aero',
    )
    expect(
      verdicts(['fixtures/hostile.aerotpl']).get('fixtures/hostile.aerotpl')?.rule,
    ).toBe('!**/fixtures/**/*.aerotpl')
  })

  it('actually lets git add the fixture, not merely match a negation', () => {
    // The end-to-end statement of the same thing, and the only one that answers
    // the question a fixture author cares about. A negated pattern can match and
    // still lose to an excluded parent directory, in which case `git add` refuses
    // and `check-ignore` alone would not have shown it.
    for (const path of fixtures) {
      mkdirSync(dirname(join(scratch, path)), { recursive: true })
      writeFileSync(join(scratch, path), 'fixture')
    }
    const stdout = git(scratch, ['add', '--dry-run', '--', ...fixtures])
    for (const path of fixtures) {
      expect(stdout).toContain(`add '${path}'`)
    }
  })

  it('does not turn a fixtures directory into a back door', () => {
    // The exception names two extensions on purpose. `!**/fixtures/**` would have
    // been shorter and would have re-admitted every secret, database and log that
    // happened to be written under a directory called fixtures.
    const smuggled = [
      'fixtures/.env',
      'fixtures/leaked.db',
      'fixtures/leaked.db-wal',
      'fixtures/leaked.sqlite',
      'fixtures/run.log',
      'fixtures/session.autosave.json',
      'tests/unit/fixtures/.env',
      'tests/unit/fixtures/leaked.db',
      'tests/unit/fixtures/run.log',
      'src-tauri/crates/core/tests/fixtures/.env',
      'src-tauri/crates/core/tests/fixtures/leaked.db',
      'src-tauri/crates/core/tests/fixtures/run.log',
    ]
    expect(notIgnoredAmong(smuggled)).toEqual([])
  })
})

describe(
  'config.json, which is deliberately not ignored',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    it('stays committable at the root and at any depth', () => {
      // PRD §6.9 names a `config.json` holding display assignment and preferences —
      // no congregation data. A bare `config.json` line would match at every depth
      // and collide with `tsconfig.json`-shaped neighbours; `session.autosave.json`
      // is specific enough to be safe bare, and its contents *are* an order of
      // service (FR-707).
      expect(
        ignoredAmong([
          'config.json',
          'src/main/config.json',
          'src-tauri/crates/core/config.json',
          'fixtures/config.json',
          'tsconfig.json',
          'package.json',
          '.prettierrc.json',
        ]),
      ).toEqual([])
    })

    it('does not shelter session.autosave.json alongside it', () => {
      expect(
        notIgnoredAmong([
          'session.autosave.json',
          'src/main/session.autosave.json',
          'src-tauri/crates/core/session.autosave.json',
        ]),
      ).toEqual([])
    })
  },
)

describe('the anchored /cache/ and /media/ patterns', { timeout: GIT_TIMEOUT_MS }, () => {
  // `cache/decks/{deck_uuid}/` holds a full visual render of a real sermon —
  // slide text and photographs rasterised to .webp (PRD §6.9, §6.10) — so it is
  // often *more* readable than the `.aero` beside it, which carries only UUIDs.
  // `media/` is the user's own images, which no extension pattern above touches.
  const dataRoot = [
    'cache/decks/9f2c1a40-0000-4000-8000-000000000000/slide-0001.webp',
    'cache/decks/9f2c1a40-0000-4000-8000-000000000000/thumb-0001.webp',
    'cache/thumbs/9f2c1a40.webp',
    'media/foto.jpg',
    'media/ab/cd/photo-of-the-congregation.png',
  ]

  // The whole reason for the leading slash. `cache` and `media` are ordinary
  // directory names; PRD §6.13 does not have these paths today, and that is the
  // point — a bare pattern would swallow the first one somebody adds, and
  // `git status` would say nothing at all about it.
  const sourceTree = [
    'src/assets/media/logo.png',
    'src/main/media/icon.svg',
    'src/shared/cache/memo.ts',
    'docs/media/diagram.png',
  ]

  it('hides the data-root spellings from git status, with the files really there', () => {
    const untracked = untrackedUnder(REAL_GITIGNORE, [...dataRoot, ...sourceTree])
    expect(dataRoot.filter((path) => untracked.has(path))).toEqual([])
    expect(sourceTree.filter((path) => !untracked.has(path))).toEqual([])
  })

  it('matches them by the anchored rule, not by some other line', () => {
    // Pins the spelling. If the slash were dropped the reported rule becomes
    // `cache/`, so this fails before the damage below has to be demonstrated.
    expect(verdicts(['cache/thumbs/x.webp']).get('cache/thumbs/x.webp')?.rule).toBe(
      '/cache/',
    )
    expect(verdicts(['media/foto.jpg']).get('media/foto.jpg')?.rule).toBe('/media/')
  })

  it('would swallow the source tree if the anchors were removed', () => {
    // The mutation check: it is not enough that the source tree survives today,
    // because it would also survive if the two lines did nothing at all. This
    // says the anchor is what saves it.
    const unanchored = mutateGitignore((line) =>
      line === '/cache/' ? 'cache/' : line === '/media/' ? 'media/' : line,
    )
    expect(unanchored, 'mutation matched nothing — the anchored lines are gone').not.toBe(
      REAL_GITIGNORE,
    )

    const untracked = untrackedUnder(unanchored, sourceTree)
    expect(sourceTree.filter((path) => untracked.has(path))).toEqual([])
  })

  it('is what hides the data root, and not some earlier extension pattern', () => {
    // The other half of the mutation: delete the lines and the renders come
    // back. `.webp`, `.jpg` and `.png` are matched by nothing else here, so
    // without this the two lines could be deleted with every test still green.
    const without = mutateGitignore((line) =>
      line === '/cache/' || line === '/media/' ? null : line,
    )
    expect(without, 'mutation matched nothing — the anchored lines are gone').not.toBe(
      REAL_GITIGNORE,
    )

    const untracked = untrackedUnder(without, dataRoot)
    expect(dataRoot.filter((path) => !untracked.has(path))).toEqual([])
  })

  it('leaves a fixtures/ directory named cache or media reachable', () => {
    // The interaction that is not visible from reading either rule. Git does not
    // descend into an excluded directory, so `!**/fixtures/**/*.aero` cannot
    // rescue anything under an ignored parent: a `fixtures/` inside the root
    // `cache/` or `media/` is unreachable no matter what the exception says.
    // The anchor is what keeps that from mattering — the fixture homes of PRD
    // §6.13 are nowhere near the repository root, so a fixture directory *named*
    // cache or media still works.
    const reachable = [
      'tests/unit/fixtures/media/hostile.aero',
      'tests/unit/fixtures/cache/hostile.aerotpl',
      'src-tauri/crates/core/tests/fixtures/media/truncated.aero',
    ]
    const unreachable = ['cache/fixtures/hostile.aero', 'media/fixtures/hostile.aerotpl']

    const untracked = untrackedUnder(REAL_GITIGNORE, [...reachable, ...unreachable])
    expect(reachable.filter((path) => !untracked.has(path))).toEqual([])
    // Asserted rather than merely noted: if a future edit ever un-anchors these
    // patterns, this is the line that explains what it cost.
    expect(unreachable.filter((path) => untracked.has(path))).toEqual([])
  })
})

describe(
  'the patterns against the files already tracked',
  { timeout: GIT_TIMEOUT_MS },
  () => {
    it('ignores none of them', () => {
      // A tracked file that also matches an ignore rule keeps working until someone
      // removes and re-adds it, at which point it quietly cannot come back. This is
      // the cheapest check here and the most expensive one to have skipped, so it
      // reads the real index rather than a fixed list that would drift.
      const tracked = git(ROOT, ['ls-files'])
        .split('\n')
        .map((line) => line.replace(/\r$/, ''))
        .filter((line) => line !== '')

      expect(tracked.length).toBeGreaterThan(0)
      expect(ignoredAmong(tracked)).toEqual([])
    })
  },
)

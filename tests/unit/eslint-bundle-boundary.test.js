/**
 * The `src/output/` ↔ `src/main/` bundle boundary (PRD §6.3) is enforced by
 * `eslint.config.js` and by nothing else. Its two halves are three rules deep —
 * a `no-restricted-imports` pattern list plus two `no-restricted-syntax`
 * selectors per directory — and when one of them stops matching, every gate in
 * the repository still reports green. That is precisely how the cycle-1 gaps
 * survived review: the rule covered less than its comment claimed, and no test
 * disagreed.
 *
 * These cases lint source text through the real config, so a narrowed selector
 * fails here instead of surfacing months later as a memory number nobody was
 * measuring. The negative controls matter as much as the positive ones: a rule
 * that is too wide gets worked around rather than reported.
 */

import { resolve } from 'node:path'

import { ESLint } from 'eslint'
import { beforeAll, describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')

/** The two rules that carry the boundary. Everything else is noise here. */
const BOUNDARY_RULES = new Set(['no-restricted-imports', 'no-restricted-syntax'])

/**
 * Budget for the `beforeAll` below, and nothing else — every `it` in this file
 * still runs under the 5 s default, which is what keeps a rule that starts
 * hanging visible.
 *
 * The number is large because the thing it bounds is variable, not because the
 * work is slow. Five consecutive measurements of the first `lintText` on this
 * machine: 21.3 s, 11.8 s, 16.5 s, and inside a Vitest worker more than 30 s.
 * Every call after it took 33–200 ms. A hook timeout here is a hang detector,
 * not a performance gate: too tight and it reports variance in module loading
 * as a failure of the boundary rules, which is the failure this file exists to
 * make legible.
 */
const WARMUP_TIMEOUT_MS = 120_000

/** @type {ESLint} */
let eslint

beforeAll(async () => {
  // One instance for the file: constructing it is what loads and validates
  // `eslint.config.js`, and doing that per case dominates the runtime.
  eslint = new ESLint({ cwd: ROOT })

  // The first `lintText` is not a lint, it is an installation. It loads
  // `eslint.config.js`, then `typescript-eslint` and `vue-eslint-parser`
  // behind it, and pays for that once per process — 12–21 s measured here,
  // against 33–200 ms for every call after it. Left inside the first `it`,
  // that cost landed on whichever case `describe.each` happened to schedule
  // first and blew the 5 s `testTimeout`, so the suite failed at a case that
  // was passing on the merits.
  //
  // Paid here instead, with a hook timeout of its own. Raising `testTimeout`
  // globally in `vitest.config.ts` would have hidden the same cost behind a
  // number every future test gets to spend, and would have left the runtime
  // attributed to a test rather than to setup. The fixture is a file the
  // boundary rules do not apply to, so nothing about the assertions below is
  // asserted here — this call exists for its side effects on module load.
  await eslint.lintText('export default 1\n', {
    filePath: resolve(ROOT, 'src', 'warmup-fixture.ts'),
    warnIgnored: false,
  })
}, WARMUP_TIMEOUT_MS)

/**
 * Lints `code` as if it were a file directly inside `src/<dir>/`. The path need
 * not exist — only its location decides which config blocks apply, which is the
 * thing under test.
 *
 * @param {string} dir Directory under `src/`, e.g. `output` or `shared`.
 * @param {string} code Source text.
 * @param {string} [file] Fixture file name; its extension decides which of the
 *   `SOURCE_EXTENSIONS` patterns has to match for the rules to apply at all.
 * @returns {Promise<Array<{ruleId: string | null, message: string}>>}
 */
async function boundaryMessages(dir, code, file = 'boundary-fixture.ts') {
  const [result] = await eslint.lintText(code, {
    filePath: resolve(ROOT, 'src', dir, file),
    warnIgnored: false,
  })
  return (result?.messages ?? []).filter((message) =>
    BOUNDARY_RULES.has(String(message.ruleId)),
  )
}

/**
 * Every spelling that reaches `src/main/` from a sibling directory. Each one is
 * a way a developer would plausibly write it, not a synthetic variation:
 * re-exports are how a barrel file leaks, and the template-literal dynamic
 * import is the ordinary shape of route-level lazy loading.
 */
const REACHES_INTO_MAIN = [
  ['static import', "import App from '../main/App.vue'\nexport default App"],
  ['star re-export', "export * from '../main/main'"],
  ['named re-export', "export { session } from '../main/stores/session'"],
  ['aliased import', "import x from '@/main/x'\nexport default x"],
  ['directory import, no trailing slash', "import x from '../main'\nexport default x"],
  [
    'dynamic import, string literal',
    "export const load = () => import('../main/App.vue')",
  ],
  ['dynamic import, bare directory', "export const load = () => import('../main')"],
  // A template literal keeps its static text in quasis, so the rule has to look
  // past the first one: here the interpolation comes *before* the `main/`
  // segment, which an implementation that only inspects `quasis[0]` misses.
  [
    'dynamic import, template literal, interpolation after the segment',
    'export const load = (n) => import(`../main/views/${n}.vue`)',
  ],
  [
    'dynamic import, template literal, interpolation before the segment',
    'export const load = (d) => import(`../${d}/main/App.vue`)',
  ],
  [
    'dynamic import, template literal, bare directory',
    'export const load = (d) => import(`../${d}/main`)',
  ],
]

/**
 * Specifiers that merely contain the letters `main`, plus the imports the
 * boundary is supposed to leave alone. If one of these ever starts failing, the
 * rule has been widened into something people will route around.
 */
const LEAVES_ALONE = [
  [
    'a directory whose name ends in main',
    "import a from '../domain/model'\nexport default a",
  ],
  [
    'a dynamic import of such a directory',
    "export const load = () => import('../domain/model')",
  ],
  [
    'a template-literal dynamic import of such a directory',
    'export const load = (n) => import(`../domain/${n}`)',
  ],
  [
    'a hyphenated sibling of the segment',
    "import a from './main-thread'\nexport default a",
  ],
  ['a dynamic import of it', "export const load = () => import('./main-thread')"],
  [
    'a longer word starting with the segment',
    "import a from './maintenance/log'\nexport default a",
  ],
  ['a dynamic import of it', "export const load = () => import('./maintenance/log')"],
  [
    'an import of src/shared/, which is allowed',
    "import a from '../shared/renderer/x'\nexport default a",
  ],
]

/** @type {Array<[string, string]>} Directory under `src/`, and the sentence its violations must open with. */
const GUARDED_DIRECTORIES = [
  ['output', 'The projector bundle (src/output/) must not import from src/main/.'],
  ['shared', 'src/shared/ must not import from src/main/.'],
]

describe.each(GUARDED_DIRECTORIES)('src/%s/', (dir, opening) => {
  it.each(REACHES_INTO_MAIN)('rejects a %s', async (_label, code) => {
    const messages = await boundaryMessages(dir, code)

    expect(messages.length).toBeGreaterThan(0)
    // Not just "something fired": the message a developer reads has to be the
    // one written for *this* directory. The two rationales differ because the
    // reasons differ, and a shared constant would let them drift into one.
    for (const message of messages) {
      expect(message.message).toContain(opening)
    }
  })

  it.each(LEAVES_ALONE)('allows %s', async (_label, code) => {
    expect(await boundaryMessages(dir, code)).toEqual([])
  })
})

/**
 * Every spelling Vite will pull into a bundle from under `src/`, minus `.vue`,
 * which needs an SFC body and gets its own case below.
 *
 * The `files` patterns carrying the two boundary blocks are the only thing that
 * decides whether a file is looked at at all, and a spelling missing from them
 * is not a weaker rule — it is no rule, silently, with every gate green. While
 * those patterns stopped at `{ts,vue}`, a `src/shared/x.js` importing
 * `src/main/` linted clean and was bundled anyway.
 */
const BUNDLED_EXTENSIONS = ['js', 'mjs', 'cjs', 'jsx', 'ts', 'mts', 'cts', 'tsx']

describe.each(GUARDED_DIRECTORIES)('src/%s/, every bundled extension', (dir, opening) => {
  it.each(BUNDLED_EXTENSIONS)('guards a .%s file', async (extension) => {
    // Distinct basenames per form, so nothing here depends on how one fixture
    // shadows another.
    const statik = await boundaryMessages(
      dir,
      "import App from '../main/App.vue'\nexport default App",
      `static-fixture.${extension}`,
    )
    const dynamic = await boundaryMessages(
      dir,
      "export const load = () => import('../main/App.vue')",
      `dynamic-fixture.${extension}`,
    )

    expect(statik.length).toBeGreaterThan(0)
    expect(dynamic.length).toBeGreaterThan(0)
    for (const message of [...statik, ...dynamic]) {
      expect(message.message).toContain(opening)
    }
  })

  // The other half: an extension added to the pattern list must not turn the
  // rule into something wider than it is on `.ts`, or the new spellings get
  // routed around rather than reported.
  it.each(BUNDLED_EXTENSIONS)(
    'still leaves near misses alone in a .%s file',
    async (extension) => {
      const code = [
        "import a from '../domain/model'",
        "import b from './main-thread'",
        "import c from './maintenance/log'",
        'export default [a, b, c]',
      ].join('\n')

      expect(await boundaryMessages(dir, code, `negative-fixture.${extension}`)).toEqual(
        [],
      )
    },
  )

  it('guards a .vue single-file component', async () => {
    const code = [
      '<script setup lang="ts">',
      "import App from '../main/App.vue'",
      '</script>',
      '',
      '<template><App /></template>',
      '',
    ].join('\n')

    const messages = await boundaryMessages(dir, code, 'sfc-fixture.vue')

    expect(messages.length).toBeGreaterThan(0)
    for (const message of messages) {
      expect(message.message).toContain(opening)
    }
  })
})

describe('the boundary is one-directional', () => {
  it('lets src/main/ import whatever it likes', async () => {
    const code =
      "import a from '../shared/renderer/x'\nimport b from './stores/session'\nexport default [a, b]"
    expect(await boundaryMessages('main', code)).toEqual([])
  })
})

describe('the two rationales', () => {
  it('say different things, because the reader is in a different position', async () => {
    const code = "import App from '../main/App.vue'\nexport default App"
    const [fromOutput] = await boundaryMessages('output', code)
    const [fromShared] = await boundaryMessages('shared', code)

    expect(fromOutput?.message).toBeDefined()
    expect(fromShared?.message).toBeDefined()
    expect(fromShared?.message).not.toBe(fromOutput?.message)
    // src/shared/ is inside the projector bundle rather than being it, so its
    // message has to name the transitive path; that is the whole reason the
    // author of a src/shared/ file would not have expected the rule.
    expect(fromShared?.message).toContain('src/output/ → src/shared/ → src/main/')
  })

  it('reads as prose after interpolation, with no doubled or missing spaces', async () => {
    for (const [dir] of GUARDED_DIRECTORIES) {
      const [violation] = await boundaryMessages(
        dir,
        "import App from '../main/App.vue'\nexport default App",
      )
      const text = String(violation?.message)

      expect(text).not.toMatch(/ {2}/)
      expect(text).not.toMatch(/[a-z],[A-Za-z]/)
      expect(text.trimEnd().endsWith('.')).toBe(true)
    }
  })
})

/**
 * Not the bundle boundary, but the same ESLint instance — and constructing a
 * second one costs another 12–21 s cold start for four lints. The config claims
 * that collocated tests under `src/` get `globals.node` through an `ignores`
 * on the browser block, and that no file lands in two globals sets; both halves
 * are checkable in the same process, so they are checked here.
 *
 * Only the `.js` spellings can carry these cases. `typescript-eslint`'s
 * recommended set switches `no-undef` off for TypeScript files — measured, not
 * assumed: `src/output/x.ts` reports neither `document` nor `process` as
 * undefined — because `tsc` owns that dimension there. So the globals lists are
 * observable exactly where they are load-bearing.
 */
describe('the two globals sets are disjoint', () => {
  /**
   * @param {string} file Path relative to the repository root.
   * @param {string} identifier A global to reference.
   * @returns {Promise<boolean>} Whether ESLint called it undefined.
   */
  async function isUndefined(file, identifier) {
    const [result] = await eslint.lintText(`${identifier}\nexport {}\n`, {
      filePath: resolve(ROOT, file),
      warnIgnored: false,
    })
    return (result?.messages ?? []).some((message) => message.ruleId === 'no-undef')
  }

  it('gives a source file under src/ the browser set and not the node set', async () => {
    expect(await isUndefined('src/output/widget.js', 'document')).toBe(false)
    expect(await isUndefined('src/output/widget.js', 'process')).toBe(true)
  })

  it('gives a collocated test under src/ the node set and not the browser set', async () => {
    // The exchange is the point. A collocated test runs under Vitest's
    // `environment: 'node'`, so `process` has to be a global there — and
    // `document` has to stay undefined, because a component test written in
    // this suite fails on a missing DOM rather than passing hollowly
    // (vitest.config.ts). If the browser block stopped excluding these files,
    // the second assertion is what would notice.
    expect(await isUndefined('src/output/widget.test.js', 'process')).toBe(false)
    expect(await isUndefined('src/output/widget.test.js', 'document')).toBe(true)
  })

  it('gives a test under tests/ the node set', async () => {
    expect(await isUndefined('tests/unit/thing.test.js', 'process')).toBe(false)
    expect(await isUndefined('tests/unit/thing.test.js', 'document')).toBe(true)
  })
})

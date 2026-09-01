import js from '@eslint/js'
import prettier from 'eslint-config-prettier/flat'
import vue from 'eslint-plugin-vue'
import globals from 'globals'
import tseslint from 'typescript-eslint'

/**
 * The half of the rationale that is a fact about the project rather than about
 * whoever tripped the rule. Stated once so the two boundaries below can never
 * disagree about why the boundary exists.
 */
const BUNDLE_SEPARATION_RATIONALE =
  'src/output/ is a separate Vite entry whose size backs the 28 MB Projector ' +
  'Output line in the PRD §5.1 memory budget, and the bundle separation is a ' +
  'contract (PRD §6.3).'

/**
 * The other half both messages share: what to do when the rule fired at
 * something that never pointed at `src/main/` in the first place. The boundary
 * matches specifier text rather than resolved paths (ADR-0021), so this is a
 * real outcome, and a reader who hits it should not have to find the ADR to
 * understand why they are being stopped.
 *
 * Each message below is attached to two rules, not one: `no-restricted-imports`
 * for the static forms and `no-restricted-syntax` for the dynamic ones. So the
 * way out is written against whichever rule the report names — pointing at one
 * of the two constants by name would be wrong for half the readers, who would
 * narrow the list that was never matching them and see the same error again.
 */
const SEGMENT_MATCH_CAVEAT =
  'If this import does not point at src/main/ at all, the rule matched a ' +
  'specifier whose own main is a standalone path segment (ADR-0021). Renaming ' +
  'that file or directory clears it; for a dependency subpath, which cannot be ' +
  'renamed, silence it with an eslint-disable comment naming the rule that ' +
  'fired, or narrow the entry for that rule in eslint.config.js.'

/** Why `src/output/` may not reach into `src/main/`: it *is* the bundle. */
const OUTPUT_ISOLATION_MESSAGE =
  'The projector bundle (src/output/) must not import from src/main/. ' +
  `${BUNDLE_SEPARATION_RATIONALE} Shared code belongs in src/shared/. ` +
  SEGMENT_MATCH_CAVEAT

/**
 * Why `src/shared/` may not either — a different reason, deliberately worded
 * differently. `src/shared/` is not the projector bundle; it is *inside* it, so
 * its imports are transitive and the ban it inherits is not obvious from where
 * the author is sitting.
 */
const SHARED_ISOLATION_MESSAGE =
  'src/shared/ must not import from src/main/. It is consumed by the projector ' +
  'bundle, so this import is pulled into src/output/ too: the path ' +
  'src/output/ → src/shared/ → src/main/ lands exactly the code that the direct ' +
  `import is forbidden for. ${BUNDLE_SEPARATION_RATIONALE} Either move the piece ` +
  'you need down into src/shared/, or leave it in src/main/ and import it only ' +
  `from src/main/. ${SEGMENT_MATCH_CAVEAT}`

/**
 * Import specifiers with `main` as a standalone path segment, for
 * `no-restricted-imports`. Two patterns cover every spelling because neither is
 * anchored: the one ending in `/main/**` matches the relative forms
 * (`../main/App.vue`, `./../main/…`, `../../main/…`) and an aliased `@/main/x`
 * alike, and the one ending in `/main`, with no trailing segment, covers the
 * directory import, which resolves to `src/main/index.ts` the moment one
 * exists.
 *
 * An explicit `@/main`, `@/main/**` pair used to sit here as well, described as
 * cover in case a path alias were introduced later. Measured through the
 * `Linter` API against this ESLint version, it never changed a result:
 * `@/main`, `@/main/x` and `@/main/stores/session` each report once with the
 * pair and once without it, and `~/main/x` — an alias prefix nobody listed —
 * reports too. Nor could a future alias change that, because matching is on
 * specifier text and never on a resolved path (ADR-0021). Dead entries that
 * read as cover are worse than no entries, so they are gone.
 *
 * What is matched is the specifier *text*, not where it resolves to, so the set
 * is wider than `src/main/`: `./main`, `./widgets/main/x` and a dependency's
 * `somelib/main` subpath are all caught from `src/output/` and `src/shared/`
 * even though none of them reaches the Control Panel. ADR-0021 accepts that
 * deliberately — a false positive is a rename, a false negative is a projector
 * bundle that grows with no symptom — and `SEGMENT_MATCH_CAVEAT` is what tells
 * the person who hit one which of the two they are looking at.
 */
const MAIN_IMPORT_GROUPS = ['**/main', '**/main/**']

/**
 * `no-restricted-imports` only inspects static import and re-export
 * declarations — verified against this ESLint version, a dynamic
 * `import('../main/App.vue')` passes it untouched. Lazy loading is exactly how
 * Control Panel code would realistically end up somewhere it does not belong,
 * so the dynamic form is closed with `no-restricted-syntax` instead.
 *
 * Two selectors, because the argument comes in two statically analysable
 * shapes. `import('../main')` is a `Literal`; a template-literal argument is a
 * `TemplateLiteral` whose static text lives in `TemplateElement` quasis — and
 * route-name lazy loading is written in the template form more often than not.
 * Every quasi is matched, not just the first, so an interpolation ahead of the
 * segment (a leading `../` followed by a substituted directory) does not hide
 * the `/main/` that comes after it.
 *
 * These are two of the argument shapes, not all of them, and they are missed
 * for two different reasons. `import(specifier)` and a template whose `main`
 * segment is itself interpolated genuinely carry no literal text to match.
 * `import('../main/' + name)`, `import(flag ? '../main/a' : '../main/b')` and
 * `import('../main/x' as string)` do carry it — `'../main/'` is right there in
 * the source — and what lets them past is the `>` in the selectors below: it is
 * a direct-child combinator, and in those forms the `Literal` hangs off a
 * `BinaryExpression`, `ConditionalExpression` or `TSAsExpression` rather than
 * off the `ImportExpression` itself.
 *
 * Loosening `>` to a descendant match would not reach the first group — there
 * is no literal in it to reach — and *would* reach the second. `>` is kept
 * anyway, because a descendant match reaches every other `Literal` under the
 * `ImportExpression` too, and none of those is specifier text: verified against
 * this ESLint version, `ImportExpression Literal[…]` also fires on
 * `import(name === 'main' ? './a.vue' : './b.vue')` and on the attributes
 * argument of `import('./x.json', { with: { type: 'main' } })`. So the second
 * group stays open — deliberately, and bounded: reaching it takes computing a
 * specifier, which is not how the accidental import this guard exists for gets
 * written (ADR-0021 — this rule guards mistakes, not an adversary).
 *
 * `(^|/)main(/|$)` requires `main` to be a whole path segment: `domain/x` and
 * `main-thread` are left alone, while the trailing `$` catches the directory
 * import that a `main/` pattern would miss.
 */
const DYNAMIC_MAIN_IMPORT_SELECTORS = [
  String.raw`ImportExpression > Literal[value=/(^|\/)main(\/|$)/]`,
  String.raw`ImportExpression > TemplateLiteral > TemplateElement[value.cooked=/(^|\/)main(\/|$)/]`,
]

/**
 * Every extension Vite will pull into a bundle from under `src/`, as one
 * constant so the blocks that narrow by directory cannot drift apart.
 *
 * Measured against Vite 8.2.1 rather than recalled: a `src/shared/` file of
 * each of the five spellings this list did not previously carry — `.cjs`,
 * `.cts`, `.jsx`, `.mts`, `.tsx` — was imported from `src/output/output.ts`,
 * and `vite build` emitted the contents of all five into
 * `dist/assets/output-*.js`. `.mjs`/`.mts`/`.jsx`/`.tsx` also resolve
 * extensionless (Vite's `DEFAULT_EXTENSIONS`) and go through the
 * `/\.(m?ts|[jt]sx)$/` transform filter; `.cjs`/`.cts` do neither, so they have
 * to be named outright in the specifier, but rolldown then loads them as
 * CommonJS and bundles them all the same. `.js`, `.ts` and `.vue` are the
 * spellings the two entry points are already written in.
 *
 * What decides whether a file is bundled is its extension, not its syntax: a
 * `src/shared/x.tsx` holding ordinary TypeScript without a single JSX element
 * in it is bundled exactly like `x.ts`, and `x.mts` is plain TypeScript under
 * another name. While the patterns stopped at `{ts,vue}`, a `src/shared/x.js`
 * importing `src/main/` passed both boundary rules with every gate green — the
 * exact bundle the rules below exist to prevent, waved through on file
 * extension.
 *
 * `tsconfig.json` spells this same set out entry by entry in its `include`;
 * tsconfig globs have no brace expansion, so the two lists cannot be one
 * constant and each file's comment names the other instead. A file in one list
 * and not the other is guarded by one gate and invisible to the rest.
 *
 * One thing this set does not buy: in `.cjs`/`.cts` the reachable form is
 * `require('../main/x')`, and `no-restricted-imports` inspects import and
 * re-export declarations only. Those two spellings are covered here against
 * `import`, not against `require`.
 */
const SOURCE_EXTENSIONS = 'js,mjs,cjs,jsx,ts,mts,cts,tsx,vue'

/** The set above as a glob, relative to whichever directory prefixes it. */
const BUNDLED_SOURCES = `**/*.{${SOURCE_EXTENSIONS}}`

/**
 * Tests that sit next to the module they exercise, the convention
 * `vitest.config.ts` states and collects. They run under `environment: 'node'`,
 * so they need `globals.node` — but they live under `src/`, whose block hands
 * out `globals.browser`. Naming them once lets the browser block exclude
 * exactly this set instead of the two blocks overlapping on it.
 */
const COLLOCATED_TESTS = `src/**/*.test.{${SOURCE_EXTENSIONS}}`

/**
 * The rule that actually earns its keep is `no-restricted-imports` at the
 * bottom. Everything above it is the ordinary recommended stack.
 *
 * Type-aware linting (`recommendedTypeChecked`) is deliberately not enabled:
 * it needs a `projectService` pass over every file on every run, and
 * `npm run typecheck` (`vue-tsc`) already covers the type dimension. If a
 * later item wants a rule that only exists in the type-aware set, that is the
 * moment to pay for it.
 */
export default tseslint.config(
  {
    // Flat config applies `ignores` globally only in a block of its own.
    ignores: [
      'dist/',
      'node_modules/',
      'src-tauri/target/',
      // Regenerated by tauri-build on every compile.
      'src-tauri/gen/',
      // Generated from Rust (NFR-33); the generator owns their style.
      'src/shared/bindings/',
    ],
  },

  js.configs.recommended,
  tseslint.configs.recommended,
  vue.configs['flat/recommended'],

  {
    // Frontend sources: both entry bundles run in the WebView2 renderer.
    // Collocated test files are cut back out — they run in Node, not in the
    // WebView2, and the block below is where they get their globals. Excluding
    // them here rather than letting both blocks match keeps every file in
    // exactly one globals set.
    files: [`src/${BUNDLED_SOURCES}`],
    ignores: [COLLOCATED_TESTS],
    languageOptions: {
      globals: globals.browser,
    },
  },

  {
    // `.vue` files are parsed by vue-eslint-parser, which needs to be told
    // which parser to hand `<script lang="ts">` blocks to.
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },

  {
    // Build tooling and the `postbuild`/`preinstall` hooks run under Node, and
    // so does Vitest — `tests/` and the collocated files under `src/` are
    // listed because a test reaching for `process` or `console` without this is
    // a `no-undef` error rather than a finding.
    //
    // Same extension set as the frontend block, for one reason: `node` will run
    // a `scripts/x.mjs` or `scripts/x.cjs` straight from an npm lifecycle hook,
    // and while this pattern stopped at `scripts/**/*.js` those two spellings
    // got browser globals by default. `COLLOCATED_TESTS` is the only pattern
    // here that reaches under `src/`, and the block above excludes exactly it,
    // so no file lands in two globals sets.
    files: [
      `scripts/${BUNDLED_SOURCES}`,
      `tests/${BUNDLED_SOURCES}`,
      `*.config.{${SOURCE_EXTENSIONS}}`,
      COLLOCATED_TESTS,
    ],
    languageOptions: {
      globals: globals.node,
    },
  },

  {
    // The two bundle roots. `vue/multi-word-component-names` guards against a
    // component name colliding with a current or future HTML element, which can
    // only happen for a component used as a tag in someone else's template.
    // These two are mounted by `createApp(...)` and never written as tags, and
    // PRD §6.13 names `src/output/Renderer.vue` explicitly. The rule stays on
    // everywhere else.
    files: ['src/main/App.vue', 'src/output/Renderer.vue'],
    rules: {
      'vue/multi-word-component-names': 'off',
    },
  },

  {
    // PRD §6.3: `src/output/` is a separate bundle whose size is a budget line,
    // not an implementation detail. Reaching into `src/main/` compiles fine and
    // looks harmless in review — it just pulls views, stores and eventually the
    // Template Builder into the projector bundle, and the cost only shows up as
    // a memory number nobody is measuring during the change.
    //
    // The reverse direction is not restricted — the Control Panel may
    // legitimately share code with the renderer. `src/shared/**` is open to
    // both as a *dependency*; what it may itself depend on is narrowed by the
    // block below.
    files: [`src/output/${BUNDLED_SOURCES}`],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [{ group: MAIN_IMPORT_GROUPS, message: OUTPUT_ISOLATION_MESSAGE }],
        },
      ],
      'no-restricted-syntax': [
        'error',
        ...DYNAMIC_MAIN_IMPORT_SELECTORS.map((selector) => ({
          selector,
          message: OUTPUT_ISOLATION_MESSAGE,
        })),
      ],
    },
  },

  {
    // The transitive half of the same contract. `src/output/` is allowed to
    // import `src/shared/`, so anything `src/shared/` imports is in the
    // projector bundle by construction — leaving this directory unguarded made
    // `src/output/` → `src/shared/` → `src/main/` an open door that produces a
    // bundle indistinguishable from the one the block above forbids.
    //
    // `src/shared/bindings/` is in the global `ignores` (generated from Rust,
    // NFR-33) and is therefore not covered here. That is intended: the
    // generator emits type-only declarations and owns that directory's style.
    files: [`src/shared/${BUNDLED_SOURCES}`],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [{ group: MAIN_IMPORT_GROUPS, message: SHARED_ISOLATION_MESSAGE }],
        },
      ],
      'no-restricted-syntax': [
        'error',
        ...DYNAMIC_MAIN_IMPORT_SELECTORS.map((selector) => ({
          selector,
          message: SHARED_ISOLATION_MESSAGE,
        })),
      ],
    },
  },

  // Last: switches off the stylistic rules Prettier owns, so the two tools
  // never disagree about the same line.
  prettier,
)

// @ts-check

/**
 * Three files have to agree about which spellings under `src/`, `scripts/` and
 * `tests/` a gate is allowed to look at: `eslint.config.js` (`SOURCE_EXTENSIONS`,
 * which both bundle-boundary blocks are built from), `tsconfig.json` (`include`,
 * which is the whole of what `vue-tsc` reads) and `vitest.config.ts`
 * (`include`, which is the whole of what `npm test` runs).
 *
 * They cannot be one constant — tsconfig globs have no brace expansion — so each
 * of the three carries a comment pointing at the other two. A comment is not a
 * gate. Every time this set has drifted so far, the symptom was the same and it
 * was silent: a `src/shared/x.js` importing `src/main/` linted clean, type-checked
 * clean and was bundled anyway; a `scripts/x.test.ts` ran in the suite and was
 * never type-checked at all. Both were found by reading, not by a red gate —
 * which is the ADR-0020 failure class, in the files added to close it.
 *
 * Read as text rather than imported, deliberately: what is being checked is that
 * the three *declarations* still say the same thing. Importing `eslint.config.js`
 * would also cost the 12–21 s plugin load that
 * `tests/unit/eslint-bundle-boundary.test.js` documents, for an answer this
 * cheaper form gives exactly.
 */

import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')

/**
 * @param {string} name Path relative to the repository root.
 * @returns {string}
 */
const read = (name) => readFileSync(resolve(ROOT, name), 'utf8')

/**
 * `SOURCE_EXTENSIONS` as `eslint.config.js` actually declares it. This is the
 * source of truth of the three; the other two files restate it.
 *
 * @returns {string[]}
 */
function sourceExtensions() {
  const match = /const SOURCE_EXTENSIONS = '([^']+)'/.exec(read('eslint.config.js'))
  if (match?.[1] === undefined) {
    throw new Error(
      'SOURCE_EXTENSIONS is no longer declared as a single-quoted string literal in ' +
        'eslint.config.js; this file reads it as text and has to be taught the new shape.',
    )
  }
  return match[1].split(',')
}

describe('the extension set the three gates share', () => {
  it('is the nine spellings vite bundles from under src/', () => {
    // Pinned rather than derived, so shrinking the set in `eslint.config.js`
    // fails here instead of quietly agreeing with itself.
    expect(sourceExtensions()).toEqual([
      'js',
      'mjs',
      'cjs',
      'jsx',
      'ts',
      'mts',
      'cts',
      'tsx',
      'vue',
    ])
  })

  it('is spelled out entry by entry in the tsconfig include', () => {
    const tsconfig = read('tsconfig.json')

    for (const extension of sourceExtensions()) {
      // `src/**` carries all nine; a component is a source file like any other.
      expect(tsconfig).toContain(`"src/**/*.${extension}"`)

      // `scripts/**` and `tests/**` carry the same set minus `.vue`: nothing in
      // either tree is a single-file component, and `vue-tsc` reading one there
      // would be a claim about a file that cannot exist.
      if (extension === 'vue') continue
      expect(tsconfig).toContain(`"scripts/**/*.${extension}"`)
      expect(tsconfig).toContain(`"tests/**/*.${extension}"`)
    }
  })

  it('is the set vitest collects from all three directories, minus .vue', () => {
    const vitestConfig = read('vitest.config.ts')
    const spellings = sourceExtensions()
      .filter((extension) => extension !== 'vue')
      .join(',')

    for (const directory of ['scripts', 'src', 'tests']) {
      expect(vitestConfig).toContain(`'${directory}/**/*.test.{${spellings}}'`)
    }
  })

  it('leaves no test file that runs but is never type-checked', () => {
    // The narrow statement of the same contract, and the one that has actually
    // been broken: every directory Vitest collects from has to appear in the
    // tsconfig include too, or a file can run green in the suite while `vue-tsc`
    // has never read a line of it.
    const vitestConfig = read('vitest.config.ts')
    const tsconfig = read('tsconfig.json')

    for (const directory of ['scripts', 'src', 'tests']) {
      expect(vitestConfig).toContain(`'${directory}/**/*.test.`)
      expect(tsconfig).toContain(`"${directory}/**/*.ts"`)
    }
  })
})

describe('the environment vitest.config.ts promises', () => {
  it('has no DOM, so a component test fails loudly instead of passing hollowly', () => {
    // `environment: 'node'` with no DOM implementation installed. The config
    // states this as the reason component tests are out of scope for now; if a
    // DOM ever appears here by accident, that limit stops being visible and the
    // decision to widen it stops being deliberate (NFR-16, installer budget).
    expect(typeof document).toBe('undefined')
    expect(typeof window).toBe('undefined')
  })

  it('is node, so a test may reach for process', () => {
    expect(typeof process).toBe('object')
  })
})

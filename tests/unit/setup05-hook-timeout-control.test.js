// @ts-check

/**
 * Proves the mechanism `check-fixture-content.e2e.test.js` and
 * `setup05-lifecycle.test.js` now rely on — passing an explicit timeout as
 * the second argument to `beforeAll` — actually overrides Vitest's separate
 * `hookTimeout` budget, rather than being silently ignored the way an
 * unpropagated `describe(..., { timeout })` option would be (that option only
 * ever covers `testTimeout`, never `hookTimeout` — see the doc comment atop
 * `check-fixture-content.e2e.test.js`).
 *
 * Same method `tests/unit/gitignore-guard.test.js` already established for
 * `testTimeout`: run the file under a flag that would fail everything by
 * default, alongside a control case that carries no override, and confirm
 * the override wins while the control genuinely fails. Run this file with:
 *
 *   npx vitest run tests/unit/setup05-hook-timeout-control.test.js --hookTimeout=1
 *
 * Expected (and verified — see PROGRESS.md / the tester report that added
 * this file): the "covered" case passes, the "control" case's `beforeAll`
 * times out and its one `it` is reported as failed — proving both that
 * `--hookTimeout=1` really does bind here (the control fails) and that the
 * per-hook `timeout` argument really does override it (the covered case
 * survives a flag that would otherwise kill every hook in the file).
 *
 * Under the suite's real config (no `--hookTimeout` flag, default 10 000 ms)
 * both cases pass — this file is not a claim that the delay below is
 * dangerous in normal runs, only a probe for whether the override argument
 * is honoured at all.
 */

import { afterAll, beforeAll, describe, expect, it } from 'vitest'

/**
 * Longer than any `--hookTimeout` flag this file is deliberately run under
 * (1 ms) but short enough to cost nothing in a normal `npm test` run.
 */
const DELAY_MS = 50

/** @type {number[]} */
const log = []

/**
 * @param {number} ms
 * @returns {Promise<void>}
 */
function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

describe('covered — beforeAll carries an explicit timeout override', () => {
  beforeAll(async () => {
    await delay(DELAY_MS)
    log.push(1)
  }, 5_000)

  afterAll(() => {
    log.length = 0
  })

  it('the hook is allowed to finish because its own timeout, not --hookTimeout, applies', () => {
    expect(log).toEqual([1])
  })
})

describe('control — beforeAll carries no override, same delay', () => {
  beforeAll(async () => {
    await delay(DELAY_MS)
  })

  it(
    'is not asserted here — this case exists to fail under --hookTimeout=1, ' +
      'proving the flag actually binds. Under the real suite config (no flag, ' +
      '10s default) it passes trivially.',
    () => {
      expect(true).toBe(true)
    },
  )
})

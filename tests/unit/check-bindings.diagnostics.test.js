// @ts-check

/**
 * `scripts/check-bindings.js` — the real `pretypecheck` entry point — had no
 * test of any kind until this file. Its pure decisions live in
 * `bindings-guard.js` and are covered by `scripts/bindings-guard.test.js`, but
 * the *printing* layer is not pure and is not covered there: which of the two
 * diagnostic branches a reader is shown for a given pair of files is decided
 * here, in `check-bindings.js`, and a wrong choice there is invisible to every
 * assertion about `describeFirstDifference`'s return value.
 *
 * The branch that most needs it is the newest: when `describeFirstDifference`
 * comes back with `null` on both sides (pure line-ending drift — see the
 * OBSERVED case in `scripts/bindings-guard.test.js`), the script must print a
 * line-ending message rather than `first difference at line N` naming a line
 * past the end of both files. That branch was exercised once by hand through a
 * real CLI run and then had nothing guarding it.
 *
 * Why the real script is imported rather than reassembled: the branch under
 * test is three lines of `main()` and does not exist anywhere else. Why cargo
 * is not run: `npm test`'s budget was deliberately brought down from 189 s and
 * a Rust build in every run would hand it straight back. `check-bindings.js` is
 * written thin precisely so the generator can be substituted — it spawns
 * `cargo` once, reads two directories, compares, prints — so `node:child_process`
 * is mocked with a stand-in that writes the "generated" files the real cargo
 * run would have written into the same `TS_RS_EXPORT_DIR` scratch directory the
 * script itself created. Everything else in the run is real: the real scratch
 * `mkdtemp`, the real `collectBindings` walk over both directories, the real
 * comparison, the real `console.error` output.
 *
 * The *committed* side is the repository's own `src/shared/bindings/`, read but
 * never written (this suite must not touch generated output — PRD §6.13, "do
 * not edit"), and each case derives its synthetic generated side from it. That
 * makes the committed directory being non-empty a precondition rather than an
 * assumption, and every case asserts it: with no `.ts` file there, a test that
 * "passed" would be comparing nothing against nothing.
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'

import { describe, expect, it, vi } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')
const COMMITTED_DIR = join(ROOT, 'src', 'shared', 'bindings')
const CHECK_BINDINGS = '../../scripts/check-bindings.js'

/**
 * Holder for the stand-in generator, set per test. `vi.hoisted` because
 * `vi.mock`'s factory is hoisted above every other statement in the file and
 * cannot close over an ordinary module-level binding.
 */
const cargo = vi.hoisted(() => ({
  /** @type {(scratch: string) => void} */
  generate: () => {},
  /** @type {{ command: string, args: string[] }[]} */
  calls: [],
}))

vi.mock('node:child_process', () => ({
  /**
   * Stands in for one `cargo test … export_bindings` run: records how it was
   * invoked and writes whatever the case under test says the generator would
   * have produced, into the directory the script redirected the export to.
   *
   * @param {string} command
   * @param {string[]} args
   * @param {{ env?: NodeJS.ProcessEnv }} options
   */
  spawnSync: (command, args, options) => {
    cargo.calls.push({ command, args })
    const scratch = options.env?.TS_RS_EXPORT_DIR
    if (!scratch) throw new Error('harness: the script did not set TS_RS_EXPORT_DIR')
    cargo.generate(scratch)
    return { status: 0, stdout: '', stderr: '', signal: null, pid: 0, output: [] }
  },
}))

/**
 * Every committed binding, keyed the way `collectBindings` keys it.
 *
 * @returns {Map<string, string>}
 */
function committedBindings() {
  /** @type {Map<string, string>} */
  const files = new Map()
  for (const entry of readdirSync(COMMITTED_DIR, { recursive: true, encoding: 'utf8' })) {
    if (!entry.endsWith('.ts')) continue
    const absolute = join(COMMITTED_DIR, entry)
    files.set(entry.split('\\').join('/'), readFileSync(absolute, 'utf8'))
  }
  expect(
    files.size,
    'precondition: src/shared/bindings/ must hold at least one generated .ts file, ' +
      'or this suite compares nothing against nothing',
  ).toBeGreaterThan(0)
  return files
}

/**
 * @param {string} scratch
 * @param {Map<string, string>} files
 */
function writeInto(scratch, files) {
  for (const [name, content] of files) {
    const absolute = join(scratch, ...name.split('/'))
    mkdirSync(dirname(absolute), { recursive: true })
    writeFileSync(absolute, content)
  }
}

/**
 * Runs the real `check-bindings.js` end to end with `generate` standing in for
 * cargo, and returns what it printed and what it set as the process exit code.
 *
 * `process.exitCode` is captured and put back immediately: the script assigns
 * it at module scope, and a leaked `1` would make a fully green `npm test` exit
 * non-zero. The assignment is complete by the time the dynamic import resolves
 * (it is the module's last top-level statement, and it awaits `main()`), so
 * there is no window in which the leaked value is observable.
 *
 * The captured value is restored verbatim, `null` included: Node types
 * `process.exitCode` as `number | string | null | undefined`, where both `null`
 * and `undefined` mean "no code was set". Normalising one to the other on the
 * way back would make this helper write a value the runner never had, so the
 * union is carried through to the return type instead — the assertions below
 * are what pin the code to `0` or `1`, and they say nothing weaker for it.
 *
 * @param {(scratch: string) => void} generate
 * @returns {Promise<{
 *   exitCode: number | string | null | undefined,
 *   stdout: string,
 *   stderr: string,
 * }>}
 */
async function runGuard(generate) {
  cargo.generate = generate
  cargo.calls = []
  /** @type {string[]} */
  const out = []
  /** @type {string[]} */
  const err = []
  const logSpy = vi.spyOn(console, 'log').mockImplementation((...args) => {
    out.push(args.join(' '))
  })
  const errorSpy = vi.spyOn(console, 'error').mockImplementation((...args) => {
    err.push(args.join(' '))
  })
  const previousExitCode = process.exitCode
  try {
    // The script does its whole job as an import side effect, so it has to be
    // re-evaluated per case rather than reused from the module cache.
    vi.resetModules()
    await import(CHECK_BINDINGS)
    return { exitCode: process.exitCode, stdout: out.join('\n'), stderr: err.join('\n') }
  } finally {
    process.exitCode = previousExitCode
    logSpy.mockRestore()
    errorSpy.mockRestore()
  }
}

describe('check-bindings.js — the drift diagnostic it prints', () => {
  it('prints the line-ending message, and no line number, for pure CRLF-vs-LF drift', async () => {
    // The case `bindings-guard.test.js` pins from the other side: after the CR
    // strip no shared line differs, so `describeFirstDifference` returns
    // `{ line: <past the end>, generated: null, committed: null }`. Quoting
    // that would print `(no such line)` twice and describe nothing that exists.
    const committed = committedBindings()
    const flipped = new Map(
      [...committed].map(([name, content]) => [
        name,
        content.includes('\r\n')
          ? content.replace(/\r\n/g, '\n')
          : content.replace(/\n/g, '\r\n'),
      ]),
    )
    // The premise, not assumed: the two sides really do differ in bytes, and
    // really are identical line for line once a trailing CR is stripped.
    for (const [name, content] of committed) {
      expect(flipped.get(name), name).not.toBe(content)
      expect(flipped.get(name)?.split(/\r?\n/), name).toEqual(content.split(/\r?\n/))
    }

    const result = await runGuard((scratch) => writeInto(scratch, flipped))

    expect(result.exitCode).toBe(1)
    for (const name of committed.keys()) {
      expect(result.stderr).toContain(`src/shared/bindings/${name} is out of date.`)
    }
    expect(result.stderr).toMatch(/differ only in line endings/)
    expect(result.stderr).toMatch(/\.gitattributes pins this tree to LF/)
    // The branch this test exists to keep the script out of.
    expect(result.stderr).not.toMatch(/first difference at line/)
    expect(result.stderr).not.toMatch(/\(no such line\)/)
  })

  it('prints the line number and both sides for ordinary content drift', async () => {
    // The other branch, asserted in the same run shape so that "the
    // line-ending message was printed" cannot be satisfied by a script that
    // prints it for everything.
    const committed = committedBindings()
    const [name, content] = [...committed][0] ?? ['', '']
    const lines = content.split('\n')
    expect(
      lines.length,
      'precondition: the binding must have a third line',
    ).toBeGreaterThan(2)
    const drifted = [...lines]
    drifted[2] = `${lines[2]} // SYNTHETIC drift, invented for this test`

    const result = await runGuard((scratch) =>
      writeInto(scratch, new Map([[name, drifted.join('\n')]])),
    )

    expect(result.exitCode).toBe(1)
    expect(result.stderr).toContain(`src/shared/bindings/${name} is out of date.`)
    expect(result.stderr).toContain('first difference at line 3:')
    expect(result.stderr).toContain(`generated now: ${JSON.stringify(drifted[2])}`)
    expect(result.stderr).toContain(`committed:     ${JSON.stringify(lines[2])}`)
    expect(result.stderr).not.toMatch(/differ only in line endings/)
    expect(result.stderr).toMatch(/npm run bindings:generate/)
  })

  it('passes, printing no diagnostic at all, when the generator reproduces the committed files', async () => {
    // The control: without this, both cases above would pass identically
    // against a script that reported drift unconditionally.
    const committed = committedBindings()

    const result = await runGuard((scratch) => writeInto(scratch, committed))

    expect(result.stderr).toBe('')
    expect(result.exitCode).toBe(0)
    expect(result.stdout).toMatch(
      new RegExp(`${committed.size} generated file\\(s\\) match the Rust types`),
    )
    // ADR-0020: without `--workspace`, `--manifest-path` names the package
    // `aeroworship` alone and the generator never reaches `aeroworship-core`,
    // where the exported types live. Asserted here because this is the one
    // test that sees the argv the script actually spawns.
    expect(cargo.calls).toHaveLength(1)
    expect(cargo.calls[0]?.command).toBe('cargo')
    expect(cargo.calls[0]?.args).toContain('--workspace')
    expect(cargo.calls[0]?.args).toContain('export_bindings')
  })

  it('fails when the generator writes nothing, rather than reading an empty run as clean', async () => {
    // `cargo test <filter>` exits 0 when the filter matches nothing, so a
    // stand-in that succeeds while producing no files is exactly the shape of
    // a ts-rs rename or a removed `#[ts(export)]`.
    const result = await runGuard(() => {})

    expect(result.exitCode).toBe(1)
    expect(result.stderr).toMatch(/produced no files at all/)
    expect(result.stderr).toMatch(/nothing was verified/)
  })
})

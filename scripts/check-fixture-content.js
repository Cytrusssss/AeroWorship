// @ts-check

/**
 * `pretest` hook (ADR-0023): rejects any staged `.aero`/`.aerotpl` file, or
 * any staged binary media/render artefact, that has not been positively
 * declared synthetic in a `fixtures.manifest.json`.
 *
 * Wired to `pretest` rather than a git hook. ADR-0023 asks for "a checker
 * bound to the npm lifecycle", explicitly pointing at `check-dist-html.js`'s
 * `postbuild` hook as the shape to copy — and a git hook cannot be that
 * shape here regardless: this repository has zero git-hook infrastructure
 * (no husky, no lefthook, nothing under `.git/hooks/` is tracked, because
 * hooks do not survive a clone), so a `pre-commit` hook would work on the
 * machine that installs it and nowhere else, which is worse than the false
 * confidence it would create. `pretest` runs on every `npm test`, which is
 * one of the six commands every one of these agents' briefs already runs
 * before reporting an item done, and is what CI would run too.
 *
 * That choice has a real, stated cost, not a hidden one: this checker reads
 * `git`'s **index** (see below), so it inspects whatever is staged *at the
 * moment `npm test` runs* — it does not run at commit time and cannot block
 * a commit the way a pre-commit hook would. A file staged and then
 * immediately committed without an intervening `npm test` slips past this
 * exact mechanism. What it does not miss, because "the index" is exactly the
 * right thing to have asked in each case:
 *
 *   - a fixture polluted with real content, staged now — caught, because it
 *     is in the index the moment this next runs;
 *   - `git add -f`, which produces exactly the same index entry `git add`
 *     would have — caught the same way, uniformly, with no special case;
 *   - a file that slipped through **before** this guard existed and has sat
 *     committed ever since — caught, because a committed-and-unmodified file
 *     is still in the index; `git ls-files` lists it same as anything else.
 *     A `git diff --cached` based checker would have missed this one
 *     entirely, since nothing about it is currently staged relative to
 *     `HEAD` — that is the gap ADR-0023 flags explicitly as the third hole
 *     `.gitignore` cannot close, and it is why this reads the index directly
 *     via `git ls-files` + `git cat-file --batch` rather than a diff against
 *     any particular commit.
 *
 * So: everything already committed is covered every time `npm test` runs: a
 * real, recurring safety net, just not a synchronous block on `git commit`.
 * Given the second is unavailable without infrastructure this repository has
 * deliberately not taken on, the first is the honest choice — see
 * `check-fixture-content.js`'s sibling doc in `tests/perf/.gitkeep` and
 * `tests/integration/.gitkeep` for where a fixture author is pointed instead.
 */

import { execFileSync } from 'node:child_process'
import process from 'node:process'
import { fileURLToPath } from 'node:url'

import {
  buildBatchRequest,
  classifyIndexedPaths,
  evaluateCandidate,
  gitlinkVerdict,
  identifyAllowlistedExtensionCandidates,
  identifyCandidates,
  isGitlinkMode,
  parseCatFileBatch,
  parseLsFilesEntry,
  summariseRun,
  unsafePathVerdict,
} from './fixture-content-guard.js'

// One level up from `scripts/`, resolved from this module's own location so
// the check behaves the same regardless of the caller's working directory —
// same reasoning as `distDir` in `check-dist-html.js`.
const root = fileURLToPath(new URL('..', import.meta.url))

// Generous on purpose: this reads the full staged content of every tracked,
// non-allowlisted file in the repository (needed to sniff media magic
// numbers — see `fixture-content-guard.js`), and `.aero` documents are
// capped at 256 KB by the schema itself (PRD Appendix C validation rules).
// 64 MiB is headroom, not a measured requirement.
const MAX_BUFFER = 64 * 1024 * 1024

/**
 * @param {readonly string[]} args
 * @param {string} [input]
 * @returns {Buffer}
 */
function git(args, input) {
  try {
    return execFileSync('git', ['-C', root, ...args], { input, maxBuffer: MAX_BUFFER })
  } catch (error) {
    const failure = /** @type {NodeJS.ErrnoException} */ (error)
    if (failure.code === 'ENOENT') {
      throw new Error(
        'fixture content guard: git was not found on PATH. This guard reads the index via ' +
          'git, so without it the guard is unverified — that is a failure, not something to ' +
          'skip past.',
        { cause: error },
      )
    }
    throw error
  }
}

/** @returns {Promise<number>} Process exit code. */
async function main() {
  // `-s` (not just `-z`) so each record carries its git index mode — needed
  // to tell a submodule (gitlink, mode 160000) apart from an ordinary
  // tracked file before deciding how to treat it (see `gitlinkVerdict` /
  // ADR-0023's W3 finding). `-z` keeps the listing NUL-delimited, so an
  // embedded `\n` or `\r` in a path never corrupts *this* parse — that
  // hazard only exists downstream, in the `git cat-file --batch` stdin
  // protocol `classifyIndexedPaths`'s `unsafe` bucket guards against.
  /** @type {Array<{ mode: string, path: string }>} */
  let entries
  try {
    const raw = git(['ls-files', '-z', '-s'])
    const records = raw
      .toString('utf8')
      .split('\0')
      .filter((record) => record.length > 0)
    entries = records.map((record) => {
      const parsed = parseLsFilesEntry(record)
      if (!parsed) {
        throw new Error(
          'fixture content guard: `git ls-files -z -s` produced a record this guard could ' +
            `not parse: ${JSON.stringify(record)}`,
        )
      }
      return parsed
    })
  } catch (error) {
    console.error('fixture content guard: could not list the git index.')
    console.error(String(error))
    return 1
  }

  // A git repository with nothing at all in its index means this call was
  // made against the wrong directory, or git itself failed silently instead
  // of throwing — either way, nothing was verified, and that must not look
  // like "clean". This repository always has tracked files; a genuinely
  // fresh `git init` with no commits is not a case this checker ever runs
  // against, so the stricter reading is the right one here.
  if (entries.length === 0) {
    console.error(
      `fixture content guard: \`git ls-files\` returned nothing under ${root} — nothing was ` +
        'verified. Confirm this is being run inside the AeroWorship working tree.',
    )
    return 1
  }

  const gitlinks = entries.filter((entry) => isGitlinkMode(entry.mode)).map((e) => e.path)
  const indexed = entries.filter((entry) => !isGitlinkMode(entry.mode)).map((e) => e.path)

  const { allowlisted, toInspect, unsafe } = classifyIndexedPaths(indexed)

  /** @type {Map<string, import('./fixture-content-guard.js').BatchEntry>} */
  let contentByPath = new Map()
  if (toInspect.length > 0) {
    /** @type {Buffer} */
    let batchOutput
    try {
      batchOutput = git(['cat-file', '--batch'], buildBatchRequest(toInspect))
    } catch (error) {
      console.error('fixture content guard: `git cat-file --batch` failed.')
      console.error(String(error))
      return 1
    }
    try {
      contentByPath = parseCatFileBatch(batchOutput, toInspect)
    } catch (error) {
      console.error(String(error))
      return 1
    }
  }

  const { candidates, unreadable } = identifyCandidates(toInspect, contentByPath)
  // The allowlist (`src-tauri/icons/` today) only excuses a path from the
  // `cat-file`/sniff round-trip above — it does not excuse the
  // declared-extension check, which needs no content at all (ADR-0023's
  // "di mana pun ia berada"; see `ALLOWLISTED_PREFIXES`'s doc). A `.aero`
  // staged under an allowlisted prefix is still a candidate, and still gets
  // rejected by `evaluateCandidate` for having no `fixtures/` ancestor.
  const allowlistedCandidates = identifyAllowlistedExtensionCandidates(allowlisted)
  // Gitlinks and `unsafe` (newline/CR-carrying) paths are automatic
  // violations, judged before — and independently of — anything content-based:
  // neither is ever handed to `git cat-file --batch`, so neither can ever be
  // reported as clean by omission (see `gitlinkVerdict` / `unsafePathVerdict`
  // in `fixture-content-guard.js`).
  const candidateVerdicts = [
    ...gitlinks.map(gitlinkVerdict),
    ...unsafe.map(unsafePathVerdict),
    ...candidates.map((candidate) => evaluateCandidate(candidate, contentByPath)),
    ...allowlistedCandidates.map((candidate) =>
      evaluateCandidate(candidate, contentByPath),
    ),
  ]

  const summary = summariseRun({
    indexedCount: entries.length,
    allowlistedCount: allowlisted.length,
    candidateVerdicts,
    unreadable,
  })

  for (const verdict of candidateVerdicts) {
    if (!verdict.ok) {
      console.error(`fixture content guard: ${verdict.path}\n    ${verdict.reason}`)
    }
  }
  for (const path of unreadable) {
    console.error(
      `fixture content guard: ${path}\n    ` +
        'listed by `git ls-files` but its content could not be confirmed present in the ' +
        'index a moment later (index changed mid-scan?) — treated as unverified, not clean.',
    )
  }

  if (summary.violationCount > 0) {
    console.error(
      `\nfixture content guard: ${summary.violationCount} of ${summary.checkedCount} ` +
        'candidate(s) rejected.\n' +
        'Every `.aero`/`.aerotpl` file, and every binary media/render artefact, staged ' +
        'anywhere in the repository must have an explicit `synthetic: true` declaration in a ' +
        '`fixtures.manifest.json` at the root of the containing `fixtures/` directory ' +
        '(ADR-0023). See `scripts/fixture-content-guard.js` for the manifest shape.',
    )
  }
  if (summary.unreadableCount > 0) {
    console.error(
      `\nfixture content guard: ${summary.unreadableCount} candidate(s) could not be verified.`,
    )
  }

  if (summary.exitCode !== 0) return summary.exitCode

  if (summary.checkedCount === 0) {
    console.log(
      `fixture content guard: ${entries.length} indexed path(s) scanned ` +
        `(${allowlisted.length} allowlisted, ${toInspect.length} inspected), 0 candidate(s) ` +
        'found — no .aero/.aerotpl and nothing sniffed as media. Nothing to declare.',
    )
  } else {
    console.log(
      `fixture content guard: ${summary.checkedCount} candidate(s) verified synthetic, clean.`,
    )
  }
  return 0
}

process.exitCode = await main()

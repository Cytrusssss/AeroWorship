// @ts-check

/**
 * `preinstall` hook: refuses a bare `npm install`, requires `npm ci`.
 *
 * A bare `npm install` is free to re-resolve semver ranges and rewrite
 * `package-lock.json` as a side effect of what looks like a read-only setup
 * step. Two people who "just installed" then build against different dependency
 * trees, and the drift lands in the lockfile diff of whatever commit happens to
 * be open. `npm ci` installs the lockfile exactly and fails instead of editing
 * it, which is the property the six verification commands need in order to mean
 * anything.
 *
 * How the three cases are told apart — verified empirically against npm 10.9.2
 * on this machine, not from memory:
 *
 * | Command                | root `preinstall` runs | `npm_command` |
 * | ---------------------- | ---------------------- | ------------- |
 * | `npm ci`               | yes                    | `ci`          |
 * | `npm install`          | yes                    | `install`     |
 * | `npm install <pkg>`    | **no**                 | —             |
 *
 * npm only runs the root lifecycle scripts for a full install, so adding a
 * dependency on purpose never reaches this file and keeps working untouched.
 * `npm_config_argv` was not usable: npm 10 no longer sets it.
 *
 * This is a guard rail, not a security boundary — `--ignore-scripts` skips it,
 * as it skips every lifecycle script.
 */

import process from 'node:process'

// Anything other than a bare `npm install` — `npm ci`, `npm run`, a different
// package manager, or a direct `node` invocation — passes through.
if (process.env.npm_command === 'install') {
  console.error(
    [
      '',
      '  Use `npm ci`, not `npm install`.',
      '',
      '  A bare `npm install` may re-resolve version ranges and quietly rewrite',
      '  package-lock.json, so the build stops being reproducible. `npm ci`',
      '  installs exactly what the lockfile pins and fails rather than editing it.',
      '',
      '  To add or update a dependency on purpose, name it:',
      '',
      '      npm install <package>            # runtime dependency',
      '      npm install --save-dev <package> # tooling',
      '      npm update <package>',
      '',
      '  Those forms do not run this check and are unaffected.',
      '  Committing the resulting package-lock.json is part of the change.',
      '',
    ].join('\n'),
  )
  process.exit(1)
}

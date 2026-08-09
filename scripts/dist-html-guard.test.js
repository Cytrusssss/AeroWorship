// @ts-check

/**
 * The failure paths matter more than the happy path here: this guard is the
 * only place a `style-src` regression becomes visible before a user sees an
 * unstyled projector (ADR-0015), so a check that silently stops detecting
 * anything is worse than no check at all.
 */

import { describe, expect, it } from 'vitest'

import {
  findCspViolations,
  formatViolation,
  hasModuleScript,
  selectDocuments,
  summariseDocuments,
} from './dist-html-guard.js'

/** The shape `vite build` produces today: external module script + linked CSS. */
const CLEAN_DOCUMENT = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <title>AeroWorship</title>
    <!-- A comment mentioning inline style and script, which must not trip the scan. -->
    <script type="module" crossorigin src="/assets/main-ByytIpAi.js"></script>
    <link rel="modulepreload" crossorigin href="/assets/_plugin-vue_export-helper.js">
    <link rel="stylesheet" crossorigin href="/assets/main-Bv9soRNb.css">
  </head>
  <body>
    <div id="app"></div>
  </body>
</html>
`

/** @param {ReturnType<typeof findCspViolations>} violations */
const kinds = (violations) => violations.map((violation) => violation.kind)

/**
 * Asserts a scan found something and hands back the first hit, so the
 * assertions below can talk about a value rather than about `T | undefined`.
 *
 * @param {ReturnType<typeof findCspViolations>} violations
 * @returns {ReturnType<typeof findCspViolations>[number]}
 */
function first(violations) {
  const [violation] = violations
  if (violation === undefined) {
    throw new Error('expected at least one violation, found none')
  }
  return violation
}

describe('findCspViolations', () => {
  it('passes the document shape vite build emits today', () => {
    expect(findCspViolations(CLEAN_DOCUMENT)).toEqual([])
  })

  it('passes a script element whose body is whitespace only', () => {
    const html = '<script type="module" src="/assets/main.js">\n  \n</script>'
    expect(findCspViolations(html)).toEqual([])
  })

  it('rejects an inline <style> block', () => {
    const html = '<head><style>body { margin: 0 }</style></head>'
    expect(kinds(findCspViolations(html))).toEqual(['style-element'])
  })

  it('rejects an inline <style> block regardless of case or attributes', () => {
    const html = '<STYLE TYPE="text/css">body{margin:0}</STYLE>'
    // `type="text/css"` is not a style attribute, so this is one violation.
    expect(kinds(findCspViolations(html))).toEqual(['style-element'])
  })

  it('rejects an inline style attribute', () => {
    const html = '<div id="app" style="display: none"></div>'
    expect(kinds(findCspViolations(html))).toEqual(['style-attribute'])
  })

  it('rejects an inline style attribute in any case', () => {
    const html = '<div STYLE="color: red"></div>'
    expect(kinds(findCspViolations(html))).toEqual(['style-attribute'])
  })

  it('rejects a single-quoted style attribute', () => {
    const html = "<div id='app' style='display: none'></div>"
    expect(kinds(findCspViolations(html))).toEqual(['style-attribute'])
  })

  it('rejects an unquoted style attribute', () => {
    const html = '<div style=display:none></div>'
    expect(kinds(findCspViolations(html))).toEqual(['style-attribute'])
  })

  it('rejects a style attribute with whitespace around the equals sign', () => {
    const html = '<div style = "color: red"></div>'
    expect(kinds(findCspViolations(html))).toEqual(['style-attribute'])
  })

  it('quotes name and value in the excerpt for every value syntax', () => {
    expect(first(findCspViolations('<div style="color:red">')).excerpt).toBe(
      'style="color:red"',
    )
    expect(first(findCspViolations("<div style='color:red'>")).excerpt).toBe(
      "style='color:red'",
    )
    expect(first(findCspViolations('<div style=red>')).excerpt).toBe('style=red')
  })

  // These are the tests that stop the boundary from being quietly relaxed
  // later: the attribute name has to stand on its own to count.
  it('passes an attribute whose name merely ends in style', () => {
    expect(findCspViolations('<div data-style="x"></div>')).toEqual([])
    expect(findCspViolations('<div xstyle="x"></div>')).toEqual([])
    expect(findCspViolations("<div my-style='x'></div>")).toEqual([])
    expect(findCspViolations('<div data-style=x></div>')).toEqual([])
  })

  it('passes a linked stylesheet, which mentions style but sets no attribute', () => {
    const html = '<link rel="stylesheet" href="/assets/style-Bv9soRNb.css">'
    expect(findCspViolations(html)).toEqual([])
  })

  it('counts a <style> element once, not also as an attribute', () => {
    expect(kinds(findCspViolations('<style>a{}</style>'))).toEqual(['style-element'])
  })

  it('rejects a <script> element with a body', () => {
    const html = '<script>window.__BOOT__ = true</script>'
    expect(kinds(findCspViolations(html))).toEqual(['inline-script'])
  })

  it('rejects an inline body on a script that also has a src attribute', () => {
    const html = '<script type="module" src="/assets/main.js">console.log(1)</script>'
    expect(kinds(findCspViolations(html))).toEqual(['inline-script'])
  })

  it('reports every violation in a document, ordered by position', () => {
    const html = [
      '<html><head>',
      '<style>a{}</style>',
      '</head><body>',
      '<div style="color:red"></div>',
      '<script>boot()</script>',
      '</body></html>',
    ].join('\n')

    expect(kinds(findCspViolations(html))).toEqual([
      'style-element',
      'style-attribute',
      'inline-script',
    ])
  })

  it('locates a violation by line and column', () => {
    const html = '<html>\n<body>\n  <div style="color:red"></div>\n</body>\n</html>'
    const violation = first(findCspViolations(html))

    expect(violation.line).toBe(3)
    expect(violation.column).toBe(8)
  })

  it('flattens and truncates a long inline script into a one-line excerpt', () => {
    const body = `\n${'x'.repeat(200)}\n`
    const violation = first(findCspViolations(`<script>${body}</script>`))

    expect(violation.excerpt).not.toContain('\n')
    expect(violation.excerpt.endsWith('…')).toBe(true)
  })
})

describe('formatViolation', () => {
  it('renders a file:line:column diagnostic naming the construct', () => {
    const violation = first(findCspViolations('<div style="color:red"></div>'))

    expect(formatViolation('dist/index.html', violation)).toContain('dist/index.html:1:6')
    expect(formatViolation('dist/index.html', violation)).toContain('style-attribute')
  })
})

describe('selectDocuments', () => {
  // The reason this function was pulled out of `check-dist-html.js` at all.
  // A recursive `readdir` on Windows returns `pages\x.html`; if that reaches
  // the `expected` membership test unnormalised, a document the build was
  // required to emit is reported absent while sitting right there, and the
  // guard fails a build for the wrong reason. Until this case existed the
  // guarantee lived in a comment.
  it('normalises Windows separators and still selects the entry', () => {
    const { documents, missing } = selectDocuments(['pages\\x.html'], [])

    expect(documents).toEqual(['pages/x.html'])
    expect(missing).toEqual([])
  })

  it('matches an expected name against the normalised nested path', () => {
    const { missing } = selectDocuments(['pages\\output.html'], ['pages/output.html'])

    expect(missing).toEqual([])
  })

  it('reports an expected document that is absent', () => {
    const { documents, missing } = selectDocuments(
      ['index.html'],
      ['index.html', 'output.html'],
    )

    expect(documents).toEqual(['index.html'])
    expect(missing).toEqual(['output.html'])
  })

  it('reports nothing missing when both expected documents are present', () => {
    const { documents, missing } = selectDocuments(
      ['index.html', 'output.html'],
      ['index.html', 'output.html'],
    )

    expect(documents).toEqual(['index.html', 'output.html'])
    expect(missing).toEqual([])
  })

  it('reports every absent name, not just the first', () => {
    expect(selectDocuments([], ['index.html', 'output.html']).missing).toEqual([
      'index.html',
      'output.html',
    ])
  })

  it('drops entries that are not .html', () => {
    const { documents } = selectDocuments(
      ['index.html', 'assets/main-ByytIpAi.js', 'assets/main.css', 'vite.svg', 'assets'],
      [],
    )

    expect(documents).toEqual(['index.html'])
  })

  it('accepts an .html suffix in any case', () => {
    expect(selectDocuments(['INDEX.HTML'], []).documents).toEqual(['INDEX.HTML'])
  })

  it('leaves a name that merely contains .html in the middle alone', () => {
    expect(selectDocuments(['index.html.map', 'x.htmlx'], []).documents).toEqual([])
  })

  it('orders the selected documents deterministically', () => {
    const listing = ['output.html', 'pages\\b.html', 'index.html', 'pages\\a.html']

    expect(selectDocuments(listing, []).documents).toEqual([
      'index.html',
      'output.html',
      'pages/a.html',
      'pages/b.html',
    ])
    // Same set, different listing order — the caller prints this list and the
    // reports are built from it, so the order has to come from the function.
    expect(selectDocuments([...listing].reverse(), []).documents).toEqual(
      selectDocuments(listing, []).documents,
    )
  })

  it('selects a directory whose name ends in .html', () => {
    // Deliberate: the listing is strings and nothing here touches the file
    // system. `check-dist-html.js` is what turns the failed read into a named
    // diagnostic, and it can only do that for a name it was handed.
    expect(selectDocuments(['weird.html'], []).documents).toEqual(['weird.html'])
  })

  it('handles an empty expected list', () => {
    expect(selectDocuments(['index.html'], [])).toEqual({
      documents: ['index.html'],
      missing: [],
    })
  })

  it('handles an empty listing', () => {
    expect(selectDocuments([], [])).toEqual({ documents: [], missing: [] })
  })

  it('does not mutate either argument', () => {
    const entries = ['output.html', 'index.html']
    const expected = ['index.html', 'output.html']

    selectDocuments(entries, expected)

    expect(entries).toEqual(['output.html', 'index.html'])
    expect(expected).toEqual(['index.html', 'output.html'])
  })
})

describe('hasModuleScript', () => {
  it('accepts the exact tag vite build emits', () => {
    // `crossorigin` sits between `type` and `src`, so a scanner that only
    // recognises the two attributes adjacent passes this by accident of
    // nothing. This is the shape read off dist/index.html.
    expect(
      hasModuleScript(
        '<script type="module" crossorigin src="/assets/main-ByytIpAi.js"></script>',
      ),
    ).toBe(true)
  })

  it('accepts the whole document vite build emits', () => {
    expect(hasModuleScript(CLEAN_DOCUMENT)).toBe(true)
  })

  it.each([
    ['reversed attribute order', '<script src="/assets/main.js" type="module"></script>'],
    ['unquoted values', '<script type=module src=/assets/main.js></script>'],
    ['single-quoted values', "<script type='module' src='/assets/main.js'></script>"],
    ['upper case', '<SCRIPT TYPE="MODULE" SRC="/assets/main.js"></SCRIPT>'],
    ['whitespace around the equals signs', '<script type = "module" src = "/x.js">'],
    ['padded type value', '<script type=" module " src="/x.js"></script>'],
    ['a trailing slash on the start tag', '<script type="module" src="/x.js"/>'],
    [
      'a second script that carries it',
      '<script>boot()</script><script type="module" src="/x.js"></script>',
    ],
  ])('accepts %s', (_label, html) => {
    expect(hasModuleScript(html)).toBe(true)
  })

  it.each([
    ['an empty document', ''],
    [
      'a document with no script at all',
      '<html><body><div id="app"></div></body></html>',
    ],
    ['a module script with no src', '<script type="module">boot()</script>'],
    ['a module script with an empty src', '<script type="module" src=""></script>'],
    [
      'a module script whose src is only whitespace',
      '<script type="module" src="   "></script>',
    ],
    ['a classic script', '<script src="/assets/main.js"></script>'],
    [
      'a script of another type',
      '<script type="text/javascript" src="/assets/main.js"></script>',
    ],
    ['a type that merely contains the word', '<script type="nomodule" src="/x.js">'],
    [
      'modulepreload, which is a link and loads nothing on its own',
      '<link rel="modulepreload" crossorigin href="/assets/_helper.js">',
    ],
  ])('rejects %s', (_label, html) => {
    expect(hasModuleScript(html)).toBe(false)
  })

  it('takes the first spelling of a duplicated attribute, as a parser does', () => {
    expect(hasModuleScript('<script type="module" src="" src="/x.js"></script>')).toBe(
      false,
    )
    expect(hasModuleScript('<script type="module" src="/x.js" src=""></script>')).toBe(
      true,
    )
  })
})

describe('summariseDocuments', () => {
  /**
   * @param {string} name
   * @param {number} violations How many hits the document carries.
   * @param {boolean} loadsModule
   * @returns {import('./dist-html-guard.js').DocumentReport}
   */
  const report = (name, violations, loadsModule) => ({
    name,
    violations: Array.from({ length: violations }, () =>
      first(findCspViolations('<style>')),
    ),
    loadsModule,
  })

  /**
   * @param {string} name
   * @returns {import('./dist-html-guard.js').DocumentReport}
   */
  const unreadable = (name) => ({ name, violations: null, loadsModule: false })

  it('returns a non-zero exit code for an empty report list', () => {
    // Fail-closed on purpose. "Nothing was examined" is the one answer this
    // guard must never dress up as "clean" — the caller checks for it first and
    // prints a better message, but the caller losing that check is exactly the
    // edit this is here to survive (ADR-0020).
    expect(summariseDocuments([])).toEqual({
      violations: 0,
      unreadable: 0,
      blank: 0,
      exitCode: 1,
    })
  })

  it('returns 0 only when something was examined and all of it was clean', () => {
    expect(summariseDocuments([report('index.html', 0, true)])).toEqual({
      violations: 0,
      unreadable: 0,
      blank: 0,
      exitCode: 0,
    })
    expect(
      summariseDocuments([report('index.html', 0, true), report('output.html', 0, true)])
        .exitCode,
    ).toBe(0)
  })

  // The full matrix, because each of the three counts can independently hold
  // the exit code away from zero and a single combination going quiet is how a
  // failing build starts passing.
  it.each([
    ['violations only', [report('a.html', 2, true)], 2, 0, 0],
    ['unreadable only', [unreadable('a.html')], 0, 1, 0],
    ['blank only', [report('a.html', 0, false)], 0, 0, 1],
    [
      'violations and unreadable',
      [report('a.html', 1, true), unreadable('b.html')],
      1,
      1,
      0,
    ],
    ['violations and blank', [report('a.html', 1, false)], 1, 0, 1],
    ['unreadable and blank', [unreadable('a.html'), report('b.html', 0, false)], 0, 1, 1],
    ['all three', [report('a.html', 3, false), unreadable('b.html')], 3, 1, 1],
  ])('fails with %s', (_label, reports, violations, unreadableCount, blank) => {
    expect(summariseDocuments(reports)).toEqual({
      violations,
      unreadable: unreadableCount,
      blank,
      exitCode: 1,
    })
  })

  it('totals violations across documents but counts blanks per document', () => {
    const summary = summariseDocuments([
      report('a.html', 2, false),
      report('b.html', 3, false),
    ])

    expect(summary.violations).toBe(5)
    expect(summary.blank).toBe(2)
  })

  it('lets an unreadable document contribute to neither other count', () => {
    // `loadsModule` is false on an unreadable report because nothing was read,
    // not because the document is blank. Counting it as blank would send the
    // reader looking for an empty file that may be perfectly fine.
    const summary = summariseDocuments([unreadable('a.html'), unreadable('b.html')])

    expect(summary).toEqual({ violations: 0, unreadable: 2, blank: 0, exitCode: 1 })
  })
})

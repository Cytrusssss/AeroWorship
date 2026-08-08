// @ts-check

/**
 * The failure paths matter more than the happy path here: this guard is the
 * only place a `style-src` regression becomes visible before a user sees an
 * unstyled projector (ADR-0015), so a check that silently stops detecting
 * anything is worse than no check at all.
 */

import { describe, expect, it } from 'vitest'

import { findCspViolations, formatViolation } from './dist-html-guard.js'

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

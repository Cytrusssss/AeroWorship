// @ts-check

/**
 * The two halves of FR-105 that are CSS rather than a WebView2 setting — no
 * scrollbars and no text selection on the Projector Output window — asserted on
 * the artefacts `vite build` actually emits, and asserted in *both* directions.
 *
 * PRD FR-105 asks for a window with "no scrollbars … no text selection", and
 * its acceptance criterion names `Ctrl+A`. Neither is reachable from Rust:
 * WebView2 has no setting for either, so `src/output/Renderer.vue` closes them
 * in an unscoped `html, body` block and
 * `src-tauri/src/services/webview_chrome.rs` says so in writing. That leaves the
 * built stylesheet as the only place a regression is observable before an
 * operator sees it — the same argument `scripts/check-dist-html.js` makes for
 * reading `dist/*.html`.
 *
 * **The second direction is not decoration.** `user-select: none` must NOT
 * reach the Control Panel bundle: an operator who cannot select text in the
 * song editor is a defect, not a hardening, and `Renderer.vue` names that as
 * the reason the rule is not in `src/shared/styles/base.css`. A check that only
 * looked at the output bundle would pass a change that moved the rule into the
 * shared stylesheet — hardening the projector and breaking the editor in the
 * same commit, with every gate green.
 *
 * **Why the built CSS and not the SFC source.** The claim is about bundle
 * *separation*, and separation is decided by `vite.config.ts` and by which
 * component imports what — not by the text of one file. A grep over
 * `Renderer.vue` cannot tell "the rule ships to the projector only" from "the
 * rule ships to both windows".
 *
 * **The cost of that, and how it is paid.** `dist/` is git-ignored and no test
 * run rebuilds it, so an artefact assertion can be green about a build that
 * predates the current source — the quiet failure this repository chases
 * elsewhere as ADR-0020. `dist_is_not_older_than_the_sources_that_produce_it`
 * below turns that into a red, so deleting the rule from `Renderer.vue` fails
 * this file whether or not the deleter rebuilt. Its one known hole is named
 * there.
 *
 * What this file does NOT prove: that WebView2 honours either declaration, that
 * a real right-drag selects nothing, or that no scrollbar appears at some
 * window size. Those are the user's runtime verification, like the rest of
 * FR-105.
 */

import { readdirSync, readFileSync, statSync } from 'node:fs'
import { join, resolve } from 'node:path'

import { describe, expect, it } from 'vitest'

const ROOT = resolve(import.meta.dirname, '../..')
const DIST = join(ROOT, 'dist')

/** How to get a `dist/` that matches the working tree; quoted in every failure. */
const REBUILD = 'run `npm run build` (its `postbuild` hook is the dist HTML guard)'

/**
 * Reads one built document, turning an absent `dist/` into a sentence that says
 * what to do instead of an ENOENT stack.
 *
 * @param {string} name File name at the root of `dist/`.
 * @returns {string}
 */
function readDocument(name) {
  try {
    return readFileSync(join(DIST, name), 'utf8')
  } catch (error) {
    throw new Error(
      `FR-105: cannot read dist/${name}, so nothing about the built stylesheets was ` +
        `verified — ${REBUILD}. (${String(error)})`,
      { cause: error },
    )
  }
}

/** One `<link …>` start tag; group 1 is everything between the name and the `>`. */
const LINK_TAG = /<link\b([^>]*)>/gi

/** One attribute, in the three value syntaxes plus the valueless form. */
const TAG_ATTRIBUTE =
  /(?<=^|\s)([^\s"'>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'`=<>]+)))?/g

/**
 * The stylesheets a built document links, as paths inside `dist/`.
 *
 * Read off the document rather than globbed off `dist/assets/`: the claim being
 * asserted is about what each *window* loads, and a file named `output-*.css`
 * that no document links is not what the projector gets. It also means a
 * stylesheet shared by both windows — `_plugin-vue_export-helper-*.css` today —
 * is counted for both, which is exactly what the Control Panel direction needs.
 *
 * @param {string} html
 * @returns {string[]} `href`s with their leading `/` removed.
 */
function linkedStylesheets(html) {
  /** @type {string[]} */
  const hrefs = []

  for (const tag of html.matchAll(LINK_TAG)) {
    /** @type {Map<string, string>} */
    const attributes = new Map()
    for (const attribute of (tag[1] ?? '').matchAll(TAG_ATTRIBUTE)) {
      const name = (attribute[1] ?? '').toLowerCase()
      // First occurrence wins, as the HTML parser does with a duplicate name.
      if (!attributes.has(name)) {
        attributes.set(name, attribute[2] ?? attribute[3] ?? attribute[4] ?? '')
      }
    }

    if (attributes.get('rel')?.trim().toLowerCase() !== 'stylesheet') continue
    const href = (attributes.get('href') ?? '').trim()
    if (href === '') continue
    hrefs.push(href.replace(/^\//, ''))
  }

  return hrefs
}

/**
 * Every stylesheet a document links, concatenated, plus the names read.
 *
 * @param {string} document File name at the root of `dist/`.
 * @returns {{ names: string[], css: string }}
 */
function stylesheetsOf(document) {
  const names = linkedStylesheets(readDocument(document))
  const css = names
    .map((name) => {
      try {
        return readFileSync(join(DIST, name), 'utf8')
      } catch (error) {
        throw new Error(
          `FR-105: dist/${document} links ${name}, which could not be read — ${REBUILD}. ` +
            `(${String(error)})`,
          { cause: error },
        )
      }
    })
    .join('\n')

  return { names, css }
}

/**
 * Declaration blocks of every rule whose selector list is exactly `selectors`,
 * compared as a set so `html,body` and `body, html` are the same rule.
 *
 * Deliberately not a CSS parser. The pattern requires a selector free of
 * braces, so a rule nested in an at-rule (`@media`, `@supports`, a cascade
 * layer) is not found — and "not found" is a failed assertion below, not a
 * silent pass. The build emits no at-rules today; if one ever wraps this rule,
 * this file goes red and sends the reader here rather than to a projector.
 *
 * @param {string} css
 * @param {readonly string[]} selectors
 * @returns {string[]} Declaration blocks, whitespace stripped.
 */
function declarationsFor(css, selectors) {
  const wanted = [...selectors]
    .map((selector) => selector.toLowerCase())
    .sort()
    .join(',')

  /** @type {string[]} */
  const blocks = []
  for (const rule of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const found = (rule[1] ?? '')
      .split(',')
      .map((selector) => selector.trim().toLowerCase())
      .filter((selector) => selector !== '')
      .sort()
      .join(',')
    if (found === wanted) blocks.push((rule[2] ?? '').replace(/\s+/g, ''))
  }

  return blocks
}

/**
 * Newest modification time under `directory`, restricted to `extensions`.
 *
 * @param {string} directory
 * @param {readonly string[]} extensions
 * @returns {{ at: number, file: string }}
 */
function newestSource(directory, extensions) {
  let newest = { at: 0, file: '(none)' }
  for (const entry of readdirSync(directory, { recursive: true, withFileTypes: true })) {
    if (!entry.isFile()) continue
    if (!extensions.some((extension) => entry.name.toLowerCase().endsWith(extension))) {
      continue
    }
    const file = join(entry.parentPath, entry.name)
    const at = statSync(file).mtimeMs
    if (at > newest.at) newest = { at, file }
  }
  return newest
}

describe('FR-105 — the projector stylesheet, as built', () => {
  it('dist_is_not_older_than_the_sources_that_produce_it', () => {
    // `.vue` and `.css` under `src/`, plus `vite.config.ts`, because that file
    // decides which bundle a stylesheet lands in and this file asserts exactly
    // that.
    //
    // Known hole, named rather than left to be discovered: a `.ts`/`.js` module
    // that imports a stylesheet of its own would change the built CSS without
    // being in this set. There is none today; when there is, widen this.
    const newest = newestSource(join(ROOT, 'src'), ['.vue', '.css'])
    const config = join(ROOT, 'vite.config.ts')
    const configAt = statSync(config).mtimeMs
    const source = configAt > newest.at ? { at: configAt, file: config } : newest

    const built = [
      join(DIST, 'output.html'),
      join(DIST, 'index.html'),
      ...linkedStylesheets(readDocument('output.html')).map((name) => join(DIST, name)),
      ...linkedStylesheets(readDocument('index.html')).map((name) => join(DIST, name)),
    ]

    for (const artefact of built) {
      expect(
        statSync(artefact).mtimeMs,
        `FR-105: ${artefact} is older than ${source.file}, so the two assertions below ` +
          'would be about a build that predates the working tree — a deleted ' +
          `\`user-select\` rule would pass. Nothing was verified; ${REBUILD}.`,
      ).toBeGreaterThanOrEqual(source.at)
    }
  })

  it('the_output_window_stylesheet_closes_selection_and_scrolling_on_html_and_body', () => {
    const { names, css } = stylesheetsOf('output.html')

    expect(
      names.length,
      'FR-105: dist/output.html links no stylesheet at all, so the projector would ship ' +
        `unstyled and nothing about selection or scrolling was verified. ${REBUILD}.`,
    ).toBeGreaterThan(0)

    const blocks = declarationsFor(css, ['html', 'body'])
    expect(
      blocks.length,
      'FR-105: no `html, body` rule in the stylesheets dist/output.html links ' +
        `(${names.join(', ')}). The unscoped block in \`src/output/Renderer.vue\` is where ` +
        'FR-105\'s "no scrollbars" and "no text selection" live; without it the projector ' +
        'scrolls on an 8 px body margin and its slide text can be selected with Ctrl+A.',
    ).toBeGreaterThan(0)

    const declarations = blocks.join(';')
    expect(
      declarations,
      "FR-105: the projector's `html, body` rule no longer carries `user-select: none`. " +
        'That declaration is the whole of "no text selection" — WebView2 has no setting ' +
        'for it (see `src-tauri/src/services/webview_chrome.rs`), so nothing else closes it.',
    ).toContain('user-select:none')
    expect(
      declarations,
      "FR-105: the projector's `html, body` rule no longer carries `overflow: hidden`. " +
        'That is "no scrollbars": a slide one pixel taller than the display raises a ' +
        'scrollbar in front of a congregation, and removing the body margin alone does not ' +
        'prevent it.',
    ).toContain('overflow:hidden')
  })

  it('no_stylesheet_the_control_panel_loads_suppresses_text_selection', () => {
    const { names, css } = stylesheetsOf('index.html')

    expect(
      names.length,
      'FR-105: dist/index.html links no stylesheet, so "the rule did not leak into the ' +
        `Control Panel" would be true only because nothing was read. ${REBUILD}.`,
    ).toBeGreaterThan(0)

    expect(
      css.toLowerCase(),
      `FR-105: a stylesheet the Control Panel loads (${names.join(', ')}) now suppresses ` +
        'text selection. FR-105 hardens the Projector Output window only: an operator who ' +
        'cannot select text in the song editor is a defect, not a hardening, which is why ' +
        '`src/output/Renderer.vue` keeps the rule out of `src/shared/styles/base.css`. ' +
        'Check whether the declaration moved into a shared stylesheet or a shared component.',
    ).not.toContain('user-select')
  })
})

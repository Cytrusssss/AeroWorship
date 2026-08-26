//! Splitting a text slot's content into slides that fit it (FR-310, US-16,
//! PRD §6.8).
//!
//! PRD §6.8 calls this "a pure Rust function of `(content, template text slot,
//! canvas)`, splitting only at line boundaries, never mid-word or mid-line.
//! Being pure and deterministic, it is unit-testable and guarantees
//! preview/output identity." Everything below follows from that sentence, and
//! two things about it need saying out loud before the code.
//!
//! **The canvas is not an argument, and cannot be.** Count the units. A line
//! box on a canvas `H` pixels tall is `size · H · line_height` pixels, because
//! Appendix B defines `size` as a *fraction of canvas height* and `line_height`
//! as a *multiple of the type size*. The slot is `box.h · H` pixels tall,
//! because `Rect` is normalised against the same canvas (FR-406). The number of
//! line boxes that fit is therefore
//!
//! ```text
//! (box.h · H) / (size · H · line_height)  =  box.h / (size · line_height)
//! ```
//!
//! and `H` cancels exactly — it is not approximately absent, it never enters.
//! The canvas *width* never enters either, because splitting happens only at
//! line boundaries, so nothing here is measured horizontally. So this module
//! takes no canvas: a parameter the function provably ignores would invite a
//! caller to believe the answer depends on it, and "recompute the split per
//! output resolution" is precisely the failure mode risk R5 is about. The
//! `build_slides` command (Appendix D) still takes a `CanvasSize` for its own
//! reasons; it has no business passing one down here.
//!
//! **What is *not* computable purely, and whose problem it is.** The vertical
//! question above is arithmetic on normalised numbers. The horizontal one — is
//! this line wider than the slot? — is not: it needs the glyph advances of a
//! font this crate cannot see, and `letter_spacing` is a fraction of canvas
//! *height* applied along a width, so the aspect ratio does not cancel there
//! the way `H` does above. That is exactly why the PRD confines splitting to
//! line boundaries. The consequence binds the renderer, not this module: **a
//! renderer that soft-wraps a long line puts more line boxes on the slide than
//! were counted here, and the split no longer holds.** Lines must be laid out
//! as authored (`white-space: pre`), with an over-wide line handled by the
//! Builder's overflow indicator (FR-407), not by reflowing it at present time.
//!
//! **What `overflows` covers, and what it does not.** One claim, and a narrow
//! one: *not one line box fits in the slot, and there is a line with something
//! in it to go there*, where a line box is the CSS line box of
//! `size · line_height` canvas heights and nothing else. Two things sit outside
//! that claim, and neither is detectable here.
//!
//! The first is the horizontal question above. The second is that **a line box
//! is not the ink in it.** How far a glyph reaches above and below the baseline
//! is the font's business — this crate never sees a font — and CSS spreads the
//! difference between the font's own line height and `line_height` as
//! half-leading, half above the text and half below. Below roughly
//! `line_height: 1.2` that half-leading turns *negative*: the first line's
//! ascenders and the last line's descenders are painted outside the slot while
//! N line boxes still fit by the arithmetic, and `overflows` stays `false`.
//! This is not a skipped-validation case — `validate_template` accepts
//! `line_height` down to 0.1 — it is an accepted document whose ink leaves its
//! box.
//!
//! It is stated rather than patched because the right answer is not this
//! crate's to give: it needs the font's metrics, which are not here, and a
//! renderer decision that has not been taken — whether a text slot *clips* its
//! content or lets ink spill past its edge. An ink margin guessed at here would
//! be a number with no evidence behind it that no caller could raise. **So it
//! travels to the renderer work as a requirement instead** (FR-402, FR-403,
//! FR-405, drawing FR-404's text slot), alongside `white-space: pre`: the one
//! renderer those items share owes this module no soft-wrapping, and owes the
//! operator a visible answer for ink that leaves its slot.
//!
//! Doc comments on [`SlideSplit`] and its fields are copied verbatim into
//! `src/shared/bindings/SlideSplit.ts` by ts-rs, so they are written for a
//! frontend reader; this module's own reasoning is in ordinary `//` comments,
//! which are not copied.

use serde::Serialize;

use super::template::{Rect, Typography};

/// The result of fitting one text slot's content: the slides it became, and
/// the type size all of them are set at.
///
/// This is **not** Appendix D's `Slide`. A `Slide` is one projected screen and
/// carries an id, an item and a template; this is the piece that answers "what
/// text goes on each of them", and `slides[i]` is what becomes that slide's
/// `content.primary` (or `content.reference`, for the slot it was computed
/// against).
///
/// Computed once, in Rust, and carried in the slide payload. Neither the
/// Control Panel preview nor the Projector Output may recompute it — that they
/// share one answer is what makes the preview trustworthy (risk R5).
// Serialize only, no Deserialize, for the same reason as `ScriptureRef`: this
// travels one way. A value arriving *from* the frontend would be a split
// nothing in this crate had computed, which is the one thing this type exists
// to prevent.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct SlideSplit {
    /// One entry per slide, in order. Lines within a slide are joined with
    /// `\n` and must be rendered as authored — the renderer may not re-wrap
    /// them, or the count this split was built on no longer holds.
    ///
    /// **This text is untrusted and must enter the DOM as a text node, never
    /// as HTML.** It is whatever a `.aero` file written by another program, a
    /// PPTX or PDF import, or an operator's paste contained (FR-703, FR-501),
    /// and building markup out of it — `innerHTML`, `v-html`, a template
    /// string — runs that file's script in the origin the Control Panel shares
    /// (ADR-0042). The safe answer is also the correct one: with
    /// `white-space: pre` the `\n` between two lines *is* a line break, so
    /// `el.textContent = slides[i]` renders exactly as intended and no `<br>`
    /// is needed anywhere.
    ///
    /// Never empty: content that produces nothing to show still produces one
    /// empty slide, so an item is never a dead end to navigate into (FR-306,
    /// FR-313).
    pub slides: Vec<String>,
    /// The type size to render every slide in this list at, as a fraction of
    /// canvas height — the template's `size`, or a smaller value down to
    /// `min_size` if shrinking was what made the content fit.
    ///
    /// It is one value for the whole list on purpose: the type does not change
    /// size between two slides of the same song.
    ///
    /// **Range.** Positive, finite, and no larger than the template's own
    /// `size`, for any document `validate_template` accepted. It is not
    /// clamped: a document that skipped validation can declare `size: 0` or
    /// `size: -0.05`, and this field reports that number back rather than
    /// inventing a plausible one in its place. What holds instead is the
    /// pairing — whenever `fontSize` is not positive and finite, `overflows`
    /// is `true` for any content with a line that shows something, because no
    /// line box fits at such a size. (Content that is empty or nothing but
    /// blank lines is the one exception, and not a leak: it renders as a single
    /// empty slide, so there is nothing to spill and nothing to warn about.)
    /// Read a non-positive `fontSize` as a broken template to report, not as a
    /// size to paint at.
    pub font_size: f64,
    /// `true` when the content has a line that shows something and the slot is
    /// too short for even a single line at `min_size`, so at least one line
    /// will spill out of its box however the content is divided. Splitting
    /// cannot fix it — only a taller slot, a smaller `min_size`, or less text
    /// can. Surface it (FR-407); the point of US-16 is that nobody discovers
    /// this from the congregation's side of the screen.
    ///
    /// Content that is empty or nothing but blank lines never sets it, however
    /// short the slot: it renders as one empty slide, and nothing spills out of
    /// anything.
    ///
    /// This is not "the content was split". That is `slides.length > 1`, and it
    /// is the ordinary, healthy outcome.
    ///
    /// **It is a vertical, line-box claim and only that.** It does not report a
    /// line too wide for its slot — no font metrics reach the crate that
    /// computed this — and it does not report glyph ink painted outside a line
    /// box that itself fits, which is what a `line_height` below about 1.2
    /// produces: CSS half-leading goes negative there, and the first line's
    /// ascenders and the last line's descenders land outside the slot with
    /// `overflows` still `false`. Both are the renderer's to detect and to
    /// surface (FR-407); a `false` here is not a promise that the slide looks
    /// right.
    pub overflows: bool,
}

/// Splits `content` into slides that fit `text_box` when set in `typography`.
///
/// Breaks fall only between lines, never inside one (PRD §6.8). Lines are the
/// content's own, and "a line" means what a renderer laying this text out with
/// `white-space: pre` will see: `\n`, `\r\n` and a lone `\r` each end one, and
/// so do U+000B, U+000C, U+0085, U+2028 and U+2029. Every one of them comes
/// back as a plain `\n` between two lines of a slide, so the same lyric splits
/// identically whichever way it was stored, and no slide carries a separator
/// this function did not count.
///
/// The result is a function of these three arguments and nothing else: no
/// clock, no locale, no canvas, no font. Called twice with equal arguments it
/// returns equal values, on any machine.
///
/// **The caller bounds the input; this function does not.** There is no upper
/// limit here on the length of `content`, on the number of lines in it, or on
/// the number of slides it becomes, and the multiplier is the *template's* as
/// much as the content's: `box.h: 0.0` is a legal slot (an invisible layer),
/// it holds zero line boxes, and that forces one line per slide — so a 2 MiB
/// paste of a million short lines becomes a million `String`s and a million
/// allocations, carried on two vector backbones that cost more than the strings
/// do: a `Vec<&str>` of the lines at 16 bytes each and a `Vec<String>` of the
/// slides at 24 bytes each, about 40 MiB, the largest single component. With
/// the allocator's per-block minimum for those million tiny strings, the call
/// peaks at roughly **58–74 MiB from 2 MiB of input, some 30–37×**. Both
/// arguments are untrusted (NFR-28), so a caller that has not already bounded
/// the item's text before reaching here has an unbounded allocation, not a slow
/// function.
///
/// The amplification is **linear, not quadratic** — a constant cost per line,
/// with no term that grows with the number of lines already seen. That is what
/// makes it survivable: a caller that caps the bytes of `content` caps this
/// function's peak memory exactly, by that factor, at every size.
///
/// No limit is imposed at this level on purpose. The right one differs per
/// call site — a song verse, a whole imported deck, a Builder preview redrawn
/// on every keystroke — and a number picked here would be one no caller who
/// needed it larger could raise. Bounding belongs where the item is built
/// (FR-312) and where a file is parsed, both of which have a size limit
/// already.
//
// **Why a slot and a typography rather than a `Layer`.** `Layer::Text` also
// carries an id, a role, a visibility flag and its effects, none of which
// change where a line break falls. Taking the two fields that do keeps every
// FR-310 case constructible without building a layer around it, and matching
// `Layer::Text { text_box, typography, .. }` on a `&Layer` hands the caller
// both by reference already.
//
// **This function does not require a validated document** — same stance as
// `validate_template`'s doc comment describes for its own callers. A validated
// one guarantees `size > 0`, `min_size <= size`, `line_height >= 0.1` and
// `max_lines >= 1`; without those guarantees the degenerate values still have
// to produce *a* deterministic answer rather than a panic or an infinity, so
// every arithmetic step below is written to survive a zero, a NaN and an
// absurdity. What it must never do is invent slides that silently drop content.
pub fn split_slides(content: &str, text_box: &Rect, typography: &Typography) -> SlideSplit {
    // The line model is the renderer's, not `str::lines`'s. `content.lines()`
    // breaks on `\n` and drops one attached `\r`, which handles the hazard that
    // lyrics arrive from a `.aero` file, a SQLite column and a browser paste
    // and those three do not agree about line endings — but it stops there,
    // while the `white-space: pre` layout this module *requires* of the
    // renderer breaks on five more characters and on a lone `\r` besides. Every
    // one it misses is a line box on the slide that was never counted, on a
    // slide already full, with `overflows` dark. See `split_lines`.
    let lines: Vec<&str> = split_lines(content);

    // `max_lines: 0` is not a declaration of zero lines. `validate_template`
    // refuses it — the minimum is 1 — so it arrives only from a document that
    // skipped validation, and reading it literally would report `overflows`
    // forever on a slot with room to spare. That is worse than it sounds: the
    // content survives either way, but an alarm that is always lit is an alarm
    // nobody reads, and the two hazards this module cannot detect at all (an
    // over-wide line, ink outside a line box that fits) already depend on this
    // one being believed. So zero means *undeclared*: the geometry alone caps
    // the count, and geometry can never seat more lines than fit.
    let declared = match typography.max_lines {
        0 => usize::MAX,
        stated => usize::from(stated),
    };
    // What shrinking aims at. Appendix B calls `min_size` the "auto-shrink
    // floor *before splitting*", so the goal is to avoid splitting: fit the
    // whole content on one slide if the floor allows it. But never past
    // `declared` — `max_lines` "drives slide splitting" and is the author's
    // stated design, not a hint. Putting a fifth line on a four-line slide is
    // not fitting the text, it is redesigning the slide.
    let wanted = declared.min(lines.len());

    let font_size = shrink_to_fit(wanted, text_box.h, typography);

    // Both limits, and the smaller wins. Geometry can only ever *lower* the
    // count: honouring `max_lines` in a box too short for it is how the
    // congregation gets the clipped line US-16 exists to prevent.
    let fitted = lines_that_fit(text_box.h, font_size, typography.line_height);
    let capacity = fitted.min(declared);

    // Zero capacity means no size and no division of the content can make a
    // line fit. The alternatives are to return no slides (which loses the
    // content outright and leaves FR-313 nothing to navigate to) or to overflow
    // in silence. Neither is acceptable, so: emit the slides one line at a
    // time, and say so.
    //
    // But only when there is ink to spill. `!lines.is_empty()` is not that
    // test: `split_lines("\n")` returns one *empty* line, not zero lines, so a
    // content of nothing but blank lines would light the alarm on a legal
    // `box.h: 0.0` slot whose render is one empty slide with nothing painted
    // outside anything. `is_blank` is already this module's word for "does not
    // contribute anything visible" — the loop below skips such lines at the
    // head of a slide and trims them off its tail — so asking it here keeps the
    // signal and the render agreeing. The same reasoning as `max_lines: 0`
    // above, at the other end: an alarm that is lit with nothing behind it is
    // an alarm nobody reads, and FR-407 is the operator's only warning for the
    // two hazards this module cannot detect at all.
    //
    // `""` and `"\n"` still differ in `font_size`, and that is left alone:
    // `shrink_to_fit` counts a blank line as a line to make room for, which is
    // what makes a stanza gap hold its space everywhere else. The difference is
    // invisible — neither content renders a glyph for the size to apply to —
    // and erasing it would mean handing `shrink_to_fit` a line count that is
    // not the one the split was built on.
    let overflows = capacity == 0 && lines.iter().any(|line| !is_blank(line));
    let per_slide = capacity.max(1);

    let mut slides: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        // A slide never opens with a gap. A blank line between stanzas has
        // already done its work once the stanza ends on the previous slide, and
        // an empty first line pushes everything below it off the vertical
        // centre for nothing.
        while index < lines.len() && is_blank(lines[index]) {
            index += 1;
        }
        if index >= lines.len() {
            break;
        }
        // `saturating_add`. The real bound on `per_slide` is `declared`, not
        // the geometry: `capacity` is `fitted.min(declared)` and `declared` is
        // a `u16` widened, so whenever `max_lines` was declared at all this is
        // at most 65_535 and the addition cannot come near overflowing. The one
        // case without that bound is `max_lines: 0`, read as *undeclared*
        // above, where `fitted` alone caps the count — and `fitted` comes from
        // a float whose cast saturates at `usize::MAX`. So the addition would
        // panic in a debug build there, long before the slice bound below could
        // do anything wrong. Kept for the declared case too: it costs nothing,
        // and the guarantee it stands in for lives in a `.min` one edit away.
        let end = index.saturating_add(per_slide).min(lines.len());
        // Blank lines *count* against the capacity above — they occupy a line
        // box and dropping them would change a stanza's spacing — but a run of
        // them at the end of a slide adds only empty height, so it is not
        // carried into what gets rendered. This trims what is shown, never
        // where the break falls: `end` is already decided.
        let mut slide = &lines[index..end];
        while let Some((last, rest)) = slide.split_last() {
            if is_blank(last) {
                slide = rest;
            } else {
                break;
            }
        }
        slides.push(slide.join("\n"));
        index = end;
    }

    // Empty content, or content that was nothing but blank lines. One empty
    // slide, not zero: FR-312's explicit blank item is exactly this case and it
    // has to be showable, and FR-306/FR-313 navigate by advancing through an
    // item's slides — an item with none of them is a hole in that sequence.
    if slides.is_empty() {
        slides.push(String::new());
    }

    SlideSplit {
        slides,
        font_size,
        overflows,
    }
}

/// The largest type size, no greater than `typography.size` and no smaller than
/// `typography.min_size`, at which `wanted` line boxes fit in a slot `height`
/// tall. Returns `typography.size` unchanged when they already fit.
//
// **One size for the whole item, not one per slide.** Per-slide shrinking would
// let the last slide of a song — the short one — render larger than the three
// before it, and a font that changes size as the operator advances is visible
// from the back row in a way a slightly small font is not. It is also circular:
// the size decides the capacity, the capacity decides which lines land on which
// slide, and per-slide shrinking would then let the contents decide the size.
// That is a fixpoint with no guarantee of a unique answer, which is a poor
// foundation for "identically in preview and output". So the size is chosen
// once, from the slot and the content's length, before any line is placed.
//
// Closed form, not a search. Fitting `n` line boxes in `height` means
// `n · s · line_height <= height`, so the largest such `s` is
// `height / (n · line_height)` — no loop, no step size, and therefore no
// tolerance to tune and nothing that could halt at a different place on a
// different machine.
fn shrink_to_fit(wanted: usize, height: f64, typography: &Typography) -> f64 {
    let size = typography.size;
    if wanted == 0 || lines_that_fit(height, size, typography.line_height) >= wanted {
        return size;
    }

    // `wanted` is bounded by the number of lines in the content, so this cast
    // is exact for any input a person could author or a file could hold.
    let exact = height / (wanted as f64 * typography.line_height);
    if !exact.is_finite() {
        // A `line_height` of zero or a NaN slot height. There is no meaningful
        // smaller size to pick, and shrinking to nothing would hide the text
        // rather than fit it, so the design's own size stands. `validate_template`
        // refuses both inputs; this is what happens when the caller skipped it.
        return size;
    }

    // `max` applies the floor, `min` is the promise never to *enlarge* text —
    // it holds even for the `min_size > size` document `validate_template`
    // rejects. Both are NaN-tolerant in the right direction: `f64::max` and
    // `f64::min` ignore a NaN operand, so a malformed `min_size` leaves `exact`
    // standing instead of poisoning the result.
    exact.max(typography.min_size).min(size)
}

/// How many line boxes of `size · line_height` fit, whole, in `height`. All
/// three are fractions of canvas height, or multiples of one, so the answer is
/// a plain count with no canvas in it.
fn lines_that_fit(height: f64, size: f64, line_height: f64) -> usize {
    let line_box = size * line_height;
    // Degenerate geometry answers zero rather than infinity: a zero-height slot
    // is legal in a template (an invisible layer), and a zero, negative or NaN
    // `size` or `line_height` reaches here only from a document that skipped
    // validation. Zero is the safe answer for all of them — it means "not one
    // line fits", which is true, and it is the case `overflows` already reports.
    //
    // Each factor is tested on its own and not merely their product, because
    // two nonsense numbers multiply to a plausible one: `size: -0.05` with
    // `line_height: -1.25` is a line box of 0.0625, and the product test alone
    // would hand back a real capacity, no `overflows`, and a *negative*
    // `font_size` for the renderer to paint at. The product is still tested
    // after them: two positive extremes can still multiply to an infinity or
    // underflow to zero.
    let usable = height.is_finite()
        && height > 0.0
        && size.is_finite()
        && size > 0.0
        && line_height.is_finite()
        && line_height > 0.0
        && line_box.is_finite()
        && line_box > 0.0;
    if !usable {
        return 0;
    }
    // One division and one `floor`, in one place, called by every path that
    // needs a capacity — including `shrink_to_fit`. Two spellings of the same
    // arithmetic could disagree on a boundary value by one ulp and hand the
    // renderer a slide with one line more than was measured.
    //
    // The float→int cast saturates rather than wrapping (and maps NaN to 0),
    // which is why an absurd ratio is harmless here.
    let fit = height / line_box;
    fit.floor().max(0.0) as usize
}

/// The lines of `content`, cut where the renderer will cut them.
//
// **Which separators, and why these.** `str::lines` knows two of them. The
// seven here are Unicode's mandatory-break characters — UAX #14 classes BK, CR,
// LF and NL — which is also the set a `white-space: pre` layout treats as
// segment breaks, and this module has no business counting lines by a narrower
// rule than the renderer it hands the count to.
//
// **Four classes, not three, and the fourth is why they are worth naming at
// all.** BK gives U+000B, U+000C, U+2028 and U+2029; CR gives U+000D; LF gives
// U+000A; and **U+0085 is class NL, a class of its own** — it is not BK. Three
// classes describe six characters, not the seven below, so a reader checking
// the set against a rule stated that way would find U+0085 unaccounted for and
// delete it. That is precisely the fatal case this module opens with: a line
// box on the slide that was never counted, on a slide already full, with
// `overflows` dark.
//
// The content is the least trustworthy of `split_slides`'s three arguments, and
// each of these reaches it by a route the application already has: PowerPoint
// exports a soft line break as U+000B (FR-501), a paste from a browser or a
// PDF carries U+2028, and a `.aero` written by another program carries whatever
// it liked (FR-703). The class of hazard is known in this crate already —
// `template::is_invisible` names U+2028 and U+2029 as the pair that "break a
// line without being control characters, so `char::is_control` walks straight
// past them". This is the same pair arriving through the other door, where the
// cost is a miscount rather than a bad name.
//
// **They become boundaries: the separator itself is consumed, the text on both
// sides of it is not.** Every one of them can have text on both sides, and
// worship text may not be lost, so that text stays and lands on the line the
// renderer will put it on — while the separator is stepped over and the slide
// is rejoined with `\n`. Consuming it is what keeps the two halves of the
// promise together: a slide can no longer *contain* one, so what was counted
// here and what gets laid out are the same lines. What is refused and deleted
// is nothing; what is *replaced* is the spelling of the break.
//
// **Where renderers disagree, this counts the break — and normalising it takes
// the disagreement away.** Browsers are not unanimous about U+000B and U+0085
// under `white-space: pre`. But because every separator leaves here rewritten
// as `\n`, a renderer that would have ignored one never gets the chance: the
// cost is not one slide boundary it could have disagreed about, it is **one
// hard line break in the rendered text that the source did not have under that
// renderer's own rules** — plus a slide boundary, on the slides where the break
// lands on the edge. That is still the right way to lean, and U+000B is why:
// PowerPoint writes it for a soft break the author typed *as* a line break
// (FR-501), so honouring it renders what was meant, while missing a break a
// renderer does honour clips a line in front of the congregation.
// `lines_that_fit`'s `floor` leans the same way for the same reason.
//
// Borrowed slices, not a normalised copy: rewriting the separators first would
// allocate a second copy of every lyric to answer a question about counting.
fn split_lines(content: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = content;

    // `char_indices().find(..)` rather than `find(is_segment_break)`, which
    // reports only *where* the separator starts and leaves its width to be
    // recovered by looking the char up again — a lookup that returns an
    // `Option` the search has already proved to be `Some`. Written that way the
    // `None` arm is unreachable for every input, and unreachable code still has
    // to say what it does: falling out of the loop there would push `rest` with
    // the separator still inside it, the one thing this function promises never
    // happens. Carrying the matched char out of the search deletes the arm
    // instead of arguing about it, and costs nothing — the closure form of
    // `str::find` walks the same `char_indices` underneath.
    while let Some((at, separator)) = rest.char_indices().find(|&(_, c)| is_segment_break(c)) {
        let (line, tail) = rest.split_at(at);
        lines.push(line);
        // CRLF is one break, so the pair is stepped over together — the case
        // `str::lines` handles by trimming, and the reason a plain `split` on
        // the predicate would not do: it would read a CRLF document as having
        // an empty line between every two lines of it. Any other pair, `\n\r`
        // included, is two breaks and an empty line between them, which is what
        // a renderer shows too.
        let width = if tail.starts_with("\r\n") {
            2
        } else {
            separator.len_utf8()
        };
        rest = &tail[width..];
    }

    // A trailing separator ends the last line rather than opening an empty one,
    // matching `str::lines`: `"a\n"` is one line, `"a\n\n"` is two.
    if !rest.is_empty() {
        lines.push(rest);
    }
    lines
}

/// Whether `c` ends a line, for the renderer this split is measured against.
#[rustfmt::skip]
fn is_segment_break(c: char) -> bool {
    matches!(
        c,
        '\u{000a}'   // LINE FEED
        | '\u{000b}' // LINE TABULATION — PowerPoint's soft break (FR-501)
        | '\u{000c}' // FORM FEED
        | '\u{000d}' // CARRIAGE RETURN — CRLF counts once, see `split_lines`
        | '\u{0085}' // NEXT LINE
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

/// Whether a line contributes nothing visible — empty, or nothing but
/// whitespace. Both are stanza separators as far as a reader is concerned, and
/// a slide that opened with one would show the difference.
fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

//! Splitting a text slot's content into slides (FR-310, US-16, PRD §6.8).
//!
//! FR-310's acceptance criterion is one sentence with three numbers in it —
//! *"a 12-line verse against a 4-line slot produces 3 slides in both preview
//! and output, with identical breaks"* — and [`the_criterion`] is that
//! sentence, number for number: twelve lines, a slot whose `max_lines` is 4
//! *and* whose geometry holds exactly four line boxes at the nominal size, and
//! three slides whose breaks are spelled out as literals. The "in both preview
//! and output" half is not a second code path to test — there is only one
//! function, and neither renderer may recompute it (risk R5) — so it is tested
//! as what makes that safe: the same arguments give the same answer twice, and
//! an algebraically equal slot gives the same answer as well.
//!
//! Everything else here exists to reject a specific wrong implementation
//! rather than to agree with the current one. Named, so a later reader can
//! check they still fail:
//!
//! - shrinking per slide instead of once for the whole item, which lets the
//!   short last slide of a song set the type larger than the three before it;
//! - ignoring `max_lines` and sizing from the geometry alone;
//! - ignoring the geometry and sizing from `max_lines` alone, which is how a
//!   line gets clipped in a slot too short for the count the author declared;
//! - reporting `overflows` for content that was merely split, or never
//!   reporting it at all;
//! - reporting `overflows` for content that is nothing but blank lines, which
//!   lights FR-407’s indicator on a render that has no ink in it to spill —
//!   or losing that report for content which merely *contains* a blank line;
//! - returning no slides for empty content, which leaves FR-306/FR-313 a hole
//!   to navigate into;
//! - dropping blank lines before the capacity is counted, so a stanza gap
//!   stops moving the break it does move;
//! - letting a slide open with a blank line;
//! - spelling `lines_that_fit` with `ceil` (or `round`) instead of `floor` —
//!   the direction that puts one more line on the slide than was measured,
//!   which is the clipped line US-16 exists to prevent;
//! - dropping auto-shrink, so content that would fit at `min_size` is split
//!   anyway;
//! - counting lines with `str::lines`, which knows two of the seven characters
//!   that end a line for the renderer this split is handed to — every one it
//!   misses is an uncounted line box on a slide already full;
//! - reading CRLF as two breaks (an empty line between every two lines of a
//!   Windows-authored file), or any other pair as one (a deleted stanza gap);
//! - carrying a separator into a slide instead of consuming it as the
//!   boundary, so what was counted and what gets laid out are different lines;
//! - letting a trailing separator open an empty last line, which makes
//!   auto-shrink aim at one line more than the item has;
//! - reading `max_lines: 0` as a ceiling of zero rather than as undeclared,
//!   which lights FR-407's overflow indicator on a slot with room to spare.
//!
//! The last one is why several fixtures below set `min_size` equal to `size`:
//! with shrinking available, a slot too short for the content is often made to
//! fit rather than split, and only a slot with no headroom left tells the
//! geometry rules apart.
//!
//! **Every fixture is synthetic.** The lines are `L1`…`L12`, invented for this
//! file. No lyric and no verse text appears anywhere: FR-310 is about where a
//! break falls between lines, and that is a property of the count, not of the
//! words.

use aeroworship_core::models::{
    split_slides, HorizontalAlign, Rect, SlideSplit, TextTransform, Typography, VerticalAlign,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The twelve lines of the acceptance criterion, spelled out rather than
/// generated: a loop that built them would also be a loop that could build
/// eleven.
const TWELVE_LINES: &str = "L1\nL2\nL3\nL4\nL5\nL6\nL7\nL8\nL9\nL10\nL11\nL12";

/// A text slot `h` tall. `x`, `y` and `w` are fixed and arbitrary — splitting
/// happens only at line boundaries, so nothing here is measured horizontally.
fn slot(h: f64) -> Rect {
    Rect {
        x: 0.08,
        y: 0.10,
        w: 0.84,
        h,
    }
}

/// The four typography fields that decide where a break falls. The rest are
/// present because `Typography` has them, and none of them can change the
/// answer.
fn type_set(size: f64, min_size: f64, line_height: f64, max_lines: u16) -> Typography {
    Typography {
        font_family: "Inter".to_string(),
        font_fallback: vec!["sans-serif".to_string()],
        weight: 700,
        size,
        min_size,
        line_height,
        letter_spacing: 0.0,
        transform: TextTransform::None,
        color: "#FFFFFF".to_string(),
        align_h: HorizontalAlign::Center,
        align_v: VerticalAlign::Middle,
        max_lines,
    }
}

/// `L1\nL2\n…\nLn`.
fn numbered(n: usize) -> String {
    (1..=n)
        .map(|i| format!("L{i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The slides as `&str`, so an expectation can be written as an array literal.
fn shown(split: &SlideSplit) -> Vec<&str> {
    split.slides.iter().map(String::as_str).collect()
}

/// How many line boxes of `size · line_height` fit whole in `height`, spelled
/// out here from Appendix B's definitions rather than borrowed from the module
/// under test — an assertion that called the implementation's own helper would
/// agree with a `ceil` just as happily as with a `floor`.
fn line_boxes_that_fit(height: f64, size: f64, line_height: f64) -> usize {
    (height / (size * line_height)).floor() as usize
}

// ---------------------------------------------------------------------------
// The acceptance criterion, and the arithmetic it rests on
// ---------------------------------------------------------------------------

/// The slot the criterion is measured against really does hold four lines.
///
/// "A 4-line slot" is two claims, not one: `max_lines` is 4, *and* the box is
/// tall enough for four line boxes at the nominal size, so no shrinking is
/// involved and the 3 slides come from the split alone. Appendix B makes both
/// checkable — `size` and `box.h` are fractions of canvas height and
/// `line_height` is a multiple of `size`, so `0.375 / (0.075 · 1.25)` is the
/// count, and it is 4 exactly. This is asserted before it is used, because a
/// fixture that quietly held 3 or 5 lines would make every expectation below
/// mean something other than what it says.
#[test]
fn the_four_line_slot_of_the_criterion_holds_exactly_four_lines() {
    assert_eq!(0.075 * 1.25, 0.09375, "one line box, in canvas heights");
    assert_eq!(0.375 / 0.09375, 4.0, "four of them fill the slot exactly");
    assert_eq!(line_boxes_that_fit(0.375, 0.075, 1.25), 4);

    // And the function agrees: four lines in that slot are one slide, set at
    // the nominal size with no shrinking.
    let split = split_slides(
        "L1\nL2\nL3\nL4",
        &slot(0.375),
        &type_set(0.075, 0.05, 1.25, 4),
    );
    assert_eq!(shown(&split), ["L1\nL2\nL3\nL4"]);
    assert_eq!(split.font_size, 0.075);
    assert!(!split.overflows);
}

/// FR-310, verbatim: *"A 12-line verse against a 4-line slot produces 3 slides
/// … with identical breaks."*
#[test]
fn the_criterion() {
    let split = split_slides(TWELVE_LINES, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));

    assert_eq!(split.slides.len(), 3, "12 lines, 4 to a slot");
    assert_eq!(
        shown(&split),
        ["L1\nL2\nL3\nL4", "L5\nL6\nL7\nL8", "L9\nL10\nL11\nL12",]
    );
    assert_eq!(split.font_size, 0.075, "four lines fit; nothing to shrink");
    assert!(
        !split.overflows,
        "content that was split is the healthy outcome, not an overflow"
    );
}

/// *"…in both preview and output, with identical breaks."*
///
/// There is one function and one answer, so the thing to pin is that the
/// answer depends on nothing else. Called twice it is equal; and since the
/// canvas is not an argument at all, a slot and a type size scaled together —
/// the same design measured against a differently sized canvas — is the same
/// split, differing only in the size it reports.
#[test]
fn preview_and_output_are_the_same_answer() {
    let once = split_slides(TWELVE_LINES, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));
    let twice = split_slides(TWELVE_LINES, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));
    assert_eq!(once, twice);

    // Same design, both dimensions doubled: 0.75 / (0.15 · 1.25) is 4 as well.
    let doubled = split_slides(TWELVE_LINES, &slot(0.75), &type_set(0.15, 0.10, 1.25, 4));
    assert_eq!(doubled.slides, once.slides);
    assert_eq!(doubled.font_size, 0.15);

    // Same line box reached through a different pair of numbers:
    // 0.075 · 1.25 and 0.09375 · 1.0 are the same height.
    let restated = split_slides(
        TWELVE_LINES,
        &slot(0.375),
        &type_set(0.09375, 0.09375, 1.0, 4),
    );
    assert_eq!(restated.slides, once.slides);
}

/// The same verse splits the same whichever way it was stored. Lyrics reach
/// this function from a `.aero` file, a SQLite column and a paste, and those
/// three do not agree about line endings.
#[test]
fn crlf_and_lf_split_identically() {
    let lf = TWELVE_LINES;
    let crlf = TWELVE_LINES.replace('\n', "\r\n");

    let a = split_slides(lf, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));
    let b = split_slides(&crlf, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));

    assert_eq!(a, b);
    assert!(
        !b.slides.iter().any(|s| s.contains('\r')),
        "a carriage return must not survive into a slide"
    );
}

/// The signature itself is part of the promise. A canvas parameter would
/// invite a caller to believe the split depends on the output resolution, and
/// "recompute per resolution" is exactly the preview/output divergence risk R5
/// is about. This fails to compile if one is ever added.
#[test]
fn split_slides_takes_no_canvas_argument() {
    let f: fn(&str, &Rect, &Typography) -> SlideSplit = split_slides;
    let split = f("L1", &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));
    assert_eq!(shown(&split), ["L1"]);
}

// ---------------------------------------------------------------------------
// One size for the whole item
// ---------------------------------------------------------------------------

/// Rejects per-slide shrinking.
///
/// Ten lines in a four-line slot is 4 + 4 + 2, and the last slide is the short
/// one. A size chosen per slide would set that tail larger than the two slides
/// before it — visible from the back row in a way a slightly small font is not
/// — and here it would be the nominal 0.1, because two lines fit at nominal.
/// The reported size is the one that fits four.
#[test]
fn one_size_covers_every_slide_of_the_item() {
    let ten = numbered(10);
    let split = split_slides(&ten, &slot(0.25), &type_set(0.1, 0.01, 1.25, 4));

    assert_eq!(
        shown(&split),
        ["L1\nL2\nL3\nL4", "L5\nL6\nL7\nL8", "L9\nL10"]
    );
    assert_eq!(
        split.font_size, 0.05,
        "0.25 / (4 · 1.25): the size that fits the fullest slide, not the last one"
    );

    // The four-line prefix on its own gets the same size. The item's length
    // past `max_lines` cannot change how big the type is.
    let four = split_slides("L1\nL2\nL3\nL4", &slot(0.25), &type_set(0.1, 0.01, 1.25, 4));
    assert_eq!(four.font_size, split.font_size);
}

/// The reported size has to be small enough for the fullest slide in the list
/// — that is what "one size for the whole item" *means*, and it is the claim a
/// per-slide implementation breaks.
#[test]
fn the_reported_size_fits_the_fullest_slide() {
    for (content, h, size, min_size, line_height, max_lines) in [
        (numbered(10), 0.25, 0.1, 0.01, 1.25, 4u16),
        (numbered(12), 0.375, 0.075, 0.05, 1.25, 4),
        (numbered(7), 0.35, 0.1, 0.1, 1.0, 8),
        (numbered(5), 0.25, 0.1, 0.01, 1.25, 4),
    ] {
        let typography = type_set(size, min_size, line_height, max_lines);
        let split = split_slides(&content, &slot(h), &typography);
        let capacity = line_boxes_that_fit(h, split.font_size, line_height);
        let fullest = split
            .slides
            .iter()
            .map(|s| s.lines().count())
            .max()
            .unwrap_or(0);
        assert!(
            fullest <= capacity,
            "{fullest} lines on a slide, but only {capacity} line boxes fit at {}",
            split.font_size
        );
    }
}

/// Appendix B calls `min_size` the auto-shrink floor *before* splitting, so
/// shrinking that avoids the split entirely is the intended outcome, not a
/// missed one. Four lines in a slot that holds two at the nominal size become
/// one slide at half the size — not two slides at full size.
#[test]
fn shrinking_can_avoid_splitting_entirely() {
    let slot_holds_two_at_nominal = line_boxes_that_fit(0.25, 0.1, 1.25);
    assert_eq!(slot_holds_two_at_nominal, 2);

    let split = split_slides("L1\nL2\nL3\nL4", &slot(0.25), &type_set(0.1, 0.04, 1.25, 4));

    assert_eq!(shown(&split), ["L1\nL2\nL3\nL4"], "one slide, not two");
    assert_eq!(split.font_size, 0.05, "0.25 / (4 · 1.25)");
    assert!(!split.overflows);
}

/// Shrinking stops at the floor. With `min_size` above the size four lines
/// would need, the content splits instead — that is the trade Appendix B
/// describes, and the floor is honoured exactly, not overshot.
#[test]
fn shrinking_stops_at_min_size_and_the_content_splits_instead() {
    let split = split_slides("L1\nL2\nL3\nL4", &slot(0.25), &type_set(0.1, 0.08, 1.25, 4));

    assert_eq!(
        split.font_size, 0.08,
        "the floor, not the 0.05 four would need"
    );
    assert_eq!(
        shown(&split),
        ["L1\nL2", "L3\nL4"],
        "0.25 / (0.08 · 1.25) is 2.5, so two lines to a slide"
    );
}

/// Shrinking only ever shrinks. A slot with room to spare does not get the
/// type enlarged to fill it.
#[test]
fn a_roomy_slot_does_not_enlarge_the_type() {
    let split = split_slides("L1\nL2", &slot(0.9), &type_set(0.075, 0.05, 1.25, 4));

    assert_eq!(shown(&split), ["L1\nL2"]);
    assert_eq!(split.font_size, 0.075, "the design's size, unchanged");
}

// ---------------------------------------------------------------------------
// `max_lines` is a ceiling, before and after shrinking
// ---------------------------------------------------------------------------

/// Rejects sizing from the geometry alone. The slot holds six line boxes, but
/// the author declared four, and `max_lines` "drives slide splitting" — it is
/// the stated design, not a hint.
#[test]
fn max_lines_caps_a_slot_with_room_to_spare() {
    assert_eq!(line_boxes_that_fit(0.5625, 0.075, 1.25), 6);

    let split = split_slides(
        TWELVE_LINES,
        &slot(0.5625),
        &type_set(0.075, 0.075, 1.25, 4),
    );

    assert_eq!(
        shown(&split),
        ["L1\nL2\nL3\nL4", "L5\nL6\nL7\nL8", "L9\nL10\nL11\nL12",],
        "four to a slide, though six would fit"
    );
}

/// Rejects sizing from `max_lines` alone. `min_size` equals `size`, so there
/// is no shrinking left to do and the slot simply holds two lines. Honouring
/// the declared four here is how the congregation gets the clipped line US-16
/// exists to prevent.
#[test]
fn geometry_lowers_the_count_below_max_lines() {
    assert_eq!(line_boxes_that_fit(0.25, 0.1, 1.25), 2);

    let eight = numbered(8);
    let split = split_slides(&eight, &slot(0.25), &type_set(0.1, 0.1, 1.25, 4));

    assert_eq!(
        shown(&split),
        ["L1\nL2", "L3\nL4", "L5\nL6", "L7\nL8"],
        "two to a slide, not the declared four"
    );
    assert!(!split.overflows, "two lines fit; nothing spills");
}

/// Shrinking may avoid a split, but it may not redesign the slide. Five lines
/// against `max_lines: 4` shrink far enough for four, and no further.
#[test]
fn shrinking_never_puts_a_fifth_line_in_a_four_line_slot() {
    let split = split_slides(
        "L1\nL2\nL3\nL4\nL5",
        &slot(0.25),
        &type_set(0.1, 0.01, 1.25, 4),
    );

    assert_eq!(
        split.font_size, 0.05,
        "0.25 / (4 · 1.25) — sized for four, not for five"
    );
    assert_eq!(shown(&split), ["L1\nL2\nL3\nL4", "L5"]);
}

/// A one-line slot is a legal design and splits every line onto its own slide.
#[test]
fn max_lines_of_one_gives_a_slide_per_line() {
    let split = split_slides("L1\nL2\nL3", &slot(0.375), &type_set(0.075, 0.05, 1.25, 1));

    assert_eq!(shown(&split), ["L1", "L2", "L3"]);
    assert!(!split.overflows, "one line fits; the slot is merely small");
}

// ---------------------------------------------------------------------------
// `overflows` means zero capacity, not "was split"
// ---------------------------------------------------------------------------

/// The criterion's own case, restated as the negative: three slides and no
/// overflow. `overflows` reported for ordinary splitting would make the
/// FR-407 indicator meaningless.
#[test]
fn a_split_item_does_not_report_overflow() {
    let split = split_slides(TWELVE_LINES, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));

    assert_eq!(split.slides.len(), 3);
    assert!(!split.overflows);
}

/// A slot too short for one line at `min_size`. No division of the content can
/// fix it, so it is reported — and the content still comes out, one line to a
/// slide, because losing it silently is worse.
#[test]
fn a_slot_too_short_for_a_single_line_reports_overflow_and_still_emits_every_line() {
    assert_eq!(line_boxes_that_fit(0.02, 0.1, 1.25), 0);

    let split = split_slides("L1\nL2\nL3", &slot(0.02), &type_set(0.1, 0.1, 1.25, 4));

    assert!(split.overflows);
    assert_eq!(shown(&split), ["L1", "L2", "L3"], "one line per slide");
    assert!(!split.slides.is_empty());
}

/// A zero-height slot — an invisible layer, legal in a template — is the same
/// case and must not swallow the content.
#[test]
fn a_zero_height_slot_overflows_rather_than_returning_nothing() {
    let split = split_slides("L1\nL2\nL3", &slot(0.0), &type_set(0.075, 0.05, 1.25, 4));

    assert!(split.overflows);
    assert_eq!(shown(&split), ["L1", "L2", "L3"]);
}

/// Nothing to show cannot spill out of anything, however short the slot is.
#[test]
fn empty_content_does_not_overflow_even_in_an_impossible_slot() {
    let split = split_slides("", &slot(0.02), &type_set(0.1, 0.1, 1.25, 4));

    assert!(!split.overflows);
    assert_eq!(shown(&split), [""]);
}

/// Content that is nothing but blank lines does not spill either, and this is
/// the case a `!lines.is_empty()` test gets wrong.
///
/// `split_lines("\n")` returns one *empty* line, not zero lines, so a check
/// spelled against the emptiness of that vector reports an overflow here — on a
/// `box.h: 0.0` slot, which is a legal invisible layer, for a render that is one
/// empty slide with no ink anywhere to spill out of anything. FR-407's indicator
/// is the operator's only warning for the two hazards this module cannot detect
/// at all, and an alarm lit with nothing behind it is an alarm nobody reads —
/// the same reasoning `max_lines: 0` gets above, at the other end.
///
/// The whitespace spellings are the second half of the claim: `"   "` is not an
/// empty line, but it paints nothing either, so the predicate has to be the
/// module's `is_blank` and not `is_empty`.
///
/// Every case asserts the slot really holds no line box before asserting what
/// follows from it. In a slot that turned out to fit one, `overflows: false`
/// would hold for the wrong reason and the case would be decoration.
#[test]
fn blank_only_content_does_not_overflow_in_a_slot_that_holds_no_line() {
    // Two slots with no capacity, reached by different routes: the zero-height
    // invisible layer, and a slot merely far too short with no shrinking
    // headroom left (`min_size == size`).
    for (h, size, min_size) in [(0.0, 0.075, 0.05), (0.02, 0.1, 0.1)] {
        for content in [
            "\n",
            "\n\n",
            "\r\n",
            " ",
            "   \n\t",
            "\u{2028}\u{2029}",
            "\t \n \n  ",
        ] {
            let split = split_slides(content, &slot(h), &type_set(size, min_size, 1.25, 4));

            assert_eq!(
                line_boxes_that_fit(h, split.font_size, 1.25),
                0,
                "the slot holds a line after all at h={h}, so {content:?} proves nothing"
            );
            assert!(
                !split.overflows,
                "overflow reported for {content:?} at h={h}, whose render is one empty slide"
            );
            assert_eq!(shown(&split), [""], "for {content:?} at h={h}");
        }
    }
}

/// The other half of the same claim: quieting the false alarm may not quiet a
/// true one. A slide with a blank line *in* it still has ink to spill, so a slot
/// that holds no line box at all is still an overflow. Asking `all` instead of
/// `any` would let one stanza gap turn the indicator off for a whole song.
#[test]
fn a_blank_line_among_real_ones_does_not_silence_the_overflow_indicator() {
    for content in [
        "\nL1",
        "L1\n\nL2",
        "L1\n   \nL2",
        "L1\n\n",
        "\n\nL1\n\nL2\n\n",
    ] {
        let split = split_slides(content, &slot(0.0), &type_set(0.075, 0.05, 1.25, 4));

        assert_eq!(
            line_boxes_that_fit(0.0, split.font_size, 1.25),
            0,
            "the slot holds a line after all, so {content:?} proves nothing"
        );
        assert!(
            split.overflows,
            "no overflow reported for {content:?}, which has a line with something in it"
        );

        // Reported, never acted on: the content still comes out.
        let recovered: Vec<&str> = split
            .slides
            .iter()
            .flat_map(|s| s.lines())
            .filter(|l| !l.trim().is_empty())
            .collect();
        let expected: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(recovered, expected, "content changed for {content:?}");
    }
}

/// `""` and `"\n"` are one empty slide either way, so they are the same answer
/// wherever that answer can be seen — including in a slot with no capacity,
/// where the two differ under any emptiness-of-the-vector test.
///
/// `font_size` is deliberately not part of this: `shrink_to_fit` counts the
/// blank line as a line to make room for, which is what makes a stanza gap hold
/// its space everywhere else, and neither content paints a glyph for the size to
/// apply to. The asymmetry is real and left alone.
#[test]
fn empty_content_and_a_lone_separator_render_the_same() {
    for h in [0.0, 0.02, 0.375, 0.9] {
        let typography = type_set(0.075, 0.05, 1.25, 4);
        let empty = split_slides("", &slot(h), &typography);
        let separator = split_slides("\n", &slot(h), &typography);

        assert_eq!(shown(&empty), [""], "at h={h}");
        assert_eq!(empty.slides, separator.slides, "at h={h}");
        assert_eq!(
            empty.overflows, separator.overflows,
            "one empty slide either way, but only one of them warns, at h={h}"
        );
    }
}

// ---------------------------------------------------------------------------
// Never zero slides
// ---------------------------------------------------------------------------

/// FR-312's explicit blank item is exactly this, and FR-306/FR-313 navigate by
/// advancing through an item's slides — an item with none is a hole in that
/// sequence.
#[test]
fn empty_content_becomes_one_empty_slide() {
    let split = split_slides("", &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));

    assert_eq!(shown(&split), [""]);
    assert_eq!(split.slides.len(), 1, "one, not zero");
}

/// Content that is nothing but blank lines is the same case: every line is
/// skipped, and the fallback still has to produce something to navigate to.
#[test]
fn content_of_nothing_but_blank_lines_becomes_one_empty_slide() {
    for content in ["\n", "\n\n\n", "   \n\t\n", "\r\n\r\n"] {
        let split = split_slides(content, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));
        assert_eq!(shown(&split), [""], "for {content:?}");
    }
}

// ---------------------------------------------------------------------------
// Blank lines
// ---------------------------------------------------------------------------

/// A stanza gap occupies a line box, so it moves the break — and it is trimmed
/// off the end of the slide it fell on, which changes what is shown but not
/// where the break fell.
///
/// Seven lines, two of them blank, in a four-line slot: the first slide's
/// window is `L1, L2, blank, blank`, so `L3` starts the second slide even
/// though only two visible lines precede it. Discard the blanks first and the
/// first slide would read `L1 L2 L3 L4` instead.
#[test]
fn blank_lines_take_up_height_and_move_the_break() {
    let split = split_slides(
        "L1\nL2\n\n\nL3\nL4\nL5",
        &slot(0.375),
        &type_set(0.075, 0.075, 1.25, 4),
    );

    assert_eq!(
        shown(&split),
        ["L1\nL2", "L3\nL4\nL5"],
        "the gap took two line boxes, then was not rendered"
    );
}

/// A blank inside a slide is part of the slide: it is the stanza's spacing,
/// and dropping it would re-space the stanza.
#[test]
fn a_blank_line_inside_a_slide_is_kept() {
    let split = split_slides(
        "L1\nL2\n\nL3\nL4\nL5\nL6",
        &slot(0.375),
        &type_set(0.075, 0.075, 1.25, 4),
    );

    assert_eq!(shown(&split), ["L1\nL2\n\nL3", "L4\nL5\nL6"]);
}

/// A slide never opens with a gap. An empty first line pushes everything below
/// it off the vertical centre for nothing.
#[test]
fn no_slide_opens_with_a_blank_line() {
    let split = split_slides(
        "L1\nL2\nL3\nL4\n\nL5\nL6",
        &slot(0.375),
        &type_set(0.075, 0.075, 1.25, 4),
    );

    assert_eq!(shown(&split), ["L1\nL2\nL3\nL4", "L5\nL6"]);
    for slide in &split.slides {
        assert!(
            !slide.starts_with('\n') && !slide.starts_with(' '),
            "slide opened with a gap: {slide:?}"
        );
    }
}

/// Leading blanks are skipped without costing the first slide any capacity.
#[test]
fn leading_blank_lines_do_not_eat_the_first_slides_capacity() {
    let split = split_slides(
        "\n\nL1\nL2\nL3\nL4\nL5",
        &slot(0.375),
        &type_set(0.075, 0.075, 1.25, 4),
    );

    assert_eq!(shown(&split), ["L1\nL2\nL3\nL4", "L5"]);
}

// ---------------------------------------------------------------------------
// The rounding direction
// ---------------------------------------------------------------------------

/// A line box that only partly fits does not count. `0.35 / 0.1` is three and
/// a half boxes, so three lines go on the slide; `ceil` would put four there
/// and `round` would too, and the fourth would be half outside the slot.
#[test]
fn a_partly_filled_line_box_does_not_count() {
    assert_eq!(line_boxes_that_fit(0.35, 0.1, 1.0), 3);

    let seven = numbered(7);
    let split = split_slides(&seven, &slot(0.35), &type_set(0.1, 0.1, 1.0, 8));

    assert_eq!(shown(&split), ["L1\nL2\nL3", "L4\nL5\nL6", "L7"]);
    assert_eq!(split.font_size, 0.1);
}

/// The same, at both ends of a whole number, so that neither `ceil` nor
/// rounding-to-nearest can pass: 3.99 boxes hold three, 4.9 hold four.
#[test]
fn the_count_is_rounded_down_at_both_ends_of_a_whole_number() {
    let eight = numbered(8);
    let just_under = split_slides(&eight, &slot(0.399), &type_set(0.1, 0.1, 1.0, 8));
    assert_eq!(
        shown(&just_under),
        ["L1\nL2\nL3", "L4\nL5\nL6", "L7\nL8"],
        "3.99 line boxes hold three lines"
    );

    let ten = numbered(10);
    let well_over = split_slides(&ten, &slot(0.49), &type_set(0.1, 0.1, 1.0, 8));
    assert_eq!(
        shown(&well_over),
        ["L1\nL2\nL3\nL4", "L5\nL6\nL7\nL8", "L9\nL10"],
        "4.9 line boxes hold four lines"
    );
}

/// When the two roundings on the shrink path disagree, the answer is one line
/// fewer — never one more.
///
/// Shrinking picks `h / (n · line_height)` and the capacity check then divides
/// `h` by `size · line_height`; algebraically that returns `n`, but the two
/// products are rounded differently and the quotient can land a hair below the
/// whole number. Here `h = 0.108`, `line_height = 1.2`, `n = 3`: the size comes
/// out at 0.03 and the capacity check reads 2.999…, so the content is split
/// rather than fitted. That is the harmless direction — an extra break the
/// operator can see, instead of a line the congregation cannot read. `ceil`
/// would fit all three and let the last one hang out of the box.
#[test]
fn when_the_two_roundings_disagree_the_answer_is_one_line_fewer_never_more() {
    let split = split_slides("L1\nL2\nL3", &slot(0.108), &type_set(0.1, 0.02, 1.2, 3));

    assert_eq!(
        shown(&split),
        ["L1\nL2", "L3"],
        "one line fewer on the slide, not one more"
    );

    let capacity = line_boxes_that_fit(0.108, split.font_size, 1.2);
    assert_eq!(capacity, 2);
    for slide in &split.slides {
        assert!(
            slide.lines().count() <= capacity,
            "a slide holds more lines than its own reported size allows: {slide:?}"
        );
    }
}

/// The same claim, swept rather than sampled: across a range of slot heights,
/// type sizes, line heights and content lengths, no slide ever carries more
/// lines than the size it is reported at can hold, nor more than `max_lines`,
/// and no line of content is ever dropped. A `ceil` fails the first of these;
/// an off-by-one anywhere in the windowing fails the last.
#[test]
fn no_slide_ever_carries_more_lines_than_it_can_hold() {
    let heights = [0.05, 0.108, 0.2, 0.25, 0.35, 0.399, 0.49, 0.5625, 0.75, 0.9];
    let sizes = [0.03, 0.05, 0.075, 0.1];
    let line_heights = [1.0, 1.2, 1.24, 1.25, 1.5];
    let maxima = [1u16, 2, 4, 6, 12];
    let lengths = [1usize, 3, 5, 7, 12, 20];

    for &h in &heights {
        for &size in &sizes {
            for &line_height in &line_heights {
                for &max_lines in &maxima {
                    for &n in &lengths {
                        let content = numbered(n);
                        // `min_size == size` on one pass and a real floor on
                        // the other, so both the shrinking and the
                        // non-shrinking path are swept.
                        for &min_size in &[size, size / 4.0] {
                            let typography = type_set(size, min_size, line_height, max_lines);
                            let split = split_slides(&content, &slot(h), &typography);
                            let label = format!(
                                "h={h} size={size} lh={line_height} max_lines={max_lines} n={n} \
                                 min_size={min_size}"
                            );

                            assert!(!split.slides.is_empty(), "no slides for {label}");
                            assert!(
                                split.font_size <= size,
                                "type enlarged past the design for {label}"
                            );

                            let capacity =
                                line_boxes_that_fit(h, split.font_size, line_height).max(1);
                            for slide in &split.slides {
                                let used = slide.lines().count();
                                assert!(
                                    used <= capacity,
                                    "{used} lines but {capacity} fit for {label}"
                                );
                                assert!(
                                    used <= usize::from(max_lines),
                                    "{used} lines past max_lines for {label}"
                                );
                                assert!(
                                    !slide.starts_with('\n'),
                                    "slide opens with a gap for {label}"
                                );
                            }

                            // Nothing invented, nothing lost: the visible lines
                            // come back in order.
                            let out: Vec<&str> = split
                                .slides
                                .iter()
                                .flat_map(|s| s.lines())
                                .filter(|l| !l.trim().is_empty())
                                .collect();
                            let expected: Vec<&str> =
                                content.lines().filter(|l| !l.trim().is_empty()).collect();
                            assert_eq!(out, expected, "content changed for {label}");

                            // `overflows` is zero capacity, never "was split".
                            assert_eq!(
                                split.overflows,
                                line_boxes_that_fit(h, split.font_size, line_height)
                                    .min(usize::from(max_lines))
                                    == 0,
                                "overflows misreported for {label}"
                            );
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Geometry a validated template cannot contain
// ---------------------------------------------------------------------------

/// `validate_template` refuses all of these, so they arrive only from a
/// document that skipped validation (NFR-28). The requirement on this function
/// is that it answers deterministically instead of panicking or dividing its
/// way to an infinity, and that it still hands back every line.
#[test]
fn degenerate_geometry_answers_instead_of_panicking() {
    let degenerate = [
        (f64::NAN, 0.075, 1.25),
        (0.375, f64::NAN, 1.25),
        (0.375, 0.075, f64::NAN),
        (0.375, 0.0, 1.25),
        (0.375, 0.075, 0.0),
        (-0.5, 0.075, 1.25),
        (f64::INFINITY, 0.075, 1.25),
        (0.375, 0.075, -1.25),
    ];

    for (h, size, line_height) in degenerate {
        let typography = type_set(size, size, line_height, 4);
        let split = split_slides("L1\nL2\nL3", &slot(h), &typography);
        assert!(
            !split.slides.is_empty(),
            "no slides for h={h} size={size} lh={line_height}"
        );
        let recovered: Vec<&str> = split.slides.iter().flat_map(|s| s.lines()).collect();
        assert_eq!(
            recovered,
            ["L1", "L2", "L3"],
            "content lost for h={h} size={size} lh={line_height}"
        );
        // Deterministic, not merely non-panicking.
        assert_eq!(split, split_slides("L1\nL2\nL3", &slot(h), &typography));
    }
}

// ---------------------------------------------------------------------------
// The line model: which characters end a line
// ---------------------------------------------------------------------------
//
// `split_slides` counts lines for a renderer laying the text out with
// `white-space: pre`, and that renderer breaks on seven characters, not on the
// two `str::lines` knows. Every one it missed would be a line box on the slide
// that was never counted, on a slide already full, with `overflows` dark — so
// the set is pinned here character by character rather than left to the two
// spellings a `\n`-only test would exercise.
//
// Each of the seven reaches the function by a route the application already
// has: PowerPoint exports a soft break as U+000B (FR-501), a paste from a
// browser or a PDF carries U+2028, a `.aero` written by another program
// carries whatever it liked (FR-703), and a `.aero`, a SQLite column and a
// paste do not agree about `\r`.

/// Unicode's mandatory-break characters — UAX #14 classes BK, CR and LF. Named
/// here independently of the module under test: a list borrowed from the
/// implementation would agree with any set it happened to contain.
const SEPARATORS: [char; 7] = [
    '\u{000a}', // LINE FEED
    '\u{000b}', // LINE TABULATION
    '\u{000c}', // FORM FEED
    '\u{000d}', // CARRIAGE RETURN
    '\u{0085}', // NEXT LINE
    '\u{2028}', // LINE SEPARATOR
    '\u{2029}', // PARAGRAPH SEPARATOR
];

/// `L1<sep>L2<sep>…<sep>Ln`: the content [`numbered`] builds, spelled with a
/// different separator.
fn numbered_with(n: usize, separator: &str) -> String {
    (1..=n)
        .map(|i| format!("L{i}"))
        .collect::<Vec<_>>()
        .join(separator)
}

/// Every spelling of the twelve-line verse, as `(label, content)`: the seven
/// single characters, plus the CRLF pair, which is one break and not two.
fn twelve_lines_every_spelling() -> Vec<(String, String)> {
    let mut spellings: Vec<(String, String)> = SEPARATORS
        .iter()
        .map(|c| {
            (
                format!("U+{:04X}", *c as u32),
                numbered_with(12, &c.to_string()),
            )
        })
        .collect();
    spellings.push(("CRLF".to_string(), numbered_with(12, "\r\n")));
    spellings
}

/// The criterion's own case, spelled eight ways. Each of the seven mandatory
/// break characters ends a line, and so does the CRLF pair, so the same verse
/// gives the same three slides however it was stored — *"identical breaks"*
/// is a claim about the content, not about which program wrote the file.
///
/// The expectation is written out as literals rather than only compared
/// against the `\n` spelling, because a function that recognised *none* of the
/// separators would make both sides of such a comparison equally wrong. The
/// equality against the `\n` split is asserted as well, since it is the
/// stronger statement once the literals hold: every separator comes back as a
/// plain `\n`, and neither the reported size nor `overflows` depends on the
/// spelling either.
#[test]
fn every_mandatory_break_character_ends_a_line() {
    let expected = ["L1\nL2\nL3\nL4", "L5\nL6\nL7\nL8", "L9\nL10\nL11\nL12"];
    let baseline = split_slides(TWELVE_LINES, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));

    for (label, content) in twelve_lines_every_spelling() {
        let split = split_slides(&content, &slot(0.375), &type_set(0.075, 0.05, 1.25, 4));

        assert_eq!(
            shown(&split),
            expected,
            "twelve lines separated by {label} did not break where twelve lines break"
        );
        assert_eq!(
            split.font_size, 0.075,
            "the size changed with the spelling ({label})"
        );
        assert!(!split.overflows, "overflow reported for {label}");
        assert_eq!(
            split, baseline,
            "the {label} spelling is not the same split as the \\n spelling"
        );
    }
}

/// A separator is a boundary the function *counted*, not a character in the
/// text. Reading one as ordinary text puts a line box on the slide that was
/// never measured, which is the clipped line US-16 exists to prevent.
///
/// The slot holds exactly two line boxes and there is no shrinking headroom
/// (`min_size == size`), so three lines have to become two slides. A separator
/// mistaken for text leaves one line, one slide, and three lines of ink in a
/// two-line box.
#[test]
fn a_separator_is_counted_as_a_break_and_not_as_text() {
    assert_eq!(line_boxes_that_fit(0.25, 0.125, 1.0), 2);

    for separator in SEPARATORS {
        let content = format!("L1{separator}L2{separator}L3");
        let split = split_slides(&content, &slot(0.25), &type_set(0.125, 0.125, 1.0, 4));

        assert_eq!(
            shown(&split),
            ["L1\nL2", "L3"],
            "U+{:04X} was not counted against the slot's two line boxes",
            separator as u32
        );
    }
}

/// No slide may *contain* a separator. What was counted here and what the
/// renderer lays out have to be the same lines: a separator carried into a
/// slide is a line box nobody counted, and `slides[i]` enters the DOM as a
/// text node under `white-space: pre`, where it would break the line anyway.
///
/// Asserted directly over the result — for every spelling, for one piece of
/// content carrying all seven at once, and at four slot heights — rather than
/// inferred from an expected string. This is the property, and it holds for
/// content this file never thought to write down.
#[test]
fn no_slide_contains_a_separator_this_function_counted() {
    // Every separator at once, with text on both sides of each: the shape a
    // `.aero` written by another program can have (FR-703).
    let all_seven: String = SEPARATORS
        .iter()
        .enumerate()
        .map(|(i, c)| format!("L{}{c}", i + 1))
        .collect::<String>()
        + "L8";

    let mut spellings: Vec<(String, String, usize)> = twelve_lines_every_spelling()
        .into_iter()
        .map(|(label, content)| (label, content, 12))
        .collect();
    spellings.push(("all seven at once".to_string(), all_seven, 8));

    for (label, content, expected_lines) in spellings {
        for h in [0.375, 0.25, 0.02, 0.9] {
            let split = split_slides(&content, &slot(h), &type_set(0.075, 0.05, 1.25, 4));

            for slide in &split.slides {
                for separator in SEPARATORS {
                    if separator == '\n' {
                        continue; // the one separator a slide is *made* of.
                    }
                    assert!(
                        !slide.contains(separator),
                        "a slide carries U+{:04X} for {label} at h={h}: {slide:?}",
                        separator as u32
                    );
                }
            }

            // And no text was lost on either side of any of them: every line
            // of the content comes back, in order, with nothing invented.
            let recovered: Vec<&str> = split
                .slides
                .iter()
                .flat_map(|s| s.lines())
                .filter(|l| !l.trim().is_empty())
                .collect();
            let intended: Vec<String> = (1..=expected_lines).map(|i| format!("L{i}")).collect();
            let intended: Vec<&str> = intended.iter().map(String::as_str).collect();
            assert_eq!(recovered, intended, "content changed for {label} at h={h}");
        }
    }
}

/// CRLF is one break. Every other pair of separators is two, with an empty
/// line between them — including `\n\r`, the pair a "swallow any two in a row"
/// rule would collapse.
///
/// The two failures are opposites and both are visible from the congregation's
/// side: reading CRLF as two breaks puts an empty line between every two lines
/// of a Windows-authored file, and collapsing any pair deletes a stanza gap
/// the author typed.
#[test]
fn only_crlf_is_one_break_and_every_other_pair_is_two() {
    for first in SEPARATORS {
        for second in SEPARATORS {
            let content = format!("L1{first}{second}L2");
            let split = split_slides(&content, &slot(0.5), &type_set(0.125, 0.125, 1.0, 4));

            let expected = if (first, second) == ('\u{000d}', '\u{000a}') {
                "L1\nL2"
            } else {
                "L1\n\nL2"
            };
            assert_eq!(
                shown(&split),
                [expected],
                "U+{:04X} followed by U+{:04X}",
                first as u32,
                second as u32
            );
        }
    }
}

/// A separator at the end of the content ends the last line; it does not open
/// an empty one. Matching `str::lines`: `"a\n"` is one line, `"a\n\n"` is two.
///
/// The consequence is visible in the reported size, which is why it is
/// asserted here rather than left to the slides: auto-shrink aims at the
/// content's own length, so a phantom trailing line makes it aim at one line
/// more than the item has and sets the whole song smaller than it needed to
/// be — while the slides look the same either way, because a blank at the end
/// of a slide is trimmed off what gets rendered. Every number below is exact
/// in binary (0.375 / 2 = 0.1875, 0.375 / 3 = 0.125), so these equalities are
/// the arithmetic itself and not a tolerance.
#[test]
fn a_trailing_separator_ends_the_last_line_rather_than_opening_an_empty_one() {
    let typography = type_set(0.25, 0.03125, 1.0, 4);

    let bare = split_slides("L1\nL2", &slot(0.375), &typography);
    assert_eq!(shown(&bare), ["L1\nL2"]);
    assert_eq!(bare.font_size, 0.1875, "0.375 / 2: sized for the two lines");

    for separator in SEPARATORS {
        let trailing = format!("L1\nL2{separator}");
        let split = split_slides(&trailing, &slot(0.375), &typography);
        assert_eq!(
            split, bare,
            "a trailing U+{:04X} added a line the content does not have",
            separator as u32
        );
    }

    // Two separators do open one, and it costs the item a size step.
    let doubled = split_slides("L1\nL2\n\n", &slot(0.375), &typography);
    assert_eq!(shown(&doubled), ["L1\nL2"], "the blank is not rendered");
    assert_eq!(
        doubled.font_size, 0.125,
        "0.375 / 3: the empty third line is real, and auto-shrink aimed at it"
    );
}

// ---------------------------------------------------------------------------
// `max_lines: 0` is undeclared, not a ceiling of zero
// ---------------------------------------------------------------------------

/// `validate_template` refuses `max_lines: 0` — its minimum is 1 — so it
/// arrives only from a document that skipped validation (NFR-28). Read
/// literally it would cap every slide at zero lines: one line per slide, and
/// `overflows` lit on a slot with room to spare. An alarm that is always on is
/// an alarm nobody reads, and FR-407's indicator is the only warning the
/// operator gets for the two hazards this module cannot detect at all.
///
/// So zero means *undeclared*, and the geometry alone caps the count. The slot
/// here holds exactly four line boxes (0.5 / 0.125) and the content is four
/// lines: one slide, no overflow, and nothing to shrink.
#[test]
fn max_lines_zero_means_undeclared_and_lets_the_geometry_decide() {
    assert_eq!(line_boxes_that_fit(0.5, 0.125, 1.0), 4);

    let split = split_slides(
        "L1\nL2\nL3\nL4",
        &slot(0.5),
        &type_set(0.125, 0.125, 1.0, 0),
    );

    assert_eq!(
        shown(&split),
        ["L1\nL2\nL3\nL4"],
        "four lines in a slot that holds four is one slide"
    );
    assert!(
        !split.overflows,
        "a slot with room to spare does not overflow because a field said 0"
    );
    assert_eq!(split.font_size, 0.125, "nothing to shrink");

    // Undeclared behaves as a ceiling above the content's length, not as one
    // below it: the same answer as the largest ceiling that can be spelled.
    let ceiling = split_slides(
        "L1\nL2\nL3\nL4",
        &slot(0.5),
        &type_set(0.125, 0.125, 1.0, u16::MAX),
    );
    assert_eq!(split, ceiling);
}

/// The same reading, where the geometry does the capping: six lines in a slot
/// that holds four become two slides — the geometry's four to a slide, not
/// zero — and still no overflow.
#[test]
fn max_lines_zero_still_splits_by_the_geometry() {
    let six = numbered(6);
    let split = split_slides(&six, &slot(0.5), &type_set(0.125, 0.125, 1.0, 0));

    assert_eq!(shown(&split), ["L1\nL2\nL3\nL4", "L5\nL6"]);
    assert!(!split.overflows);
}

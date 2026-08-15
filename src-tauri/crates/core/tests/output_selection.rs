//! Which display the Projector Output is opened on (FR-102, FR-106,
//! ADR-0040).
//!
//! `select_output_monitor` is the whole of the answer to "the first non-primary
//! display". Everything around it needs an `AppHandle` and is therefore beyond
//! the reach of any test: Tauri 2.11.5's mock runtime hardcodes
//! `available_monitors()` to an empty `Vec` and `tauri::Monitor`'s fields are
//! `pub(crate)`, so nothing can put two displays in front of the setup hook.
//! The choice being a pure function on this crate's public surface is what
//! makes the wrong answer observable at all — the same reason `flag_primary`
//! lives here, and the same reason the tests below are written to *reject*
//! specific wrong implementations rather than merely to agree with the current
//! one.
//!
//! The wrong implementations that have to fail here, named so a later reader
//! can check they still do:
//!
//! - `monitors[1]`, or "the first non-primary entry encountered" — list order
//!   is `EnumDisplayMonitors` order, which is promised nothing;
//! - "the leftmost display", forgetting to exclude the primary;
//! - anything that takes `abs()` of `x`, or reads it as unsigned — a projector
//!   arranged to the left of the control display has a negative `x`;
//! - anything that answers differently for two orderings of the same displays.
//!
//! ADR-0040's central claim is that the result is a function of the **set** of
//! displays and not of the list, which is exactly why `id` is in the sort key.
//! That claim is asserted by enumerating every permutation of an arrangement
//! and demanding one answer, not by spot-checking two orderings.
//!
//! Ids are spelled as string literals for the reason ADR-0037 gives and the
//! other two monitor test files repeat: they are strings FR-103 persists, so a
//! test that followed the scheme tag wherever it moved would prove nothing.

use aeroworship_core::models::{select_output_monitor, Monitor};

const DISPLAY_1: &str = r"\\.\DISPLAY1";
const DISPLAY_2: &str = r"\\.\DISPLAY2";
const DISPLAY_3: &str = r"\\.\DISPLAY3";
const DISPLAY_4: &str = r"\\.\DISPLAY4";

/// One display as the OS would report it, built through the same constructor
/// the service uses so the ids under test are the ids that ship.
fn display(name: &str, position: (i32, i32), is_primary: bool) -> Monitor {
    Monitor::from_os_report(Some(name), (1920, 1080), position, 1.0, is_primary)
}

/// The id of the display that would be projected onto, or `None` for "open no
/// output window at all".
fn chosen(monitors: &[Monitor]) -> Option<&str> {
    select_output_monitor(monitors).map(|monitor| monitor.id.as_str())
}

/// Every ordering of `monitors`.
//
// Enumerated in full rather than sampled. The property under test is that no
// ordering changes the answer, and a sample cannot support that; with four
// displays there are 24 orderings, which costs nothing.
fn permutations(monitors: &[Monitor]) -> Vec<Vec<Monitor>> {
    if monitors.is_empty() {
        return vec![Vec::new()];
    }
    let mut orderings = Vec::new();
    for (index, head) in monitors.iter().enumerate() {
        let mut remaining = monitors.to_vec();
        remaining.remove(index);
        for mut ordering in permutations(&remaining) {
            ordering.insert(0, head.clone());
            orderings.push(ordering);
        }
    }
    orderings
}

/// The list order of an arrangement, for a failure message that says which
/// ordering broke the invariant.
fn ids(monitors: &[Monitor]) -> Vec<&str> {
    monitors.iter().map(|monitor| monitor.id.as_str()).collect()
}

// ---------------------------------------------------------------------------
// The wrong answer that used to be untestable: list order
// ---------------------------------------------------------------------------

/// FR-102 / ADR-0040 — "first non-primary display" is read from position, not
/// from the enumeration. The leftmost non-primary display is the *last* entry
/// of the list here, so `monitors[1]` and "the first non-primary entry
/// encountered" both answer `DISPLAY3` and both must fail.
#[test]
fn the_leftmost_non_primary_wins_when_it_is_the_last_entry_of_the_list() {
    let monitors = [
        display(DISPLAY_2, (1920, 0), true),
        display(DISPLAY_3, (3840, 0), false),
        display(DISPLAY_1, (0, 0), false),
    ];

    assert_eq!(
        chosen(&monitors),
        Some(r"gdi:\\.\DISPLAY1"),
        "the display at x=0 is leftmost; list position must not decide"
    );
}

/// FR-102 / ADR-0040 — the same rule with the leftmost display first in the
/// list, so the suite cannot be satisfied by an implementation that simply
/// takes the last entry. Together with the test above, no fixed index passes
/// both.
#[test]
fn the_leftmost_non_primary_wins_when_it_is_the_first_entry_of_the_list() {
    let monitors = [
        display(DISPLAY_1, (0, 0), false),
        display(DISPLAY_3, (3840, 0), false),
        display(DISPLAY_2, (1920, 0), true),
    ];

    assert_eq!(chosen(&monitors), Some(r"gdi:\\.\DISPLAY1"));
}

/// FR-102 / ADR-0040 — the leftmost display overall is the *primary*, and it
/// must not be chosen: a fullscreen black window on the operator's own screen
/// is the failure this whole function exists to prevent. The answer is the
/// leftmost of what remains.
#[test]
fn the_primary_is_excluded_even_when_it_is_the_leftmost_display_of_all() {
    let monitors = [
        display(DISPLAY_1, (-1920, 0), true),
        display(DISPLAY_2, (0, 0), false),
        display(DISPLAY_3, (1920, 0), false),
    ];

    assert_eq!(
        chosen(&monitors),
        Some(r"gdi:\\.\DISPLAY2"),
        "the primary is leftmost here and is still not an output display"
    );
}

// ---------------------------------------------------------------------------
// The property ADR-0040 is built around: an answer about the set, not the list
// ---------------------------------------------------------------------------

/// ADR-0040 — "hasilnya fungsi dari **himpunan** monitor, bukan dari
/// daftarnya". Every one of the 24 orderings of a four-display arrangement must
/// produce the same display. This is the test `id` is in the sort key for; drop
/// it and two displays sharing an `(x, y)` would be decided by whichever
/// arrived first.
#[test]
fn every_ordering_of_the_same_displays_selects_the_same_display() {
    let arrangement = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (-1920, 0), false),
        display(DISPLAY_3, (-1920, -1080), false),
        display(DISPLAY_4, (1920, 0), false),
    ];

    let orderings = permutations(&arrangement);
    assert_eq!(
        orderings.len(),
        24,
        "precondition: all orderings enumerated"
    );

    for ordering in &orderings {
        assert_eq!(
            chosen(ordering),
            // x=-1920 twice, and the upper of the two wins on y.
            Some(r"gdi:\\.\DISPLAY3"),
            "ordering {:?} gave a different answer",
            ids(ordering)
        );
    }
}

/// ADR-0040 — the same property on the arrangement where it is hardest: two
/// non-primary displays at the *same* corner, so only `id` separates them. If
/// the tiebreak were "whichever came first", these six orderings would not
/// agree.
#[test]
fn every_ordering_agrees_even_when_two_displays_share_a_corner() {
    let arrangement = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_3, (1920, 0), false),
        display(DISPLAY_2, (1920, 0), false),
    ];

    for ordering in permutations(&arrangement) {
        assert_eq!(
            chosen(&ordering),
            Some(r"gdi:\\.\DISPLAY2"),
            "ordering {:?} gave a different answer",
            ids(&ordering)
        );
    }
}

// ---------------------------------------------------------------------------
// The sort key, one level at a time
// ---------------------------------------------------------------------------

/// ADR-0040 — smallest `x` first. Asserted with the candidates apart on `x`
/// alone so nothing else can be doing the work.
#[test]
fn the_smallest_x_wins() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_3, (3840, 0), false),
        display(DISPLAY_2, (1920, 0), false),
    ];

    assert_eq!(chosen(&monitors), Some(r"gdi:\\.\DISPLAY2"));
}

/// ADR-0040 — `x` outranks `y`, so the key is `(x, y)` and not `(y, x)`.
/// Every other assertion in this file happens to sit on an arrangement where
/// the two orders agree — either the candidates share an `x` or they share a
/// `y` — so without this case a transposed key passes the whole suite. Here the
/// candidates disagree on both axes: `DISPLAY2` is further left, `DISPLAY3` is
/// higher, and reading order picks the left one.
#[test]
fn a_smaller_x_beats_a_smaller_y_when_the_two_axes_disagree() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (-1920, 1080), false),
        display(DISPLAY_3, (1920, 0), false),
    ];

    assert_eq!(
        chosen(&monitors),
        Some(r"gdi:\\.\DISPLAY2"),
        "x=-1920 comes first even though its y is the larger of the two"
    );

    // The same arrangement with the axes' roles swapped between the two
    // candidates, so the answer cannot be "whichever display is lower down"
    // either.
    let mirrored = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (-1920, -1080), false),
        display(DISPLAY_3, (1920, -2160), false),
    ];

    assert_eq!(
        chosen(&mirrored),
        Some(r"gdi:\\.\DISPLAY2"),
        "x still decides when the other candidate has the smaller y"
    );
}

/// ADR-0040 — a tie on `x` is broken by the smaller `y`: two displays stacked
/// vertically at the same left edge, and the upper one is "first" in reading
/// order. The lower display is first in the list, so list order would answer
/// the other way.
#[test]
fn a_tie_on_x_is_broken_by_the_smaller_y() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 1080), false),
        display(DISPLAY_3, (1920, 0), false),
    ];

    assert_eq!(
        chosen(&monitors),
        Some(r"gdi:\\.\DISPLAY3"),
        "same x, so the upper display comes first in reading order"
    );
}

/// ADR-0040 — `y` is signed too. A display stacked *above* the primary at the
/// same left edge has a negative `y`, and it is the one that comes first.
#[test]
fn a_tie_on_x_is_broken_by_a_negative_y_as_well() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), false),
        display(DISPLAY_3, (1920, -1080), false),
    ];

    assert_eq!(chosen(&monitors), Some(r"gdi:\\.\DISPLAY3"));
}

/// ADR-0040 — a tie on `x` *and* `y` is broken by the smaller `id`, which is
/// what makes the function total and order-free. Not a reachable arrangement
/// today (displays do not overlap), which is precisely why nothing else
/// exercises it. Asserted in both list orders: the winner is once the second
/// entry and once the first, so "keep whichever arrived first" fails one of
/// them.
#[test]
fn a_tie_on_x_and_y_is_broken_by_the_smaller_id() {
    let primary = display(DISPLAY_1, (0, 0), true);
    let earlier_id = display(DISPLAY_2, (1920, 0), false);
    let later_id = display(DISPLAY_3, (1920, 0), false);
    assert!(
        earlier_id.id < later_id.id,
        "precondition: id order is known"
    );

    assert_eq!(
        chosen(&[primary.clone(), later_id.clone(), earlier_id.clone()]),
        Some(r"gdi:\\.\DISPLAY2")
    );
    assert_eq!(
        chosen(&[primary, earlier_id, later_id]),
        Some(r"gdi:\\.\DISPLAY2")
    );
}

// ---------------------------------------------------------------------------
// Negative coordinates: the display arranged left of the primary
// ---------------------------------------------------------------------------

/// FR-102 / Appendix D — a projector arranged to the left of the control
/// display reports a negative `x`, and it is further left than anything at a
/// positive one. An implementation comparing `abs(x)`, or reading `x` as
/// unsigned, answers `DISPLAY3` here: `|-3840|` is larger than `1920`, and
/// `-3840 as u32` is larger still.
#[test]
fn a_display_left_of_the_primary_wins_despite_its_negative_x() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_3, (1920, 0), false),
        display(DISPLAY_2, (-3840, 0), false),
    ];

    assert_eq!(
        chosen(&monitors),
        Some(r"gdi:\\.\DISPLAY2"),
        "x=-3840 is the leftmost corner; abs() or an unsigned read inverts this"
    );
}

/// FR-102 / Appendix D — two displays both left of the primary, so the answer
/// is decided between two negative numbers. `-3840 < -1920`; under `abs()` the
/// comparison flips.
#[test]
fn the_more_negative_x_wins_between_two_displays_left_of_the_primary() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (-1920, 0), false),
        display(DISPLAY_3, (-3840, 0), false),
    ];

    assert_eq!(chosen(&monitors), Some(r"gdi:\\.\DISPLAY3"));
}

// ---------------------------------------------------------------------------
// The two refusals (ADR-0040), both meaning "open no output window"
// ---------------------------------------------------------------------------

/// ADR-0040 — no display is flagged primary, which is the state `flag_primary`
/// leaves behind when the OS gave no answer to "which display is primary".
/// Treating every display as non-primary would let this cover the display the
/// operator is looking at with a fullscreen black window mid-service, so the
/// answer is to open nothing. Two candidates present, so `None` cannot come
/// from there being nothing to choose from.
#[test]
fn no_primary_display_yields_none_even_though_candidates_exist() {
    let monitors = [
        display(DISPLAY_1, (0, 0), false),
        display(DISPLAY_2, (1920, 0), false),
    ];

    assert_eq!(chosen(&monitors), None);
}

/// ADR-0040 — the single-display machine seen through the same defect: one
/// display, unflagged. Still nothing to project onto.
#[test]
fn a_lone_unflagged_display_yields_none() {
    let monitors = [display(DISPLAY_1, (0, 0), false)];

    assert_eq!(chosen(&monitors), None);
}

/// ADR-0040 — the second refusal: no non-primary display. Every entry is
/// flagged primary, a state the OS does not produce but which an id collision
/// in `flag_primary` can, and there is still no display to project onto.
#[test]
fn every_display_being_primary_yields_none() {
    let monitors = [
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), true),
    ];

    assert_eq!(chosen(&monitors), None);
}

/// FR-106 — the ordinary single-display machine: one display, and it is the
/// primary. This is the case that keeps a fullscreen window off the operator's
/// only screen, and the one every development machine runs under.
#[test]
fn a_single_primary_display_yields_none() {
    let monitors = [display(DISPLAY_1, (0, 0), true)];

    assert_eq!(chosen(&monitors), None);
}

/// ADR-0040 — zero displays. `connected_monitors` resolves its dead error
/// branch to an empty list, so this reaches the selection rather than being
/// impossible, and it must not index into nothing.
#[test]
fn an_empty_list_yields_none() {
    assert_eq!(chosen(&[]), None);
}

// ---------------------------------------------------------------------------
// The FR-102 acceptance criterion itself, and what the caller reads back
// ---------------------------------------------------------------------------

/// FR-102 — "With 2 displays connected, both windows are correctly placed."
/// The half of that criterion this crate owns: with exactly two displays the
/// output goes on the non-primary one, whichever order they are enumerated in.
/// Asserted for the primary as either entry, because on a laptop with a
/// projector the primary is rarely entry zero.
#[test]
fn with_two_displays_the_output_goes_on_the_non_primary_one_in_either_order() {
    let primary = display(DISPLAY_1, (0, 0), true);
    let projector = display(DISPLAY_2, (1920, 0), false);

    assert_eq!(
        chosen(&[primary.clone(), projector.clone()]),
        Some(r"gdi:\\.\DISPLAY2")
    );
    assert_eq!(
        chosen(&[projector.clone(), primary.clone()]),
        Some(r"gdi:\\.\DISPLAY2")
    );

    // And with the projector arranged to the left, which is the same two
    // displays rearranged in Display Settings.
    let projector_on_the_left = display(DISPLAY_2, (-1920, 0), false);
    assert_eq!(
        chosen(&[primary.clone(), projector_on_the_left.clone()]),
        Some(r"gdi:\\.\DISPLAY2")
    );
    assert_eq!(
        chosen(&[projector_on_the_left, primary]),
        Some(r"gdi:\\.\DISPLAY2")
    );
}

/// FR-102 / FR-108 — the caller places and sizes the window from the record
/// this returns (`set_position`, then `set_size`, in physical pixels), so the
/// whole record must be the chosen display's own, not a copy of another
/// entry's geometry. Compared as a whole struct, and checked to be an element
/// of the input rather than a rebuilt value.
#[test]
fn the_selected_record_is_the_chosen_displays_own_entry() {
    let projector =
        Monitor::from_os_report(Some(DISPLAY_2), (3840, 2160), (-3840, -120), 2.0, false);
    let monitors = [display(DISPLAY_1, (0, 0), true), projector.clone()];

    let selected = select_output_monitor(&monitors).expect("a non-primary display is present");

    assert_eq!(selected, &projector);
    assert!(
        std::ptr::eq(selected, &monitors[1]),
        "the reference must point at the entry that was passed in"
    );
    assert!(
        !selected.is_primary,
        "an output display is never the primary"
    );
}

/// FR-102 — whatever is returned is one of the displays given, and it is never
/// the primary. Asserted across every ordering of a mixed arrangement so the
/// guarantee does not depend on the list either.
#[test]
fn the_result_is_always_a_non_primary_entry_of_the_input() {
    let arrangement = [
        display(DISPLAY_1, (1920, 0), true),
        display(DISPLAY_2, (0, 0), false),
        display(DISPLAY_3, (0, 1080), false),
        display(DISPLAY_4, (-1920, 540), false),
    ];

    for ordering in permutations(&arrangement) {
        let selected = select_output_monitor(&ordering).expect("candidates are present");

        assert!(!selected.is_primary);
        assert!(
            ordering.contains(selected),
            "ordering {:?} returned a display that was not in it",
            ids(&ordering)
        );
    }
}

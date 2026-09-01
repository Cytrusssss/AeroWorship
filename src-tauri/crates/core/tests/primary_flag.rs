//! Which display is flagged primary, and the blank-name class that decides
//! identity (FR-101, FR-103, ADR-0037, Appendix D).
//!
//! `flag_primary` is the whole of the answer to "which entry of `list_monitors`
//! is the primary". It used to be three lines inside `commands/display.rs`,
//! where no test could reach it: Tauri 2.11.5's mock runtime hardcodes
//! `available_monitors()` to an empty `Vec` and `tauri::Monitor`'s fields are
//! `pub(crate)`, so nothing can put two displays in front of a command. An
//! implementation that flagged entry zero would have passed every test that
//! could be written, and on a machine with a projector the primary is rarely
//! entry zero. Now that the matching is a pure function on this crate's public
//! surface, that wrong answer has to be made to fail here — the first test
//! below is the one that does it, and it is written so that replacing the body
//! of `flag_primary` with `is_primary: index == 0` turns it red.
//!
//! The blank-name tests are here rather than in `monitor_identity.rs` because
//! their subject is the same one: a name of `""` or `"   "` is folded to "no
//! name", so two blank-named displays fall to the `pos:` branch and get
//! *different* ids instead of sharing `gdi:`. Sharing one id is exactly how a
//! primary flag lands on the wrong display, which is what the last section
//! demonstrates end to end.
//!
//! Ids are spelled as string literals, never built from the crate's own
//! constants, for the reason ADR-0037 gives and `monitor_identity.rs` repeats:
//! these strings are persisted by FR-103, and a test that followed the scheme
//! tag wherever it moved would prove nothing.

use aeroworship_core::models::{flag_primary, monitor_id, Monitor};

const DISPLAY_1: &str = r"\\.\DISPLAY1";
const DISPLAY_2: &str = r"\\.\DISPLAY2";
const DISPLAY_3: &str = r"\\.\DISPLAY3";

/// One display as the OS would report it, built through the same constructor
/// the command uses so the ids under test are the ids that ship.
fn display(name: &str, position: (i32, i32), reported_primary: bool) -> Monitor {
    Monitor::from_os_report(Some(name), (1920, 1080), position, 1.0, reported_primary)
}

/// The flags of a result, in list order — the only thing most of these tests
/// look at, and the shape that makes an off-by-one visible.
fn flags(monitors: &[Monitor]) -> Vec<bool> {
    monitors.iter().map(|monitor| monitor.is_primary).collect()
}

// ---------------------------------------------------------------------------
// The wrong answer that used to be untestable
// ---------------------------------------------------------------------------

/// FR-101 — "The primary display is identified by [`Monitor::is_primary`],
/// never by position in this list." The primary is the *last* of three, which
/// is the ordinary case on a laptop with a projector attached.
///
/// This is the test that has to reject `is_primary: index == 0`. Two things
/// make it do so and neither is redundant: entry two is asserted flagged (an
/// index-zero implementation leaves it false), and entry zero is asserted
/// *unflagged even though it arrived with `is_primary: true`* (an
/// implementation that OR-ed the incoming flag in, or that trusted entry zero,
/// leaves it true).
#[test]
fn the_primary_is_found_at_a_non_zero_index_and_entry_zero_is_not_assumed() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), false),
        display(DISPLAY_3, (3840, 0), false),
    ];

    let flagged = flag_primary(monitors, Some(r"gdi:\\.\DISPLAY3"));

    assert_eq!(
        flags(&flagged),
        vec![false, false, true],
        "the primary is the third entry; only it may carry the flag"
    );
}

/// FR-101 — the middle entry, so a test suite cannot be satisfied by an
/// implementation that flags the last entry instead of the first.
#[test]
fn the_primary_is_found_in_the_middle_of_the_list() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), false),
        display(DISPLAY_2, (1920, 0), false),
        display(DISPLAY_3, (3840, 0), false),
    ];

    let flagged = flag_primary(monitors, Some(r"gdi:\\.\DISPLAY2"));

    assert_eq!(flags(&flagged), vec![false, true, false]);
}

/// FR-101 — exactly one entry is flagged wherever the primary sits. Run over
/// every index of a three-display arrangement so no single position is a
/// special case, and so "flags the right one" is asserted as a property rather
/// than as three coincidences.
#[test]
fn exactly_one_entry_is_flagged_for_every_position_the_primary_can_occupy() {
    let names = [DISPLAY_1, DISPLAY_2, DISPLAY_3];

    for (index, name) in names.iter().enumerate() {
        let monitors = names
            .iter()
            .enumerate()
            .map(|(position, name)| display(name, (1920 * position as i32, 0), false))
            .collect();

        let flagged = flag_primary(monitors, Some(monitor_id(Some(name), (0, 0)).as_str()));
        let expected: Vec<bool> = (0..names.len()).map(|entry| entry == index).collect();

        assert_eq!(
            flags(&flagged),
            expected,
            "primary at index {index} ({name}) was not the entry flagged"
        );
    }
}

// ---------------------------------------------------------------------------
// The incoming flag is an input, not evidence
// ---------------------------------------------------------------------------

/// FR-101 — "An entry's own `is_primary` on the way in is ignored". The flag is
/// overwritten, not OR-ed: every entry arrives claiming to be primary and only
/// the one `primary_id` names comes back that way. A stale `true` from any
/// source — an earlier enumeration, a caller's placeholder — cannot survive.
#[test]
fn an_incoming_is_primary_is_overwritten_rather_than_or_ed() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), true),
        display(DISPLAY_3, (3840, 0), true),
    ];

    let flagged = flag_primary(monitors, Some(r"gdi:\\.\DISPLAY2"));

    assert_eq!(flags(&flagged), vec![false, true, false]);
}

/// FR-101 — the same rule in the other direction: the entry `primary_id` names
/// is flagged even though it arrived `false`. Together with the test above this
/// pins the flag as a function of `primary_id` alone.
#[test]
fn the_named_entry_is_flagged_even_though_it_arrived_unflagged() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), false),
    ];

    let flagged = flag_primary(monitors, Some(r"gdi:\\.\DISPLAY2"));

    assert_eq!(flags(&flagged), vec![false, true]);
}

// ---------------------------------------------------------------------------
// Nothing to flag
// ---------------------------------------------------------------------------

/// FR-101 — `primary_monitor()` returning `None` (the display was detached
/// between the two OS calls) becomes `primary_id: None`, and then no entry is
/// marked. Every entry arrives `true` so that "nothing is flagged" cannot be
/// satisfied by simply passing the input through.
#[test]
fn a_primary_id_of_none_flags_nothing_even_when_every_entry_arrived_flagged() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), true),
    ];

    let flagged = flag_primary(monitors, None);

    assert_eq!(flags(&flagged), vec![false, false]);
}

/// FR-101 — a `primary_id` that names no entry leaves everything unflagged
/// rather than falling back to an arbitrary display. This is the state the app
/// is in for an instant when a display is swapped between the two OS calls.
#[test]
fn a_primary_id_matching_no_entry_flags_nothing() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), false),
        display(DISPLAY_2, (1920, 0), false),
    ];

    let flagged = flag_primary(monitors, Some(r"gdi:\\.\DISPLAY9"));

    assert_eq!(flags(&flagged), vec![false, false]);
}

/// FR-101 — an empty `primary_id` is not a wildcard and not a blank match. No
/// derived id is empty (`monitor_identity.rs` pins that), so the comparison
/// must simply find nothing; a `contains`/`starts_with` test would flag every
/// entry here.
#[test]
fn an_empty_primary_id_flags_nothing() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), true),
        display(DISPLAY_2, (1920, 0), false),
    ];

    let flagged = flag_primary(monitors, Some(""));

    assert_eq!(flags(&flagged), vec![false, false]);
}

/// FR-101 — ids are compared whole. A `primary_id` that is a prefix of a real
/// id, or a real id that is a prefix of `primary_id`, must not match: the
/// scheme tags exist so `pos:` and `gdi:` ids stay distinguishable, and a
/// substring comparison would defeat that.
#[test]
fn a_primary_id_is_matched_whole_and_not_by_prefix_or_substring() {
    let monitors = vec![
        display(DISPLAY_1, (0, 0), false),
        Monitor::from_os_report(None, (1920, 1080), (1920, 0), 1.0, false),
    ];

    assert_eq!(
        flags(&flag_primary(monitors.clone(), Some("gdi:"))),
        vec![false, false]
    );
    assert_eq!(
        flags(&flag_primary(monitors.clone(), Some("pos:"))),
        vec![false, false]
    );
    assert_eq!(
        flags(&flag_primary(monitors.clone(), Some(r"\\.\DISPLAY1"))),
        vec![false, false],
        "the bare OS name is not the id; only the tagged form is"
    );
    assert_eq!(
        flags(&flag_primary(monitors, Some(r"gdi:\\.\DISPLAY10"))),
        vec![false, false],
        "DISPLAY1's id is a prefix of DISPLAY10's and must not match it"
    );
}

// ---------------------------------------------------------------------------
// Degenerate list sizes
// ---------------------------------------------------------------------------

/// FR-101 — zero displays is a state the app renders anyway (the command
/// resolves its dead error branch to an empty list), so the matching must
/// survive it rather than index into nothing.
#[test]
fn no_displays_yields_no_entries_whatever_the_primary_id_is() {
    assert!(flag_primary(Vec::new(), Some(r"gdi:\\.\DISPLAY1")).is_empty());
    assert!(flag_primary(Vec::new(), None).is_empty());
}

/// FR-106 — the single-display machine, where the one entry is the primary.
/// Kept explicit because it is the arrangement most development machines have,
/// and the one under which every wrong implementation still looks right.
#[test]
fn a_single_display_is_flagged_when_it_is_the_primary_and_not_otherwise() {
    let sole = vec![display(DISPLAY_1, (0, 0), false)];
    assert_eq!(
        flags(&flag_primary(sole.clone(), Some(r"gdi:\\.\DISPLAY1"))),
        vec![true]
    );
    assert_eq!(flags(&flag_primary(sole.clone(), None)), vec![false]);
    assert_eq!(
        flags(&flag_primary(sole, Some(r"gdi:\\.\DISPLAY2"))),
        vec![false]
    );
}

// ---------------------------------------------------------------------------
// Two entries carrying one id
// ---------------------------------------------------------------------------

/// FR-101 — every entry whose id matches is flagged, not just the first. The
/// documented reason: picking one of two indistinguishable entries could only
/// mean picking by list position, which is the thing this function exists to
/// prevent. If ids ever collide, the honest report is that the answer is
/// ambiguous, and a caller can see it.
#[test]
fn every_entry_sharing_the_primary_id_is_flagged_not_only_the_first() {
    // Same OS name at two positions: position does not enter the `gdi:` id, so
    // these two entries carry one id.
    let monitors = vec![
        display(DISPLAY_1, (0, 0), false),
        display(DISPLAY_2, (1920, 0), false),
        display(DISPLAY_1, (3840, 0), false),
    ];
    assert_eq!(monitors[0].id, monitors[2].id, "precondition: ids collide");

    let flagged = flag_primary(monitors, Some(r"gdi:\\.\DISPLAY1"));

    assert_eq!(flags(&flagged), vec![true, false, true]);
}

// ---------------------------------------------------------------------------
// What the function must leave alone
// ---------------------------------------------------------------------------

/// FR-101 — "Order is preserved", and `is_primary` is the only field touched.
/// Compared as whole structs against the input with just that field rewritten,
/// so a reordering, a dropped entry or a field quietly rebuilt from something
/// else fails here.
#[test]
fn order_and_every_other_field_survive_untouched() {
    let monitors = vec![
        Monitor::from_os_report(Some(DISPLAY_1), (3840, 2160), (-1920, -120), 2.0, false),
        Monitor::from_os_report(Some(DISPLAY_2), (1920, 1080), (0, 0), 1.25, false),
    ];

    let flagged = flag_primary(monitors.clone(), Some(r"gdi:\\.\DISPLAY2"));

    assert_eq!(
        flagged,
        vec![
            Monitor {
                is_primary: false,
                ..monitors[0].clone()
            },
            Monitor {
                is_primary: true,
                ..monitors[1].clone()
            },
        ]
    );
}

// ---------------------------------------------------------------------------
// The blank-name class (ADR-0037)
// ---------------------------------------------------------------------------

/// ADR-0037 — `Some("")` is not a name. It takes the `pos:` fallback rather
/// than producing the bare tag `gdi:`, which identifies nothing.
#[test]
fn an_empty_name_falls_to_the_position_branch_and_never_yields_a_bare_gdi_tag() {
    let id = monitor_id(Some(""), (1920, 0));

    assert_eq!(id, "pos:1920,0");
    assert!(!id.starts_with("gdi:"), "{id} identifies nothing");
}

/// ADR-0037 — a whitespace-only name is the same defect as an empty one for the
/// only reader that matters, a picker row rendering as blank, and the same
/// defect for the id. Several shapes of whitespace, because the fold is a trim
/// and not an `== " "`.
#[test]
fn a_whitespace_only_name_is_treated_exactly_as_an_empty_one() {
    for blank in ["   ", "\t", "\n", " \t\r\n "] {
        let id = monitor_id(Some(blank), (1920, 0));
        assert_eq!(
            id,
            monitor_id(Some(""), (1920, 0)),
            "{blank:?} should fold to the same no-name case as \"\""
        );
        assert_eq!(id, "pos:1920,0");
        assert!(!id.starts_with("gdi:"), "{id:?} identifies nothing");
    }
}

/// ADR-0037 — the reason the fold exists. Two displays both reporting a blank
/// name, in different shapes, must still be told apart; sharing one id is
/// precisely how a primary flag lands on the wrong display. Under a `gdi:`
/// derivation these two would collide.
#[test]
fn two_blank_named_displays_get_distinct_ids_instead_of_sharing_one() {
    let first = Monitor::from_os_report(Some(""), (1920, 1080), (0, 0), 1.0, false);
    let second = Monitor::from_os_report(Some("   "), (1920, 1080), (1920, 0), 1.0, false);

    assert_ne!(first.id, second.id);
    assert_eq!(first.id, "pos:0,0");
    assert_eq!(second.id, "pos:1920,0");
}

/// ADR-0037 / Appendix D — a blank name gets the same fallback as no name at
/// all: the picker shows the id, an ugly row rather than a blank one. Asserted
/// for both blank shapes, and asserted non-blank rather than merely non-empty,
/// since `"   "` is non-empty and still renders as nothing.
#[test]
fn a_blank_name_falls_back_to_the_id_the_same_way_a_missing_one_does() {
    for blank in [Some(""), Some("   "), None] {
        let monitor = Monitor::from_os_report(blank, (1920, 1080), (0, 0), 1.0, false);

        assert_eq!(monitor.name, monitor.id, "{blank:?} should show its id");
        assert!(
            !monitor.name.trim().is_empty(),
            "{blank:?} produced a blank picker row"
        );
    }
}

/// ADR-0037 — only the emptiness *test* trims. A name that survives it is used
/// verbatim, spaces and all: FR-103 has already persisted the id derived from
/// it, so trimming the name would silently rewrite a stored key and point an
/// assignment at nothing.
#[test]
fn a_name_that_survives_the_blank_test_keeps_its_surrounding_whitespace() {
    let padded = format!(" {DISPLAY_1} ");

    assert_eq!(
        monitor_id(Some(&padded), (0, 0)),
        r"gdi: \\.\DISPLAY1 ",
        "the id must carry the name exactly as the OS reported it"
    );
    assert_ne!(
        monitor_id(Some(&padded), (0, 0)),
        monitor_id(Some(DISPLAY_1), (0, 0)),
        "trimming would merge two ids FR-103 may have persisted separately"
    );

    let monitor = Monitor::from_os_report(Some(&padded), (1920, 1080), (0, 0), 1.0, false);
    assert_eq!(
        monitor.name, padded,
        "the display name is not trimmed either"
    );
}

// ---------------------------------------------------------------------------
// The two halves together: the mechanism the fold prevents
// ---------------------------------------------------------------------------

/// FR-101 + ADR-0037 — end to end on the path that has no OS names at all. The
/// blank-named second display is the primary; because the fold sent both
/// entries to the `pos:` branch they have distinct ids, so the flag lands on
/// the second and only on the second. Had both kept a `gdi:`-shaped id derived
/// from a blank name, they would share one id and this would flag both.
#[test]
fn a_blank_named_primary_is_flagged_on_its_own_entry_and_no_other() {
    let monitors = vec![
        Monitor::from_os_report(Some("   "), (1920, 1080), (0, 0), 1.0, true),
        Monitor::from_os_report(Some(""), (1920, 1080), (1920, 0), 1.0, false),
        Monitor::from_os_report(None, (1920, 1080), (3840, 0), 1.0, false),
    ];

    // The id `primary_monitor()` would derive for the middle display.
    let primary_id = monitor_id(Some(""), (1920, 0));
    let flagged = flag_primary(monitors, Some(primary_id.as_str()));

    assert_eq!(flags(&flagged), vec![false, true, false]);
}

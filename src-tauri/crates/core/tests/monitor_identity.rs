//! Display identity and the wire record built from an OS report (FR-101,
//! ADR-0037, Appendix D).
//!
//! Both items under test — `monitor_id` and `Monitor::from_os_report` — are
//! `pub` and name no `tauri` type, so they are exercised here through the
//! crate's public surface only, the same way `tests/schema.rs` exercises `db`.
//!
//! **What is deliberately not here.** The `is_primary` flag is not on
//! `tauri::Monitor`; it is synthesised by calling `primary_monitor()` a second
//! time and matching its derived id back against every entry of
//! `available_monitors()`. That matching is `models::flag_primary` and is
//! covered by `tests/primary_flag.rs`; only the OS read around it still needs
//! an `AppHandle`. What this file pins is everything the matching stands on —
//! that the derivation is deterministic, and that two displays which differ get
//! different ids — so those are named below as preconditions rather than
//! dressed up as a test of the matching itself.
//!
//! Scheme tags are asserted as string literals, never against the constants in
//! `models::monitor`. ADR-0037's whole argument is that a key already written
//! to a church's database must stay recognisable as belonging to this scheme;
//! a test comparing the output against the same constant that produced it
//! would follow the tag silently wherever it moved and prove nothing.

use aeroworship_core::models::{monitor_id, Monitor};

/// The shape Windows reports: `MONITORINFOEXW.szDevice`.
const DISPLAY_1: &str = r"\\.\DISPLAY1";
const DISPLAY_2: &str = r"\\.\DISPLAY2";

// ---------------------------------------------------------------------------
// Scheme tags (ADR-0037)
// ---------------------------------------------------------------------------

/// ADR-0037 — a named display's id is the `gdi:` tag followed by the OS name,
/// verbatim. The literal is spelled out in full because this string is what
/// FR-103 persists.
#[test]
fn a_named_display_gets_the_gdi_scheme_tag_and_the_os_name_verbatim() {
    assert_eq!(monitor_id(Some(DISPLAY_1), (0, 0)), r"gdi:\\.\DISPLAY1");
    assert_eq!(monitor_id(Some(DISPLAY_2), (1920, 0)), r"gdi:\\.\DISPLAY2");
}

/// ADR-0037 — the fallback branch. Untaken on Windows in practice (`name` is
/// `None` only if `GetMonitorInfoW` fails), which is exactly why it needs a
/// test: nothing else executes it. Both coordinates appear, comma-separated,
/// behind the `pos:` tag.
#[test]
fn an_unnamed_display_gets_the_pos_scheme_tag_and_both_coordinates() {
    assert_eq!(monitor_id(None, (0, 0)), "pos:0,0");
    assert_eq!(monitor_id(None, (1920, 0)), "pos:1920,0");
    assert_eq!(monitor_id(None, (-1920, -540)), "pos:-1920,-540");
}

/// ADR-0037 — the two schemes carry different guarantees, so an id from one
/// must never be readable as an id from the other. Includes the adversarial
/// case of a display whose OS name is itself spelled like a position: the tag
/// is a prefix, not a search, so `pos:0,0` as a *name* still lands in the
/// `gdi:` scheme.
#[test]
fn the_two_schemes_are_distinguishable_from_the_id_alone() {
    let named = monitor_id(Some(DISPLAY_1), (0, 0));
    assert!(named.starts_with("gdi:"), "{named} lost its scheme tag");
    assert!(!named.starts_with("pos:"));

    let unnamed = monitor_id(None, (0, 0));
    assert!(unnamed.starts_with("pos:"), "{unnamed} lost its scheme tag");
    assert!(!unnamed.starts_with("gdi:"));

    let misleading = monitor_id(Some("pos:0,0"), (2560, 0));
    assert_eq!(misleading, "gdi:pos:0,0");
    assert!(misleading.starts_with("gdi:"));
}

// ---------------------------------------------------------------------------
// The stability the id is chosen for, and the point where it stops (ADR-0037)
// ---------------------------------------------------------------------------

/// ADR-0037 — "Yang stabil: … penyusunan ulang display". A stored FR-103
/// assignment has to survive the user dragging the displays around in Display
/// Settings, so position must not reach the named id at all.
#[test]
fn a_named_id_survives_the_display_being_rearranged() {
    let before = monitor_id(Some(DISPLAY_2), (1920, 0));
    let after_moving_it_left_of_the_primary = monitor_id(Some(DISPLAY_2), (-1920, -540));
    assert_eq!(before, after_moving_it_left_of_the_primary);
}

/// ADR-0037 point 3 — the `pos:` branch does *not* survive a rearrangement.
/// That weaker guarantee is the stated reason the two branches carry different
/// tags; if this ever started passing as equality, the separate tag would have
/// lost its justification without anyone noticing.
#[test]
fn a_pos_id_does_not_survive_the_display_being_rearranged() {
    let before = monitor_id(None, (1920, 0));
    let after = monitor_id(None, (-1920, -540));
    assert_ne!(before, after);
}

// ---------------------------------------------------------------------------
// The wire record (Appendix D, FR-108)
// ---------------------------------------------------------------------------

/// FR-101 / Appendix D — every field lands where the frontend expects it, and
/// nothing is transposed: `size` is (width, height) and `position` is (x, y).
/// Compared as a whole struct so a field silently dropped from the mapping
/// fails here too.
#[test]
fn from_os_report_maps_every_field_in_place() {
    let monitor = Monitor::from_os_report(Some(DISPLAY_1), (2560, 1440), (0, 0), 1.25, true);

    assert_eq!(
        monitor,
        Monitor {
            id: r"gdi:\\.\DISPLAY1".to_owned(),
            name: DISPLAY_1.to_owned(),
            width: 2560,
            height: 1440,
            x: 0,
            y: 0,
            scale_factor: 1.25,
            is_primary: true,
        }
    );
}

/// FR-108 — the output must render at the projector's native resolution
/// whatever the control display is scaled to, which only works if these
/// numbers are physical pixels. A 4K panel at 200% is the case where dividing
/// by `scale_factor` would look plausible and be wrong: it would report
/// 1920×1080 for a 3840×2160 display.
#[test]
fn width_and_height_stay_physical_pixels_and_are_not_divided_by_the_scale_factor() {
    let monitor = Monitor::from_os_report(Some(DISPLAY_1), (3840, 2160), (0, 0), 2.0, true);

    assert_eq!(monitor.width, 3840);
    assert_eq!(monitor.height, 2160);
}

/// Appendix D — "Negative for a display arranged to the left of the primary."
/// The virtual-screen origin is the primary's top-left, so a projector placed
/// left of, or above, the control display reports negative coordinates; they
/// must reach the frontend unclamped.
#[test]
fn a_position_left_of_and_above_the_primary_stays_negative() {
    let monitor = Monitor::from_os_report(Some(DISPLAY_2), (1920, 1080), (-1920, -120), 1.0, false);

    assert_eq!(monitor.x, -1920);
    assert_eq!(monitor.y, -120);
    assert!(!monitor.is_primary);
}

/// Appendix D types `name` as a plain `string`, so the no-name case has to
/// fall back to something; the documented choice is the id, so that a picker
/// shows an ugly row rather than a blank one. The name is therefore non-empty
/// and carries the scheme tag.
#[test]
fn an_unnamed_display_falls_back_to_its_id_as_a_name() {
    let monitor = Monitor::from_os_report(None, (1920, 1080), (-1920, 0), 1.0, false);

    assert_eq!(monitor.id, "pos:-1920,0");
    assert_eq!(monitor.name, monitor.id);
    assert!(!monitor.name.is_empty());
}

// ---------------------------------------------------------------------------
// Preconditions for the `is_primary` matching (`models::flag_primary`)
//
// The matching is exercised directly in `tests/primary_flag.rs`; these pin the
// properties of the id derivation it relies on.
// ---------------------------------------------------------------------------

/// Precondition — the command derives an id twice from two independent OS
/// reports of the same display (one from `primary_monitor()`, one from the
/// `available_monitors()` entry) and compares the strings. That comparison is
/// only ever true if the derivation is a pure function of what it is given.
#[test]
fn the_id_a_monitor_record_carries_is_the_one_monitor_id_derives() {
    let named = Monitor::from_os_report(Some(DISPLAY_1), (1920, 1080), (0, 0), 1.0, true);
    assert_eq!(named.id, monitor_id(Some(DISPLAY_1), (0, 0)));

    let unnamed = Monitor::from_os_report(None, (1920, 1080), (1920, 0), 1.0, false);
    assert_eq!(unnamed.id, monitor_id(None, (1920, 0)));
}

/// Precondition — the case this machine cannot produce: two displays of
/// identical size and scale factor, differing only in position. If their ids
/// collided, the primary flag would be set on both. Named branch, where
/// position does not enter the id at all and the names alone must separate
/// them.
#[test]
fn two_displays_identical_but_for_position_get_distinct_ids() {
    let first = Monitor::from_os_report(Some(DISPLAY_1), (1920, 1080), (0, 0), 1.0, false);
    let second = Monitor::from_os_report(Some(DISPLAY_2), (1920, 1080), (1920, 0), 1.0, false);

    assert_ne!(first.id, second.id);

    // The second display is the primary: exactly one entry matches the key
    // `primary_monitor()` would derive for it.
    let primary_id = monitor_id(Some(DISPLAY_2), (1920, 0));
    let matches = [&first, &second]
        .iter()
        .filter(|monitor| monitor.id == primary_id)
        .count();
    assert_eq!(matches, 1);
    assert_eq!(second.id, primary_id);
}

/// Precondition — the same pair on the fallback branch, where position is the
/// only thing separating them. Monitors do not overlap, so distinct positions
/// are what makes `pos:` usable as an identity at a given instant.
#[test]
fn two_unnamed_displays_are_separated_by_their_positions() {
    let first = Monitor::from_os_report(None, (1920, 1080), (0, 0), 1.0, false);
    let second = Monitor::from_os_report(None, (1920, 1080), (1920, 0), 1.0, false);

    assert_ne!(first.id, second.id);
    assert_eq!(first.id, "pos:0,0");
    assert_eq!(second.id, "pos:1920,0");
}

/// Precondition — `primary_monitor()` reporting a display that is in no entry
/// of `available_monitors()` must leave every entry unflagged rather than
/// flagging an arbitrary one. The id it derives is simply absent from the set.
#[test]
fn a_primary_report_matching_no_entry_flags_nothing() {
    let entries = [
        Monitor::from_os_report(Some(DISPLAY_1), (1920, 1080), (0, 0), 1.0, false),
        Monitor::from_os_report(Some(DISPLAY_2), (1920, 1080), (1920, 0), 1.0, false),
    ];

    let primary_id = monitor_id(Some(r"\\.\DISPLAY3"), (3840, 0));

    assert!(!entries.iter().any(|monitor| monitor.id == primary_id));
}

/// Precondition — a display detached between the two OS calls is reported by
/// `primary_monitor()` as `None`, and the command turns that into "no primary
/// id at all". No derived id may ever equal the empty-key case, so nothing can
/// be flagged by accident.
///
/// Each case is asserted against the id it must actually produce rather than
/// merely against non-emptiness. The blank-name case is the reason: it now
/// takes the `pos:` branch, and "non-empty" would have been satisfied just as
/// well by the bare tag `gdi:` — a key that identifies nothing and that two
/// blank-named displays would share.
#[test]
fn no_derived_id_is_empty_so_a_missing_primary_cannot_match() {
    for (id, expected) in [
        (monitor_id(Some(DISPLAY_1), (0, 0)), r"gdi:\\.\DISPLAY1"),
        (monitor_id(None, (0, 0)), "pos:0,0"),
        (monitor_id(Some(""), (0, 0)), "pos:0,0"),
    ] {
        assert!(!id.is_empty());
        assert_eq!(id, expected);
    }
}

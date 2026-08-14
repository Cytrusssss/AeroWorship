//! One connected display, as the frontend sees it (FR-101, Appendix D).
//!
//! The shape is fixed by Appendix D and is `camelCase` on the wire, so this is
//! also the first type in the tree to carry `#[serde(rename_all)]`. That
//! attribute is only honoured by the generator because `ts-rs`'s `serde-compat`
//! feature is on; see the note next to `ts-rs` in `crates/core/Cargo.toml` for
//! why turning it off would produce a plausible-looking but wrong `.ts`.
//!
//! Nothing here names a `tauri` type. The shell reads the OS through
//! `AppHandle::available_monitors()` and hands the plain numbers to
//! [`Monitor::from_os_report`]; the derivation of the id is the part whose
//! correctness matters, so it lives here as a pure function (PRD §6.1).
//!
//! Doc comments on the struct and its fields are copied verbatim into
//! `src/shared/bindings/Monitor.ts` by ts-rs, so they are written for a
//! frontend reader. Anything that is this crate's own business — why a
//! decision was taken, where it stops holding — is in ordinary `//` comments,
//! which are not copied.

use serde::Serialize;

/// A display reported by the operating system.
///
/// All sizes and positions are physical pixels in the virtual-screen
/// coordinate space, exactly as the OS reports them: not logical pixels, and
/// not divided by `scaleFactor`.
// The division is left to the caller on purpose. FR-108 needs the output
// display's native resolution regardless of what the control display is scaled
// to, so a conversion applied here — where it is not yet known which display is
// which — would have to be undone there.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Monitor {
    /// Opaque, machine-local identity. Use it to refer to a display; do not
    /// parse it and do not show it to a user — `name` is the field for that.
    // This is the key FR-103 persists an output-display assignment under. What
    // it is derived from, and the cases where it stops being stable, are on
    // `monitor_id` below.
    pub id: String,
    /// What the OS calls this display — on Windows the device name, such as
    /// `\\.\DISPLAY1`.
    // Appendix D types this as a plain `string`, so the no-name case has to
    // fall back to something. It falls back to `id` rather than to an empty
    // string: a picker with two blank rows is worse than one with two ugly
    // ones. "No name" is decided by `reported_name`, so a name the OS reports
    // as blank gets the same fallback as a name it does not report at all — a
    // row of spaces is exactly the blank row this fallback exists to prevent.
    pub name: String,
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// X of the top-left corner in the virtual-screen space. Negative for a
    /// display arranged to the left of the primary.
    pub x: i32,
    /// Y of the top-left corner in the virtual-screen space.
    pub y: i32,
    /// Logical-to-physical pixel ratio: 1.0 at 96 DPI, 1.5 at 150% scaling.
    pub scale_factor: f64,
    /// Whether the OS considers this the primary display.
    pub is_primary: bool,
}

impl Monitor {
    /// Builds the wire record from what the OS reported about one display.
    ///
    /// `size` is `(width, height)` and `position` is `(x, y)`, both in physical
    /// pixels.
    // Taken as pairs rather than as four scalars because two adjacent `u32`s
    // and two adjacent `i32`s in an argument list are exactly the kind of thing
    // that gets silently transposed.
    pub fn from_os_report(
        name: Option<&str>,
        size: (u32, u32),
        position: (i32, i32),
        scale_factor: f64,
        is_primary: bool,
    ) -> Self {
        // Normalised once, here, so the id and the display name cannot end up
        // disagreeing about whether this display was named.
        let name = reported_name(name);
        let id = monitor_id(name, position);
        Self {
            name: name.map_or_else(|| id.clone(), str::to_owned),
            id,
            width: size.0,
            height: size.1,
            x: position.0,
            y: position.1,
            scale_factor,
            is_primary,
        }
    }
}

/// Marks the one display the OS calls primary, and unmarks every other.
///
/// `primary_id` is the id derived from the OS's separate answer to "which
/// display is primary"; `None` means it gave no answer, and then no entry is
/// marked. An entry's own `is_primary` on the way in is ignored: what this
/// returns depends only on the ids and on `primary_id`.
///
/// Order is preserved. The primary display is identified by this flag, never by
/// its position in the list.
//
// **Why this is a function and not three lines inside the command.** The OS is
// read through an `AppHandle`, and under Tauri 2.11.5's mock runtime
// `primary_monitor()` and `available_monitors()` are hardcoded to `None` and an
// empty `Vec` (`test/mock_runtime.rs:789,797`) while `tauri::Monitor`'s fields
// are `pub(crate)` (`window/mod.rs:58-64`), so no test anywhere can put two
// displays in front of a command. An implementation that simply marked entry
// zero would therefore pass every test that can be written against the command
// — and on a machine with a projector the primary is rarely the first entry.
// Splitting the matching out is what makes the wrong answer observable.
//
// Two entries carrying the same id would both come back marked. That case is
// not reachable from `monitor_id` today (names are unique among attached
// displays, and the fallback keys on position, which cannot repeat because
// displays do not overlap), and if it ever becomes reachable, marking both is
// the honest report: choosing one of two indistinguishable entries could only
// mean choosing by list position, which is the thing this function exists to
// avoid.
pub fn flag_primary(monitors: Vec<Monitor>, primary_id: Option<&str>) -> Vec<Monitor> {
    monitors
        .into_iter()
        .map(|monitor| Monitor {
            // Compared as a whole string. `primary_id` is derived by the same
            // `monitor_id` that produced `monitor.id`, so equality here means
            // the two OS reports describe the same display; a prefix or
            // substring test would let a `pos:` id match a `gdi:` one whose
            // name happens to contain it.
            is_primary: primary_id == Some(monitor.id.as_str()),
            ..monitor
        })
        .collect()
}

/// Prefix for an id derived from the name the OS gives a display.
//
// The scheme tag is not decoration. This id is persisted by FR-103, and the
// identity the OS hands us today is weaker than that job deserves (see
// `monitor_id`); if a later item derives ids from display hardware instead,
// every key already written to a church's database has to be recognisable as
// belonging to the old scheme, so it can be dropped rather than silently
// matched against the wrong display.
const SCHEME_OS_NAME: &str = "gdi:";

/// Prefix for the fallback id, derived from the display's position.
const SCHEME_POSITION: &str = "pos:";

/// What the OS actually named this display, or `None` if it named nothing
/// usable.
//
// `Some("")` is not a name, and neither is `Some("   ")`. The Tauri API types
// the name as `Option<String>`, but `None` is not the only way a name can be
// missing: the string is whatever `MONITORINFOEXW.szDevice` held, and nothing
// in that API promises it is non-blank. Both blank forms are folded into `None`
// here so there is exactly one "unnamed" case for the rest of this module to
// reason about.
//
// Whitespace-only is folded in with empty because the two are the same defect
// seen by the only reader who matters: a picker row rendering as blank. They
// are also the same defect for the id — `gdi:` and `gdi:   ` are identities
// derived from nothing, and two displays reporting the same blank name would
// collide on one id, which is precisely how a primary flag lands on the wrong
// display.
//
// Only the emptiness *test* trims; a name that survives it is used verbatim.
// Trimming the name itself would change the id derived from it, and FR-103 has
// already persisted that id.
fn reported_name(name: Option<&str>) -> Option<&str> {
    name.filter(|name| !name.trim().is_empty())
}

/// Derives a display's identity from what the OS reports about it.
///
/// The result is opaque and machine-local. It is stable across a resolution
/// change, a scaling change, a rearrangement of the displays and a restart with
/// the same displays attached — but not across every replug; see below.
//
// **What this is built from, and why.** Of everything the installed Tauri
// (2.11.5) exposes about a monitor — name, size, position, work area, scale
// factor — only the *name* is not a measurement of the display's current
// settings. Size changes when the resolution changes, scale factor changes when
// the user drags the scaling slider, position changes when displays are
// rearranged; all three are things FR-103's stored assignment has to survive,
// so none of them may enter the id. On Windows the name is
// `MONITORINFOEXW.szDevice` (`\\.\DISPLAY1`), which is unique among attached
// displays at any instant and does not move when settings change.
//
// **Where it is not stable, stated plainly.** `\\.\DISPLAYn` is a slot, not a
// monitor. Windows assigns the numbers and reuses them: unplug the projector,
// plug a different one into the same port, and it is `\\.\DISPLAY2` again — so
// a stored assignment now points at different hardware. Two identical monitors
// are told apart only by which slot they landed in, so swapping their cables
// swaps their ids. Measured evidence that the slot name is the wrong key in
// principle: Windows itself does not use it. The machine this was written on
// lists seven monitors under `HKLM\SYSTEM\CurrentControlSet\Enum\DISPLAY`,
// keyed by EDID manufacturer/product code (`AUO5C2D`, `LEN63EB`, …), never by
// slot. The stable key exists at the OS level; it is simply not on the Tauri
// API's surface, and reaching it means Win32 FFI (`EnumDisplayDevices` with
// `EDD_GET_DEVICE_INTERFACE_NAME`, or `QueryDisplayConfig`) — a decision of its
// own, and not this item's.
//
// **The fallback.** `name` is `Option` in the Tauri API, documented as `None`
// when "the monitor doesn't exist anymore". On Windows it is `None` only if
// `GetMonitorInfoW` fails, which does not happen for a handle
// `EnumDisplayMonitors` has just produced, so the branch is unreachable in
// practice. It exists so the function is total, and it uses position because
// that is the only remaining field unique across displays at a given instant —
// monitors do not overlap. An id from this branch does not survive a
// rearrangement, a worse guarantee than the primary branch gives, which is why
// the two carry different scheme tags. A name the OS reports as blank takes
// this branch as well (`reported_name`): `gdi:` on its own identifies nothing,
// and two blank-named displays would share it.
pub fn monitor_id(name: Option<&str>, position: (i32, i32)) -> String {
    match reported_name(name) {
        Some(name) => format!("{SCHEME_OS_NAME}{name}"),
        None => format!("{SCHEME_POSITION}{},{}", position.0, position.1),
    }
}

//! Serde types that become the generated TypeScript contract (NFR-33).
//!
//! Template geometry is stored normalised to 0–1, never in pixels (FR-406).

pub mod monitor;
pub mod scripture;

pub use monitor::{flag_primary, monitor_id, select_output_monitor, Monitor};
pub use scripture::{
    parse_reference, parse_scripture_ref, resolve_reference, BookIndex, BookMatch, ParsedReference,
    ScriptureRef,
};

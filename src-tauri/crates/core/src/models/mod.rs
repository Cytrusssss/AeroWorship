//! Serde types that become the generated TypeScript contract (NFR-33).
//!
//! Template geometry is stored normalised to 0–1, never in pixels (FR-406).

pub mod monitor;

pub use monitor::{flag_primary, monitor_id, Monitor};

//! Orchestration that has to own an `AppHandle` or a `WebviewWindow`.
//!
//! A service here holds the Tauri objects and the sequencing; every decision it
//! makes that could be wrong on a machine unlike this one is a pure function in
//! [`aeroworship_core`] (PRD §6.1, ADR-0008). The split is not stylistic: no
//! test can put two displays in front of an `AppHandle` (see the note on
//! `flag_primary`), so anything left on this side of the boundary is verified by
//! a human with a projector or not at all.
//!
//! Commands call services; services do not call commands.

pub mod display;

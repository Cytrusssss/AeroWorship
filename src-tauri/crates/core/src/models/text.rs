//! Unicode character classes the model modules have to agree about.
//!
//! One table, two callers. [`scripture`](super::scripture) deletes these
//! characters so a book spelling stays reachable; [`template`](super::template)
//! refuses them so a template name cannot lie about what it says. The rule
//! itself — "which code points are `Cf`" — is a fact about Unicode and not
//! about either domain, so it lives here rather than in whichever module needed
//! it first, and neither module can drift from the other by patching its own
//! copy.
//!
//! Nothing here is `pub`: there are no serde types in this module and nothing
//! of it reaches the generated TypeScript (NFR-33).

/// Unicode general category `Cf`, as of Unicode 16.0.
//
// **Why `Cf` is written out instead of pulled from a crate.** A Unicode
// property lookup would be a new dependency against a 15 MB installer
// (NFR-16), for a property whose entire assigned set is the two dozen ranges
// below. This is a snapshot of Unicode 16.0. A format character added in a
// later Unicode would pass through, which is what both callers did before this
// table existed and is no worse -- the table can only go stale in the direction
// it started from.
//
// `Cc` needs no table: `char::is_control` *is* the `Cc` test.
//
// The exhaustive check lives in `tests/scripture_reference.rs`
// (`exactly_the_control_and_format_characters_are_deleted`), which walks all
// 1 114 112 code points against a range list generated from two independent
// implementations of the UCD. It reaches this function through
// `scripture::is_invisible`, so it covers the table for both callers.
#[rustfmt::skip]
pub(crate) fn is_format_char(c: char) -> bool {
    matches!(
        c,
        '\u{00ad}'                   // SOFT HYPHEN
        | '\u{0600}'..='\u{0605}'     // Arabic number signs
        | '\u{061c}'                 // ARABIC LETTER MARK
        | '\u{06dd}'                 // ARABIC END OF AYAH
        | '\u{070f}'                 // SYRIAC ABBREVIATION MARK
        | '\u{0890}'..='\u{0891}'     // Arabic pound and piastre marks
        | '\u{08e2}'                 // ARABIC DISPUTED END OF AYAH
        | '\u{180e}'                 // MONGOLIAN VOWEL SEPARATOR
        | '\u{200b}'..='\u{200f}'     // ZWSP, ZWNJ, ZWJ, LRM, RLM
        | '\u{202a}'..='\u{202e}'     // bidi embedding and override
        | '\u{2060}'..='\u{2064}'     // WORD JOINER and invisible operators
        | '\u{2066}'..='\u{206f}'     // bidi isolates and deprecated formats
        | '\u{feff}'                 // ZERO WIDTH NO-BREAK SPACE (BOM)
        | '\u{fff9}'..='\u{fffb}'     // interlinear annotation
        | '\u{110bd}'                // KAITHI NUMBER SIGN
        | '\u{110cd}'                // KAITHI NUMBER SIGN ABOVE
        | '\u{13430}'..='\u{1343f}'   // Egyptian hieroglyph format controls
        | '\u{1bca0}'..='\u{1bca3}'   // shorthand format controls
        | '\u{1d173}'..='\u{1d17a}'   // musical format controls
        | '\u{e0001}'                // LANGUAGE TAG
        | '\u{e0020}'..='\u{e007f}'   // tag characters
    )
}

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

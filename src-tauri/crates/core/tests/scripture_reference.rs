//! Reading a scripture reference out of free text (FR-205, and the silent
//! `None` FR-206 is built on).
//!
//! FR-205's acceptance criterion is four strings — `Yoh 3:16`, `John 3:16-18`,
//! `Kej 1:1-2:3`, `Mzm 23` — and each of the four has a test below that spells
//! its expected `ScriptureRef` out as a struct literal. No helper recomputes
//! the answer, because a helper that derived `end_chapter` from the input would
//! agree with any parser that derived it the same wrong way.
//!
//! Everything else here exists to reject a specific wrong implementation rather
//! than to agree with the current one. Named, so a later reader can check they
//! still fail:
//!
//! - completing a mixed range (`Kej 1-2:3`) by inventing `start_verse: 1`;
//! - swapping a backwards range instead of refusing it;
//! - breaking a spelling collision by the lower `book_id`;
//! - accepting chapter or verse `0`, or saturating a number above 65535;
//! - deleting the abbreviation dot instead of treating it as a separator,
//!   which folds `I.Yohanes` and `I Yohanes` apart;
//! - finding the book token by splitting at the first space, which loses
//!   `1 Raja-raja`, `Hakim-hakim` and `Kidung Agung`.
//!
//! The last two are the reason several assertions look redundant: `Yoh.` and
//! `Yoh` meet under either dot rule, and `Yohanes 3:16` parses under either
//! scan direction, so only the awkward spellings tell the implementations
//! apart.
//!
//! **The book data below is a synthetic fixture.** Book names, abbreviations
//! and the 1–66 canonical order are public facts, not imported content; the
//! ids are the ones `bible_books` uses (Kejadian 1, Hakim-hakim 7, 1 Raja-raja
//! 11, Mazmur 19, Kidung Agung 22, Yesaya/Isaiah 23, Yohanes/John 43,
//! 1–3 Yohanes 62–64, Yudas/Jude 65). Two fixtures are deliberately malformed,
//! and say so where they are defined. No verse text appears anywhere in this
//! file: FR-205 is about references, and a reference is not scripture.

use aeroworship_core::models::{
    parse_reference, parse_scripture_ref, resolve_reference, BookIndex, BookMatch, ParsedReference,
    ScriptureRef,
};

/// Indonesian names and abbreviations, one language's worth.
fn indonesian() -> BookIndex {
    BookIndex::build(
        "id",
        [
            (1u8, "Kejadian"),
            (1, "Kej"),
            (7, "Hakim-hakim"),
            (7, "Hak"),
            (11, "1 Raja-raja"),
            (11, "1Raj"),
            (19, "Mazmur"),
            (19, "Mzm"),
            (22, "Kidung Agung"),
            (22, "Kid"),
            (43, "Yohanes"),
            (43, "Yoh"),
            (62, "1 Yohanes"),
            (63, "2 Yohanes"),
            (64, "3 Yohanes"),
        ],
    )
}

/// English names and abbreviations. A separate index, built for a separate
/// language code — that is the whole of how the language is selected.
fn english() -> BookIndex {
    BookIndex::build(
        "en",
        [
            (1u8, "Genesis"),
            (1, "Gen"),
            (19, "Psalms"),
            (19, "Ps"),
            (23, "Isaiah"),
            (23, "Isa"),
            (43, "John"),
            (43, "Jn"),
            (62, "1 John"),
        ],
    )
}

/// Deliberately malformed data: `Jud` is given to both Judges (7) and Jude
/// (65), and `Jude` is Jude's *full name* as well as a made-up abbreviation of
/// Judges. Nothing in the schema prevents an imported Bible from doing this,
/// and the point of the fixture is that neither book wins — not the lower id,
/// and not the full name.
fn colliding() -> BookIndex {
    BookIndex::build(
        "en",
        [
            (7u8, "Judges"),
            (7, "Jud"),
            (7, "Jude"),
            (65, "Jude"),
            (65, "Jud"),
            (43, "John"),
        ],
    )
}

// ---------------------------------------------------------------------------
// FR-205's acceptance criterion, one test per string, expectations literal
// ---------------------------------------------------------------------------

/// FR-205 — "`Yoh 3:16` ... resolve correctly". One verse: both ends of the
/// closed interval are that verse, filled in by the parser so no consumer has
/// to reconstruct them.
#[test]
fn yoh_3_16_resolves_to_a_single_verse() {
    assert_eq!(
        parse_scripture_ref("Yoh 3:16", &indonesian()),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );
}

/// FR-205 — "`John 3:16-18` ... resolve correctly". The bare `18` after the
/// dash is a *verse* of chapter 3, not chapter 18, because the left-hand side
/// named a verse.
#[test]
fn john_3_16_18_resolves_to_a_verse_range_inside_one_chapter() {
    assert_eq!(
        parse_scripture_ref("John 3:16-18", &english()),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(18),
        })
    );
}

/// FR-205 — "`Kej 1:1-2:3` ... resolve correctly". The cross-chapter form: the
/// end chapter differs from the start chapter and both verses are set.
#[test]
fn kej_1_1_to_2_3_resolves_across_chapters() {
    assert_eq!(
        parse_scripture_ref("Kej 1:1-2:3", &indonesian()),
        Some(ScriptureRef {
            book_id: 1,
            start_chapter: 1,
            start_verse: Some(1),
            end_chapter: 2,
            end_verse: Some(3),
        })
    );
}

/// FR-205 — "`Mzm 23` ... resolve correctly". A whole chapter: both verses
/// `None`, meaning "every verse", and the end chapter equal to the start.
#[test]
fn mzm_23_resolves_to_a_whole_chapter() {
    assert_eq!(
        parse_scripture_ref("Mzm 23", &indonesian()),
        Some(ScriptureRef {
            book_id: 19,
            start_chapter: 23,
            start_verse: None,
            end_chapter: 23,
            end_verse: None,
        })
    );
}

// ---------------------------------------------------------------------------
// The shape of the record: closed interval, both ends always filled in
// ---------------------------------------------------------------------------

/// FR-205 — a whole-chapter *range*, the fifth form. Both verses stay `None`
/// and the bare number after the dash is a chapter, because the left-hand side
/// named no verse. Compare with `John 3:16-18`, where the same spelling means
/// a verse: the two together pin down which side the bare number follows.
#[test]
fn a_bare_number_after_the_dash_follows_the_left_hand_side() {
    assert_eq!(
        parse_scripture_ref("Mzm 23-24", &indonesian()),
        Some(ScriptureRef {
            book_id: 19,
            start_chapter: 23,
            start_verse: None,
            end_chapter: 24,
            end_verse: None,
        }),
        "no verse on the left, so 24 is a chapter"
    );

    assert_eq!(
        parse_scripture_ref("Mzm 23:1-6", &indonesian()),
        Some(ScriptureRef {
            book_id: 19,
            start_chapter: 23,
            start_verse: Some(1),
            end_chapter: 23,
            end_verse: Some(6),
        }),
        "a verse on the left, so 6 is a verse of the same chapter"
    );
}

/// FR-205 — a range that begins mid-chapter and ends on a whole chapter has no
/// representation in `ScriptureRef`, and no spelling of it parses. Refused,
/// not completed: reading `Kej 1-2:3` as `Kej 1:1-2:3` would be an assumption
/// about what the operator meant. Both neighbouring well-formed spellings are
/// asserted alongside so the refusal cannot come from the range machinery being
/// broken generally.
#[test]
fn a_mixed_range_is_refused_rather_than_completed() {
    let books = indonesian();

    assert_eq!(parse_reference("Kej 1-2:3"), None);
    assert_eq!(parse_scripture_ref("Kej 1-2:3", &books), None);

    assert_eq!(
        parse_scripture_ref("Kej 1-2", &books),
        Some(ScriptureRef {
            book_id: 1,
            start_chapter: 1,
            start_verse: None,
            end_chapter: 2,
            end_verse: None,
        })
    );
    assert_eq!(
        parse_scripture_ref("Kej 1:1-2:3", &books),
        Some(ScriptureRef {
            book_id: 1,
            start_chapter: 1,
            start_verse: Some(1),
            end_chapter: 2,
            end_verse: Some(3),
        })
    );
}

/// FR-205 / Appendix D — the invariant stated as a property over every form:
/// `start_verse` and `end_verse` are `None` together or `Some` together.
#[test]
fn both_verses_are_null_together_or_set_together() {
    let books = indonesian();

    for input in [
        "Yoh 3:16",
        "Yoh 3:16-18",
        "Kej 1:1-2:3",
        "Mzm 23",
        "Mzm 23-24",
        "Mzm 151:1",
        "Yoh 3",
    ] {
        let reference = parse_scripture_ref(input, &books)
            .unwrap_or_else(|| panic!("{input} is a well-formed reference"));
        assert_eq!(
            reference.start_verse.is_none(),
            reference.end_verse.is_none(),
            "{input} produced a half-open range: {reference:?}"
        );
        assert!(
            (reference.end_chapter, reference.end_verse)
                >= (reference.start_chapter, reference.start_verse),
            "{input} produced a backwards range: {reference:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The refusals (all `None`, which FR-206 turns into a full-text search)
// ---------------------------------------------------------------------------

/// FR-205 — a backwards range is a typo, and it is refused rather than
/// silently swapped: a parser that corrected it would put a passage on screen
/// that nobody asked for. Both the within-chapter and the cross-chapter
/// spelling, and the whole-chapter one. The equal-ended range is asserted too,
/// because "refuse when the end is not after the start" must not become
/// "refuse when the end is not *strictly* after the start".
#[test]
fn a_backwards_range_is_refused_not_swapped() {
    let books = indonesian();

    assert_eq!(parse_reference("Yoh 3:18-16"), None);
    assert_eq!(parse_scripture_ref("Yoh 3:18-16", &books), None);
    assert_eq!(parse_reference("Kej 2:3-1:1"), None);
    assert_eq!(parse_scripture_ref("Kej 2:3-1:1", &books), None);
    assert_eq!(parse_scripture_ref("Mzm 24-23", &books), None);

    assert_eq!(
        parse_scripture_ref("Yoh 3:16-16", &books),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        }),
        "a range of one verse runs forwards"
    );
}

/// FR-205 — chapters and verses are numbered from 1, so a `0` at either end of
/// either field is not a reference. Every position asserted separately: a check
/// that only looked at the start would pass three of these.
#[test]
fn chapter_or_verse_zero_is_refused() {
    let books = indonesian();

    for input in [
        "Yoh 0",
        "Yoh 0:1",
        "Yoh 3:0",
        "Yoh 3:16-0",
        "Yoh 0:1-3:16",
        "Mzm 0-3",
        "Mzm 1-0",
        "Kej 1:1-0:3",
    ] {
        assert_eq!(parse_reference(input), None, "{input} must not parse");
        assert_eq!(
            parse_scripture_ref(input, &books),
            None,
            "{input} must not resolve"
        );
    }
}

/// FR-205 — a number no Bible could contain is refused outright rather than
/// clamped. `65535` is accepted and `65536` is not, so the boundary is asserted
/// from both sides; a saturating read would answer chapter 65535 to all three
/// of the rejected spellings.
#[test]
fn a_number_above_65535_is_refused_not_clamped() {
    let books = indonesian();

    assert_eq!(
        parse_scripture_ref("Yoh 65535", &books),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 65535,
            start_verse: None,
            end_chapter: 65535,
            end_verse: None,
        }),
        "65535 fits and is not a validity question this parser answers"
    );

    for input in [
        "Yoh 65536",
        "Yoh 65536:1",
        "Yoh 3:65536",
        "Yoh 3:16-70000",
        "Yoh 99999999999999:1",
    ] {
        assert_eq!(parse_reference(input), None, "{input} must not parse");
        assert_eq!(
            parse_scripture_ref(input, &books),
            None,
            "{input} must not resolve"
        );
    }
}

/// FR-205 / FR-206 — a bare book name is a real thing to type and is still not
/// a passage: it has no chapter, and FR-207 could not render it without
/// inventing one. `None`, which is a full-text search and not an error.
#[test]
fn a_book_name_without_a_chapter_is_not_a_reference() {
    assert_eq!(parse_reference("Yohanes"), None);
    assert_eq!(parse_scripture_ref("Yohanes", &indonesian()), None);
    assert_eq!(parse_scripture_ref("Mazmur", &indonesian()), None);
    assert_eq!(parse_scripture_ref("Yoh.", &indonesian()), None);
}

/// FR-206 — everything that is not a reference at all answers `None`: ordinary
/// search text, a range with nothing after the dash, a doubled colon, a chapter
/// with no book in front of it, and nothing at all. None of these is an error
/// state.
#[test]
fn text_that_is_not_a_reference_answers_none() {
    let books = indonesian();

    for input in ["kasih karunia", "3:16", "", "   ", "Yoh 3:16-", "Yoh 3::16"] {
        assert_eq!(parse_reference(input), None, "{input:?} must not parse");
        assert_eq!(
            parse_scripture_ref(input, &books),
            None,
            "{input:?} must not resolve"
        );
    }
}

/// FR-205 — a verse list is not one of the supported forms, so `Yoh 3:16,18`
/// does not resolve. The comma is interior, so trailing-punctuation trimming
/// does not reach it.
#[test]
fn a_verse_list_does_not_resolve() {
    assert_eq!(parse_scripture_ref("Yoh 3:16,18", &indonesian()), None);
    assert_eq!(parse_scripture_ref("Yoh 3:16, 18", &indonesian()), None);
}

/// FR-205 — the same string at the grammar level. `parse_reference`'s own
/// contract names the verse list among the things it answers `None` to, and it
/// is a public function: a caller that resolves the token itself, or that reads
/// the range before resolving, gets `chapter 18` out of a string whose author
/// wrote no such thing. The comma is not a tail character, so the split puts it
/// at the end of the book token instead of rejecting the input.
#[test]
fn a_verse_list_is_not_a_reference_at_the_grammar_level() {
    assert_eq!(parse_reference("Yoh 3:16,18"), None);
}

/// FR-205 — nothing here is checked against verse data. Psalm 151 exists in
/// some canons and not in the one that will be imported first, and either way
/// that is a question for FR-207 and the `bible_verses` table, not for a
/// parser that has never seen a Bible. It parses, deliberately.
#[test]
fn numbers_are_not_validated_against_any_bible() {
    assert_eq!(
        parse_scripture_ref("Mzm 151:1", &indonesian()),
        Some(ScriptureRef {
            book_id: 19,
            start_chapter: 151,
            start_verse: Some(1),
            end_chapter: 151,
            end_verse: Some(1),
        })
    );
    assert_eq!(
        parse_scripture_ref("Yoh 3:999", &indonesian()),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(999),
            end_chapter: 3,
            end_verse: Some(999),
        })
    );
}

// ---------------------------------------------------------------------------
// Finding the book token: scanned from the right, not split at the first space
// ---------------------------------------------------------------------------

/// FR-205 — book names contain spaces and hyphens, so the token is found by
/// scanning back from the numeric tail. Splitting at the first space answers
/// `1` for `1 Raja-raja` and `Kidung` for `Kidung Agung`, and every one of
/// these becomes `None`.
#[test]
fn the_book_token_is_scanned_from_the_right() {
    let books = indonesian();

    assert_eq!(
        parse_scripture_ref("1 Raja-raja 2:3", &books),
        Some(ScriptureRef {
            book_id: 11,
            start_chapter: 2,
            start_verse: Some(3),
            end_chapter: 2,
            end_verse: Some(3),
        })
    );
    assert_eq!(
        parse_scripture_ref("Hakim-hakim 5:3", &books),
        Some(ScriptureRef {
            book_id: 7,
            start_chapter: 5,
            start_verse: Some(3),
            end_chapter: 5,
            end_verse: Some(3),
        })
    );
    assert_eq!(
        parse_scripture_ref("Kidung Agung 2:1-4", &books),
        Some(ScriptureRef {
            book_id: 22,
            start_chapter: 2,
            start_verse: Some(1),
            end_chapter: 2,
            end_verse: Some(4),
        })
    );
    assert_eq!(
        parse_scripture_ref("1 Yohanes 4:8", &books),
        Some(ScriptureRef {
            book_id: 62,
            start_chapter: 4,
            start_verse: Some(8),
            end_chapter: 4,
            end_verse: Some(8),
        })
    );
}

/// FR-205 — the operator types fast and omits the space. There is no space to
/// split at here at all, so the scan has to find the boundary between letters
/// and digits.
#[test]
fn a_reference_with_no_space_before_the_chapter_parses() {
    let books = indonesian();

    assert_eq!(
        parse_scripture_ref("Yoh3:16", &books),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );
    assert_eq!(
        parse_scripture_ref("Mzm23", &books),
        Some(ScriptureRef {
            book_id: 19,
            start_chapter: 23,
            start_verse: None,
            end_chapter: 23,
            end_verse: None,
        })
    );
    assert_eq!(
        parse_scripture_ref("1Yohanes1:9", &books),
        Some(ScriptureRef {
            book_id: 62,
            start_chapter: 1,
            start_verse: Some(9),
            end_chapter: 1,
            end_verse: Some(9),
        })
    );
}

/// FR-205 / Appendix D — `parse_reference` knows no books, and hands back the
/// token exactly as it was written: not lowercased, not stripped of its dot,
/// not resolved. That is what lets the grammar be tested without any data.
#[test]
fn the_book_token_is_returned_as_written() {
    assert_eq!(
        parse_reference("  Yoh.  3 : 16 "),
        Some(ParsedReference {
            book: "Yoh.",
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );
    assert_eq!(
        parse_reference("1 Raja-raja 2"),
        Some(ParsedReference {
            book: "1 Raja-raja",
            start_chapter: 2,
            start_verse: None,
            end_chapter: 2,
            end_verse: None,
        })
    );
    assert_eq!(
        parse_reference("Qwerty 1:1").map(|parsed| parsed.book),
        Some("Qwerty"),
        "the grammar does not know what is and is not a book"
    );
}

// ---------------------------------------------------------------------------
// Spellings the operator actually types
// ---------------------------------------------------------------------------

/// FR-205 — `-`, and the en and em dashes a word processor or a phone keyboard
/// substitutes for it, plus a dash with spaces around it.
#[test]
fn en_and_em_dashes_are_accepted_as_the_range_dash() {
    let books = indonesian();
    let expected = Some(ScriptureRef {
        book_id: 43,
        start_chapter: 3,
        start_verse: Some(16),
        end_chapter: 3,
        end_verse: Some(18),
    });

    assert_eq!(parse_scripture_ref("Yoh 3:16-18", &books), expected);
    assert_eq!(parse_scripture_ref("Yoh 3:16\u{2013}18", &books), expected);
    assert_eq!(parse_scripture_ref("Yoh 3:16\u{2014}18", &books), expected);
    assert_eq!(parse_scripture_ref("Yoh 3:16 - 18", &books), expected);
}

/// FR-205 — a reference typed at the end of a sentence keeps its full stop,
/// and a reference pasted out of a list keeps its comma. Both are dropped, at
/// the ends only.
#[test]
fn trailing_sentence_punctuation_is_ignored() {
    let books = indonesian();
    let expected = Some(ScriptureRef {
        book_id: 43,
        start_chapter: 3,
        start_verse: Some(16),
        end_chapter: 3,
        end_verse: Some(16),
    });

    assert_eq!(parse_scripture_ref("Yoh 3:16.", &books), expected);
    assert_eq!(parse_scripture_ref("Yoh 3:16,", &books), expected);
    assert_eq!(parse_scripture_ref("  Yoh 3:16 .  ", &books), expected);
}

/// FR-205 — surplus whitespace anywhere is ignored, including inside a book
/// name and around the colon, and including tabs.
#[test]
fn surplus_whitespace_is_ignored() {
    let books = indonesian();

    assert_eq!(
        parse_scripture_ref("   Yoh    3 :  16   ", &books),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );
    assert_eq!(
        parse_scripture_ref("\tKidung  Agung\t2:1\t", &books),
        Some(ScriptureRef {
            book_id: 22,
            start_chapter: 2,
            start_verse: Some(1),
            end_chapter: 2,
            end_verse: Some(1),
        })
    );
}

// ---------------------------------------------------------------------------
// Normalisation, applied to the stored spelling and the typed token alike
// ---------------------------------------------------------------------------

/// FR-205 — the dot is a *separator*, not a deletion. `Yoh.` meeting `Yoh`
/// works under either rule and proves nothing on its own; `I.Yohanes` is the
/// case that tells them apart, because deleting the dot leaves `iyohanes`,
/// which no ordinal rule then rescues.
#[test]
fn an_abbreviation_dot_separates_rather_than_disappears() {
    let books = indonesian();

    assert_eq!(
        parse_scripture_ref("Yoh. 3:16", &books),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );

    let first_john_4_8 = Some(ScriptureRef {
        book_id: 62,
        start_chapter: 4,
        start_verse: Some(8),
        end_chapter: 4,
        end_verse: Some(8),
    });
    assert_eq!(
        parse_scripture_ref("I.Yohanes 4:8", &books),
        first_john_4_8,
        "the dot stands in for the space in `I Yohanes`"
    );
    assert_eq!(
        parse_scripture_ref("1. Yohanes 4:8", &books),
        first_john_4_8
    );
    assert_eq!(books.lookup("I.Yohanes"), BookMatch::Unique(62));
}

/// FR-205 — a leading `I`, `II` or `III` is the book number, and every
/// spelling of it is one key: `1 Yohanes`, `1Yohanes`, `I Yohanes`,
/// `1. Yohanes`, `i yohanes`.
#[test]
fn roman_and_arabic_book_numbers_are_one_spelling() {
    let books = indonesian();

    for token in [
        "1 Yohanes",
        "1Yohanes",
        "I Yohanes",
        "1. Yohanes",
        "i yohanes",
        "1  yohanes",
    ] {
        assert_eq!(
            books.lookup(token),
            BookMatch::Unique(62),
            "{token} is 1 Yohanes"
        );
    }

    assert_eq!(books.lookup("II Yohanes"), BookMatch::Unique(63));
    assert_eq!(books.lookup("2Yohanes"), BookMatch::Unique(63));
    assert_eq!(
        books.lookup("III Yohanes"),
        BookMatch::Unique(64),
        "`iii` must not be read as `i` followed by `ii`"
    );
    assert_eq!(books.lookup("3 Yohanes"), BookMatch::Unique(64));
}

/// FR-205 — the Roman form is only a book number when a separator follows it,
/// so `Isaiah` stays `isaiah` and does not become `1saiah`. Both halves are
/// asserted: `Isaiah` resolves, and `I Saiah` — which a rule without the
/// separator requirement would fold onto it — does not.
#[test]
fn a_roman_prefix_with_no_separator_is_part_of_the_name() {
    let books = english();

    assert_eq!(
        parse_scripture_ref("Isaiah 40:31", &books),
        Some(ScriptureRef {
            book_id: 23,
            start_chapter: 40,
            start_verse: Some(31),
            end_chapter: 40,
            end_verse: Some(31),
        })
    );
    assert_eq!(books.lookup("Isaiah"), BookMatch::Unique(23));
    assert_eq!(
        books.lookup("I Saiah"),
        BookMatch::Unknown,
        "`Isaiah` must not normalise to `1 saiah`"
    );
    assert_eq!(books.lookup("1 Saiah"), BookMatch::Unknown);
}

/// FR-205 — the converse: `IYohanes`, with no separator at all, is not
/// `1 Yohanes`. Somebody typing it gets a full-text search, which is the
/// honest answer; guessing here is what would make `Isaiah` unreachable.
#[test]
fn a_book_number_run_together_with_the_name_is_not_recognised() {
    let books = indonesian();

    assert_eq!(books.lookup("IYohanes"), BookMatch::Unknown);
    assert_eq!(parse_scripture_ref("IYohanes 4:8", &books), None);
}

/// FR-205 — case is folded with Unicode rules, not ASCII ones. The fixture is
/// stored shouting and looked up quietly with an accented letter, so an
/// `eq_ignore_ascii_case` implementation answers `Unknown`.
#[test]
fn case_is_folded_beyond_ascii() {
    let books = indonesian();
    assert_eq!(books.lookup("YOHANES"), BookMatch::Unique(43));
    assert_eq!(books.lookup("yOh"), BookMatch::Unique(43));

    let accented = BookIndex::build("es", [(2u8, "ÉXODO")]);
    assert_eq!(accented.lookup("éxodo"), BookMatch::Unique(2));
    assert_eq!(
        parse_scripture_ref("Éxodo 20:3", &accented),
        Some(ScriptureRef {
            book_id: 2,
            start_chapter: 20,
            start_verse: Some(3),
            end_chapter: 20,
            end_verse: Some(3),
        })
    );
}

// ---------------------------------------------------------------------------
// The index: one language, chosen once
// ---------------------------------------------------------------------------

/// FR-205 — the language is chosen when the index is built, and the resolving
/// call has no language parameter to get wrong. An English token against the
/// Indonesian index is simply unknown, and the other way round.
#[test]
fn the_language_is_chosen_when_the_index_is_built() {
    let indonesian = indonesian();
    let english = english();

    assert_eq!(indonesian.language_code(), "id");
    assert_eq!(english.language_code(), "en");

    assert_eq!(
        parse_scripture_ref("John 3:16", &indonesian),
        None,
        "an English name is not in the Indonesian index"
    );
    assert_eq!(
        parse_scripture_ref("Yoh 3:16", &english),
        None,
        "an Indonesian abbreviation is not in the English index"
    );

    assert_eq!(
        parse_scripture_ref("Yoh 3:16", &indonesian),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );
    assert_eq!(
        parse_scripture_ref("John 3:16", &english),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        })
    );
}

/// FR-205 — a spelling two different books claim resolves to neither. Not the
/// lower id, and not the full name over the abbreviation: `Jude` is Jude's own
/// name and is still ambiguous, because the fixture also gives it to Judges.
#[test]
fn a_spelling_collision_has_no_winner() {
    let books = colliding();

    assert_eq!(books.lookup("Jud"), BookMatch::Ambiguous);
    assert_eq!(
        books.lookup("Jude"),
        BookMatch::Ambiguous,
        "a full name does not outrank an abbreviation"
    );
    assert_eq!(
        books.lookup("Judges"),
        BookMatch::Unique(7),
        "the uncontested spelling still resolves"
    );
    assert_eq!(books.lookup("John"), BookMatch::Unique(43));

    assert_eq!(
        parse_scripture_ref("Jud 1:3", &books),
        None,
        "answering book 7 here would be right half the time and never checkable"
    );
    assert_eq!(parse_scripture_ref("Jude 1:3", &books), None);
}

/// FR-205 — two rows for the *same* book are not a collision. A name that
/// equals an abbreviation, the same row imported twice, or two spellings that
/// normalise together all still resolve.
#[test]
fn two_rows_for_the_same_book_are_not_a_collision() {
    let books = BookIndex::build(
        "id",
        [(43u8, "Yoh"), (43, "yoh."), (43, "YOH"), (43, "Yoh")],
    );

    assert_eq!(books.lookup("Yoh"), BookMatch::Unique(43));
    assert_eq!(
        books.ambiguous_spellings().collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
}

/// FR-205 / FR-208 — an ambiguous spelling is a fact about imported data worth
/// reporting once, so it is listed rather than merely counted, and listed in
/// sorted order: an unordered diagnostic is one nobody can assert on.
#[test]
fn ambiguous_spellings_are_listed_in_sorted_order() {
    assert_eq!(
        colliding().ambiguous_spellings().collect::<Vec<_>>(),
        vec!["jud", "jude"]
    );
    assert_eq!(
        indonesian().ambiguous_spellings().collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "well-formed data reports nothing"
    );
}

/// FR-205 / FR-206 — ambiguous and unknown are one answer at the top level,
/// because FR-206 wants a silent fall back to full-text search for both, and
/// two different answers one level down, where the difference is actionable.
#[test]
fn ambiguous_and_unknown_collapse_only_at_the_top_level() {
    let books = colliding();

    assert_eq!(books.lookup("Jud"), BookMatch::Ambiguous);
    assert_eq!(books.lookup("Qwerty"), BookMatch::Unknown);
    assert_ne!(books.lookup("Jud"), books.lookup("Qwerty"));

    assert_eq!(parse_scripture_ref("Jud 1:3", &books), None);
    assert_eq!(parse_scripture_ref("Qwerty 1:3", &books), None);
}

/// FR-205 — "no Bible imported for this language" is told apart from "the
/// operator mistyped a book", which is worth checking before blaming the
/// operator: an empty index answers `Unknown` to perfectly good tokens too.
#[test]
fn an_empty_index_is_told_apart_from_a_mistyped_book() {
    let no_bible = BookIndex::build("de", Vec::<(u8, &str)>::new());
    assert!(no_bible.is_empty());
    assert_eq!(no_bible.language_code(), "de");
    assert_eq!(no_bible.lookup("Yoh"), BookMatch::Unknown);
    assert_eq!(parse_scripture_ref("Yoh 3:16", &no_bible), None);

    let books = indonesian();
    assert!(!books.is_empty());
    assert_eq!(books.lookup("Qwerty"), BookMatch::Unknown);
}

/// FR-205 — a spelling that normalises to nothing is dropped rather than
/// stored under an empty key, and a token that normalises to nothing matches
/// nothing.
#[test]
fn a_spelling_that_normalises_to_nothing_is_dropped() {
    let junk = BookIndex::build("id", [(5u8, "  "), (6, "."), (7, "")]);
    assert!(junk.is_empty());

    let books = indonesian();
    assert_eq!(books.lookup(""), BookMatch::Unknown);
    assert_eq!(books.lookup(" . "), BookMatch::Unknown);
}

// ---------------------------------------------------------------------------
// The two halves joined
// ---------------------------------------------------------------------------

/// FR-205 / Appendix D — `resolve_reference` attaches a book id to a range the
/// grammar already produced, and copies the range across unchanged. Asserted
/// on a value built by hand, so the two halves are shown to compose without
/// going through `parse_reference` at all.
#[test]
fn resolve_reference_attaches_a_book_id_to_an_already_parsed_range() {
    let books = indonesian();
    let parsed = ParsedReference {
        book: "Kej.",
        start_chapter: 1,
        start_verse: Some(1),
        end_chapter: 2,
        end_verse: Some(3),
    };

    assert_eq!(
        resolve_reference(&parsed, &books),
        Some(ScriptureRef {
            book_id: 1,
            start_chapter: 1,
            start_verse: Some(1),
            end_chapter: 2,
            end_verse: Some(3),
        })
    );

    let unknown = ParsedReference {
        book: "Qwerty",
        ..parsed
    };
    assert_eq!(resolve_reference(&unknown, &books), None);

    let ambiguous = ParsedReference {
        book: "Jud",
        ..parsed
    };
    assert_eq!(resolve_reference(&ambiguous, &colliding()), None);
}

// ---------------------------------------------------------------------------
// The canon is 1–66, and what falls outside it is dropped *and reported*
// ---------------------------------------------------------------------------

/// A deliberately malformed import: four rows carry ids no canon has — `0`
/// below the range, `67` one past its end, and `200` and `255` far outside —
/// alongside three sound ones. `build` must keep the sound rows and drop the
/// rest. The four are listed out of order on purpose, so a report that merely
/// echoes row order cannot pass for a sorted one.
fn out_of_canon() -> BookIndex {
    BookIndex::build(
        "id",
        [
            (200u8, "Kitab Duaratus"),
            (0, "Kitab Nol"),
            (255, "Kitab Duaratuslimapuluhlima"),
            (67, "Kitab Enampuluhtujuh"),
            (1, "Kejadian"),
            (43, "Yohanes"),
            (43, "Yoh"),
        ],
    )
}

/// FR-205 — `ScriptureRef::book_id` promises the 1–66 order `bible_books`
/// uses, so a row whose `book_id` is not in that range is dropped by `build`
/// rather than stored. Its spelling therefore names no book, and a reference
/// written with it is not a reference at all — FR-206 takes over silently.
#[test]
fn a_book_id_outside_the_canon_is_dropped() {
    let books = out_of_canon();

    for spelling in [
        "Kitab Nol",
        "Kitab Enampuluhtujuh",
        "Kitab Duaratus",
        "Kitab Duaratuslimapuluhlima",
    ] {
        assert_eq!(
            books.lookup(spelling),
            BookMatch::Unknown,
            "{spelling} carries a book_id outside 1-66 and must not resolve"
        );
        assert_eq!(
            parse_scripture_ref(&format!("{spelling} 3:16"), &books),
            None,
            "a reference built on {spelling} must fall back to full-text search"
        );
    }

    assert_eq!(
        books.lookup("Kejadian"),
        BookMatch::Unique(1),
        "the sound rows of the same import still resolve"
    );
    assert_eq!(books.lookup("Yoh"), BookMatch::Unique(43));
}

/// FR-205 — the bound is closed on both sides: `1` and `66` are books, `0` and
/// `67` are not. Asserted on all four so an off-by-one on either end fails
/// here rather than in a service that queries zero verses.
#[test]
fn the_canonical_range_is_inclusive_at_both_ends() {
    let books = BookIndex::build(
        "id",
        [
            (0u8, "Batas Bawah"),
            (1, "Kejadian"),
            (66, "Wahyu"),
            (67, "Batas Atas"),
        ],
    );

    assert_eq!(
        books.lookup("Kejadian"),
        BookMatch::Unique(1),
        "book 1 is the first book of the canon, not one below its start"
    );
    assert_eq!(
        books.lookup("Wahyu"),
        BookMatch::Unique(66),
        "book 66 is the last book of the canon, not one past its end"
    );
    assert_eq!(
        books.lookup("Batas Bawah"),
        BookMatch::Unknown,
        "book 0 is below the canon"
    );
    assert_eq!(
        books.lookup("Batas Atas"),
        BookMatch::Unknown,
        "book 67 is past the canon"
    );

    assert_eq!(
        books.rejected_book_ids().collect::<Vec<_>>(),
        vec![0, 67],
        "exactly the two out-of-range ids are reported"
    );
}

/// FR-205 — a dropped row must not poison a spelling a sound row also claims.
/// `Yoh` belongs to Yohanes (43); an import that additionally hands `Yoh` to a
/// non-existent book 200 has one usable row, not a collision, so `Yoh 3:16`
/// still resolves to 43 and `ambiguous_spellings` stays empty.
///
/// This fails the moment the range check is done *after* the row is entered
/// into the map: the broken row would land first, and the sound one would then
/// be read as a second book claiming the same spelling.
#[test]
fn a_dropped_row_does_not_make_a_sound_spelling_ambiguous() {
    for rows in [
        [(43u8, "Yoh"), (200u8, "Yoh")],
        [(200u8, "Yoh"), (43u8, "Yoh")],
    ] {
        let books = BookIndex::build("id", rows);

        assert_eq!(
            books.lookup("Yoh"),
            BookMatch::Unique(43),
            "the only row that can denote a book wins outright, in row order {rows:?}"
        );
        assert_eq!(
            books.ambiguous_spellings().collect::<Vec<_>>(),
            Vec::<&str>::new(),
            "a row that was dropped cannot collide with anything"
        );
        assert_eq!(
            parse_scripture_ref("Yoh 3:16", &books),
            Some(ScriptureRef {
                book_id: 43,
                start_chapter: 3,
                start_verse: Some(16),
                end_chapter: 3,
                end_verse: Some(16),
            })
        );
        assert_eq!(books.rejected_book_ids().collect::<Vec<_>>(), vec![200]);
    }
}

/// FR-205 / FR-208 — dropped ids are reported in ascending order, whatever
/// order the rows arrived in. An unordered diagnostic is one nobody can assert
/// on, and an import report that lists nothing is indistinguishable from a
/// sound Bible.
#[test]
fn rejected_book_ids_are_listed_in_ascending_order() {
    assert_eq!(
        out_of_canon().rejected_book_ids().collect::<Vec<_>>(),
        vec![0, 67, 200, 255],
        "reported ascending, not in the scrambled order the rows carried"
    );
}

/// FR-205 / FR-208 — one broken id typically arrives on a book's name row and
/// every abbreviation row at once, so it is named once, not once per row.
#[test]
fn rejected_book_ids_names_each_id_once() {
    let books = BookIndex::build(
        "id",
        [
            (200u8, "Kitab Duaratus"),
            (200, "Duaratus"),
            (200, "Drt"),
            (99, "Kitab Sembilanpuluhsembilan"),
            (200, "Kitab Duaratus"),
            (99, "Smb"),
            (43, "Yohanes"),
        ],
    );

    assert_eq!(
        books.rejected_book_ids().collect::<Vec<_>>(),
        vec![99, 200],
        "each id is named once however many rows carried it"
    );
    assert_eq!(books.lookup("Yohanes"), BookMatch::Unique(43));
}

/// FR-205 — sound data reports nothing, so a non-empty report is always a real
/// finding about an imported Bible.
#[test]
fn rejected_book_ids_is_empty_for_sound_data() {
    for books in [indonesian(), english(), colliding()] {
        assert_eq!(
            books.rejected_book_ids().collect::<Vec<_>>(),
            Vec::<u8>::new(),
            "well-formed data for {} reports nothing",
            books.language_code()
        );
    }
}

// ---------------------------------------------------------------------------
// Invisible characters: deleted, never separators, and trimmed at both ends
//
// Every code point named below is `Cc` or `Cf` — a character that occupies no
// space on screen. The audit case is one paste out of a word processor into the
// search box: the operator sees `Yoh 3:16`, the parser sees something else, and
// FR-206's silent fall back to full-text search leaves nothing on screen to say
// why, in the middle of a service.
//
// The category table in `scripture.rs` is written out by hand rather than taken
// from a crate (NFR-16 forbids the dependency), so this file checks it against
// the UCD exhaustively rather than trusting it — see
// `CONTROL_AND_FORMAT_RANGES`.
// ---------------------------------------------------------------------------

/// FR-205 — the three invisibles that actually arrive with imported data, plus
/// the bidi and ANSI ones that arrive with hostile data, are *deleted* from a
/// spelling rather than kept or turned into a separator. Asserted on the
/// **stored** side: a name typeset with a soft hyphen has to be found by the
/// name as anyone would type it.
///
/// A spelling carrying an invisible is not a broken row, and none of the three
/// import diagnostics may claim it is.
#[test]
fn a_stored_spelling_carrying_invisible_characters_is_found_by_the_clean_one() {
    let books = BookIndex::build(
        "id",
        [
            (1u8, "Kej\u{00ad}adian"),          // SOFT HYPHEN, from typeset data
            (19, "\u{feff}Mazmur"),             // BOM on the first line of a CSV
            (43, "Yohanes\u{200b}"),            // ZWSP, from a Word export
            (22, "Kidung Ag\u{200c}ung"),       // ZWNJ
            (7, "\u{202e}Hakim-hakim\u{202c}"), // a bidi override
            (62, "1 Yo\u{1b}hanes"),            // ESCAPE, i.e. `Cc` rather than `Cf`
            (11, "1 Raja-\u{2060}raja"),        // WORD JOINER
        ],
    );

    assert_eq!(books.lookup("Kejadian"), BookMatch::Unique(1));
    assert_eq!(books.lookup("Mazmur"), BookMatch::Unique(19));
    assert_eq!(books.lookup("Yohanes"), BookMatch::Unique(43));
    assert_eq!(
        books.lookup("Kidung Agung"),
        BookMatch::Unique(22),
        "the ZWNJ is deleted, not turned into a second word break"
    );
    assert_eq!(books.lookup("Hakim-hakim"), BookMatch::Unique(7));
    assert_eq!(books.lookup("1 Yohanes"), BookMatch::Unique(62));
    assert_eq!(books.lookup("1 Raja-raja"), BookMatch::Unique(11));

    assert_eq!(
        parse_scripture_ref("Kejadian 1:1", &books),
        Some(ScriptureRef {
            book_id: 1,
            start_chapter: 1,
            start_verse: Some(1),
            end_chapter: 1,
            end_verse: Some(1),
        })
    );

    assert_eq!(
        books.ambiguous_spellings().collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
    assert_eq!(
        books.rejected_book_ids().collect::<Vec<_>>(),
        Vec::<u8>::new()
    );
    assert_eq!(
        books.blank_spelling_book_ids().collect::<Vec<_>>(),
        Vec::<u8>::new(),
        "a spelling that merely carries an invisible still normalises to something"
    );
}

/// FR-205 — the same rule on the **typed** side, one representative per range
/// of the `Cf` table plus two from `Cc`. Each is spliced into the middle of a
/// book name, where neither the trim at the ends nor the tail scan can rescue
/// it: if the character is not deleted the token does not resolve.
#[test]
fn an_invisible_inside_a_typed_book_name_is_deleted() {
    let books = indonesian();

    for (invisible, name) in [
        ('\u{00ad}', "SOFT HYPHEN"),
        ('\u{061c}', "ARABIC LETTER MARK"),
        ('\u{180e}', "MONGOLIAN VOWEL SEPARATOR"),
        ('\u{200b}', "ZERO WIDTH SPACE"),
        ('\u{200c}', "ZERO WIDTH NON-JOINER"),
        ('\u{200d}', "ZERO WIDTH JOINER"),
        ('\u{200e}', "LEFT-TO-RIGHT MARK"),
        ('\u{202a}', "LEFT-TO-RIGHT EMBEDDING"),
        ('\u{202b}', "RIGHT-TO-LEFT EMBEDDING"),
        ('\u{202c}', "POP DIRECTIONAL FORMATTING"),
        ('\u{202d}', "LEFT-TO-RIGHT OVERRIDE"),
        ('\u{202e}', "RIGHT-TO-LEFT OVERRIDE"),
        ('\u{2060}', "WORD JOINER"),
        ('\u{2066}', "LEFT-TO-RIGHT ISOLATE"),
        ('\u{2067}', "RIGHT-TO-LEFT ISOLATE"),
        ('\u{2068}', "FIRST STRONG ISOLATE"),
        ('\u{2069}', "POP DIRECTIONAL ISOLATE"),
        ('\u{feff}', "ZERO WIDTH NO-BREAK SPACE (BOM)"),
        ('\u{110bd}', "KAITHI NUMBER SIGN"),
        ('\u{1b}', "ESCAPE"),
        ('\u{7f}', "DELETE"),
    ] {
        let token = format!("Yoh{invisible}anes");
        assert_eq!(
            books.lookup(&token),
            BookMatch::Unique(43),
            "{name} (U+{:04X}) must vanish, not separate and not remain",
            invisible as u32
        );
        assert_eq!(
            parse_scripture_ref(&format!("Yoh{invisible}anes 3:16"), &books),
            Some(ScriptureRef {
                book_id: 43,
                start_chapter: 3,
                start_verse: Some(16),
                end_chapter: 3,
                end_verse: Some(16),
            }),
            "a reference written with {name} still resolves"
        );
    }
}

/// FR-205 — a `Cc` character that is *also* `White_Space` keeps separating. The
/// whitespace test runs before the invisibility test for exactly this reason,
/// and the order is load-bearing.
///
/// **`1<TAB>Yohanes` does not tell the two orders apart** and is asserted here
/// only as the behaviour it is: deleting the tab gives `1yohanes`, which
/// `normalise_leading_ordinal` already folds onto `1 yohanes` through its digit
/// branch. The cases that do discriminate are the Roman ordinal — rescued only
/// when a separator survives, by design, so that `Isaiah` is not `1saiah` — and
/// a two-word name, which no ordinal rule touches at all.
#[test]
fn whitespace_that_is_also_a_control_character_still_separates() {
    let books = indonesian();

    for separator in ['\t', '\n', '\u{000b}', '\u{000c}', '\r', '\u{0085}'] {
        let code = separator as u32;

        assert_eq!(
            books.lookup(&format!("1{separator}Yohanes")),
            BookMatch::Unique(62),
            "1<U+{code:04X}>Yohanes is 1 Yohanes"
        );
        assert_eq!(
            books.lookup(&format!("I{separator}Yohanes")),
            BookMatch::Unique(62),
            "I<U+{code:04X}>Yohanes: deleting the separator leaves `iyohanes`, \
             which no ordinal rule rescues"
        );
        assert_eq!(
            books.lookup(&format!("Kidung{separator}Agung")),
            BookMatch::Unique(22),
            "Kidung<U+{code:04X}>Agung: deleting the separator leaves `kidungagung`"
        );
        assert_eq!(
            books.lookup(&format!("Hakim{separator}hakim")),
            BookMatch::Unknown,
            "the separator becomes a space, so it does not fold onto `hakim-hakim`"
        );
    }
}

/// FR-205 — U+00A0 and U+202F are `Zs`, not `Cf`: Rust calls them whitespace
/// and they must go on separating. U+202F sits one code point past the end of
/// the `202A..202E` bidi range, so a table widened by one swallows it.
#[test]
fn no_break_spaces_are_separators_and_not_invisibles() {
    let books = indonesian();

    for space in ['\u{00a0}', '\u{202f}'] {
        let code = space as u32;

        assert_eq!(
            books.lookup(&format!("Kidung{space}Agung")),
            BookMatch::Unique(22),
            "U+{code:04X} separates the two words rather than vanishing"
        );
        assert_eq!(
            books.lookup(&format!("I{space}Yohanes")),
            BookMatch::Unique(62),
            "U+{code:04X} separates the Roman ordinal from the name"
        );
        assert_eq!(
            parse_scripture_ref(&format!("Kidung{space}Agung 2:1"), &books),
            Some(ScriptureRef {
                book_id: 22,
                start_chapter: 2,
                start_verse: Some(1),
                end_chapter: 2,
                end_verse: Some(1),
            })
        );
    }
}

/// FR-205 — an invisible at either end of the whole input is trimmed away, so a
/// reference pasted with a BOM in front of it or a zero-width space behind it
/// resolves exactly like the clean spelling.
///
/// The trailing end is the one that was broken: a leading invisible rides
/// inside the book token and is dissolved at resolution, while a trailing one is
/// not a tail character and used to leave `parse_tail` with an empty tail.
#[test]
fn an_invisible_at_either_end_of_the_input_is_trimmed() {
    let books = indonesian();
    let expected = Some(ScriptureRef {
        book_id: 43,
        start_chapter: 3,
        start_verse: Some(16),
        end_chapter: 3,
        end_verse: Some(16),
    });

    for input in [
        "\u{feff}Yoh 3:16",
        "Yoh 3:16\u{200b}",
        "\u{200b}Yoh 3:16\u{200b}",
        "\u{feff}Yoh 3:16\u{feff}",
        "\u{202a}Yoh 3:16\u{202c}",
        "Yoh 3:16\u{00ad}",
        "Yoh 3:16\u{1b}",
    ] {
        assert_eq!(
            parse_scripture_ref(input, &books),
            expected,
            "{input:?} is `Yoh 3:16` with invisible characters at the ends"
        );
    }
}

/// FR-205 — the invisible trim and the sentence-punctuation trim are one pass,
/// so they compose in either order and with surplus whitespace around them.
#[test]
fn invisible_characters_trim_alongside_sentence_punctuation() {
    let books = indonesian();
    let expected = Some(ScriptureRef {
        book_id: 43,
        start_chapter: 3,
        start_verse: Some(16),
        end_chapter: 3,
        end_verse: Some(16),
    });

    for input in [
        "Yoh 3:16\u{200b}.",
        "Yoh 3:16.\u{200b}",
        " Yoh 3:16\u{200b} ",
        "\u{feff} Yoh 3:16 ,\u{200b}",
        "\u{200b} Yoh 3:16 ;\u{feff} ",
    ] {
        assert_eq!(
            parse_scripture_ref(input, &books),
            expected,
            "{input:?} is `Yoh 3:16` under a single combined trim"
        );
    }
}

/// FR-205 / FR-206 — input that is nothing but invisible characters is not a
/// reference. Trimming leaves an empty string, and the answer is the ordinary
/// silent `None`, not a panic and not a passage.
#[test]
fn input_that_is_entirely_invisible_is_not_a_reference() {
    let books = indonesian();

    for input in [
        "\u{feff}",
        "\u{200b}\u{200c}\u{200d}",
        "\u{feff}\u{00ad} \u{202e}",
        "\u{1b}",
        "\u{2060}\u{2066}\u{2069}",
        "\u{110bd}",
    ] {
        assert_eq!(parse_reference(input), None, "{input:?} must not parse");
        assert_eq!(
            parse_scripture_ref(input, &books),
            None,
            "{input:?} must not resolve"
        );
    }
}

/// FR-205 — the numeric tail is deliberately *not* made immune to invisibles.
/// `Yoh 3:<ZWSP>16` answers `None`, which FR-206 turns into a full-text search;
/// it never answers a passage nobody typed. The mechanism is asserted too: the
/// invisible moves the boundary scan so the colon falls into the book token,
/// and a token holding a colon resolves to no book.
///
/// The one arrangement that still works is the documented exception —
/// `Yoh <ZWSP>3:16`, where nothing but whitespace stands between the name and
/// the invisible — and it is asserted so "the tail is not immunised" cannot
/// quietly grow into "anything with an invisible in it fails".
#[test]
fn an_invisible_inside_the_numeric_tail_is_not_a_reference() {
    let books = indonesian();

    for input in [
        "Yoh 3:\u{200b}16",
        "Yoh 3\u{200b}:16",
        "Mzm 23-\u{200b}24",
        "Yoh 3:16-\u{feff}18",
        "Yoh 1\u{200b}6",
    ] {
        assert_eq!(
            parse_scripture_ref(input, &books),
            None,
            "{input:?} must fall back to full-text search rather than invent a passage"
        );
    }

    let parsed = parse_reference("Yoh 3:\u{200b}16").expect("the grammar still reads a number");
    assert_eq!(
        books.lookup(parsed.book),
        BookMatch::Unknown,
        "the colon swept into the book token survives normalisation, so nothing resolves"
    );

    assert_eq!(
        parse_scripture_ref("Yoh \u{200b}3:16", &books),
        Some(ScriptureRef {
            book_id: 43,
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        }),
        "an invisible between the name and the chapter rides inside the token and dissolves"
    );
}

/// FR-205 / FR-208 — an ambiguous spelling is handed back to a caller that will
/// print it, and normalisation has already deleted every `Cc` and `Cf`
/// character, so an ANSI escape sequence introducer and a bidi override cannot
/// reach a screen or a log through this diagnostic.
///
/// This is one class of trouble closed, not the category: the strings are still
/// unbounded import data and a caller must escape and truncate them.
#[test]
fn a_reported_ambiguous_spelling_carries_no_invisible_characters() {
    let books = BookIndex::build(
        "en",
        [
            (7u8, "Ju\u{1b}d"),
            (65, "\u{202e}Jud\u{202c}"),
            (43, "John"),
        ],
    );

    let reported = books.ambiguous_spellings().collect::<Vec<_>>();
    assert_eq!(
        reported,
        vec!["jud"],
        "both rows normalise to the same visible spelling, and it is reported once"
    );
    assert!(
        !reported
            .iter()
            .any(|spelling| spelling.chars().any(|c| c.is_control())),
        "no control character may reach a caller through this diagnostic"
    );
}

/// Unicode general categories `Cc` and `Cf`, as inclusive code point ranges.
///
/// **Not copied from `scripture.rs`.** Generated from two independent
/// implementations of the UCD and cross-checked against one another: ICU 77.1
/// as shipped in Node 22.16 (`process.versions.unicode === "16.0"`, queried
/// with `/\p{General_Category=Format}/u`) and CPython 3.12's `unicodedata`
/// (`unidata_version == "15.0.0"`). Both enumerate the same 170 `Cf` code
/// points. `Cc` is closed for all time at U+0000–U+001F and U+007F–U+009F.
///
/// Neither tool is a dependency of this project; they were the audit
/// instrument, and this literal is its result. NFR-16 keeps a Unicode property
/// crate out of a 15 MB installer, so the table in `scripture.rs` has to be
/// hand-written — which is precisely why it needs a check written from
/// somewhere other than itself.
const CONTROL_AND_FORMAT_RANGES: [(u32, u32); 23] = [
    (0x0000, 0x001f),
    (0x007f, 0x009f),
    (0x00ad, 0x00ad),
    (0x0600, 0x0605),
    (0x061c, 0x061c),
    (0x06dd, 0x06dd),
    (0x070f, 0x070f),
    (0x0890, 0x0891),
    (0x08e2, 0x08e2),
    (0x180e, 0x180e),
    (0x200b, 0x200f),
    (0x202a, 0x202e),
    (0x2060, 0x2064),
    (0x2066, 0x206f),
    (0xfeff, 0xfeff),
    (0xfff9, 0xfffb),
    (0x110bd, 0x110bd),
    (0x110cd, 0x110cd),
    (0x13430, 0x1343f),
    (0x1bca0, 0x1bca3),
    (0x1d173, 0x1d17a),
    (0xe0001, 0xe0001),
    (0xe0020, 0xe007f),
];

/// FR-205 — every Unicode code point, checked against the UCD rather than
/// against the implementation's own table.
///
/// A character is deleted from a spelling exactly when it is `Cc` or `Cf` *and*
/// is not `White_Space`; everything else either separates or stays, and either
/// way `Yo<c>h` is then not `Yoh`. Both halves of that sentence are asserted for
/// all 1 114 112 code points, so a range that is too wide fails here just as
/// loudly as one that is too narrow.
///
/// The `White_Space` exception is not a second rule: it is the branch order in
/// `normalise_spelling` seen from outside. Tab, newline, vertical tab, form
/// feed, carriage return and NEL are `Cc` and are separators, and this test is
/// the exhaustive form of that.
#[test]
fn exactly_the_control_and_format_characters_are_deleted() {
    let books = indonesian();
    let mut token = String::new();

    for code_point in 0..=0x10_ffffu32 {
        let Some(c) = char::from_u32(code_point) else {
            continue; // a surrogate, which is not a character
        };

        token.clear();
        token.push('Y');
        token.push('o');
        token.push(c);
        token.push('h');

        let is_control_or_format = CONTROL_AND_FORMAT_RANGES
            .iter()
            .any(|(first, last)| (*first..=*last).contains(&code_point));
        let expected_deleted = is_control_or_format && !c.is_whitespace();
        let was_deleted = books.lookup(&token) == BookMatch::Unique(43);

        assert_eq!(
            was_deleted,
            expected_deleted,
            "U+{code_point:04X}: deleted={was_deleted}, but the UCD says \
             Cc/Cf={is_control_or_format} and White_Space={}",
            c.is_whitespace()
        );
    }
}

// ---------------------------------------------------------------------------
// A spelling that normalises to nothing is dropped *and reported*
// ---------------------------------------------------------------------------

/// A deliberately malformed import of the second kind: one shifted CSV column,
/// so every abbreviation cell arrived blank while the names came through
/// intact. Nothing about this index is empty, ambiguous or out of canon — which
/// is the whole difficulty, and the reason `blank_spelling_book_ids` exists.
fn blank_abbreviations() -> BookIndex {
    BookIndex::build(
        "id",
        [
            (1u8, "Kejadian"),
            (1, ""),
            (19, "Mazmur"),
            (19, "   "),
            (43, "Yohanes"),
            (43, "."),
            (62, "1 Yohanes"),
            (62, "\u{feff}\u{200b}\u{00ad}"),
        ],
    )
}

/// FR-205 / FR-208 — a spelling that normalises to nothing is dropped, and the
/// book it named is reported. Without the report this import looks sound from
/// every other angle: not empty, nothing ambiguous, no id out of canon, and four
/// books quietly findable only by their full names.
#[test]
fn a_blank_spelling_is_dropped_and_the_book_is_reported() {
    let books = blank_abbreviations();

    assert_eq!(
        books.blank_spelling_book_ids().collect::<Vec<_>>(),
        vec![1, 19, 43, 62],
        "every book whose abbreviation cell arrived blank is named"
    );

    assert!(
        !books.is_empty(),
        "the names came through, so `is_empty` says nothing about this fault"
    );
    assert_eq!(
        books.ambiguous_spellings().collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
    assert_eq!(
        books.rejected_book_ids().collect::<Vec<_>>(),
        Vec::<u8>::new()
    );

    assert_eq!(books.lookup("Kejadian"), BookMatch::Unique(1));
    assert_eq!(books.lookup("Yohanes"), BookMatch::Unique(43));
    assert_eq!(
        books.lookup("Kej"),
        BookMatch::Unknown,
        "the abbreviation was never stored, so it resolves to nothing"
    );
    assert_eq!(
        books.lookup(""),
        BookMatch::Unknown,
        "and the blank key was not stored either"
    );
}

/// FR-205 — a spelling of nothing but invisible characters is blank in exactly
/// the sense the empty cell is: it normalises to nothing, so nobody could ever
/// type it. Dropped, reported, and reported even when the same book has other
/// spellings that work — the report names a row to go and look at, not a book
/// that is unreachable.
#[test]
fn a_spelling_of_only_invisible_characters_is_blank() {
    let books = BookIndex::build(
        "id",
        [
            (43u8, "Yohanes"),
            (43, "\u{200b}"),
            (19, "\u{feff}\u{00ad}\u{202e}\u{2060}"),
            (7, "\u{1b}"),
            (22, "\u{110bd}\u{180e}"),
        ],
    );

    assert_eq!(
        books.blank_spelling_book_ids().collect::<Vec<_>>(),
        vec![7, 19, 22, 43]
    );
    assert_eq!(
        books.lookup("Yohanes"),
        BookMatch::Unique(43),
        "book 43 is reported and is still reachable; the two are different facts"
    );
    assert_eq!(books.lookup("\u{200b}"), BookMatch::Unknown);
    assert_eq!(books.lookup("\u{feff}\u{00ad}"), BookMatch::Unknown);
}

/// FR-205 / FR-208 — reported in ascending order and once per book, whatever
/// order the rows arrived in and however many of them were blank. An import
/// that blanks a column blanks it for every book at once, and a per-row list
/// would be sixty-six lines saying one thing.
#[test]
fn blank_spelling_book_ids_are_ascending_and_named_once() {
    let books = BookIndex::build(
        "id",
        [
            (40u8, ""),
            (7, "  "),
            (40, "."),
            (3, "\u{feff}"),
            (7, ""),
            (40, "\u{200b}\u{00ad}"),
            (43, "Yohanes"),
        ],
    );

    assert_eq!(
        books.blank_spelling_book_ids().collect::<Vec<_>>(),
        vec![3, 7, 40],
        "ascending and deduplicated, not the scrambled order the rows carried"
    );
}

/// FR-205 — sound data reports nothing here either, so a non-empty report is
/// always a real finding. `out_of_canon` is included on purpose: it is broken in
/// the *other* way, and the two reports must not bleed into each other.
#[test]
fn blank_spelling_book_ids_is_empty_for_sound_data() {
    for books in [indonesian(), english(), colliding(), out_of_canon()] {
        assert_eq!(
            books.blank_spelling_book_ids().collect::<Vec<_>>(),
            Vec::<u8>::new(),
            "no spelling in the {} fixture normalises to nothing",
            books.language_code()
        );
    }
}

/// FR-205 — a row broken on *both* sides appears in one report, not two. The
/// canonical-id check runs first, so an id no canon has is reported as an id no
/// canon has; calling it a blank spelling as well would name a book to go and
/// look at that does not exist.
#[test]
fn a_row_broken_on_both_sides_is_reported_only_as_an_out_of_canon_id() {
    let books = BookIndex::build(
        "id",
        [
            (200u8, ""),
            (0, "\u{200b}"),
            (67, "   "),
            (255, "."),
            (43, "Yohanes"),
        ],
    );

    assert_eq!(
        books.rejected_book_ids().collect::<Vec<_>>(),
        vec![0, 67, 200, 255]
    );
    assert_eq!(
        books.blank_spelling_book_ids().collect::<Vec<_>>(),
        Vec::<u8>::new(),
        "an id outside 1-66 names no book, so there is no book to report a blank cell for"
    );
    assert_eq!(books.lookup("Yohanes"), BookMatch::Unique(43));
}

// ---------------------------------------------------------------------------
// The semicolon: trimmed at the ends like the comma, refused in the middle
// ---------------------------------------------------------------------------

/// FR-205 — one entry copied out of a pasted list keeps whichever separator
/// followed it, and `;` is trimmed exactly as `,` is. Trimming one and not the
/// other made `Yoh 3:16;` fall back to full-text search for no reason anybody
/// had written down.
#[test]
fn a_trailing_semicolon_is_trimmed_like_a_trailing_comma() {
    let books = indonesian();
    let expected = Some(ScriptureRef {
        book_id: 43,
        start_chapter: 3,
        start_verse: Some(16),
        end_chapter: 3,
        end_verse: Some(16),
    });

    for input in [
        "Yoh 3:16;",
        "Yoh 3:16 ; ",
        "  Yoh 3:16;  ",
        "Yoh 3:16;.",
        "Yoh 3:16,;",
    ] {
        assert_eq!(
            parse_scripture_ref(input, &books),
            expected,
            "{input:?} is one entry out of a pasted list"
        );
    }

    assert_eq!(
        parse_reference("Yoh 3:16;"),
        Some(ParsedReference {
            book: "Yoh",
            start_chapter: 3,
            start_verse: Some(16),
            end_chapter: 3,
            end_verse: Some(16),
        }),
        "the trim happens in the grammar, so a caller reading the range sees it too"
    );
}

/// FR-205 — a semicolon in the *middle* is still a verse list, which is not a
/// supported form. Trimming at the ends must not grow into accepting one.
#[test]
fn a_semicolon_separated_verse_list_does_not_resolve() {
    let books = indonesian();

    for input in ["Yoh 3:16;18", "Yoh 3:16; 18", "Yoh 3;16", "Mzm 23;24"] {
        assert_eq!(parse_reference(input), None, "{input:?} must not parse");
        assert_eq!(
            parse_scripture_ref(input, &books),
            None,
            "{input:?} must not resolve"
        );
    }
}

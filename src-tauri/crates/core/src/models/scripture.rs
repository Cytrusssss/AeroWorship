//! Scripture references: the wire shape, the grammar, and the book lookup
//! (FR-205, Appendix D).
//!
//! Appendix D calls `parse_scripture_ref` "pure, unit-tested". The fact that
//! `Yoh` means Yohanes is not in this file and will never be: names and
//! abbreviations live in `bible_book_names` and `bible_book_abbreviations`,
//! one row per language, and they arrive with a Bible import (FR-208). A
//! function that had to ask SQLite what `Yoh` means would not be pure, and a
//! claim wider than the thing it describes has already cost this repository
//! twice (ADR-0020, ADR-0021).
//!
//! So the work is split in two, and the split *is* the design:
//!
//! 1. [`parse_reference`] is the grammar. It knows no books at all. It cuts
//!    free text into a **book token** — the substring as written, unresolved —
//!    plus a chapter and verse range. No I/O, no SQL, no table.
//! 2. [`BookIndex`] is the lookup table, built by the caller from rows it read
//!    itself and passed in by reference. Nothing here opens a database.
//!
//! Every FR-205 acceptance case is therefore reachable without a line of SQL,
//! and "pure" is literally true of both halves rather than true of a diagram.
//!
//! Doc comments on [`ScriptureRef`] and its fields are copied verbatim into
//! `src/shared/bindings/ScriptureRef.ts` by ts-rs, so they are written for a
//! frontend reader; this crate's own reasoning is in ordinary `//` comments,
//! which are not copied.

use std::collections::{BTreeMap, BTreeSet};
use std::iter::Peekable;
use std::ops::RangeInclusive;
use std::str::Chars;

use serde::Serialize;

/// A resolved passage: one book, and a range running from one point to
/// another.
///
/// The range is closed and both ends are always filled in, so a consumer never
/// has to reconstruct a missing end. `Yoh 3:16` has both ends equal;
/// `Mzm 23` — a whole chapter — has `startChapter === endChapter` and both
/// verses `null`; `Kej 1:1-2:3` crosses chapters.
///
/// `startVerse` and `endVerse` are `null` together or set together. `null`
/// means "every verse of that chapter", so a range that begins mid-chapter and
/// ends on a whole chapter has no representation here — and no spelling of it
/// parses.
///
/// Nothing here has been checked against an actual Bible. The book exists in
/// the lookup table the reference was resolved against; the numbers are only
/// known to be at least 1 and to run forwards. Whether chapter 151 of Psalms
/// exists is a question about verse data, not about a reference.
// **Why one flat record with both ends filled in, rather than an enum.**
// The four shapes FR-205 lists (single verse, verse range, whole chapter,
// cross-chapter range) are the same closed interval seen four ways, and the
// consumer that matters is FR-207, which will turn this into one range scan
// over `bible_verses` ordered by `(chapter, verse)`. A tagged union would make
// that consumer re-derive the interval from a variant it has to match on, in
// TypeScript as well as in Rust. Filling both ends in the parser means the
// derivation happens once, here, where it is unit-tested.
//
// **The invariants are enforced where values are made, not in the type.** The
// fields are public because this is a wire record, so a constructor could not
// hold a line a struct literal walks around. Every value produced in this crate
// comes from `parse_reference`, which refuses a mixed range outright, by way of
// `resolve_reference`, which can only attach an id `BookIndex` holds — and
// `BookIndex::build` drops any row outside 1–66. So the range `book_id`'s doc
// promises the frontend is one this crate actually keeps, rather than one it
// inherits from a foreign key in a database `build` does not require.
//
// That makes the next paragraph more pressing, not less: two invariants now
// ride on the same gate. If a later item gives this type `Deserialize` so
// FR-207's `get_scripture` can take one as an argument, it must re-validate
// **both** — the both-or-neither verse pair, and the 1–66 book id. A value
// arriving from the frontend has not been through the parser, the frontend is
// not a trusted source of invariants, and a `book_id` of 200 taken straight off
// the wire selects no rows and shows a blank screen in the middle of a service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct ScriptureRef {
    /// Canonical book id, in the 1–66 order `bible_books` uses.
    pub book_id: u8,
    /// First chapter of the passage. Chapters are numbered from 1.
    pub start_chapter: u16,
    /// First verse, numbered from 1, or `null` for "from the start of the
    /// chapter".
    pub start_verse: Option<u16>,
    /// Last chapter of the passage. Never earlier than `startChapter`.
    pub end_chapter: u16,
    /// Last verse, numbered from 1, or `null` for "to the end of the chapter".
    /// `null` exactly when `startVerse` is `null`.
    pub end_verse: Option<u16>,
}

/// The book ids `bible_books` uses: 1–66, the canon in order.
//
// Named once so the bound on `ScriptureRef::book_id` is not a literal repeated
// wherever it is checked. It is a fact rather than a setting: an import that
// carries ids outside it is carrying broken data, not a wider Bible, and the
// item that ever widens the canon would be adding `bible_books` rows and a
// migration — not an importer reading someone else's interchange file.
const CANONICAL_BOOK_IDS: RangeInclusive<u8> = 1..=66;

/// What the grammar alone can tell about a reference: a range, and the text
/// that was standing where a book name should be.
///
/// `book` is the substring of the input as it was written — not lowercased,
/// not stripped of dots, not resolved. Resolving it needs a [`BookIndex`], and
/// keeping the two apart is what makes [`parse_reference`] independent of any
/// language and of any data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedReference<'a> {
    /// The book token, exactly as it appeared in the input.
    pub book: &'a str,
    /// First chapter, at least 1.
    pub start_chapter: u16,
    /// First verse, at least 1, or `None` for a whole chapter.
    pub start_verse: Option<u16>,
    /// Last chapter, never earlier than `start_chapter`.
    pub end_chapter: u16,
    /// Last verse; `None` exactly when `start_verse` is `None`.
    pub end_verse: Option<u16>,
}

/// The whole of FR-205 in one call: parse free text, resolve the book against
/// `books`, or answer `None`.
///
/// `None` covers everything FR-206 sends to full-text search: text that is not
/// a reference at all, a reference whose book is unknown to `books`, and a
/// reference whose book token is ambiguous in that language. There is no error
/// state, by design — see [`BookIndex::lookup`] for the one distinction a
/// caller may still want to make and where to make it.
pub fn parse_scripture_ref(input: &str, books: &BookIndex) -> Option<ScriptureRef> {
    resolve_reference(&parse_reference(input)?, books)
}

/// Attaches a book id to an already-parsed reference, or answers `None` when
/// `books` cannot name exactly one book for the token.
pub fn resolve_reference(parsed: &ParsedReference<'_>, books: &BookIndex) -> Option<ScriptureRef> {
    match books.lookup(parsed.book) {
        // Ambiguous and Unknown collapse to the same answer on purpose: FR-206
        // wants a silent fall back to full-text search for both. They are told
        // apart where the difference is actionable, not here — see
        // `BookIndex::ambiguous_spellings`.
        BookMatch::Ambiguous | BookMatch::Unknown => None,
        BookMatch::Unique(book_id) => Some(ScriptureRef {
            book_id,
            start_chapter: parsed.start_chapter,
            start_verse: parsed.start_verse,
            end_chapter: parsed.end_chapter,
            end_verse: parsed.end_verse,
        }),
    }
}

/// Reads free text as a reference *without knowing any book*, returning the
/// book token together with the range.
///
/// The accepted forms, where `c` is a chapter and `v` a verse:
///
/// | Input | Meaning |
/// | --- | --- |
/// | `Book c` | the whole of chapter `c` |
/// | `Book c:v` | one verse |
/// | `Book c:v-v` | verses of one chapter |
/// | `Book c:v-c:v` | a range across chapters |
/// | `Book c-c` | whole chapters |
///
/// After a dash, a bare number continues whatever the left-hand side was: a
/// verse when the left side named a verse, a chapter when it did not. Surplus
/// whitespace anywhere is ignored, invisible characters at either end — a BOM, a
/// soft hyphen, a zero-width space arriving with a paste — are dropped, and `-`,
/// `–` and `—` are all accepted as the range dash.
///
/// `None` means "not a reference", and it is not an error — FR-206 turns it
/// into a full-text search. It is answered for: no chapter at all (`Yohanes`),
/// no book token at all (`3:16`), a chapter or verse of 0, a number that no
/// Bible could contain (above 65535), a range that runs backwards
/// (`Yoh 3:18-16`, `Kej 2:3-1:1`), a range mixing a whole chapter with a verse
/// (`Kej 1-2:3`), a verse list (`Yoh 3:16,18` or `Yoh 3:16;18` — not a
/// supported form), a dot standing where a colon should (`Yoh 3.16`), and
/// anything left over after the range.
//
// **How the book token is found.** By scanning from the *right*, not by
// splitting on the first space. Book names contain spaces and hyphens —
// `1 Raja-raja`, `Kidung Agung`, `Song of Songs` — so the only reliable
// landmark is the trailing run of digits, colons and dashes. Everything before
// it is the token, whatever it looks like; deciding whether it names a book is
// `BookIndex`'s job and needs data this function does not have.
//
// **Why no chapter means no reference.** A bare book name is a real thing to
// type, but it is not a passage, and FR-207 could not render it without
// inventing a chapter. Sending it to full-text search (FR-206) is the honest
// answer and costs nothing.
pub fn parse_reference(input: &str) -> Option<ParsedReference<'_>> {
    // Sentence punctuation and invisible characters at either end are dropped
    // before anything else, so `Yoh 3:16.` typed at the end of a line and
    // `Yoh 3:16<ZWSP>` pasted out of Word both parse like `Yoh 3:16`. Only at
    // the ends: a dot *inside* the token is the abbreviation dot, and
    // `BookIndex` handles it.
    //
    // Both list separators are trimmed, not just the comma. `Yoh 3:16,` and
    // `Yoh 3:16;` are the same event -- one entry copied out of a pasted list --
    // and `is_list_separator` already declares the two to be one category for
    // the tail scan. Trimming one and not the other made `Yoh 3:16;` fail to
    // parse for no reason anybody had written down, which FR-206 turns into a
    // full-text search: the operator types a reference and nothing happens.
    //
    // **Invisible characters are trimmed by `is_invisible` -- the predicate
    // `normalise_spelling` already deletes with, not a second list.** The two
    // ends were not equally safe before. A leading one rides inside the book
    // token and is dissolved at resolution, so `<BOM>Yoh 3:16` already worked;
    // a trailing one is not a tail character, so the boundary scan below left
    // it at the end of the book token, `parse_tail` was handed an empty tail,
    // and `Yoh 3:16<ZWSP>` -- one paste out of a Word document into the search
    // box -- answered `None`. FR-206 turns that into a full-text search, so the
    // operator types a reference, gets a word search, and has nothing on screen
    // to say why, in the middle of a service. Deleted rather than treated as a
    // separator, for the reason `normalise_spelling` gives: a character nobody
    // can see must not change the answer, in either direction.
    //
    // **The ends only. The numeric tail is left alone, deliberately.**
    // `Yoh 3:<ZWSP>16` still answers `None`. Immunising the tail would mean
    // teaching `is_tail_char`, `skip_whitespace` and `take_number` to step over
    // invisibles -- three further places that would all have to agree that
    // `1<ZWSP>6` is sixteen -- and it would buy no correctness, only a kinder
    // answer to a rarer paste. An invisible *inside* the tail moves the
    // boundary scan so that digits, colons or dashes fall into the book token,
    // and those survive `normalise_spelling`: the token resolves to no book and
    // the whole reference is `None`. The one arrangement in which the token
    // still resolves is the one where nothing but whitespace and dots stands
    // between the name and the invisible -- `Yoh <ZWSP>3:16` -- and that parses
    // correctly already. What is left here is therefore `None`, the answer
    // FR-206 gives for any text this grammar cannot read, and never a passage
    // nobody asked for.
    let trimmed = input.trim_matches(|c: char| {
        c.is_whitespace() || c == '.' || is_list_separator(c) || is_invisible(c)
    });

    // A dot between two digits — `Yoh 3.16`, the continental spelling of
    // `Yoh 3:16` — is refused here rather than in `is_tail_char`, because it is
    // the *only* dot the tail scan must not swallow. Making `.` a tail
    // character would break `Yoh. 3:16` and `1. Yohanes 4:8`, where the
    // abbreviation dot sits in exactly the place the right-to-left scan would
    // then hand to the tail, leaving the tail starting on a dot and the book
    // token short of it. This grammar has no `.` separator; reading `3.16` as
    // chapter 3 verse 16 would be a guess, and letting it through as
    // `book "Yoh 3.", chapter 16` is worse — it invents a chapter nobody typed.
    if has_dot_between_digits(trimmed) {
        return None;
    }

    // Split at the last character that cannot belong to the numeric tail. The
    // tail may therefore be empty (no chapter) and the book token may not.
    let boundary = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !is_tail_char(*c))
        .map(|(index, c)| index + c.len_utf8())?;
    let (book, tail) = trimmed.split_at(boundary);

    parse_tail(book.trim_end(), tail)
}

/// Reads the numeric part and validates it, or answers `None`.
fn parse_tail<'a>(book: &'a str, tail: &str) -> Option<ParsedReference<'a>> {
    let mut chars = tail.chars().peekable();

    let start_chapter = take_number(&mut chars)?;
    let start_verse = if eat(&mut chars, is_colon) {
        Some(take_number(&mut chars)?)
    } else {
        None
    };

    let (end_chapter, end_verse) = if eat(&mut chars, is_dash) {
        let number = take_number(&mut chars)?;
        if eat(&mut chars, is_colon) {
            (number, Some(take_number(&mut chars)?))
        } else if start_verse.is_some() {
            // `John 3:16-18`: the left side named a verse, so the bare number
            // continues it inside the same chapter. This is the universal
            // reading of the form and the one FR-205's own example uses.
            (start_chapter, Some(number))
        } else {
            // `Mzm 23-24`: no verse anywhere, so the bare number is a chapter.
            (number, None)
        }
    } else {
        (start_chapter, start_verse)
    };

    skip_whitespace(&mut chars);
    if chars.next().is_some() {
        // Something in the tail was not consumed — `Yoh 3:16-`, `Yoh 3::16`, or
        // the `,18` of a verse list.
        return None;
    }

    if start_chapter == 0 || end_chapter == 0 {
        return None;
    }
    if start_verse == Some(0) || end_verse == Some(0) {
        return None;
    }
    // A whole-chapter start with a verse end (`Kej 1-2:3`), or the reverse.
    // Refused rather than guessed: reading it as `Kej 1:1-2:3` would be an
    // assumption about what the operator meant, and `ScriptureRef` is defined
    // so the shape cannot exist.
    if start_verse.is_none() != end_verse.is_none() {
        return None;
    }
    // Backwards ranges are refused, not silently swapped. `Yoh 3:18-16` is a
    // typo, and a parser that corrected it would put a passage on the screen
    // that nobody asked for; FR-206's fallback shows the operator their own
    // text instead. `None` sorts before `Some` here, which is harmless: by the
    // check above the two ends are both `None` or both `Some`.
    if (end_chapter, end_verse) < (start_chapter, start_verse) {
        return None;
    }

    Some(ParsedReference {
        book,
        start_chapter,
        start_verse,
        end_chapter,
        end_verse,
    })
}

/// Characters that can appear in the numeric tail of a reference.
//
// **Why the list separators are here even though the grammar rejects them.**
// `,` and `;` are not part of any accepted form — `Yoh 3:16,18` is a verse
// list, and this parser reads one passage. But they do turn up *inside the
// tail* of text an operator pastes, and a character the right-to-left scan
// stops at is a character that ends the book token: without them the split of
// `Yoh 3:16,18` is `book "Yoh 3:16,"` plus tail `18`, and the parser answers
// "chapter 18" about a string that names no chapter 18.
//
// Admitting them to the tail and letting `parse_tail` refuse them as
// unconsumed input is the smaller of the two honest fixes. The alternative — an
// early "input contains a comma, give up" test — would be a second rejection
// rule sitting next to the first, and it would have to explain why it does not
// contradict the trailing-punctuation trim that *accepts* `Yoh 3:16,`. This way
// there is one rule to know, the same one that already refuses `Yoh 3::16`:
// whatever the grammar cannot consume, the reference is not.
fn is_tail_char(c: char) -> bool {
    c.is_ascii_digit() || is_colon(c) || is_dash(c) || is_list_separator(c) || c.is_whitespace()
}

/// The punctuation that separates entries in a pasted list of references.
fn is_list_separator(c: char) -> bool {
    matches!(c, ',' | ';')
}

/// Whether `text` contains a dot flanked by digits, as in `Yoh 3.16`.
fn has_dot_between_digits(text: &str) -> bool {
    let mut chars = text.chars().peekable();
    let mut previous_is_digit = false;

    while let Some(c) = chars.next() {
        if c == '.' && previous_is_digit && chars.peek().is_some_and(|next| next.is_ascii_digit()) {
            return true;
        }
        previous_is_digit = c.is_ascii_digit();
    }

    false
}

fn is_colon(c: char) -> bool {
    c == ':'
}

/// `-`, plus the en and em dashes a word processor or a phone keyboard
/// substitutes for it.
fn is_dash(c: char) -> bool {
    matches!(c, '-' | '\u{2013}' | '\u{2014}')
}

fn skip_whitespace(chars: &mut Peekable<Chars<'_>>) {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
}

/// Consumes the next non-whitespace character if it satisfies `wanted`.
fn eat(chars: &mut Peekable<Chars<'_>>, wanted: fn(char) -> bool) -> bool {
    skip_whitespace(chars);
    if chars.peek().copied().is_some_and(wanted) {
        chars.next();
        true
    } else {
        false
    }
}

/// Reads a run of ASCII digits.
///
/// `None` for no digits at all, and `None` for a run that would not fit a
/// `u16`. That ceiling is the one place a very long number is dealt with: the
/// value is checked against it on every digit, so nothing here can overflow
/// however many digits arrive, and `Yoh 99999999999999:1` simply is not a
/// reference. The largest chapter in any Bible is 150 and the largest verse
/// 176, so refusing above 65535 refuses nothing real.
fn take_number(chars: &mut Peekable<Chars<'_>>) -> Option<u16> {
    skip_whitespace(chars);

    let mut value: u32 = 0;
    let mut any = false;
    while let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
        chars.next();
        any = true;
        value = value * 10 + digit;
        if value > u32::from(u16::MAX) {
            return None;
        }
    }

    if !any {
        return None;
    }
    u16::try_from(value).ok()
}

/// Every spelling of a book name in **one** language, and the book each one
/// points at.
///
/// The caller builds it from `bible_book_names` and `bible_book_abbreviations`
/// and hands it to the parser; the parser never reads a table itself. That is
/// what keeps [`parse_scripture_ref`] pure.
//
// **Why a prepared index and not a `(language_code, rows)` pair of arguments.**
// The language is chosen once, at construction, and the resolving call has no
// language parameter at all. A caller therefore cannot pass Indonesian rows
// with an English language code — the mistake has no way to be expressed. The
// alternative shape, `resolve(token, language_code, rows)`, makes that mistake
// available at every call site and detectable at none of them: it would resolve
// `John` against Indonesian rows and answer `None`, which FR-206 turns into a
// full-text search, which looks like a user typo. A silent wrong answer at the
// one place FR-205 is supposed to be exact.
//
// **Why it holds the language code it will never match on.** Lookup does not
// consult it — the rows were already filtered by the caller's `WHERE
// language_code = ?`. It is stored so the index can say which language it
// speaks in a diagnostic, and so a caller holding two of them cannot mix them
// up while logging.
#[derive(Debug, Clone)]
pub struct BookIndex {
    language_code: String,
    // Sorted, not hashed. There are a few hundred spellings, so the difference
    // in lookup cost is not measurable, and a `BTreeMap` makes
    // `ambiguous_spellings` deterministic — an unordered diagnostic is a
    // diagnostic nobody can assert on.
    spellings: BTreeMap<String, Entry>,
    // Sound data leaves this empty, and an empty `BTreeSet` allocates nothing,
    // so remembering what was thrown away costs a healthy index one pointer's
    // worth of struct.
    rejected_book_ids: BTreeSet<u8>,
    // The other half of the same promise, for the other reason a row is
    // dropped. Same shape, same cost, and kept apart because the two are not
    // the same fault: one is data that names no book, the other a book named
    // by nothing typeable.
    blank_spelling_book_ids: BTreeSet<u8>,
}

/// What one normalised spelling points at inside a [`BookIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Unique(u8),
    Ambiguous,
}

/// The answer to "which book is this token?".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookMatch {
    /// Exactly one book in this language is spelled that way.
    Unique(u8),
    /// Two or more different books share this spelling in this language, so
    /// the reference cannot be resolved without guessing.
    Ambiguous,
    /// No book in this language is spelled that way.
    Unknown,
}

impl BookIndex {
    /// Builds the index for one language from `(book_id, spelling)` pairs.
    ///
    /// Feed it both the names and the abbreviations of that language; the two
    /// tables are separate relations but they answer the same question, and a
    /// full name is not treated as outranking an abbreviation. Order does not
    /// matter and repeats are harmless.
    ///
    /// **A pair whose `book_id` falls outside 1–66 is dropped.** There are 66
    /// books and nothing else is one, so such a row can only ever name a
    /// passage that does not exist; keeping it would let a `ScriptureRef` carry
    /// an id its own contract rules out. Dropped ids are remembered — ask
    /// [`BookIndex::rejected_book_ids`], which is empty for sound data.
    ///
    /// The language code is recorded, not verified: BCP-47 has no short list of
    /// valid values, and this crate is not the place to invent one. A code no
    /// Bible was ever imported for simply produces an index with no spellings
    /// in it, so every reference resolves to `None` and FR-206 takes over. Ask
    /// [`BookIndex::is_empty`] to tell that case apart from a mistyped book.
    ///
    /// Spellings are matched case-insensitively, ignoring dots and surplus
    /// whitespace and deleting invisible characters, with a leading
    /// `I`/`II`/`III` read as `1`/`2`/`3` — so a stored `1 Yohanes` is found by
    /// `1 Yohanes`, `1Yohanes`, `I Yohanes` and `i yohanes` alike.
    ///
    /// **A pair whose spelling normalises to nothing is dropped as well**, and
    /// the book it named is remembered the same way — ask
    /// [`BookIndex::blank_spelling_book_ids`]. A blank abbreviation column, or
    /// a cell holding only punctuation, cannot be typed and so cannot resolve.
    ///
    /// **`build` enforces no size limit whatever, and the caller must.** It
    /// reads the whole iterator, and it keeps one owned `String` per distinct
    /// normalised spelling for as long as the index lives; there is no cap on
    /// how many rows may arrive and none on how long one spelling may be. A
    /// real language has a few hundred spellings of at most about thirty
    /// characters, so nothing here needs a limit in order to work — but the
    /// shape of the input is a fact about the file the importer read, not about
    /// this function, and a limit invented here would be a number with no
    /// evidence behind it that a caller could not raise. **FR-208 must cap the
    /// row count and the per-spelling length before calling**, along with
    /// everything else it decides about a file it did not write. Three kinds of
    /// bad row are refused below; size is deliberately not a fourth, and this
    /// paragraph exists so that reads as a decision rather than as an omission.
    pub fn build<S, I>(language_code: &str, spellings: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = (u8, S)>,
    {
        let mut index = BTreeMap::new();
        let mut rejected_book_ids = BTreeSet::new();
        let mut blank_spelling_book_ids = BTreeSet::new();

        for (book_id, spelling) in spellings {
            // Refused the same way, and for the same kind of reason, as a
            // spelling that normalises to nothing: the row cannot denote a
            // book, so the only thing storing it could achieve is putting an
            // id into a `ScriptureRef` that FR-207 will query to zero rows —
            // a blank screen with no error anywhere.
            //
            // The check belongs *here* rather than in some caller because
            // `build` is public and takes an iterator from anywhere. The
            // foreign keys on `bible_book_names` and `bible_book_abbreviations`
            // do keep a bad id out of SQLite, and SETUP-04 turns
            // `foreign_keys` on — but that protection lives in a layer this
            // function does not require, and a guarantee borrowed from a layer
            // below is not a guarantee this API may advertise.
            if !CANONICAL_BOOK_IDS.contains(&book_id) {
                rejected_book_ids.insert(book_id);
                continue;
            }
            // Recorded, not merely skipped, for the reason
            // `rejected_book_ids` gives: a drop nobody can see turns a
            // half-broken Bible into one that looks sound. An abbreviation
            // column that arrived blank on forty of sixty-six books — one
            // shifted CSV column — otherwise leaves `is_empty` false, both
            // other diagnostics empty, and forty books findable only by their
            // full names.
            //
            // The *book* is reported, not the string. A spelling that
            // normalises to nothing has no key to report — that is precisely
            // why it was dropped — so the only text left to hand back would be
            // the raw cell: untrusted import data of unbounded length (see
            // `ambiguous_spellings`), and by construction a cell holding
            // nothing legible anyway. The `book_id` is a `u8` the check above
            // has already bounded to 1–66, so it is safe to print, and it is
            // the thing FR-208 can act on — it names the row to go and look at.
            let Some(key) = normalise_spelling(spelling.as_ref()) else {
                blank_spelling_book_ids.insert(book_id);
                continue;
            };
            index
                .entry(key)
                // Two rows for the *same* book — a name that equals an
                // abbreviation, or the same row twice — are not a collision.
                // Two different books are, and then neither wins: picking the
                // lower id would answer `Judges` to somebody who typed `Jud`
                // meaning `Jude`, and be right half the time with no way for
                // anyone to notice.
                .and_modify(|entry| {
                    if *entry != Entry::Unique(book_id) {
                        *entry = Entry::Ambiguous;
                    }
                })
                .or_insert(Entry::Unique(book_id));
        }

        Self {
            language_code: language_code.to_owned(),
            spellings: index,
            rejected_book_ids,
            blank_spelling_book_ids,
        }
    }

    /// The BCP-47 code this index was built for.
    pub fn language_code(&self) -> &str {
        &self.language_code
    }

    /// Whether the index holds no spellings at all — no Bible has been
    /// imported for this language.
    ///
    /// Worth checking before blaming the operator's typing: an empty index
    /// answers [`BookMatch::Unknown`] to every token, including perfectly good
    /// ones.
    pub fn is_empty(&self) -> bool {
        self.spellings.is_empty()
    }

    /// Resolves one book token.
    ///
    /// The token is matched after the same normalisation the stored spellings
    /// went through, so case, abbreviation dots, doubled spaces and Roman
    /// versus Arabic book numbers do not have to agree with the data.
    //
    // **Why the caller can see the difference between ambiguous and unknown,
    // even though FR-205 does not need it.** `parse_scripture_ref` collapses
    // both to `None` because FR-206 demands exactly that, silently. But the
    // two say different things about the *data*: unknown is an operator typing
    // something that is not a book, which is a non-event, while ambiguous is
    // two books claiming one spelling in one language — nothing in the schema
    // prevents it (`bible_book_abbreviations` is keyed by
    // `(book_id, language_code, abbreviation)`), and it means an imported
    // Bible carries a spelling no operator can ever use. Collapsing them at
    // this level too would delete that signal in the one place it can still be
    // seen.
    pub fn lookup(&self, token: &str) -> BookMatch {
        let Some(key) = normalise_spelling(token) else {
            return BookMatch::Unknown;
        };
        match self.spellings.get(&key) {
            Some(Entry::Unique(book_id)) => BookMatch::Unique(*book_id),
            Some(Entry::Ambiguous) => BookMatch::Ambiguous,
            None => BookMatch::Unknown,
        }
    }

    /// Every normalised spelling that more than one book claims, in sorted
    /// order.
    ///
    /// Empty for well-formed data. A non-empty result is a fact about an
    /// imported Bible, not about anything an operator typed, and it is worth
    /// surfacing once at import time (FR-208) rather than once per keystroke:
    /// each entry is a spelling that will silently never resolve.
    ///
    /// **The strings are import data, and they are not trusted.** Each one is a
    /// cell of the file [`BookIndex::build`] was fed, lowercased and with its
    /// separators tidied — nothing more. `build` neither shortens it nor gives
    /// it a length limit, so a single entry may be as long as the file allowed.
    /// A caller that puts these on a screen or in a log **must escape and
    /// truncate them at that boundary**; this crate cannot do it, because it
    /// does not know which boundary that is, and an escape correct for a
    /// terminal is wrong for HTML.
    ///
    /// Normalisation does delete every control and format character (`Cc` and
    /// `Cf`), so an ANSI escape sequence and a bidi override cannot reach a
    /// caller through here. That closes one class of trouble, not the category:
    /// the length is still unbounded, and a spelling may legitimately be in any
    /// script and any writing direction.
    pub fn ambiguous_spellings(&self) -> impl Iterator<Item = &str> {
        self.spellings
            .iter()
            .filter(|(_, entry)| matches!(entry, Entry::Ambiguous))
            .map(|(spelling, _)| spelling.as_str())
    }

    /// Every out-of-range `book_id` [`BookIndex::build`] refused, in ascending
    /// order.
    ///
    /// Empty for well-formed data. Like [`BookIndex::ambiguous_spellings`],
    /// this is a fact about an imported Bible rather than about anything an
    /// operator typed, and it belongs in a report shown once at import time
    /// (FR-208) instead of once per keystroke: every row carrying such an id
    /// was dropped, so the spellings on those rows resolve to nothing and the
    /// book behind them is unreachable however it is typed. Dropping them
    /// without a way to say so would leave a half-broken Bible looking exactly
    /// like a sound one.
    pub fn rejected_book_ids(&self) -> impl Iterator<Item = u8> + '_ {
        // Deduplicated rather than listed per row: the actionable fact is
        // "this import mentions book 200", and one broken id typically arrives
        // on a name row and every abbreviation row of the same book at once.
        self.rejected_book_ids.iter().copied()
    }

    /// Every book that [`BookIndex::build`] found a blank spelling for, in
    /// ascending order.
    ///
    /// Empty for well-formed data. This is the counterpart of
    /// [`BookIndex::rejected_book_ids`] for the other way a row is dropped: the
    /// id named a real book, but the spelling normalised to nothing — an empty
    /// abbreviation cell, or one holding only punctuation or invisible
    /// characters — so nothing an operator could type would ever have found it.
    ///
    /// A book appears here when *any* of its rows was blank, which alone does
    /// not mean the book is unreachable: usually one of its spellings is
    /// missing while the others still work, and a report should say that rather
    /// than call the import broken. Ask [`BookIndex::lookup`] for the spelling
    /// that matters to tell the two cases apart.
    ///
    /// Ids only, never the offending text. A blank spelling has no normalised
    /// form to hand back in the first place, and the raw cell would carry the
    /// untrusted-input problem [`BookIndex::ambiguous_spellings`] describes for
    /// no gain: the actionable fact is which book to go and look at.
    pub fn blank_spelling_book_ids(&self) -> impl Iterator<Item = u8> + '_ {
        // Deduplicated for the reason `rejected_book_ids` gives, plus one of
        // its own: an import that blanks a column blanks it for every book at
        // once, and a per-row list would be sixty-six lines saying one thing.
        self.blank_spelling_book_ids.iter().copied()
    }
}

/// Reduces a book name, an abbreviation or a typed token to the single form
/// they are compared in.
///
/// `None` when nothing is left — the empty string, spaces, a lone dot.
//
// Applied to both sides, always. That symmetry is the point: every question
// about what the parser tolerates ("does `Yoh.` work?", "does `1Yoh` match
// `1 Yohanes`'s abbreviation `1Yoh`?") becomes a question about this one
// function, instead of a rule that has to be remembered in two places and
// eventually is not.
//
// What it does, and why each part:
//
// * **Lowercase**, Unicode-aware rather than ASCII-only. Nothing in this crate
//   restricts the language to Indonesian and English, and the two that FR-205
//   names are simply the ones a Bible will be imported for first.
// * **A dot is a separator, not a deletion.** `Yoh.` and `Yoh` have to meet,
//   and so do `I.Yohanes` and `I Yohanes`; deleting the dot would fold the
//   second pair into `iyohanes` versus `i yohanes` and lose it.
// * **Whitespace runs collapse to one space**, so `1  Yohanes` is `1 Yohanes`.
// * **Characters that render as nothing are deleted**, not turned into a
//   separator -- see `is_invisible`. Deleting is what makes the fix work:
//   `Kej<SHY>adian` from typeset data has to become `kejadian`, and turning the
//   soft hyphen into a space would only move the failure to `kej adian`.
//   Whitespace is tested first, so the C0 characters that *are* whitespace
//   (tab, newline, vertical tab, form feed, carriage return) keep separating
//   rather than vanishing.
// * **A leading `I`, `II` or `III` becomes `1`, `2` or `3`**, and a leading
//   digit run is separated from the name by exactly one space. That is what
//   makes `1 Yohanes`, `1Yohanes`, `I Yohanes` and `1. Yohanes` one key. The
//   Roman form is only recognised when a separator follows it, so `Isaiah`
//   stays `isaiah` rather than becoming `1saiah`.
fn normalise_spelling(spelling: &str) -> Option<String> {
    let mut key = String::with_capacity(spelling.len());

    for c in spelling.chars() {
        if c.is_whitespace() || c == '.' {
            if !key.is_empty() && !key.ends_with(' ') {
                key.push(' ');
            }
            continue;
        }
        if is_invisible(c) {
            continue;
        }
        key.extend(c.to_lowercase());
    }
    while key.ends_with(' ') {
        key.pop();
    }

    if key.is_empty() {
        return None;
    }
    Some(normalise_leading_ordinal(&key).unwrap_or(key))
}

/// Whether `c` occupies no space on screen and carries no meaning for a
/// reference: Unicode general categories `Cc` (control) and `Cf` (format).
//
// **Why these are deleted rather than kept.** A character nobody can see is a
// character nobody can type, so keeping one in a key makes that key unreachable
// -- and unreachable *silently*, because an unresolved token is `None`, and
// `None` is FR-206 falling back to full-text search with no diagnostic
// anywhere. `ambiguous_spellings` would be empty, `rejected_book_ids` would be
// empty, `is_empty` would be false, and the book would be gone. The realistic
// arrivals are all `Cf`: a BOM on the first line of an import (FR-208), a soft
// hyphen from typeset data, a zero-width space from a Word export.
//
// **Why `Cf` is written out instead of pulled from a crate.** A Unicode
// property lookup would be a new dependency against a 15 MB installer
// (NFR-16), for a property whose entire assigned set is the two dozen ranges
// below. This is a snapshot of Unicode 16.0. A format character added in a
// later Unicode would pass through, which is exactly the behaviour this
// function replaces and no worse -- the table can only go stale in the
// direction it started from.
//
// `Cc` needs no table: `char::is_control` *is* the `Cc` test. `White_Space`
// overlaps both categories in one direction only (tab is `Cc`; U+00A0 and
// U+202F are neither), which is why the caller tests whitespace first and this
// function is never asked about a separator.
fn is_invisible(c: char) -> bool {
    c.is_control() || is_format_char(c)
}

/// Unicode general category `Cf`, as of Unicode 16.0.
#[rustfmt::skip]
fn is_format_char(c: char) -> bool {
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

/// Rewrites the `1`/`I` in `1 Yohanes` into one canonical form, or `None` if
/// the spelling does not start with a book number.
fn normalise_leading_ordinal(key: &str) -> Option<String> {
    let digits = key.find(|c: char| !c.is_ascii_digit()).unwrap_or(key.len());
    if digits > 0 {
        // A token that is nothing but digits cannot be a book name, and
        // splitting it would produce a trailing space. Left alone; it will not
        // match anything, which is the right answer.
        let rest = key[digits..].trim_start();
        if rest.is_empty() {
            return None;
        }
        return Some(format!("{} {rest}", &key[..digits]));
    }

    // Longest first: `iii` must not be read as `i` followed by `ii`.
    for (roman, arabic) in [("iii", '3'), ("ii", '2'), ("i", '1')] {
        let Some(rest) = key
            .strip_prefix(roman)
            .and_then(|rest| rest.strip_prefix(' '))
        else {
            continue;
        };
        if !rest.is_empty() {
            return Some(format!("{arabic} {rest}"));
        }
    }

    None
}

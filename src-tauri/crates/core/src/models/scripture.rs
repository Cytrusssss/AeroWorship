use std::collections::{BTreeMap, BTreeSet};
use std::iter::Peekable;
use std::ops::RangeInclusive;
use std::str::Chars;

use serde::Serialize;

use super::text::is_format_char;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct ScriptureRef {
    pub book_id: u8,
    pub start_chapter: u16,
    pub start_verse: Option<u16>,
    pub end_chapter: u16,
    pub end_verse: Option<u16>,
}

const CANONICAL_BOOK_IDS: RangeInclusive<u8> = 1..=66;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedReference<'a> {
    pub book: &'a str,
    pub start_chapter: u16,
    pub start_verse: Option<u16>,
    pub end_chapter: u16,
    pub end_verse: Option<u16>,
}

pub fn parse_scripture_ref(input: &str, books: &BookIndex) -> Option<ScriptureRef> {
    resolve_reference(&parse_reference(input)?, books)
}

pub fn resolve_reference(parsed: &ParsedReference<'_>, books: &BookIndex) -> Option<ScriptureRef> {
    match books.lookup(parsed.book) {
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

pub fn parse_reference(input: &str) -> Option<ParsedReference<'_>> {
    let trimmed = input.trim_matches(|c: char| {
        c.is_whitespace() || c == '.' || is_list_separator(c) || is_invisible(c)
    });

    if has_dot_between_digits(trimmed) {
        return None;
    }

    let boundary = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !is_tail_char(*c))
        .map(|(index, c)| index + c.len_utf8())?;
    let (book, tail) = trimmed.split_at(boundary);

    parse_tail(book.trim_end(), tail)
}

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
            (start_chapter, Some(number))
        } else {
            (number, None)
        }
    } else {
        (start_chapter, start_verse)
    };

    skip_whitespace(&mut chars);
    if chars.next().is_some() {
        return None;
    }

    if start_chapter == 0 || end_chapter == 0 {
        return None;
    }
    if start_verse == Some(0) || end_verse == Some(0) {
        return None;
    }
    if start_verse.is_none() != end_verse.is_none() {
        return None;
    }
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

fn is_tail_char(c: char) -> bool {
    c.is_ascii_digit() || is_colon(c) || is_dash(c) || is_list_separator(c) || c.is_whitespace()
}

fn is_list_separator(c: char) -> bool {
    matches!(c, ',' | ';')
}

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

fn is_dash(c: char) -> bool {
    matches!(c, '-' | '\u{2013}' | '\u{2014}')
}

fn skip_whitespace(chars: &mut Peekable<Chars<'_>>) {
    while chars.peek().is_some_and(|c| c.is_whitespace()) {
        chars.next();
    }
}

fn eat(chars: &mut Peekable<Chars<'_>>, wanted: fn(char) -> bool) -> bool {
    skip_whitespace(chars);
    if chars.peek().copied().is_some_and(wanted) {
        chars.next();
        true
    } else {
        false
    }
}

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

#[derive(Debug, Clone)]
pub struct BookIndex {
    language_code: String,
    spellings: BTreeMap<String, Entry>,
    rejected_book_ids: BTreeSet<u8>,
    blank_spelling_book_ids: BTreeSet<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    Unique(u8),
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookMatch {
    Unique(u8),
    Ambiguous,
    Unknown,
}

impl BookIndex {
    pub fn build<S, I>(language_code: &str, spellings: I) -> Self
    where
        S: AsRef<str>,
        I: IntoIterator<Item = (u8, S)>,
    {
        let mut index = BTreeMap::new();
        let mut rejected_book_ids = BTreeSet::new();
        let mut blank_spelling_book_ids = BTreeSet::new();

        for (book_id, spelling) in spellings {
            if !CANONICAL_BOOK_IDS.contains(&book_id) {
                rejected_book_ids.insert(book_id);
                continue;
            }
            let Some(key) = normalise_spelling(spelling.as_ref()) else {
                blank_spelling_book_ids.insert(book_id);
                continue;
            };
            index
                .entry(key)
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

    pub fn language_code(&self) -> &str {
        &self.language_code
    }

    pub fn is_empty(&self) -> bool {
        self.spellings.is_empty()
    }

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

    pub fn ambiguous_spellings(&self) -> impl Iterator<Item = &str> {
        self.spellings
            .iter()
            .filter(|(_, entry)| matches!(entry, Entry::Ambiguous))
            .map(|(spelling, _)| spelling.as_str())
    }

    pub fn rejected_book_ids(&self) -> impl Iterator<Item = u8> + '_ {
        self.rejected_book_ids.iter().copied()
    }

    pub fn blank_spelling_book_ids(&self) -> impl Iterator<Item = u8> + '_ {
        self.blank_spelling_book_ids.iter().copied()
    }
}

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

fn is_invisible(c: char) -> bool {
    c.is_control() || is_format_char(c)
}

fn normalise_leading_ordinal(key: &str) -> Option<String> {
    let digits = key.find(|c: char| !c.is_ascii_digit()).unwrap_or(key.len());
    if digits > 0 {
        let rest = key[digits..].trim_start();
        if rest.is_empty() {
            return None;
        }
        return Some(format!("{} {rest}", &key[..digits]));
    }

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

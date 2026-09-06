use serde::Serialize;

use super::template::{Rect, Typography};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]

pub struct SlideSplit {
    pub slides: Vec<String>,
    pub font_size: f64,
    pub overflows: bool,
}

pub fn split_slides(content: &str, text_box: &Rect, typography: &Typography) -> SlideSplit {
    let lines: Vec<&str> = split_lines(content);

    let declared = match typography.max_lines {
        0 => usize::MAX,
        stated => usize::from(stated),
    };
    let wanted = declared.min(lines.len());

    let font_size = shrink_to_fit(wanted, text_box.h, typography);

    let fitted = lines_that_fit(text_box.h, font_size, typography.line_height);
    let capacity = fitted.min(declared);

    let overflows = capacity == 0 && lines.iter().any(|line| !is_blank(line));
    let per_slide = capacity.max(1);

    let mut slides: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        while index < lines.len() && is_blank(lines[index]) {
            index += 1;
        }
        if index >= lines.len() {
            break;
        }
        let end = index.saturating_add(per_slide).min(lines.len());
        let mut slide = &lines[index..end];
        while let Some((last, rest)) = slide.split_last() {
            if is_blank(last) {
                slide = rest;
            } else {
                break;
            }
        }
        slides.push(slide.join("\n"));
        index = end;
    }

    if slides.is_empty() {
        slides.push(String::new());
    }

    SlideSplit {
        slides,
        font_size,
        overflows,
    }
}

fn shrink_to_fit(wanted: usize, height: f64, typography: &Typography) -> f64 {
    let size = typography.size;
    if wanted == 0 || lines_that_fit(height, size, typography.line_height) >= wanted {
        return size;
    }

    let exact = height / (wanted as f64 * typography.line_height);
    if !exact.is_finite() {
        return size;
    }

    exact.max(typography.min_size).min(size)
}

fn lines_that_fit(height: f64, size: f64, line_height: f64) -> usize {
    let line_box = size * line_height;
    let usable = height.is_finite()
        && height > 0.0
        && size.is_finite()
        && size > 0.0
        && line_height.is_finite()
        && line_height > 0.0
        && line_box.is_finite()
        && line_box > 0.0;
    if !usable {
        return 0;
    }
    let fit = height / line_box;
    fit.floor().max(0.0) as usize
}

fn split_lines(content: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut rest = content;

    while let Some((at, separator)) = rest.char_indices().find(|&(_, c)| is_segment_break(c)) {
        let (line, tail) = rest.split_at(at);
        lines.push(line);
        let width = if tail.starts_with("\r\n") {
            2
        } else {
            separator.len_utf8()
        };
        rest = &tail[width..];
    }

    if !rest.is_empty() {
        lines.push(rest);
    }
    lines
}

#[rustfmt::skip]
fn is_segment_break(c: char) -> bool {
    matches!(
        c,
        '\u{000a}'   // LINE FEED
        | '\u{000b}' // LINE TABULATION — PowerPoint's soft break (FR-501)
        | '\u{000c}' // FORM FEED
        | '\u{000d}' // CARRIAGE RETURN — CRLF counts once, see `split_lines`
        | '\u{0085}' // NEXT LINE
        | '\u{2028}' // LINE SEPARATOR
        | '\u{2029}' // PARAGRAPH SEPARATOR
    )
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

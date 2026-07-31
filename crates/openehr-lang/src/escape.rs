//! The `master03` string-escape decoder — one home for ODIN, BEL and cADL.
//!
//! `LANG/docs/odin/master03-basics.adoc` §File Encoding (verbatim twin:
//! `AM/docs/ADL2/master03-file_encoding.adoc` §File Encoding) defines the
//! ASCII encoding of unicode as "`\u` escaped UTF-16" in two spellings:
//!
//! - `\uHHHH` — "4 hex digits which will be the same (possibly 0-filled on the
//!   left) as the unicode code-point number expressed in hexadecimal; this
//!   applies to unicode codepoints in the range `U+0000` - `U+FFFF` (the 'base
//!   multi-lingual plane', BMP)";
//! - `\uHHHHHHHH` — "8 hex digits to encode unicode code-points in the range
//!   `U+10000` through `U+10FFFF` (non-BMP planes); the algorithm is described
//!   in IETF RFC 2781".
//!
//! plus the six customary quoted forms `\r \n \t \\ \" \'` of §Special
//! Character Sequences.
//!
//! NOTE: the released text contradicts itself about `\u`. §File Encoding
//! sanctions the two `\u` spellings; §Special Character Sequences, some ten
//! lines later, closes its six-item list (which does NOT include `\u`) with
//! "Any other character combination starting with a backslash is illegal".
//! Adjudicated for §File Encoding — it is the specific rule, it states the
//! ranges and defers to RFC 2781, and reading the general sentence as a ban
//! would make the whole §File Encoding provision dead text. Both sections are
//! cited here because both are normative as published.
//!
//! NOTE: **no openEHR spec governs which UTF-16 spelling the eight digits
//! carry — our own design/extension.** The chapter says "`\u` escaped UTF-16"
//! and defers the algorithm to RFC 2781, which is a surrogate-pair
//! specification; but the code-point gloss ("the same … as the unicode
//! code-point number expressed in hexadecimal") is stated only for the 4-digit
//! form, so a zero-filled scalar reading is also defensible. Both are decoded,
//! disambiguated by the first four digits, which are disjoint by construction:
//! a UTF-16 high surrogate opens `D800`-`DBFF` (RFC 2781 §2.1), while the
//! zero-filled spelling of `U+10000`-`U+1FFFF` opens `0000`-`0001`. Eight hex
//! digits opening with anything else are read as the 4-digit form followed by
//! literal hex text, so a plain `A` never changes meaning because hex
//! characters happen to follow it.

/// A `master03` escape-sequence defect.
///
/// Every variant is a decode failure with no sound fallback: the alternative
/// would be to emit a replacement character or pass the escape through
/// verbatim, both of which turn an authoring defect into silently wrong text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EscapeError {
    /// A `\uHHHHHHHH` escape read as a zero-filled scalar whose code point
    /// falls outside `U+10000`-`U+10FFFF`, the only range the 8-digit form
    /// encodes (§File Encoding).
    #[error(
        "the 8-digit unicode escape '\\u{escape}' denotes U+{code_point:04X}, outside the U+10000-U+10FFFF range the 8-digit form encodes"
    )]
    NonBmpEscapeOutOfRange {
        /// The eight hex digits, as authored.
        escape: String,
        /// The code point they denote.
        code_point: u32,
    },
    /// A `\uHHHHHHHH` escape opening with a UTF-16 high surrogate whose second
    /// half is not a low surrogate (`DC00`-`DFFF`, RFC 2781 §2.1).
    #[error(
        "the 8-digit unicode escape '\\u{escape}' opens with a high surrogate but its second half is not a low surrogate (DC00-DFFF)"
    )]
    MalformedSurrogatePair {
        /// The eight hex digits, as authored.
        escape: String,
    },
    /// A `\uHHHH` escape naming a UTF-16 surrogate code point. A surrogate is
    /// not a Unicode scalar value and appears only as half of a pair
    /// (RFC 2781 §2.1), so on its own it denotes no character.
    #[error(
        "the unicode escape '\\u{escape}' names the surrogate code point U+{code_point:04X}, which is not a character"
    )]
    LoneSurrogate {
        /// The four hex digits, as authored.
        escape: String,
        /// The surrogate code point they name.
        code_point: u32,
    },
}

/// The lowest UTF-16 high surrogate (RFC 2781 §2.1).
const HIGH_SURROGATE_FIRST: u32 = 0xD800;
/// The highest UTF-16 high surrogate.
const HIGH_SURROGATE_LAST: u32 = 0xDBFF;
/// The lowest UTF-16 low surrogate.
const LOW_SURROGATE_FIRST: u32 = 0xDC00;
/// The highest UTF-16 low surrogate.
const LOW_SURROGATE_LAST: u32 = 0xDFFF;
/// The first non-BMP code point — the low end of the 8-digit form's range.
const NON_BMP_FIRST: u32 = 0x0001_0000;
/// The last Unicode code point — the high end of the 8-digit form's range.
const NON_BMP_LAST: u32 = 0x0010_FFFF;

/// Decode the `master03` escape sequences of an undelimited string body.
///
/// # Errors
/// [`EscapeError`] for a `\u` escape that denotes no character: an 8-digit
/// form outside `U+10000`-`U+10FFFF`, a malformed surrogate pair, or a lone
/// surrogate.
pub fn decode(inner: &str) -> Result<String, EscapeError> {
    if !inner.contains('\\') {
        return Ok(inner.to_owned());
    }
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('r') => out.push('\r'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            // `\\` and a lone trailing `\` both yield one literal backslash.
            Some('\\') | None => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') => decode_unicode(&mut chars, &mut out)?,
            // TODO: reject every backslash sequence outside the six §Special
            // Character Sequences forms plus `\u` ("Any other character
            // combination starting with a backslash is illegal") instead of
            // passing it through verbatim; each lexer that feeds this decoder
            // rejects them structurally today, so the pass-through only serves
            // direct callers — tracked as issue #1344.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    Ok(out)
}

/// Check that every `master03` escape sequence of an undelimited string body
/// decodes, discarding the decoded text.
///
/// The lexers call this so a semantically-unsound `\u` fails the LEX, next to
/// the structural escape check, rather than surfacing later.
///
/// # Errors
/// As [`decode`].
pub fn validate(inner: &str) -> Result<(), EscapeError> {
    decode(inner).map(|_| ())
}

/// Decode one `\u…` escape, whose `\u` prefix `chars` has already yielded.
///
/// Consumes exactly the hex digits the chosen spelling uses, so any trailing
/// hex text stays literal.
fn decode_unicode(chars: &mut std::str::Chars<'_>, out: &mut String) -> Result<(), EscapeError> {
    let digits: String = chars
        .clone()
        .take(8)
        .take_while(char::is_ascii_hexdigit)
        .collect();
    let first: String = digits.chars().take(4).collect();
    let first_value = if first.chars().count() == 4 {
        hex_value(&first)
    } else {
        None
    };
    let Some(first_value) = first_value else {
        // Fewer than four hex digits: not a well-formed `\u` escape at all.
        // Every lexer feeding this decoder already refuses it; a direct caller
        // gets it back verbatim.
        out.push('\\');
        out.push('u');
        return Ok(());
    };
    let has_eight = digits.chars().count() >= 8;

    if has_eight && (HIGH_SURROGATE_FIRST..=HIGH_SURROGATE_LAST).contains(&first_value) {
        let second: String = digits.chars().skip(4).take(4).collect();
        let low = hex_value(&second)
            .filter(|low| (LOW_SURROGATE_FIRST..=LOW_SURROGATE_LAST).contains(low));
        let Some(low) = low else {
            return Err(EscapeError::MalformedSurrogatePair { escape: digits });
        };
        // RFC 2781 §2.1: the pair encodes `0x10000 + (hi - 0xD800) * 0x400 +
        // (lo - 0xDC00)`, which is total over the two surrogate ranges.
        let code_point = NON_BMP_FIRST
            + (first_value - HIGH_SURROGATE_FIRST) * 0x400
            + (low - LOW_SURROGATE_FIRST);
        push_non_bmp(code_point, digits, out)?;
        advance(chars, 8);
        return Ok(());
    }

    if has_eight && first_value <= 0x0001 {
        let Some(code_point) = hex_value(&digits) else {
            out.push('\\');
            out.push('u');
            return Ok(());
        };
        push_non_bmp(code_point, digits, out)?;
        advance(chars, 8);
        return Ok(());
    }

    // The 4-digit BMP form.
    if (HIGH_SURROGATE_FIRST..=LOW_SURROGATE_LAST).contains(&first_value) {
        return Err(EscapeError::LoneSurrogate {
            escape: first,
            code_point: first_value,
        });
    }
    let Some(ch) = char::from_u32(first_value) else {
        return Err(EscapeError::LoneSurrogate {
            escape: first,
            code_point: first_value,
        });
    };
    out.push(ch);
    advance(chars, 4);
    Ok(())
}

/// Push the character an 8-digit escape denotes, refusing anything outside the
/// `U+10000`-`U+10FFFF` range that form encodes.
fn push_non_bmp(code_point: u32, escape: String, out: &mut String) -> Result<(), EscapeError> {
    if !(NON_BMP_FIRST..=NON_BMP_LAST).contains(&code_point) {
        return Err(EscapeError::NonBmpEscapeOutOfRange { escape, code_point });
    }
    let Some(ch) = char::from_u32(code_point) else {
        return Err(EscapeError::NonBmpEscapeOutOfRange { escape, code_point });
    };
    out.push(ch);
    Ok(())
}

/// The value of a hex-digit string, or `None` if it is empty, over-long, or not
/// all hex.
fn hex_value(digits: &str) -> Option<u32> {
    if digits.is_empty() || digits.len() > 8 {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

/// Drop `n` characters from `chars`.
fn advance(chars: &mut std::str::Chars<'_>, n: usize) {
    for _ in 0..n {
        chars.next();
    }
}

#[cfg(test)]
mod tests {
    use super::{EscapeError, decode};

    /// Decode `inner`, failing the test with the defect if it does not.
    fn decoded(inner: &str) -> String {
        decode(inner).expect("the fixture should decode")
    }

    /// The six customary quoted forms of §Special Character Sequences.
    #[test]
    fn the_six_quoted_forms_decode() {
        assert_eq!(decoded(r"a\rb\nc\td\\e"), "a\rb\nc\td\\e");
        assert_eq!(decoded(r#"\""#), "\"");
        assert_eq!(decoded(r"\'"), "'");
    }

    /// The 4-digit form covers the BMP (`U+0000`-`U+FFFF`).
    #[test]
    fn four_digit_escapes_decode_bmp_code_points() {
        assert_eq!(decoded(r"\u0041"), "A");
        assert_eq!(decoded(r"\u00E9t\u00e9"), "\u{e9}t\u{e9}");
        assert_eq!(decoded(r"\uFFFD"), "\u{FFFD}");
    }

    /// RFC 2781 §2.1 surrogate pairs, at both ends of the non-BMP range.
    #[test]
    fn surrogate_pairs_decode_non_bmp_code_points() {
        assert_eq!(decoded(r"\uD800DC00"), "\u{10000}");
        assert_eq!(decoded(r"\uDBFFDFFF"), "\u{10FFFF}");
        assert_eq!(decoded(r"\uD83DDE00"), "\u{1F600}");
    }

    /// The zero-filled 8-digit spelling, whose first four digits are `0001`.
    #[test]
    fn zero_filled_eight_digit_escapes_decode() {
        assert_eq!(decoded(r"\u00010000"), "\u{10000}");
        assert_eq!(decoded(r"\u0001F600"), "\u{1F600}");
    }

    /// An 8-digit spelling of a BMP code point is outside the 8-digit form's
    /// stated range, so it is a defect rather than a silent BMP character.
    #[test]
    fn an_eight_digit_bmp_value_is_refused() {
        assert_eq!(
            decode(r"\u0000FFFF"),
            Err(EscapeError::NonBmpEscapeOutOfRange {
                escape: "0000FFFF".to_owned(),
                code_point: 0xFFFF,
            })
        );
    }

    /// A high surrogate whose second half is not a low surrogate.
    #[test]
    fn a_high_surrogate_without_a_low_half_is_refused() {
        assert_eq!(
            decode(r"\uD8000041"),
            Err(EscapeError::MalformedSurrogatePair {
                escape: "D8000041".to_owned(),
            })
        );
    }

    /// A lone surrogate, and the reversed pair whose first half is a LOW
    /// surrogate, are both lone-surrogate defects.
    #[test]
    fn lone_and_reversed_surrogates_are_refused() {
        assert_eq!(
            decode(r"\uD800"),
            Err(EscapeError::LoneSurrogate {
                escape: "D800".to_owned(),
                code_point: 0xD800,
            })
        );
        assert_eq!(
            decode(r"\uDC00D800"),
            Err(EscapeError::LoneSurrogate {
                escape: "DC00".to_owned(),
                code_point: 0xDC00,
            })
        );
    }

    /// Eight hex digits opening outside both disambiguating windows read as the
    /// 4-digit form followed by literal hex text.
    #[test]
    fn hex_text_after_a_four_digit_escape_stays_literal() {
        assert_eq!(decoded(r"\u00410042"), "A0042");
    }
}

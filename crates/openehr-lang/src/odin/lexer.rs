//! ODIN lexer — a self-contained `logos` tokenizer for Object Data Instance
//! Notation, transcribed from the vendored `odin.g4` / `odin_values.g4` (which
//! import `base_lexer.g4`) at `crates/openehr-lang/vendor/grammar/` and the
//! normative text `docs/specs/openehr/LANG/docs/odin/`.
//!
//! ODIN is a standalone leaf-data notation (it backs BMM `.bmm`/`.idx` files
//! and the ADL description/terminology/annotation sections alike), so this
//! lexer is deliberately independent of any ADL/cADL tokens — it covers only
//! the ODIN value + structure subset.

use logos::Logos;

/// A lexed ODIN token together with its byte span in the source.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    /// The token.
    pub(crate) token: Token,
    /// Byte range of the token in the source.
    pub(crate) span: std::ops::Range<usize>,
}

/// An ODIN token (the value + structure subset of `base_lexer.g4`).
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip "\u{feff}")] // leading UTF-8 BOM
#[logos(skip r"[ \t\r\n]+")] // WS / LINE
#[logos(skip(r"--[^\n]*", allow_greedy = true))] // CMT_LINE `-- … EOL`
pub(crate) enum Token {
    /// `SYM_TRUE` — fully case-insensitive (`base_lexer.g4`
    /// `SYM_TRUE : [Tt][Rr][Uu][Ee]`; `LANG/docs/odin/master07-leaf_data`
    /// §Boolean Data "Boolean values can be indicated by the following values
    /// (case-insensitive)"; `AM/docs/ADL1.4/master04-dadl` §Symbols, the dADL
    /// lex rule `[Tt][Rr][Uu][Ee]`).
    #[regex("[Tt][Rr][Uu][Ee]")]
    True,
    /// `SYM_FALSE` — fully case-insensitive (same citations as [`Token::True`]).
    #[regex("[Ff][Aa][Ll][Ss][Ee]")]
    False,

    /// `SYM_INFINITY` — an unbounded interval endpoint, case-insensitive
    /// (`AM/docs/ADL1.4/master04-dadl` §Intervals of Ordered Primitive Types:
    /// "The allowable values for `N` and `M` include any value in the range of
    /// the relevant type, as well as: `infinity` / `-infinity` / `*`", and
    /// §Symbols `[Ii][Nn][Ff][Ii][Nn][Ii][Tt][Yy]`).
    ///
    /// NOTE: `LANG/docs/odin/master07-leaf_data` §Intervals of Ordered
    /// Primitive Types states only "any value in the range of the relevant
    /// type", so accepting the marker in ODIN generally is a dADL-1.4-grounded
    /// superset of the ODIN chapter — deliberate, since this reader serves the
    /// 1.4 dADL front end as well.
    #[regex("[Ii][Nn][Ff][Ii][Nn][Ii][Tt][Yy]")]
    Infinity,

    /// `ISO8601_DATE_TIME` (with optional partial `??` fields / timezone).
    ///
    /// The `??`-partial family is the full set of
    /// `AM/docs/ADL1.4/master04-dadl` §Partial Date/Times: `…Thh:mm:??`,
    /// `…Thh:??:??`, `yyyy-MM-ddT??:??:??`, `yyyy-MM-??T??:??:??` and
    /// `yyyy-??-??T??:??:??` (the last three are absent from
    /// `LANG/docs/odin/master07-leaf_data`, which lists only the first two —
    /// the dADL chapter's larger set is accepted here, a superset for the
    /// ODIN-only callers).
    #[regex(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2}T([0-9]{2}(:[0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|:\?\?:\?\?)?|\?\?:\?\?:\?\?)(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned()
    )]
    #[regex(
        r"[0-9]{4}-([0-9]{2}-\?\?|\?\?-\?\?)T\?\?:\?\?:\?\?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned()
    )]
    // The space-separated date/time form of the `AM/docs/ADL1.4/master08-adl`
    // §Revision History Section example (`time_committed = <2004-11-02
    // 09:31:04+1000>`). NOTE: that example contradicts its own chapter set —
    // `master04-dadl` §Complete Date/Times mandates the ISO 8601 extended form
    // with the `T` designator, and neither the dADL lex rules nor the vendored
    // `base_lexer.g4` `ISO8601_DATE_TIME` admit a space. The form is accepted
    // (an upstream spec-example defect, not an authoring error to punish) and
    // normalised to the `T` form on read, so every consumer sees valid
    // ISO 8601.
    #[regex(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9]{2}:[0-9]{2}(:[0-9]{2}([.,][0-9]+)?)?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().replacen(' ', "T", 1)
    )]
    DateTime(String),
    /// `ISO8601_DATE` (with optional partial `??` fields).
    #[regex(r"[0-9]{4}-([0-9]{2}(-([0-9]{2}|\?\?))?|\?\?-\?\?)", |lex| lex.slice().to_owned())]
    Date(String),
    /// `ISO8601_TIME` (with optional partial `??` fields / timezone).
    #[regex(
        r"[0-9]{2}:([0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|\?\?:\?\?)(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned()
    )]
    Time(String),
    /// `ISO8601_DURATION`, requiring at least one component (never bare `P`).
    #[regex(
        r"-?P([0-9]+[YyMmWwDd])+(T([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|T([0-9]+[Hh])?([0-9]+[Mm])?([0-9]+([.,][0-9]+)?[Ss])?)?|-?PT([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|-?PT([0-9]+[Hh])([0-9]+[Mm])?|-?PT[0-9]+[Mm]",
        |lex| lex.slice().to_owned()
    )]
    Duration(String),

    /// A term-code reference `[terminology(ver)?::code]` (`TERM_CODE_REF`).
    #[regex(r"\[[a-zA-Z0-9._\-]+(\([a-zA-Z0-9._\-]+\))?::[a-zA-Z0-9._\-]+\]", |lex| lex.slice().to_owned())]
    TermCodeRef(String),
    /// A *local* term-code reference `[at0200]` — a bracketed code with no
    /// `terminology::` qualifier (`AM/docs/ADL1.4/master04-dadl` §Symbols,
    /// `V_LOCAL_TERM_CODE_REF : \[{ALPHANUM}{NAMECHAR}*\]`).
    ///
    /// NOTE (spec-internal conflict, resolved in favour of the lex rules + the
    /// worked examples): the same chapter's yacc grammar admits only
    /// `V_QUALIFIED_TERM_CODE_REF` under `primitive_object_value`, so by the
    /// production rules alone `[at0200]` would not be a leaf value — yet the
    /// chapter's own §Lists of Built-in Types example lists `[at0200], ...`
    /// beside `"en", ...`, and `LANG/docs/odin/master07-leaf_data` §Lists of
    /// Built-in Types repeats that example verbatim. Two normative passages
    /// against one production rule: the local code is read as a leaf value.
    ///
    /// The pattern deliberately requires an ALPHA first character (narrower
    /// than the 1.4 `ALPHANUM`) so that the container keys `[1]`, `[01234]`
    /// and `[2004-06-11]` keep lexing as `'[' key ']'`, which the 1.4 lex rule
    /// would otherwise swallow whole.
    #[regex(r"\[[a-zA-Z][a-zA-Z0-9._\-]*\]", |lex| lex.slice().to_owned())]
    LocalTermCodeRef(String),
    /// An embedded URI `<scheme:…>` (`EMBEDDED_URI`).
    #[regex(r"<[ \t\r\n]*[a-zA-Z][a-zA-Z0-9+.\-]*:[^>]*>", |lex| lex.slice().to_owned())]
    EmbeddedUri(String),
    /// An ADL path (`base_lexer.g4 ADL_PATH`) — an object-reference target.
    #[regex(
        r"(/[a-zA-Z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+|[a-zA-Z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?(/[a-zA-Z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+",
        |lex| lex.slice().to_owned()
    )]
    Path(String),

    /// `REAL` — `DIGIT+ '.' DIGIT+` with optional `E` suffix.
    #[regex(r"[0-9]+\.[0-9]+([eE][+\-]?[0-9]+)?", |lex| lex.slice().to_owned())]
    Real(String),
    /// `INTEGER` — `DIGIT+` with optional `E` suffix.
    #[regex(r"[0-9]+([eE][+\-]?[0-9]+)?", |lex| lex.slice().to_owned())]
    Integer(String),
    /// A double-quoted `STRING` (may span lines); escapes validated per
    /// `master03` (`\r \n \t \\ \" \'` + `\uHHHH`/`\uHHHHHHHH`), and the
    /// multi-line whitespace leaders removed per
    /// `LANG/docs/odin/master07-leaf_data` §String Data (see
    /// [`validate_string`]).
    #[regex(r#""([^"\\]|\\.)*""#, validate_string)]
    String(String),
    /// A single-quoted `CHARACTER`.
    #[regex(r"'([^'\\\r\n]|\\.)'", |lex| lex.slice().to_owned())]
    Character(String),

    /// `ALPHA_UC_ID` — upper-initial identifier (ODIN keys / `rm_type_id`).
    #[regex(r"[A-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    AlphaUcId(String),
    /// `ALPHA_LC_ID` — lower-initial identifier (`rm_attribute_id` / ODIN key).
    #[regex(r"[a-z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    AlphaLcId(String),
    /// `ALPHA_UNDERSCORE_ID` — `_`-initial identifier (meta ids).
    #[regex(r"_[a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    AlphaUnderscoreId(String),

    /// `<`
    #[token("<")]
    Lt,
    /// `>`
    #[token(">")]
    Gt,
    /// `<=` / `≤`
    #[token("<=")]
    #[token("\u{2264}")]
    Le,
    /// `>=` / `≥`
    #[token(">=")]
    #[token("\u{2265}")]
    Ge,
    /// `(`
    #[token("(")]
    LParen,
    /// `)`
    #[token(")")]
    RParen,
    /// `[`
    #[token("[")]
    LBracket,
    /// `]`
    #[token("]")]
    RBracket,
    /// `=`
    #[token("=")]
    Eq,
    /// `,`
    #[token(",")]
    Comma,
    /// `;`
    #[token(";")]
    SemiColon,
    /// `...` (`SYM_LIST_CONTINUE`).
    #[token("...")]
    ListContinue,
    /// `..` (`SYM_IVL_SEP`).
    #[token("..")]
    IvlSep,
    /// `|` (`SYM_IVL_DELIM`).
    #[token("|")]
    IvlDelim,
    /// `+/-` / `±` (`SYM_PLUS_OR_MINUS`).
    #[token("+/-")]
    #[token("\u{00B1}")]
    PlusOrMinus,
    /// `+`
    #[token("+")]
    Plus,
    /// `-`
    #[token("-")]
    Minus,
    /// `/` — a bare object-reference root path.
    #[token("/")]
    Slash,
    /// `*` — "equivalent to infinity" as an interval endpoint
    /// (`AM/docs/ADL1.4/master04-dadl` §Intervals of Ordered Primitive Types).
    #[token("*")]
    Star,
}

/// Validate `master03` string escapes and return the raw (still-quoted) slice
/// with multi-line whitespace leaders removed.
///
/// Illegal escapes (anything other than `\r \n \t \\ \" \'` or a `\u` + 4/8
/// hex-digit sequence) fail the lex (`LANG/docs/odin/master03-basics`
/// §Special Character Sequences + `AM/docs/ADL1.4/master03-file_encoding`
/// §Special Character Sequences, which define `\"` as *the* encoding of a
/// literal double quote).
///
/// NOTE (adjudicated as descriptive, not a decoding rule):
/// `AM/docs/ADL1.4/master04-dadl` §String Data — and verbatim
/// `LANG/docs/odin/master07-leaf_data` §String Data — illustrate quoting with
/// `"… what one might call a &quot;phrase&quot;."`. No chapter defines a
/// decoding step for that form, the two `master03` chapters make `\"` the
/// normative literal-quote encoding, and decoding `&quot;` would be
/// irreversible for text that legitimately contains it. The sequence is
/// therefore carried through verbatim, as authored.
fn validate_string(lex: &logos::Lexer<Token>) -> Result<String, ()> {
    let raw = lex.slice();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'\\' {
            let Some(&next) = bytes.get(i + 1) else {
                return Err(());
            };
            match next {
                b'r' | b'n' | b't' | b'\\' | b'"' | b'\'' => i += 2,
                b'u' => {
                    let hex_start = i + 2;
                    let count = raw
                        .get(hex_start..)
                        .unwrap_or_default()
                        .chars()
                        .take_while(char::is_ascii_hexdigit)
                        .count();
                    if count >= 8 {
                        i = hex_start + 8;
                    } else if count >= 4 {
                        i = hex_start + 4;
                    } else {
                        return Err(());
                    }
                }
                _ => return Err(()),
            }
        } else {
            i += 1;
        }
    }
    Ok(strip_line_leaders(raw, leader_budget(lex)))
}

/// How many leading whitespace characters may be removed from each
/// continuation line of a multi-line string: the column at which the string's
/// first line of content starts.
///
/// `LANG/docs/odin/master07-leaf_data` §String Data (verbatim in
/// `AM/docs/ADL1.4/master04-dadl` §String Data): "The exact contents of the
/// string are computed as being the characters between the double quote
/// characters, with the removal of white space leaders up to the left-most
/// character of the first line of the string." The left-most character of the
/// string's first line is the one immediately after the opening `"`, so its
/// column is the budget.
fn leader_budget(lex: &logos::Lexer<Token>) -> usize {
    let start = lex.span().start;
    let src = lex.source();
    let Some(before) = src.get(..start) else {
        return 0;
    };
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    // +1 for the opening quote itself.
    before
        .get(line_start..)
        .map_or(0, |indent| indent.chars().count() + 1)
}

/// Remove up to `budget` leading whitespace characters from every line of
/// `raw` after the first (see [`leader_budget`]). Single-line strings are
/// returned untouched.
fn strip_line_leaders(raw: &str, budget: usize) -> String {
    if !raw.contains('\n') {
        return raw.to_owned();
    }
    let mut out = String::with_capacity(raw.len());
    for (idx, line) in raw.split_inclusive('\n').enumerate() {
        if idx == 0 {
            out.push_str(line);
            continue;
        }
        let kept = line
            .char_indices()
            .take(budget)
            .take_while(|(_, c)| *c == ' ' || *c == '\t')
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        out.push_str(line.get(kept..).unwrap_or(line));
    }
    out
}

/// Lex `src` into a spanned ODIN token vector.
///
/// # Errors
/// Returns the byte span of the first token that fails to lex (an unrecognised
/// character or an illegal string escape).
pub(crate) fn lex(src: &str) -> Result<Vec<Spanned>, std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(res) = lexer.next() {
        match res {
            Ok(token) => out.push(Spanned {
                token,
                span: lexer.span(),
            }),
            Err(()) => return Err(lexer.span()),
        }
    }
    Ok(out)
}

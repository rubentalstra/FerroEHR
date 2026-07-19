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
    /// `True` / `true` (`SYM_TRUE`).
    #[token("True")]
    #[token("true")]
    True,
    /// `False` / `false` (`SYM_FALSE`).
    #[token("False")]
    #[token("false")]
    False,

    /// `ISO8601_DATE_TIME` (with optional partial `??` fields / timezone).
    #[regex(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}(:[0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|:\?\?:\?\?)?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned()
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
    /// `master03` (`\r \n \t \\ \" \'` + `\uHHHH`/`\uHHHHHHHH`).
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
}

/// Validate `master03` string escapes and return the raw slice.
///
/// Illegal escapes (anything other than `\r \n \t \\ \" \'` or a `\u` + 4/8
/// hex-digit sequence) fail the lex (`LANG/docs/odin` + `ADL2/master03`).
fn validate_string(lex: &logos::Lexer<Token>) -> Result<String, ()> {
    let raw = lex.slice();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            let Some(&next) = bytes.get(i + 1) else {
                return Err(());
            };
            match next {
                b'r' | b'n' | b't' | b'\\' | b'"' | b'\'' => i += 2,
                b'u' => {
                    let hex_start = i + 2;
                    let count = raw[hex_start.min(raw.len())..]
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
    Ok(raw.to_owned())
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

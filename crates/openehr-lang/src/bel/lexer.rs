//! BEL lexer — a `logos` tokenizer for the openEHR Basic Expression Language.
//!
//! Spec oracle: `docs/specs/openehr/LANG/docs/BEL/` (`master03-language.adoc`
//! and the syntax appendix `masterAppA-syntax.adoc`, whose normative token
//! forms match the vendored ADL grammar `base_lexer.g4`/`base_expressions.g4`).
//! This is a focused subset of that lexer covering exactly the BEL surface:
//! statements, assertions, assignments, operators (text + symbol forms),
//! literals, variables, paths and the `matches { … }` constraint delimiters —
//! it is deliberately self-contained so `openehr-lang` needs no dependency on
//! the ADL crate (dependency arrows point `openehr-adl → openehr-lang`).

use logos::Logos;

/// A BEL token with its byte span in the source.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned {
    /// The token.
    pub(crate) token: Token,
    /// Byte range of the token in the original source.
    pub(crate) span: std::ops::Range<usize>,
}

/// A BEL token. Text-bearing variants carry the owned source slice verbatim
/// (delimiters included; the parser decodes them).
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")] // whitespace (base_lexer.g4 WS/LINE)
#[logos(skip(r"--[^\n]*", allow_greedy = true))] // `-- … EOL` comments (base_lexer.g4 CMT_LINE)
pub(crate) enum Token {
    // ── operators: text + unicode symbol forms fold to one variant
    //    (base_expressions.g4 / adl_keywords.g4). ──
    /// `matches` / `is_in` / `∈` (`SYM_MATCHES`).
    #[token("matches")]
    #[token("is_in")]
    #[token("\u{2208}")]
    Matches,
    /// `and` / `∧` (`SYM_AND`).
    #[token("and")]
    #[token("\u{2227}")]
    And,
    /// `or` / `∨` (`SYM_OR`).
    #[token("or")]
    #[token("\u{2228}")]
    Or,
    /// `xor` (`SYM_XOR`).
    #[token("xor")]
    Xor,
    /// `not` / `~` / `∼` / `¬` / `!` (`SYM_NOT`).
    #[token("not")]
    #[token("~")]
    #[token("\u{223C}")]
    #[token("\u{00AC}")]
    #[token("!")]
    Not,
    /// `implies` / `®` / `->` (`SYM_IMPLIES`).
    #[token("implies")]
    #[token("\u{00AE}")]
    #[token("->")]
    Implies,
    /// `for_all` / `∀` (`SYM_FOR_ALL`).
    #[token("for_all")]
    #[token("\u{2200}")]
    ForAll,
    /// `exists` (`SYM_EXISTS`).
    #[token("exists")]
    Exists,
    /// `there_exists` / `∃` (`SYM_THERE_EXISTS`).
    #[token("there_exists")]
    #[token("\u{2203}")]
    ThereExists,
    /// `in` — quantifier binding keyword (`for_all v in coll`).
    #[token("in")]
    In,

    /// `True` / `true`.
    #[token("True")]
    #[token("true")]
    True,
    /// `False` / `false`.
    #[token("False")]
    #[token("false")]
    False,

    // ── ISO-8601 value literals (base_lexer.g4); ordered date-time before date
    //    before time so the longest form wins. ──
    /// `ISO8601_DATE_TIME`.
    #[regex(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}(:[0-9]{2}(:[0-9]{2}([.,][0-9]+)?)?)?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned(), priority = 8
    )]
    DateTime(String),
    /// `ISO8601_DATE`.
    #[regex(r"[0-9]{4}-[0-9]{2}-[0-9]{2}", |lex| lex.slice().to_owned(), priority = 8)]
    Date(String),
    /// `ISO8601_TIME`.
    #[regex(
        r"[0-9]{2}:[0-9]{2}(:[0-9]{2}([.,][0-9]+)?)?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned(), priority = 8
    )]
    Time(String),
    /// `ISO8601_DURATION`, requiring at least one component (never bare `P`);
    /// the ADL `base_lexer.g4` form. Higher priority than the identifier tokens
    /// so `P1Y2M` classifies as a duration, not an upper-initial identifier.
    #[regex(
        r"-?P([0-9]+[YyMmWwDd])+(T([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|T([0-9]+[Hh])?([0-9]+[Mm])?([0-9]+([.,][0-9]+)?[Ss])?)?|-?PT([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|-?PT([0-9]+[Hh])([0-9]+[Mm])?|-?PT[0-9]+[Mm]",
        |lex| lex.slice().to_owned(), priority = 8
    )]
    Duration(String),

    // ── numbers ──
    /// `REAL` — `DIGIT+ '.' DIGIT+` with optional exponent.
    #[regex(r"[0-9]+\.[0-9]+([eE][+\-]?[0-9]+)?", |lex| lex.slice().to_owned())]
    Real(String),
    /// `INTEGER`.
    #[regex(r"[0-9]+", |lex| lex.slice().to_owned())]
    Integer(String),
    /// A double-quoted `STRING` (may span lines). Escapes are validated per
    /// `ADL2/master03-file_encoding.adoc` §Special Character Sequences (see
    /// [`validate_string`]).
    #[regex(r#""([^"\\]|\\.)*""#, validate_string)]
    String(String),
    /// A single-quoted `CHARACTER`. An escaped character must be one of the
    /// six legal quoted forms (see [`validate_char`]).
    #[regex(r"'([^'\\\r\n]|\\.)'", validate_char)]
    Character(String),

    /// A delimited contained regexp `{ /re/ }` / `{ ^re^ }` — a single token so
    /// a `matches { /regex/ }` constraint right-hand side lexes atomically
    /// (`base_lexer.g4` `CONTAINED_REGEXP`). Higher priority than `{`/`/`.
    #[regex(
        r#"\{[ \t\r]*(/([^/\r\n\\]|\\.)+/|\^([^\^\r\n\\]|\\.)+\^)[ \t\r]*(;[ \t\r]*"([^"\\]|\\.)*")?[ \t\r]*\}"#,
        |lex| lex.slice().to_owned()
    )]
    ContainedRegexp(String),

    // ── variables + paths (base_lexer.g4) ──
    /// `$name/path` (`VARIABLE_WITH_PATH`).
    #[regex(r"\$[a-z][a-zA-Z0-9_]*(/[a-zA-Z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+", |lex| lex.slice().to_owned())]
    VariableWithPath(String),
    /// `$name` (`VARIABLE_ID`).
    #[regex(r"\$[a-z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Variable(String),
    /// An ADL path (`ADL_PATH`), each segment `ALPHA_LC_ID ('[' predicate
    /// ']')?`: a movable path pattern `//seg(/seg)*` (ADL1.4
    /// `master07-paths.adoc` §Grammar `movable_path: SYM_MOVABLE_LEADER
    /// relative_path`), absolute `(/seg)+`, or relative `seg(/seg)*` — a
    /// single-segment relative path needs its `[predicate]` to be a path
    /// token (the same yacc's `relative_path: path_segment`; a bare
    /// identifier stays a name reference, which the parser resolves the
    /// same way).
    #[regex(
        r"//?[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?(/[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)*|[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?(/[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+|[a-z][a-zA-Z0-9_]*\[[^\]\r\n]*\]",
        |lex| lex.slice().to_owned()
    )]
    Path(String),

    // ── identifiers ──
    /// `ALPHA_UC_ID` — upper-initial (type ids, constant names).
    #[regex(r"[A-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    UpperId(String),
    /// `ALPHA_LC_ID` — lower-initial (attribute ids, tags, function names).
    #[regex(r"[a-z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    LowerId(String),

    // ── delimiters + symbol operators ──
    /// `(`
    #[token("(")]
    LParen,
    /// `)`
    #[token(")")]
    RParen,
    /// `{`
    #[token("{")]
    LCurly,
    /// `}`
    #[token("}")]
    RCurly,
    /// `[`
    #[token("[")]
    LBracket,
    /// `]`
    #[token("]")]
    RBracket,
    /// `,`
    #[token(",")]
    Comma,
    /// `:=` / `::=` (`SYM_ASSIGNMENT`).
    #[token(":=")]
    #[token("::=")]
    Assign,
    /// `:` (declaration / quantifier binding).
    #[token(":")]
    Colon,
    /// `..` (interval separator inside a constraint).
    #[token("..")]
    DotDot,
    /// `.` (a lone dot — inside constraint content / codes).
    #[token(".")]
    Dot,
    /// `;` (assumed-value separator / list separator inside a constraint).
    #[token(";")]
    SemiColon,
    /// `@` (terminology binding separator in `[ac1@terminology]`).
    #[token("@")]
    At,
    /// `/=` / `!=` / `≠` (`SYM_NE`) — plus the ADL 1.4 spelling `<>`
    /// (ADL1.4 `master06-assertions.adoc` §Equality Operators and its yacc
    /// `SYM_NE`; a superset for the BEL callers).
    #[token("/=")]
    #[token("!=")]
    #[token("<>")]
    #[token("\u{2260}")]
    Ne,
    /// `=` (`SYM_EQ`).
    #[token("=")]
    Eq,
    /// `<=` / `≤` (`SYM_LE`).
    #[token("<=")]
    #[token("\u{2264}")]
    Le,
    /// `>=` / `≥` (`SYM_GE`).
    #[token(">=")]
    #[token("\u{2265}")]
    Ge,
    /// `<` (`SYM_LT`).
    #[token("<")]
    Lt,
    /// `>` (`SYM_GT`).
    #[token(">")]
    Gt,
    /// `|` — interval delimiter (inside a constraint).
    #[token("|")]
    Bar,
    /// `+` (`SYM_PLUS`).
    #[token("+")]
    Plus,
    /// `-` (`SYM_MINUS`).
    #[token("-")]
    Minus,
    /// `*` / `∗` (`SYM_STAR`).
    #[token("*")]
    #[token("\u{2217}")]
    Star,
    /// `/` (`SYM_SLASH` — division).
    #[token("/")]
    Slash,
    /// `%` (`SYM_PERCENT`).
    #[token("%")]
    Percent,
    /// `^` (`SYM_CARAT` — exponent).
    #[token("^")]
    Caret,
}

/// Validate a `STRING` token's escape sequences: the six legal quoted forms
/// `\r \n \t \\ \" \'` plus the `\uHHHH`/`\uHHHHHHHH` ASCII-encoded-unicode
/// forms — "Any other character combination starting with a backslash is
/// illegal" (`ADL2/master03-file_encoding.adoc` §Special Character Sequences +
/// §File Encoding). The BEL lexer keeps the slice verbatim (this lexer is
/// deliberately self-contained — the ODIN reader carries its own copy with
/// multi-line leader stripping on top).
fn validate_string(lex: &logos::Lexer<Token>) -> Result<String, ()> {
    let raw = lex.slice();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        if byte == b'\\' {
            match bytes.get(i + 1) {
                Some(b'r' | b'n' | b't' | b'\\' | b'"' | b'\'') => i += 2,
                Some(b'u') => {
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
    Ok(raw.to_owned())
}

/// Validate a `CHARACTER` token: an escaped character must be one of the six
/// legal quoted forms — "Any other character combination starting with a
/// backslash is illegal" (`ADL2/master03-file_encoding.adoc` §Special
/// Character Sequences). The `\uHHHH` forms cannot fit the single-character
/// token.
fn validate_char(lex: &logos::Lexer<Token>) -> Result<String, ()> {
    let raw = lex.slice();
    let bytes = raw.as_bytes();
    if bytes.get(1) == Some(&b'\\')
        && !matches!(
            bytes.get(2),
            Some(b'r' | b'n' | b't' | b'\\' | b'"' | b'\'')
        )
    {
        return Err(());
    }
    Ok(raw.to_owned())
}

/// Lex `src` into a spanned token vector.
///
/// # Errors
/// Returns [`crate::bel::BelError::Lex`] at the byte offset of the first
/// character that fails to tokenize.
pub(crate) fn lex(src: &str) -> Result<Vec<Spanned>, crate::bel::BelError> {
    let mut out = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(res) = lexer.next() {
        match res {
            Ok(token) => out.push(Spanned {
                token,
                span: lexer.span(),
            }),
            Err(()) => {
                return Err(crate::bel::BelError::Lex {
                    at: lexer.span().start,
                    text: lexer.slice().to_owned(),
                });
            }
        }
    }
    Ok(out)
}

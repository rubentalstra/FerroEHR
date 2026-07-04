//! AQL lexer — a `logos` tokenizer transcribed from the authoritative
//! `AqlLexer.g4` (vendored at `vendor/grammar/`). No ANTLR runtime: the grammar
//! is the spec, this is a hand-written DFA lexer against it.
//!
//! Faithfulness notes:
//! - AQL keywords are **case-insensitive** (the grammar builds them from
//!   case-insensitive letter fragments), so each keyword uses
//!   `ignore(case)`.
//! - The grammar's grouped function-id tokens (`STRING_FUNCTION_ID`,
//!   `NUMERIC_FUNCTION_ID`, `DATE_TIME_FUNCTION_ID`) are **not** pre-grouped
//!   here: names like `length`/`abs`/`now` lex as [`Token::Identifier`] and the
//!   parser classifies a `name(args)` call. Structurally-distinct calls
//!   (aggregates, `terminology(...)`) keep dedicated keyword tokens because
//!   their argument grammar differs.
//! - `// PORT NOTE:` quoted temporal literals (`DATE`/`TIME`/`DATETIME` in the
//!   grammar) are lexed as [`Token::String`]; typing them as temporals is a
//!   later semantic concern (the parser accepts a string where a primitive is
//!   expected). This keeps the lexer free of the fiddly ISO 8601-vs-string
//!   priority tangle.

use logos::Logos;

/// A comparison operator (`COMPARISON_OPERATOR` in the grammar).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompOp {
    /// `=`
    Eq,
    /// `!=`
    Ne,
    /// `>`
    Gt,
    /// `>=`
    Ge,
    /// `<`
    Lt,
    /// `<=`
    Le,
}

/// An AQL token. Slices that carry text (identifiers, literals) hold an owned
/// `String` lexed from the source.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")] // WS -> skip
pub enum Token {
    // ── structural keywords (case-insensitive) ──────────────────────────────
    #[token("select", ignore(case))]
    Select,
    #[token("as", ignore(case))]
    As,
    #[token("from", ignore(case))]
    From,
    #[token("where", ignore(case))]
    Where,
    #[token("order", ignore(case))]
    Order,
    #[token("by", ignore(case))]
    By,
    #[token("desc", ignore(case))]
    Desc,
    #[token("descending", ignore(case))]
    Descending,
    #[token("asc", ignore(case))]
    Asc,
    #[token("ascending", ignore(case))]
    Ascending,
    #[token("limit", ignore(case))]
    Limit,
    #[token("offset", ignore(case))]
    Offset,
    #[token("distinct", ignore(case))]
    Distinct,
    #[token("version", ignore(case))]
    Version,
    #[token("latest_version", ignore(case))]
    LatestVersion,
    #[token("all_versions", ignore(case))]
    AllVersions,
    #[token("top", ignore(case))]
    Top,
    #[token("forward", ignore(case))]
    Forward,
    #[token("backward", ignore(case))]
    Backward,
    #[token("contains", ignore(case))]
    Contains,
    #[token("and", ignore(case))]
    And,
    #[token("or", ignore(case))]
    Or,
    #[token("not", ignore(case))]
    Not,
    #[token("exists", ignore(case))]
    Exists,
    #[token("like", ignore(case))]
    Like,
    #[token("matches", ignore(case))]
    Matches,

    // aggregate + terminology (distinct argument grammar → dedicated tokens)
    #[token("count", ignore(case))]
    Count,
    #[token("min", ignore(case))]
    Min,
    #[token("max", ignore(case))]
    Max,
    #[token("sum", ignore(case))]
    Sum,
    #[token("avg", ignore(case))]
    Avg,
    #[token("terminology", ignore(case))]
    Terminology,

    // literal keywords
    #[token("true", ignore(case))]
    True,
    #[token("false", ignore(case))]
    False,
    #[token("null", ignore(case))]
    Null,

    // ── operators & symbols ─────────────────────────────────────────────────
    #[token("=", |_| CompOp::Eq)]
    #[token("!=", |_| CompOp::Ne)]
    #[token(">=", |_| CompOp::Ge)]
    #[token("<=", |_| CompOp::Le)]
    #[token(">", |_| CompOp::Gt)]
    #[token("<", |_| CompOp::Lt)]
    Comparison(CompOp),

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftCurly,
    #[token("}")]
    RightCurly,
    #[token(",")]
    Comma,
    #[token("/")]
    Slash,
    #[token("*")]
    Asterisk,
    #[token(";")]
    Semicolon,
    #[token("-")]
    Minus,
    /// `--` — optional statement terminator / comment lead-in in the grammar.
    #[token("--")]
    DoubleDash,

    // ── literals & names ────────────────────────────────────────────────────
    // A `$name` query parameter.
    #[regex(r"\$[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Parameter(String),

    // `idNN[.NN]*` / `atNN[.NN]*` node codes (higher priority than Identifier).
    // The grammar's `CODE_STR` permits leading-zero runs (`at0001`), so a plain
    // `[0-9]+(.[0-9]+)*` after the `id`/`at` prefix is the faithful shape.
    #[regex(r"id[0-9]+(\.[0-9]+)*", |lex| lex.slice().to_owned())]
    IdCode(String),
    #[regex(r"at[0-9]+(\.[0-9]+)*", |lex| lex.slice().to_owned())]
    AtCode(String),

    // Archetype HRID, e.g. `openEHR-EHR-OBSERVATION.blood_pressure.v1`
    // (optionally namespaced). Detected by the `-x-x.…vN` shape.
    #[regex(
        r"([a-zA-Z][a-zA-Z0-9_.]*::)?[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z][a-zA-Z0-9_]*\.[a-zA-Z][a-zA-Z0-9_-]*\.v[0-9]+(\.[0-9]+)*",
        |lex| lex.slice().to_owned()
    )]
    ArchetypeHrid(String),

    // A term code, e.g. `local::at0001` or `SNOMED-CT::1234|text|`.
    #[regex(
        r"[a-zA-Z0-9._]+(\([a-zA-Z0-9._]+\))?::[a-zA-Z0-9._]+(\|[^|\[\]]+\|)?",
        |lex| lex.slice().to_owned()
    )]
    TermCode(String),

    // A URI, e.g. `http://example.org/x`. Recognised by a `scheme://` lead.
    #[regex(r"[a-zA-Z][a-zA-Z0-9+.\-]*://[^ \t\r\n{}]*", |lex| lex.slice().to_owned())]
    Uri(String),

    // A contained regex, e.g. `{/pattern/}` or `{/pattern/; 'name'}` (used in a
    // node predicate's `objectPath MATCHES CONTAINED_REGEX`). Whole thing is one
    // token so its inner `/` and `{}` are not mistaken for other symbols.
    #[regex(
        r"\{[ \t\r\n]*/(\\.|[^/\r\n])*/[ \t\r\n]*(;[ \t\r\n]*'([^'\\]|\\.)*')?[ \t\r\n]*\}",
        |lex| lex.slice().to_owned()
    )]
    ContainedRegex(String),

    // Scientific and plain numerics (order: sci before plain via length).
    #[regex(r"[0-9]+[eE][+\-]?[0-9]+", |lex| lex.slice().to_owned())]
    SciInteger(String),
    #[regex(r"[0-9]*\.[0-9]+[eE][+\-]?[0-9]+", |lex| lex.slice().to_owned())]
    SciReal(String),
    #[regex(r"[0-9]*\.[0-9]+", |lex| lex.slice().to_owned())]
    Real(String),
    #[regex(r"[0-9]+", |lex| lex.slice().to_owned())]
    Integer(String),

    // Single- or double-quoted string (also carries quoted temporals; see the
    // module PORT NOTE). Escapes are preserved in the slice, unescaped later.
    #[regex(r"'([^'\\]|\\.)*'", |lex| lex.slice().to_owned())]
    #[regex(r#""([^"\\]|\\.)*""#, |lex| lex.slice().to_owned())]
    String(String),

    // Plain identifier (lowest-priority word token).
    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Identifier(String),
}

/// Lex `src` into a token vector, or report the byte span of the first token
/// that fails to lex.
///
/// # Errors
/// Returns [`LexError`] on the first unrecognized token.
pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let mut out = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(res) = lexer.next() {
        match res {
            Ok(tok) => out.push(tok),
            Err(()) => {
                return Err(LexError {
                    span: lexer.span(),
                    slice: lexer.slice().to_owned(),
                });
            }
        }
    }
    Ok(out)
}

/// A lexing failure at a byte span.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unexpected token {slice:?} at bytes {span:?}")]
pub struct LexError {
    /// Byte range of the offending input.
    pub span: std::ops::Range<usize>,
    /// The offending slice.
    pub slice: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        lex(src).unwrap_or_else(|e| panic!("lex failed: {e}"))
    }

    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(toks("SELECT"), vec![Token::Select]);
        assert_eq!(toks("select"), vec![Token::Select]);
        assert_eq!(toks("Contains"), vec![Token::Contains]);
    }

    #[test]
    fn simple_select_from() {
        // SELECT c FROM COMPOSITION c
        let t = toks("SELECT c FROM COMPOSITION c");
        assert_eq!(
            t,
            vec![
                Token::Select,
                Token::Identifier("c".into()),
                Token::From,
                Token::Identifier("COMPOSITION".into()),
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn path_with_comparison_and_string() {
        // e/ehr_id/value = 'x'
        let t = toks("e/ehr_id/value = 'x'");
        assert_eq!(
            t,
            vec![
                Token::Identifier("e".into()),
                Token::Slash,
                Token::Identifier("ehr_id".into()),
                Token::Slash,
                Token::Identifier("value".into()),
                Token::Comparison(CompOp::Eq),
                Token::String("'x'".into()),
            ]
        );
    }

    #[test]
    fn node_codes_and_archetype_hrid() {
        assert_eq!(toks("at0001"), vec![Token::AtCode("at0001".into())]);
        assert_eq!(toks("id1.2.3"), vec![Token::IdCode("id1.2.3".into())]);
        assert_eq!(
            toks("openEHR-EHR-OBSERVATION.blood_pressure.v1"),
            vec![Token::ArchetypeHrid(
                "openEHR-EHR-OBSERVATION.blood_pressure.v1".into()
            )]
        );
    }

    #[test]
    fn id_prefix_alone_is_identifier() {
        // `id` / `at` without a code are plain identifiers.
        assert_eq!(toks("id"), vec![Token::Identifier("id".into())]);
        assert_eq!(
            toks("attribute"),
            vec![Token::Identifier("attribute".into())]
        );
    }

    #[test]
    fn numbers_and_params() {
        assert_eq!(toks("42"), vec![Token::Integer("42".into())]);
        assert_eq!(toks("3.14"), vec![Token::Real("3.14".into())]);
        assert_eq!(toks("1e10"), vec![Token::SciInteger("1e10".into())]);
        assert_eq!(toks("$ehrId"), vec![Token::Parameter("$ehrId".into())]);
    }

    #[test]
    fn comparison_operators() {
        assert_eq!(toks(">="), vec![Token::Comparison(CompOp::Ge)]);
        assert_eq!(toks("!="), vec![Token::Comparison(CompOp::Ne)]);
        assert_eq!(toks("<"), vec![Token::Comparison(CompOp::Lt)]);
    }

    #[test]
    fn contains_query_shape() {
        // FROM EHR e CONTAINS COMPOSITION c
        let t = toks("FROM EHR e CONTAINS COMPOSITION c");
        assert_eq!(
            t,
            vec![
                Token::From,
                Token::Identifier("EHR".into()),
                Token::Identifier("e".into()),
                Token::Contains,
                Token::Identifier("COMPOSITION".into()),
                Token::Identifier("c".into()),
            ]
        );
    }

    #[test]
    fn term_code_and_predicate_brackets() {
        // [openEHR-EHR-OBSERVATION.x.v1] and local::at0001
        assert_eq!(
            toks("local::at0001"),
            vec![Token::TermCode("local::at0001".into())]
        );
        let t = toks("[at0001]");
        assert_eq!(
            t,
            vec![
                Token::LeftBracket,
                Token::AtCode("at0001".into()),
                Token::RightBracket
            ]
        );
    }
}

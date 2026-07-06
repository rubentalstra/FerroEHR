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
//!   priority tangle. Per the QUERY spec §Dates and Times NOTE, the *typing*
//!   of a quoted value as a date/time is resolved from the identified-path
//!   context in the semantic pass, not from the literal — so an untyped
//!   `Token::String` is the faithful carrier here. (F-08-06: all temporal
//!   literals are indistinguishable from strings at this layer by design.)
//! - `// PORT NOTE:` (F-08-05) the grammar's single-row function-id groups
//!   (`STRING_FUNCTION_ID`/`NUMERIC_FUNCTION_ID`/`DATE_TIME_FUNCTION_ID` —
//!   `length`, `abs`, `now`, …) are **not** reserved here: they lex as
//!   [`Token::Identifier`] and the parser classifies a `name(args)` call
//!   (`AqlParser.g4 functionCall` explicitly also admits a bare `IDENTIFIER`
//!   name). This makes the accepted set a *superset* of the grammar (it never
//!   rejects valid AQL; it additionally tolerates these words as identifiers).
//!   Per ADR-008 a superset accept-envelope is the sanctioned direction; the
//!   reserved-word restriction is a semantic concern, not a syntax one.

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
#[logos(skip r"[ \t\r\n\f]+")] // WS -> skip (AqlLexer.g4 `WS`)
#[logos(skip "\u{feff}")] // UNICODE_BOM -> skip (F-08-13; AqlLexer.g4 `UNICODE_BOM`)
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
    /// `--` — the grammar's `SYM_DOUBLE_DASH` optional statement terminator.
    ///
    /// Per `AqlLexer.g4` `COMMENT`, `--` introduces a line comment on a hidden
    /// channel when it is followed by a space (`-- text`) or immediately by an
    /// end-of-line/EOF (bare `--\n` / `--<EOF>`); the callback (F-08-04) skips
    /// those, consuming to end of line. Only the rare `--` immediately followed
    /// by a non-space, non-newline char (e.g. `--foo`, two minus signs) is
    /// emitted as this token — matching ANTLR's `SYM_DOUBLE_DASH` fallback.
    #[token("--", line_comment)]
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
    // (optionally namespaced). Detected by the `-x-x.…vN` shape. The version
    // tail admits the grammar's `VERSION_ID` `-rc`/`-alpha` pre-release suffix
    // (`…v1.0.0-rc.2`), and the namespace prefix admits `-` per `NAMESPACE`/
    // `LABEL` (`NAME_CHAR` includes `-`) — both F-08-07.
    #[regex(
        r"([a-zA-Z][a-zA-Z0-9_.\-]*::)?[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z][a-zA-Z0-9_]*\.[a-zA-Z][a-zA-Z0-9_-]*\.v[0-9]+(\.[0-9]+)*((-rc|-alpha)(\.[0-9]+)?)?",
        |lex| lex.slice().to_owned()
    )]
    ArchetypeHrid(String),

    // A term code, e.g. `local::at0001`, `SNOMED-CT::1234|text|` or
    // `ISO_639-1::en`. Per `AqlLexer.g4` `TERM_CODE`, every code segment is
    // `TERM_CODE_CHAR+` where `TERM_CODE_CHAR = NAME_CHAR | '.'` and
    // `NAME_CHAR = WORD_CHAR | '-'` — so hyphens are legal in both the
    // terminology id and the code (F-08-01). A `::` is still required, so a
    // bare subtraction like `a-b` (no `::`) is unaffected.
    #[regex(
        r"[a-zA-Z0-9._\-]+(\([a-zA-Z0-9._\-]+\))?::[a-zA-Z0-9._\-]+(\|[^|\[\]]+\|)?",
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

/// Callback for the `--` token implementing `AqlLexer.g4` `COMMENT` (F-08-04).
///
/// The grammar treats `--` as a line comment (hidden channel, i.e. skipped)
/// when it is followed by a space and text, or immediately by end-of-line/EOF.
/// Anything else (`--` glued to a non-space token) is emitted as the
/// `SYM_DOUBLE_DASH` terminator. When skipping, the rest of the line is
/// consumed (the trailing newline is handled by the `WS` skip).
fn line_comment(lex: &mut logos::Lexer<Token>) -> logos::Filter<()> {
    let rem = lex.remainder();
    let is_comment = rem.is_empty() || rem.starts_with([' ', '\t', '\r', '\n']);
    if is_comment {
        let end = rem.find(['\r', '\n']).unwrap_or(rem.len());
        lex.bump(end);
        logos::Filter::Skip
    } else {
        logos::Filter::Emit(())
    }
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
    fn hyphenated_term_codes_lex_as_one_token() {
        // F-08-01: TERM_CODE_CHAR includes '-' (via NAME_CHAR), so hyphenated
        // terminology ids lex as a single TERM_CODE, not `id Minus code`.
        assert_eq!(
            toks("SNOMED-CT::1234"),
            vec![Token::TermCode("SNOMED-CT::1234".into())]
        );
        assert_eq!(
            toks("ISO_639-1::en"),
            vec![Token::TermCode("ISO_639-1::en".into())]
        );
        assert_eq!(
            toks("SNOMED-CT::1234|cyanosis|"),
            vec![Token::TermCode("SNOMED-CT::1234|cyanosis|".into())]
        );
        // The parenthesised terminology-version form also admits hyphens.
        assert_eq!(
            toks("snomed-ct(3.1)::3415004"),
            vec![Token::TermCode("snomed-ct(3.1)::3415004".into())]
        );
    }

    #[test]
    fn subtraction_is_not_a_term_code_regression() {
        // F-08-01 must not regress plain subtraction: without `::` there is no
        // TERM_CODE, so `a-b` and `a - 1` stay separate tokens.
        assert_eq!(
            toks("a-b"),
            vec![
                Token::Identifier("a".into()),
                Token::Minus,
                Token::Identifier("b".into()),
            ]
        );
        assert_eq!(
            toks("a - 1"),
            vec![
                Token::Identifier("a".into()),
                Token::Minus,
                Token::Integer("1".into()),
            ]
        );
    }

    #[test]
    fn archetype_hrid_version_suffixes_and_namespace_hyphen() {
        // F-08-07: `VERSION_ID` `-rc`/`-alpha` pre-release suffixes and a
        // hyphenated namespace both lex as a single ARCHETYPE_HRID.
        assert_eq!(
            toks("openEHR-EHR-OBSERVATION.blood_pressure.v1.0.0-rc.2"),
            vec![Token::ArchetypeHrid(
                "openEHR-EHR-OBSERVATION.blood_pressure.v1.0.0-rc.2".into()
            )]
        );
        assert_eq!(
            toks("openEHR-EHR-OBSERVATION.blood_pressure.v2-alpha"),
            vec![Token::ArchetypeHrid(
                "openEHR-EHR-OBSERVATION.blood_pressure.v2-alpha".into()
            )]
        );
        assert_eq!(
            toks("org-x::openEHR-EHR-OBSERVATION.blood_pressure.v1"),
            vec![Token::ArchetypeHrid(
                "org-x::openEHR-EHR-OBSERVATION.blood_pressure.v1".into()
            )]
        );
    }

    #[test]
    fn line_comments_are_skipped() {
        // F-08-04: `-- text` to end of line is a comment (skipped).
        assert_eq!(
            toks("SELECT c -- trailing comment\nFROM COMPOSITION c"),
            vec![
                Token::Select,
                Token::Identifier("c".into()),
                Token::From,
                Token::Identifier("COMPOSITION".into()),
                Token::Identifier("c".into()),
            ]
        );
        // A bare `--` at end-of-input is a comment too (skipped).
        assert_eq!(toks("c --"), vec![Token::Identifier("c".into())]);
        assert_eq!(toks("c --\n"), vec![Token::Identifier("c".into())]);
        // `--` glued to a non-space token is the `SYM_DOUBLE_DASH` terminator.
        assert_eq!(
            toks("--foo"),
            vec![Token::DoubleDash, Token::Identifier("foo".into())]
        );
    }

    #[test]
    fn utf_bom_is_skipped() {
        // F-08-13: a leading BOM is skipped, not a lex error.
        assert_eq!(toks("\u{feff}SELECT"), vec![Token::Select]);
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

//! ADL2 lexer — a `logos` tokenizer transcribed from the authoritative
//! `base_lexer.g4` + `adl_keywords.g4` (vendored at `vendor/grammar/`). No
//! ANTLR runtime: the grammar is the spec, this is a hand-written DFA lexer.
//!
//! The token set is the *union* consumed by the outer artefact grammar
//! (`adl2.g4`), ODIN (`odin.g4`/`odin_values.g4`), cADL
//! (`cadl2.g4`/`cadl2_primitives.g4`) and the rules sub-syntax
//! (`base_expressions.g4`) — because a whole ADL2 source file is lexed once as
//! a single stream. The outer parser only *parses* the identification header and the
//! ODIN sections; the cADL definition and rules bodies are captured as raw
//! spans, so their tokens only need to *lex* here, not classify perfectly.
//!
//! Faithfulness notes (each a deliberate, spec-cited decision):
//! - `// NOTE:` Section keywords (`language`, `definition`, …) are NOT lexed as
//!   dedicated tokens. `base_lexer`/`adl_keywords` anchor them with a leading
//!   `'\n'` precisely so an un-anchored occurrence stays an `ALPHA_LC_ID`
//!   (`adl_keywords.g4` `SYM_LANGUAGE : '\n'[Ll]…`). We reproduce that by
//!   lexing them as [`Token::AlphaLcId`] and detecting section headers in the
//!   outer parser by column-0 position (`odin` keys like `language = <…>` are
//!   indented, so they never read as headers). This also makes multi-line
//!   strings safe: a `STRING` token maximally munches across newlines, so a
//!   section keyword appearing inside a quoted value can never be mistaken for
//!   a header.
//! - `// NOTE:` Booleans are accepted in the canonical `True`/`False` and the
//!   all-lower `true`/`false` forms; the grammar's fully case-insensitive
//!   `SYM_TRUE`/`SYM_FALSE` (`base_lexer.g4`) is a superset — an all-caps
//!   `TRUE` lexes as an identifier here. ODIN corpora use the canonical forms.
//! - `// NOTE:` A leading UTF-8 BOM is skipped though `master03` forbids it;
//!   18 vendored ADL2 corpus sources carry one, so tolerating it is the
//!   pragmatic superset (a rejection would be a lexer-level FAIL the corpus
//!   does not intend).
//! - `// NOTE:` Duration *constraint* patterns (`base_lexer.g4`
//!   `DURATION_CONSTRAINT_PATTERN`, letters only) can be identical to an
//!   uppercase identifier (`PWD`); a higher token priority classifies the
//!   real pattern while longer type names (`POINT_EVENT`) win by length. Date/
//!   time constraint patterns contain `-`/`:` and never collide.

use logos::Logos;

/// A lexed token together with its byte span in the original source.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    /// The token.
    pub token: Token,
    /// Byte range of the token in the original source.
    pub span: std::ops::Range<usize>,
}

/// An ADL2 token. Text-bearing variants hold the owned source slice
/// (verbatim, including any delimiters — the parser strips/decodes them).
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip "\u{feff}")] // leading UTF-8 BOM (see module NOTE)
#[logos(skip r"[ \t\r\n]+")] // WS / LINE (base_lexer.g4)
#[logos(skip(r"--[^\n]*", allow_greedy = true))] // CMT_LINE `-- … EOL` incl. the `----…` overlay separator (base_lexer.g4)
pub enum Token {
    // ── ADL / cADL keywords (adl_keywords.g4); text + unicode symbol forms
    //    fold to one variant. Exact-literal `#[token]`s outrank the
    //    ALPHA_*_ID regexes by logos priority, so keywords beat identifiers. ──
    /// `matches` / `is_in` / `∈` (`SYM_MATCHES`).
    #[token("matches")]
    #[token("is_in")]
    #[token("\u{2208}")]
    SymMatches,
    /// `∉` — "not in" (`~matches` is lexed as `SymNot` `SymMatches`).
    #[token("\u{2209}")]
    SymNotMatches,
    /// `and` / `∧` (`SYM_AND`).
    #[token("and")]
    #[token("\u{2227}")]
    SymAnd,
    /// `or` / `∨` (`SYM_OR`).
    #[token("or")]
    #[token("\u{2228}")]
    SymOr,
    /// `xor` (`SYM_XOR`).
    #[token("xor")]
    SymXor,
    /// `not` / `~` / `∼` / `¬` / `!` (`SYM_NOT`).
    #[token("not")]
    #[token("~")]
    #[token("\u{223C}")]
    #[token("\u{00AC}")]
    #[token("!")]
    SymNot,
    /// `implies` / `®` / `->` (`SYM_IMPLIES`).
    #[token("implies")]
    #[token("\u{00AE}")]
    #[token("->")]
    SymImplies,
    /// `for_all` / `∀` (`SYM_FOR_ALL`).
    #[token("for_all")]
    #[token("\u{2200}")]
    SymForAll,
    /// `exists` (`SYM_EXISTS`).
    #[token("exists")]
    SymExists,
    /// `there_exists` / `∃` (`SYM_THERE_EXISTS`).
    #[token("there_exists")]
    #[token("\u{2203}")]
    SymThereExists,
    /// `occurrences` (`SYM_OCCURRENCES`).
    #[token("occurrences")]
    SymOccurrences,
    /// `existence` (`SYM_EXISTENCE`).
    #[token("existence")]
    SymExistence,
    /// `cardinality` (`SYM_CARDINALITY`).
    #[token("cardinality")]
    SymCardinality,
    /// `ordered` (`SYM_ORDERED`).
    #[token("ordered")]
    SymOrdered,
    /// `unordered` (`SYM_UNORDERED`).
    #[token("unordered")]
    SymUnordered,
    /// `unique` (`SYM_UNIQUE`).
    #[token("unique")]
    SymUnique,
    /// `use_node` (`SYM_USE_NODE`).
    #[token("use_node")]
    SymUseNode,
    /// `use_archetype` (`SYM_USE_ARCHETYPE`).
    #[token("use_archetype")]
    SymUseArchetype,
    /// `allow_archetype` (`SYM_ALLOW_ARCHETYPE`).
    #[token("allow_archetype")]
    SymAllowArchetype,
    /// `include` (`SYM_INCLUDE`).
    #[token("include")]
    SymInclude,
    /// `exclude` (`SYM_EXCLUDE`).
    #[token("exclude")]
    SymExclude,
    /// `after` (`SYM_AFTER`).
    #[token("after")]
    SymAfter,
    /// `before` (`SYM_BEFORE`).
    #[token("before")]
    SymBefore,
    /// `closed` (`SYM_CLOSED`).
    #[token("closed")]
    SymClosed,
    /// `then` (`SYM_THEN`).
    #[token("then")]
    SymThen,

    // ── boolean word symbols (base_lexer.g4 SYM_TRUE / SYM_FALSE) ──
    /// `True` / `true`.
    #[token("True")]
    #[token("true")]
    SymTrue,
    /// `False` / `false`.
    #[token("False")]
    #[token("false")]
    SymFalse,

    // ── codes (base_lexer.g4). Higher priority than ALPHA_*_ID so `id1`/`at3`
    //    classify as codes, not identifiers. ──
    /// `id1` `.1`* — the AOM root id-code (`ROOT_ID_CODE`).
    #[regex(r"id1(\.1)*", |lex| lex.slice().to_owned(), priority = 22)]
    RootIdCode(String),
    /// `id` `CODE_STR` — an id-code (`ID_CODE`).
    #[regex(r"id(0|[1-9][0-9]*)(\.(0|[1-9][0-9]*))*", |lex| lex.slice().to_owned(), priority = 20)]
    IdCode(String),
    /// `at` `CODE_STR` — an at-code / term code (`AT_CODE`). Permissive on
    /// leading zeros (`at0000`, `at0.1`) to admit legacy corpus at-codes.
    #[regex(r"at[0-9]+(\.[0-9]+)*", |lex| lex.slice().to_owned(), priority = 20)]
    AtCode(String),
    /// `ac` `CODE_STR` — a value-set (constraint) code (`AC_CODE`).
    #[regex(r"ac[0-9]+(\.[0-9]+)*", |lex| lex.slice().to_owned(), priority = 20)]
    AcCode(String),

    // ── archetype identifiers (base_lexer.g4 ARCHETYPE_HRID / ARCHETYPE_REF) ──
    /// An archetype HRID or version-partial reference — one token covering both
    /// `ARCHETYPE_HRID` (full `vN.M.P`) and `ARCHETYPE_REF` (partial `vN`); the
    /// parser resolves which. e.g. `openEHR-EHR-OBSERVATION.blood_pressure.v1`.
    #[regex(
        r"([a-zA-Z][a-zA-Z0-9_.\-]*::)?[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z][a-zA-Z0-9_]*-[a-zA-Z][a-zA-Z0-9_]*\.[a-zA-Z][a-zA-Z0-9_-]*\.v[0-9]+(\.[0-9]+)*((-rc|-alpha|-beta)(\.[0-9]+)?)?",
        |lex| lex.slice().to_owned()
    )]
    ArchetypeId(String),
    /// `DIGIT+ '.' DIGIT+ '.' DIGIT+ [pre-release]` — a 3-part version
    /// (`VERSION_ID`); e.g. an `adl_version`/`rm_release` meta value. The
    /// 3+-part form also carries a dotted-numeric OID `uid` (e.g.
    /// `2.4.34.666.7.2`) — `master07.05` admits an OID or GUID `uid`, and both
    /// arrive here as a single string token the identification parser records
    /// verbatim.
    #[regex(r"[0-9]+\.[0-9]+\.[0-9]+(\.[0-9]+)*((-rc|-alpha|-beta)(\.[0-9]+)?)?", |lex| lex.slice().to_owned())]
    VersionId(String),
    /// A GUID (`base_lexer.g4 GUID`).
    #[regex(r"[0-9a-fA-F]+-[0-9a-fA-F]+-[0-9a-fA-F]+-[0-9a-fA-F]+-[0-9a-fA-F]+", |lex| lex.slice().to_owned())]
    Guid(String),

    // ── ISO8601 date/time/duration VALUES (base_lexer.g4) ──
    /// `ISO8601_DATE_TIME` (with optional partial `??` fields / timezone).
    #[regex(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}(:[0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|:\?\?:\?\?)?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned()
    )]
    Iso8601DateTime(String),
    /// `ISO8601_DATE` (with optional partial `??` fields).
    #[regex(
        r"[0-9]{4}-([0-9]{2}(-([0-9]{2}|\?\?))?|\?\?-\?\?)",
        |lex| lex.slice().to_owned()
    )]
    Iso8601Date(String),
    /// `ISO8601_TIME` (with optional partial `??` fields / timezone).
    #[regex(
        r"[0-9]{2}:([0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|\?\?:\?\?)(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned()
    )]
    Iso8601Time(String),
    /// `ISO8601_DURATION`, requiring at least one component (never bare `P`).
    #[regex(
        r"-?P([0-9]+[YyMmWwDd])+(T([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|T([0-9]+[Hh])?([0-9]+[Mm])?([0-9]+([.,][0-9]+)?[Ss])?)?|-?PT([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|-?PT([0-9]+[Hh])([0-9]+[Mm])?|-?PT[0-9]+[Mm]",
        |lex| lex.slice().to_owned()
    )]
    Iso8601Duration(String),

    // ── constraint PATTERNS (base_lexer.g4) ──
    /// `DATE_TIME_CONSTRAINT_PATTERN` — e.g. `yyyy-mm-ddThh:mm:ss`.
    #[regex(
        r"(yyyy|YYYY|yyy|YYY)-(mm|MM|\?\?|XX|xx)-(dd|DD|\?\?|XX|xx)T(hh|HH|\?\?|XX|xx):(mm|MM|\?\?|XX|xx):(ss|SS|\?\?|XX|xx)(\u{00B1}(hh|HH)(:?(mm|MM))?|Z)?",
        |lex| lex.slice().to_owned()
    )]
    DateTimeConstraintPattern(String),
    /// `DATE_CONSTRAINT_PATTERN` — e.g. `yyyy-mm-??`.
    #[regex(
        r"(yyyy|YYYY|yyy|YYY)-(mm|MM|\?\?|XX|xx)-(dd|DD|\?\?|XX|xx)",
        |lex| lex.slice().to_owned()
    )]
    DateConstraintPattern(String),
    /// `TIME_CONSTRAINT_PATTERN` — e.g. `hh:??:XX`.
    #[regex(
        r"(hh|HH|\?\?|XX|xx):(mm|MM|\?\?|XX|xx):(ss|SS|\?\?|XX|xx)(\u{00B1}(hh|HH)(:?(mm|MM))?|Z)?",
        |lex| lex.slice().to_owned()
    )]
    TimeConstraintPattern(String),
    /// `DURATION_CONSTRAINT_PATTERN` — letters only, e.g. `PYMD`, `PWD`, `PT`.
    #[regex(
        r"P([Yy][Mm]?[Ww]?[Dd]?|[Mm][Ww]?[Dd]?|[Ww][Dd]?|[Dd])(T([Hh][Mm]?[Ss]?|[Mm][Ss]?|[Ss]))?|PT([Hh][Mm]?[Ss]?|[Mm][Ss]?|[Ss])",
        |lex| lex.slice().to_owned(),
        priority = 20
    )]
    DurationConstraintPattern(String),

    // ── composed primitives (base_lexer.g4) ──
    /// A delimited contained regexp `{ /re/ }` / `{ ^re^ }` with optional
    /// `;"assumed"` (`CONTAINED_REGEXP`).
    #[regex(
        r#"\{[ \t\r]*(/([^/\r\n\\]|\\.)+/|\^([^\^\r\n\\]|\\.)+\^)[ \t\r]*(;[ \t\r]*"([^"\\]|\\.)*")?[ \t\r]*\}"#,
        |lex| lex.slice().to_owned()
    )]
    ContainedRegexp(String),
    /// A term-code reference `[terminology(ver)?::code]` (`TERM_CODE_REF`).
    #[regex(r"\[[a-zA-Z0-9._\-]+(\([a-zA-Z0-9._\-]+\))?::[a-zA-Z0-9._\-]+\]", |lex| lex.slice().to_owned())]
    TermCodeRef(String),
    /// An embedded URI `<scheme:…>` (`EMBEDDED_URI`).
    #[regex(r"<[ \t\r\n]*[a-zA-Z][a-zA-Z0-9+.\-]*:[^>]*>", |lex| lex.slice().to_owned())]
    EmbeddedUri(String),
    /// An ADL path (`base_lexer.g4 ADL_PATH`): absolute `(/seg)+` or relative
    /// `seg(/seg)+`. Each `ADL_PATH_SEGMENT` is `ALPHA_LC_ID ('[' predicate
    /// ']')?` — the segment head is **lower-case-initial** per the grammar, so
    /// an upper-initial run (e.g. the duration `PWD/PT0S` pattern/value form)
    /// is never mis-lexed as a path.
    #[regex(
        r"(/[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+|[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?(/[a-z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+",
        |lex| lex.slice().to_owned()
    )]
    AdlPath(String),

    // ── atomic primitives (base_lexer.g4) ──
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

    // ── rule/assertion variable ids (base_lexer.g4) ──
    /// `$name/path` (`VARIABLE_WITH_PATH`).
    #[regex(r"\$[a-z][a-zA-Z0-9_]*(/[a-zA-Z][a-zA-Z0-9_]*(\[[^\]\r\n]*\])?)+", |lex| lex.slice().to_owned())]
    VariableWithPath(String),
    /// `$name` (`VARIABLE_ID`).
    #[regex(r"\$[a-z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    VariableId(String),

    // ── identifiers (base_lexer.g4) ──
    /// `ALPHA_UC_ID` — upper-initial identifier (RM type ids, ODIN keys).
    #[regex(r"[A-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    AlphaUcId(String),
    /// `ALPHA_LC_ID` — lower-initial identifier (attribute ids, ODIN keys,
    /// section/artefact keywords).
    #[regex(r"[a-z][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    AlphaLcId(String),
    /// `ALPHA_UNDERSCORE_ID` — `_`-initial identifier (`_default`, meta ids).
    #[regex(r"_[a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    AlphaUnderscoreId(String),

    // ── symbols (base_lexer.g4) ──
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
    /// `{`
    #[token("{")]
    LCurly,
    /// `}`
    #[token("}")]
    RCurly,
    /// `,` (`SYM_COMMA`).
    #[token(",")]
    SymComma,
    /// `;` (`SYM_SEMI_COLON`).
    #[token(";")]
    SymSemiColon,
    /// `:=` / `::=` (`SYM_ASSIGNMENT`).
    #[token(":=")]
    #[token("::=")]
    SymAssignment,
    /// `:` — colon (rule declarations, quantifier binding).
    #[token(":")]
    SymColon,
    /// `/=` / `!=` / `≠` (`SYM_NE`).
    #[token("/=")]
    #[token("!=")]
    #[token("\u{2260}")]
    SymNe,
    /// `=` (`SYM_EQ`).
    #[token("=")]
    SymEq,
    /// `<=` / `≤` (`SYM_LE`).
    #[token("<=")]
    #[token("\u{2264}")]
    SymLe,
    /// `>=` / `≥` (`SYM_GE`).
    #[token(">=")]
    #[token("\u{2265}")]
    SymGe,
    /// `>` (`SYM_GT`).
    #[token(">")]
    SymGt,
    /// `<` (`SYM_LT`).
    #[token("<")]
    SymLt,
    /// `...` (`SYM_LIST_CONTINUE`).
    #[token("...")]
    SymListContinue,
    /// `..` (`SYM_IVL_SEP`).
    #[token("..")]
    SymIvlSep,
    /// `|` (`SYM_IVL_DELIM`).
    #[token("|")]
    SymIvlDelim,
    /// `+/-` / `±` (`SYM_PLUS_OR_MINUS`).
    #[token("+/-")]
    #[token("\u{00B1}")]
    SymPlusOrMinus,
    /// `+` (`SYM_PLUS`).
    #[token("+")]
    SymPlus,
    /// `-` (`SYM_MINUS`).
    #[token("-")]
    SymMinus,
    /// `*` / `∗` (`SYM_STAR` — the "any" wildcard / multiplication).
    #[token("*")]
    #[token("\u{2217}")]
    SymStar,
    /// `/` (`SYM_SLASH` — division / bare path root).
    #[token("/")]
    SymSlash,
    /// `%` (`SYM_PERCENT`).
    #[token("%")]
    SymPercent,
    /// `^` (`SYM_CARAT` — exponent).
    #[token("^")]
    SymCarat,
    /// `@` — terminology binding separator in the OPT `[acN@terminology]` form.
    #[token("@")]
    SymAt,
}

/// Validate `master03` string escapes and return the raw slice (delimiters
/// retained; the parser decodes).
///
/// Illegal escapes (anything other than `\r \n \t \\ \" \'` or a
/// `\u` + 4/8 hex-digit sequence) fail the lex, per
/// `ADL2/master03-file_encoding.adoc`.
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
                    // \uHHHH (4) or \uHHHHHHHH (8) hex digits.
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

/// Lex `src` into a spanned token vector.
///
/// # Errors
/// Returns a [`SyntaxError`] ([`SyntaxErrorCode::Sunk`]) at the byte span of
/// the first token that fails to lex (an unrecognised character or an illegal
/// string escape).
pub fn lex(src: &str) -> Result<Vec<Spanned>, crate::error::SyntaxError> {
    let mut out = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(res) = lexer.next() {
        match res {
            Ok(token) => out.push(Spanned {
                token,
                span: lexer.span(),
            }),
            Err(()) => {
                return Err(crate::error::SyntaxError::at(
                    crate::error::SyntaxErrorCode::Sunk,
                    format!("unrecognised token {:?}", lexer.slice()),
                    lexer.span(),
                    src,
                ));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::panic)] // test assertions panic by design
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Token> {
        lex(src)
            .unwrap_or_else(|e| panic!("lex failed: {e}"))
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn codes_and_identifiers() {
        assert_eq!(toks("id1"), vec![Token::RootIdCode("id1".into())]);
        assert_eq!(toks("id1.1"), vec![Token::RootIdCode("id1.1".into())]);
        assert_eq!(toks("id2"), vec![Token::IdCode("id2".into())]);
        assert_eq!(toks("id0.1"), vec![Token::IdCode("id0.1".into())]);
        assert_eq!(toks("at0000"), vec![Token::AtCode("at0000".into())]);
        assert_eq!(toks("at0.1"), vec![Token::AtCode("at0.1".into())]);
        assert_eq!(toks("ac1"), vec![Token::AcCode("ac1".into())]);
        assert_eq!(
            toks("OBSERVATION"),
            vec![Token::AlphaUcId("OBSERVATION".into())]
        );
        assert_eq!(toks("items"), vec![Token::AlphaLcId("items".into())]);
        // `id`/`at` without a code are plain identifiers.
        assert_eq!(toks("identity"), vec![Token::AlphaLcId("identity".into())]);
    }

    #[test]
    fn archetype_and_version_ids() {
        assert_eq!(
            toks("openehr-TEST_PKG-WHOLE.most_minimal.v2.0.0"),
            vec![Token::ArchetypeId(
                "openehr-TEST_PKG-WHOLE.most_minimal.v2.0.0".into()
            )]
        );
        // partial version (ARCHETYPE_REF shape) folds into the same token.
        assert_eq!(
            toks("openehr-TASK_PLANNING-TASK_PLAN.good_include.v0"),
            vec![Token::ArchetypeId(
                "openehr-TASK_PLANNING-TASK_PLAN.good_include.v0".into()
            )]
        );
        assert_eq!(toks("2.0.5"), vec![Token::VersionId("2.0.5".into())]);
        assert_eq!(toks("1.0.2"), vec![Token::VersionId("1.0.2".into())]);
    }

    #[test]
    fn iso_values_and_partials() {
        assert_eq!(
            toks("2004-06-01"),
            vec![Token::Iso8601Date("2004-06-01".into())]
        );
        assert_eq!(toks("2004-06"), vec![Token::Iso8601Date("2004-06".into())]);
        assert_eq!(
            toks("2004-06-??"),
            vec![Token::Iso8601Date("2004-06-??".into())]
        );
        assert_eq!(
            toks("2004-06-01T10:30:00"),
            vec![Token::Iso8601DateTime("2004-06-01T10:30:00".into())]
        );
        assert_eq!(
            toks("10:30:00"),
            vec![Token::Iso8601Time("10:30:00".into())]
        );
        assert_eq!(toks("P1Y2M"), vec![Token::Iso8601Duration("P1Y2M".into())]);
        assert_eq!(toks("PT30M"), vec![Token::Iso8601Duration("PT30M".into())]);
        assert_eq!(toks("P0W"), vec![Token::Iso8601Duration("P0W".into())]);
    }

    #[test]
    fn constraint_patterns() {
        assert_eq!(
            toks("yyyy-mm-dd"),
            vec![Token::DateConstraintPattern("yyyy-mm-dd".into())]
        );
        assert_eq!(
            toks("yyyy-??-XX"),
            vec![Token::DateConstraintPattern("yyyy-??-XX".into())]
        );
        assert_eq!(
            toks("hh:mm:ss"),
            vec![Token::TimeConstraintPattern("hh:mm:ss".into())]
        );
        assert_eq!(
            toks("yyyy-mm-ddThh:mm:ss"),
            vec![Token::DateTimeConstraintPattern(
                "yyyy-mm-ddThh:mm:ss".into()
            )]
        );
        assert_eq!(
            toks("PYMD"),
            vec![Token::DurationConstraintPattern("PYMD".into())]
        );
        // a real type name that starts with `P` but has other letters wins by
        // length as an identifier.
        assert_eq!(
            toks("POINT_EVENT"),
            vec![Token::AlphaUcId("POINT_EVENT".into())]
        );
    }

    #[test]
    fn interval_and_range_symbols() {
        // `1..5` must lex as INTEGER SYM_IVL_SEP INTEGER, not a REAL.
        assert_eq!(
            toks("1..5"),
            vec![
                Token::Integer("1".into()),
                Token::SymIvlSep,
                Token::Integer("5".into()),
            ]
        );
        assert_eq!(toks("1.5"), vec![Token::Real("1.5".into())]);
        assert_eq!(
            toks("|>=0.0..<10.0|"),
            vec![
                Token::SymIvlDelim,
                Token::SymGe,
                Token::Real("0.0".into()),
                Token::SymIvlSep,
                Token::SymLt,
                Token::Real("10.0".into()),
                Token::SymIvlDelim,
            ]
        );
    }

    #[test]
    fn term_code_ref_and_embedded_uri() {
        assert_eq!(
            toks("[ISO_639-1::en]"),
            vec![Token::TermCodeRef("[ISO_639-1::en]".into())]
        );
        assert_eq!(
            toks("<http://loinc.org/id/9272-6>"),
            vec![Token::EmbeddedUri("<http://loinc.org/id/9272-6>".into())]
        );
        // a `<[…]>` value block is NOT a URI: `<` then term code then `>`.
        assert_eq!(
            toks("<[ISO_639-1::en]>"),
            vec![
                Token::SymLt,
                Token::TermCodeRef("[ISO_639-1::en]".into()),
                Token::SymGt,
            ]
        );
    }

    #[test]
    fn brackets_with_codes_are_not_term_codes() {
        // `[id2]` / `[ac1]` have no `::` so they split into bracket + code.
        assert_eq!(
            toks("[id2]"),
            vec![
                Token::LBracket,
                Token::IdCode("id2".into()),
                Token::RBracket
            ]
        );
        assert_eq!(
            toks("[ac1]"),
            vec![
                Token::LBracket,
                Token::AcCode("ac1".into()),
                Token::RBracket
            ]
        );
    }

    #[test]
    fn unicode_operators_lex() {
        assert_eq!(toks("\u{2208}"), vec![Token::SymMatches]);
        assert_eq!(toks("\u{2227}"), vec![Token::SymAnd]);
        assert_eq!(toks("\u{2203}"), vec![Token::SymThereExists]);
        assert_eq!(toks("matches"), vec![Token::SymMatches]);
        assert_eq!(toks("*"), vec![Token::SymStar]);
        assert_eq!(toks("\u{2217}"), vec![Token::SymStar]);
    }

    #[test]
    fn comments_and_overlay_separator_are_skipped() {
        assert_eq!(
            toks("id1 -- a trailing comment\nitems"),
            vec![
                Token::RootIdCode("id1".into()),
                Token::AlphaLcId("items".into())
            ]
        );
        assert_eq!(
            toks("--------------\ntemplate_overlay"),
            vec![Token::AlphaLcId("template_overlay".into())]
        );
    }

    #[test]
    fn strings_and_escapes() {
        assert_eq!(
            toks(r#""a\"x'c\\d""#),
            vec![Token::String(r#""a\"x'c\\d""#.into())]
        );
        // multi-line string.
        assert_eq!(
            toks("\"line1\nline2\""),
            vec![Token::String("\"line1\nline2\"".into())]
        );
        // illegal escape (`\d`) is a lex error per master03.
        assert!(lex(r#""bad \d escape""#).is_err());
        // BOM is skipped, not an error.
        assert_eq!(
            toks("\u{feff}archetype"),
            vec![Token::AlphaLcId("archetype".into())]
        );
    }

    #[test]
    fn boolean_word_symbols() {
        assert_eq!(toks("True"), vec![Token::SymTrue]);
        assert_eq!(toks("False"), vec![Token::SymFalse]);
        assert_eq!(toks("true"), vec![Token::SymTrue]);
    }
}

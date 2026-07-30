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
//! - `// NOTE:` Keywords are lexed CASE-INSENSITIVELY, as both normative
//!   lexical specifications require: `adl_keywords.g4` spells every keyword
//!   `[Mm][Aa][Tt][Cc][Hh][Ee][Ss]`-style, and the ADL 1.4 chapter's own
//!   lexical specification does the same
//!   (`ADL1.4/master05-cadl.adoc` §Symbols L1326-1354, incl.
//!   `[Ii][Nn][Ff][Ii][Nn][Ii][Tt][Yy] -> SYM_INFINITY`). Identifiers cannot
//!   collide: an RM type id or attribute id that IS a keyword in some casing
//!   (`MATCHES`, `ORDERED`) would be unlexable as an identifier in ANY casing
//!   under those lexers too, so the keyword reading is the spec's own.
//!   Booleans follow the same rule (`base_lexer.g4` `SYM_TRUE`,
//!   `master05` L1337), so `TRUE`/`fAlSe` lex as booleans. The single
//!   exception is `there_exists`, which both lexers spell case-SENSITIVELY —
//!   see the note on [`Token::SymThereExists`].
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
    #[token("matches", ignore(case))]
    #[token("is_in", ignore(case))]
    #[token("\u{2208}")]
    SymMatches,
    /// `∉` — "not in" (`~matches` is lexed as `SymNot` `SymMatches`).
    #[token("\u{2209}")]
    SymNotMatches,
    /// `and` / `∧` (`SYM_AND`).
    #[token("and", ignore(case))]
    #[token("\u{2227}")]
    SymAnd,
    /// `or` / `∨` (`SYM_OR`).
    #[token("or", ignore(case))]
    #[token("\u{2228}")]
    SymOr,
    /// `xor` (`SYM_XOR`).
    #[token("xor", ignore(case))]
    SymXor,
    /// `not` / `~` / `∼` / `¬` / `!` (`SYM_NOT`).
    #[token("not", ignore(case))]
    #[token("~")]
    #[token("\u{223C}")]
    #[token("\u{00AC}")]
    #[token("!")]
    SymNot,
    /// `implies` / `®` / `->` (`SYM_IMPLIES`).
    #[token("implies", ignore(case))]
    #[token("\u{00AE}")]
    #[token("->")]
    SymImplies,
    /// `for_all` / `∀` (`SYM_FOR_ALL`).
    #[token("for_all", ignore(case))]
    #[token("\u{2200}")]
    SymForAll,
    /// `exists` (`SYM_EXISTS`).
    #[token("exists", ignore(case))]
    SymExists,
    /// `there_exists` / `∃` (`SYM_THERE_EXISTS`).
    ///
    /// The ONE keyword that is NOT case-folded: `adl_keywords.g4` spells it as
    /// the plain literal `SYM_THERE_EXISTS: 'there_exists' | '∃' ;` rather than
    /// in the `[Tt][Hh]…` form every other keyword on that list uses, and the
    /// ADL 1.4 chapter's own §Symbols lexer has no `there_exists` rule at all
    /// (`ADL1.4/master05-cadl.adoc` L1329-1340 stops at `SYM_EXISTS`). Folding
    /// it anyway would swallow `There_Exists`, a well-formed `ALPHA_UC_ID`
    /// under both lexers.
    #[token("there_exists")]
    #[token("\u{2203}")]
    SymThereExists,
    /// `occurrences` (`SYM_OCCURRENCES`).
    #[token("occurrences", ignore(case))]
    SymOccurrences,
    /// `existence` (`SYM_EXISTENCE`).
    #[token("existence", ignore(case))]
    SymExistence,
    /// `cardinality` (`SYM_CARDINALITY`).
    #[token("cardinality", ignore(case))]
    SymCardinality,
    /// `ordered` (`SYM_ORDERED`).
    #[token("ordered", ignore(case))]
    SymOrdered,
    /// `unordered` (`SYM_UNORDERED`).
    #[token("unordered", ignore(case))]
    SymUnordered,
    /// `unique` (`SYM_UNIQUE`).
    #[token("unique", ignore(case))]
    SymUnique,
    /// `infinity` (`SYM_INFINITY`) — the unbounded interval endpoint of
    /// `ADL1.4/master05-cadl.adoc` §Keywords L50 + §Symbols L1349, whose own
    /// worked example is `rate matches {|0..infinity|}` (L771).
    #[token("infinity", ignore(case))]
    SymInfinity,
    /// `use_node` (`SYM_USE_NODE`).
    #[token("use_node", ignore(case))]
    SymUseNode,
    /// `use_archetype` (`SYM_USE_ARCHETYPE`).
    #[token("use_archetype", ignore(case))]
    SymUseArchetype,
    /// `allow_archetype` (`SYM_ALLOW_ARCHETYPE`).
    #[token("allow_archetype", ignore(case))]
    SymAllowArchetype,
    /// `include` (`SYM_INCLUDE`).
    #[token("include", ignore(case))]
    SymInclude,
    /// `exclude` (`SYM_EXCLUDE`).
    #[token("exclude", ignore(case))]
    SymExclude,
    /// `after` (`SYM_AFTER`).
    #[token("after", ignore(case))]
    SymAfter,
    /// `before` (`SYM_BEFORE`).
    #[token("before", ignore(case))]
    SymBefore,
    /// `closed` (`SYM_CLOSED`).
    #[token("closed", ignore(case))]
    SymClosed,
    /// `then` (`SYM_THEN`).
    #[token("then", ignore(case))]
    SymThen,

    // ── boolean word symbols (base_lexer.g4 SYM_TRUE / SYM_FALSE) ──
    /// `True` / `true` (case-insensitive per `base_lexer.g4` `SYM_TRUE`).
    #[token("true", ignore(case))]
    SymTrue,
    /// `False` / `false` (case-insensitive per `base_lexer.g4` `SYM_FALSE`).
    #[token("false", ignore(case))]
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
    //
    // The three date/time VALUE tokens carry an explicit high priority so they
    // win every tie against the constraint-PATTERN tokens below, whose fields
    // admit literal date/time numbers (`ADL1.4/master05-cadl.adoc` §Patterns
    // L894). Where a text is BOTH a legal value and a legal all-literal
    // pattern (`1995-??-??`), the value reading is the established one and the
    // pattern adds nothing — the pattern tokens keep the shapes only they
    // match (`1995-??-XX`, `1995-mm-dd`).
    /// `ISO8601_DATE_TIME` (with optional partial `??` fields / timezone).
    ///
    /// The `??`-partial family covers the whole set of
    /// `AM/docs/ADL1.4/master04-dadl` §Partial Date/Times, including the
    /// `yyyy-MM-ddT??:??:??`, `yyyy-MM-??T??:??:??` and `yyyy-??-??T??:??:??`
    /// forms — a whole-file lex failure here would reject an artefact whose
    /// ODIN sections `openehr_lang::odin` accepts.
    #[regex(
        r"[0-9]{4}-[0-9]{2}-[0-9]{2}T([0-9]{2}(:[0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|:\?\?:\?\?)?|\?\?:\?\?:\?\?)(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned(),
        priority = 30
    )]
    #[regex(
        r"[0-9]{4}-([0-9]{2}-\?\?|\?\?-\?\?)T\?\?:\?\?:\?\?(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned(),
        priority = 30
    )]
    Iso8601DateTime(String),
    /// `ISO8601_DATE` (with optional partial `??` fields).
    #[regex(
        r"[0-9]{4}-([0-9]{2}(-([0-9]{2}|\?\?))?|\?\?-\?\?)",
        |lex| lex.slice().to_owned(),
        priority = 30
    )]
    Iso8601Date(String),
    /// `ISO8601_TIME` (with optional partial `??` fields / timezone).
    #[regex(
        r"[0-9]{2}:([0-9]{2}(:([0-9]{2}([.,][0-9]+)?|\?\?))?|\?\?:\?\?)(Z|[+\-][0-9]{4})?",
        |lex| lex.slice().to_owned(),
        priority = 30
    )]
    Iso8601Time(String),
    /// `ISO8601_DURATION`, requiring at least one component (never bare `P`).
    #[regex(
        r"-?P([0-9]+[YyMmWwDd])+(T([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|T([0-9]+[Hh])?([0-9]+[Mm])?([0-9]+([.,][0-9]+)?[Ss])?)?|-?PT([0-9]+[HhMm])*[0-9]+([.,][0-9]+)?[Ss]|-?PT([0-9]+[Hh])([0-9]+[Mm])?|-?PT[0-9]+[Mm]",
        |lex| lex.slice().to_owned()
    )]
    Iso8601Duration(String),

    // ── constraint PATTERNS (base_lexer.g4 + the ADL 1.4 chapter's own
    //    lexical spec, `ADL1.4/master05-cadl.adoc` §Symbols L1415-1426) ──
    //
    // Three spec-grounded widenings over `base_lexer.g4`'s transcription, all
    // supersets that reject nothing the narrower form accepted:
    // 1. Every field also admits a LITERAL date/time number — master05 L894:
    //    "the 'yyyy' etc match strings can be replaced by literal date/time
    //    numbers. For example, `yyyy-??-XX` could be transformed into
    //    `1995-??-XX`".
    // 2. The timezone modifier admits the ASCII `+`/`-` forms, not only the
    //    literal `±` character — master05 §Patterns L852 ("the addition of a
    //    patterns such as `+hh:mm`, `+hhmm`, and `-hh`") and the
    //    <<timezone_constraints>> table L900-906, whose `±` column head is
    //    glossed "commencing with '+' or '-'".
    // 3. The date/time separator is `[T ]`, per the chapter's own
    //    `V_ISO8601_DATE_TIME_CONSTRAINT_PATTERN` (master05 L1422:
    //    `…[dD?X][dD?X][ T][hH?X][hH?X]:…`); `base_lexer.g4` L37 spells only
    //    `'T'`, so the space form is the 1.4 chapter's superset.
    //
    // The explicit low priority keeps the date/time VALUE tokens above winning
    // every equal-length tie (see the note on those tokens).
    /// `DATE_TIME_CONSTRAINT_PATTERN` — e.g. `yyyy-mm-ddThh:mm:ss`.
    #[regex(
        r"(yyyy|YYYY|yyy|YYY|[0-9]{4})-(mm|MM|\?\?|XX|xx|[0-9]{2})-(dd|DD|\?\?|XX|xx|[0-9]{2})[T ](hh|HH|\?\?|XX|xx|[0-9]{2}):(mm|MM|\?\?|XX|xx|[0-9]{2}):(ss|SS|\?\?|XX|xx|[0-9]{2})([+\-\u{00B1}](hh|HH)(:?(mm|MM))?|Z)?",
        |lex| lex.slice().to_owned(),
        priority = 3
    )]
    DateTimeConstraintPattern(String),
    /// `DATE_CONSTRAINT_PATTERN` — e.g. `yyyy-mm-??`, `1995-??-XX`.
    #[regex(
        r"(yyyy|YYYY|yyy|YYY|[0-9]{4})-(mm|MM|\?\?|XX|xx|[0-9]{2})-(dd|DD|\?\?|XX|xx|[0-9]{2})",
        |lex| lex.slice().to_owned(),
        priority = 3
    )]
    DateConstraintPattern(String),
    /// `TIME_CONSTRAINT_PATTERN` — e.g. `hh:??:XX`, `hh:mm:ss+hh:mm`.
    #[regex(
        r"(hh|HH|\?\?|XX|xx|[0-9]{2}):(mm|MM|\?\?|XX|xx|[0-9]{2}):(ss|SS|\?\?|XX|xx|[0-9]{2})([+\-\u{00B1}](hh|HH)(:?(mm|MM))?|Z)?",
        |lex| lex.slice().to_owned(),
        priority = 3
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
    /// A single-quoted `CHARACTER`. An escaped character must be one of the
    /// six legal quoted forms (see [`validate_char`]).
    #[regex(r"'([^'\\\r\n]|\\.)'", validate_char)]
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
/// Validate a `CHARACTER` token: an escaped character must be one of the six
/// legal quoted forms `\r \n \t \\ \" \'` — "Any other character combination
/// starting with a backslash is illegal" (`ADL2/master03-file_encoding.adoc`
/// §Special Character Sequences). The `\uHHHH` forms cannot fit the
/// single-character token.
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
                    // \uHHHH (4) or \uHHHHHHHH (8) hex digits.
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

/// Lex `src` into a spanned token vector.
///
/// # Errors
/// Returns a [`SyntaxError`](crate::error::SyntaxError) ([`SyntaxErrorCode::Sunk`](crate::error::SyntaxErrorCode::Sunk)) at the byte span of
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
    fn character_escapes() {
        // The six legal quoted forms and a plain/unicode character lex
        // (`ADL2/master03-file_encoding.adoc` §Special Character Sequences).
        for ok in [
            r"'\n'", r"'\t'", r"'\r'", r"'\\'", r#"'\"'"#, r"'\''", "'x'", "'ü'",
        ] {
            assert!(lex(ok).is_ok(), "legal character must lex: {ok}");
        }
        // "Any other character combination starting with a backslash is
        // illegal" — an unknown escape fails the lex.
        assert!(lex(r"'\q'").is_err());
        assert!(lex(r"'\d'").is_err());
    }

    #[test]
    fn boolean_word_symbols() {
        assert_eq!(toks("True"), vec![Token::SymTrue]);
        assert_eq!(toks("False"), vec![Token::SymFalse]);
        assert_eq!(toks("true"), vec![Token::SymTrue]);
    }

    /// `ADL1.4/master05-cadl.adoc` §Symbols L1326-1354 + `adl_keywords.g4`
    /// spell every keyword in the `[Mm][Aa]…` case-insensitive form.
    #[test]
    fn keywords_are_case_insensitive() {
        assert_eq!(toks("MATCHES"), vec![Token::SymMatches]);
        assert_eq!(toks("Is_In"), vec![Token::SymMatches]);
        assert_eq!(toks("OCCURRENCES"), vec![Token::SymOccurrences]);
        assert_eq!(toks("Existence"), vec![Token::SymExistence]);
        assert_eq!(toks("CaRdInAlItY"), vec![Token::SymCardinality]);
        assert_eq!(toks("ORDERED"), vec![Token::SymOrdered]);
        assert_eq!(toks("UNORDERED"), vec![Token::SymUnordered]);
        assert_eq!(toks("Unique"), vec![Token::SymUnique]);
        assert_eq!(toks("INFINITY"), vec![Token::SymInfinity]);
        assert_eq!(toks("Use_Node"), vec![Token::SymUseNode]);
        assert_eq!(toks("ALLOW_ARCHETYPE"), vec![Token::SymAllowArchetype]);
        assert_eq!(toks("Include"), vec![Token::SymInclude]);
        assert_eq!(toks("EXCLUDE"), vec![Token::SymExclude]);
        assert_eq!(toks("Before"), vec![Token::SymBefore]);
        assert_eq!(toks("AFTER"), vec![Token::SymAfter]);
        assert_eq!(toks("TRUE"), vec![Token::SymTrue]);
        assert_eq!(toks("fAlSe"), vec![Token::SymFalse]);
        assert_eq!(toks("NOT"), vec![Token::SymNot]);
        assert_eq!(toks("Implies"), vec![Token::SymImplies]);
        // The one keyword the grammars spell case-sensitively.
        assert_eq!(toks("there_exists"), vec![Token::SymThereExists]);
        assert_eq!(
            toks("There_Exists"),
            vec![Token::AlphaUcId("There_Exists".into())]
        );
    }

    /// `infinity` is its own keyword token (`master05` §Keywords L50, §Symbols
    /// L1349), used as an interval endpoint (`|0..infinity|`, L771).
    #[test]
    fn infinity_is_a_keyword_not_an_identifier() {
        assert_eq!(
            toks("|0..infinity|"),
            vec![
                Token::SymIvlDelim,
                Token::Integer("0".into()),
                Token::SymIvlSep,
                Token::SymInfinity,
                Token::SymIvlDelim,
            ]
        );
        assert_eq!(
            toks("|-infinity..5.0|"),
            vec![
                Token::SymIvlDelim,
                Token::SymMinus,
                Token::SymInfinity,
                Token::SymIvlSep,
                Token::Real("5.0".into()),
                Token::SymIvlDelim,
            ]
        );
    }

    /// Literal-substituted pattern fields (`master05` §Patterns L894), the
    /// ASCII timezone modifiers (L852 + the tz table L900-906) and the
    /// space-separated date/time pattern (§Symbols L1422 `[ T]`).
    #[test]
    fn pattern_variants() {
        assert_eq!(
            toks("1995-??-XX"),
            vec![Token::DateConstraintPattern("1995-??-XX".into())]
        );
        assert_eq!(
            toks("1995-mm-dd"),
            vec![Token::DateConstraintPattern("1995-mm-dd".into())]
        );
        assert_eq!(
            toks("hh:mm:ss+hh:mm"),
            vec![Token::TimeConstraintPattern("hh:mm:ss+hh:mm".into())]
        );
        assert_eq!(
            toks("hh:mm:ss-hh"),
            vec![Token::TimeConstraintPattern("hh:mm:ss-hh".into())]
        );
        assert_eq!(
            toks("hh:mm:ss+hhmm"),
            vec![Token::TimeConstraintPattern("hh:mm:ss+hhmm".into())]
        );
        assert_eq!(
            toks("hh:mm:ssZ"),
            vec![Token::TimeConstraintPattern("hh:mm:ssZ".into())]
        );
        assert_eq!(
            toks("yyyy-mm-dd hh:mm:XX"),
            vec![Token::DateTimeConstraintPattern(
                "yyyy-mm-dd hh:mm:XX".into()
            )]
        );
        assert_eq!(
            toks("yyyy-mm-ddThh:mm:ss\u{00B1}hh:mm"),
            vec![Token::DateTimeConstraintPattern(
                "yyyy-mm-ddThh:mm:ss\u{00B1}hh:mm".into()
            )]
        );
        // A text that is both a legal VALUE and a legal all-literal pattern
        // stays the value reading (see the note on the ISO8601 value tokens).
        assert_eq!(
            toks("2004-06-01"),
            vec![Token::Iso8601Date("2004-06-01".into())]
        );
    }

    /// The `^…^` regex delimiter is a `CONTAINED_REGEXP` in its own right
    /// (`master05` §Regular Expression L696-702; `V_REGEXP` L1476).
    #[test]
    fn caret_delimited_regexp_lexes() {
        assert_eq!(
            toks("{^km/h|mi/h^}"),
            vec![Token::ContainedRegexp("{^km/h|mi/h^}".into())]
        );
        assert_eq!(
            toks(r"{/km\/h|mi\/h/}"),
            vec![Token::ContainedRegexp(r"{/km\/h|mi\/h/}".into())]
        );
    }
}

#![allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::match_same_arms
)]

//! ODIN reader (openEHR **LANG 1.0.0**; canonical grammar: specifications-BASE
//! `odin.g4`).
//!
//! This is a focused, dependency-free reader for the ODIN subset that the
//! openEHR **BMM** schema files use (they are ODIN documents). It parses an
//! ODIN text into an [`Node`] tree that the [`crate::bmm`] layer walks. It is
//! deliberately small and grows toward the full ODIN grammar as the ADL/AOM
//! phases (P8/P9) need more of it.
//!
//! ## Grammar handled
//!
//! ```text
//! document   := pair*
//! pair       := IDENT '=' node
//! hash_entry := '[' STRING ']' '=' node
//! node       := ('(' IDENT ')')? '<' body '>'
//! body       := hash_entry+ | pair+ | scalar_or_list | (empty)
//! scalar     := STRING | INT | BOOL | INTERVAL | IDENT
//! ```
//!
//! Both attribute-objects (`name = <..>`) and keyed hashes (`["k"] = <..>`)
//! collapse to an ordered [`Odin::Map`]; downstream BMM code does not need the
//! distinction. The ODIN list-continuation token `...` is dropped.

use std::fmt;

/// A parsed ODIN value, optionally carrying an ODIN type name (`(TYPE) <..>`).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The `(TYPE_NAME)` prefix of a typed object, if present
    /// (e.g. `P_BMM_SINGLE_PROPERTY`).
    pub type_name: Option<String>,
    /// The value payload.
    pub value: Odin,
}

/// The payload of an [`Node`].
#[derive(Debug, Clone, PartialEq)]
pub enum Odin {
    /// A quoted string, or a bare enum symbol.
    Str(String),
    /// An integer literal (e.g. a `precision` of `-1`).
    Int(i64),
    /// A real/decimal literal (e.g. `30.42`).
    Real(f64),
    /// `True` / `False`.
    Bool(bool),
    /// The raw text between `|..|` (e.g. an occurrence `>=0`).
    Interval(String),
    /// A manifest list of scalars (`<"A", "B">`); the trailing `...` is dropped.
    List(Vec<Odin>),
    /// An ordered key→node map (attribute-object or keyed hash).
    Map(Vec<(String, Node)>),
    /// An empty `< >`.
    Empty,
}

/// An ODIN parse error with a short message.
#[derive(Debug, Clone, PartialEq)]
pub struct OdinError(pub String);

impl fmt::Display for OdinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ODIN parse error: {}", self.0)
    }
}
impl std::error::Error for OdinError {}

impl Node {
    /// Look up a child by key, if this node's value is a map.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        match &self.value {
            Odin::Map(m) => m.iter().find(|(k, _)| k == key).map(|(_, n)| n),
            _ => None,
        }
    }

    /// The ordered map entries, or an empty slice if this is not a map.
    #[must_use]
    pub fn entries(&self) -> &[(String, Node)] {
        match &self.value {
            Odin::Map(m) => m,
            _ => &[],
        }
    }

    /// This node as a string, if it is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Odin::Str(s) => Some(s),
            _ => None,
        }
    }

    /// This node as a bool, if it is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match &self.value {
            Odin::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// This node as an integer, if it is one.
    #[must_use]
    pub fn as_int(&self) -> Option<i64> {
        match &self.value {
            Odin::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// This node as a real, if it is one (an integer literal also coerces).
    #[must_use]
    pub fn as_real(&self) -> Option<f64> {
        match &self.value {
            Odin::Real(v) => Some(*v),
            #[allow(clippy::cast_precision_loss)]
            Odin::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// This node as the raw interval text, if it is one.
    #[must_use]
    pub fn as_interval(&self) -> Option<&str> {
        match &self.value {
            Odin::Interval(s) => Some(s),
            _ => None,
        }
    }

    /// A list of the string scalars in this node — a single `Str` yields a
    /// one-element vec, a `List` yields its string members, anything else the
    /// empty vec. Non-string list members are skipped.
    #[must_use]
    pub fn str_list(&self) -> Vec<&str> {
        match &self.value {
            Odin::Str(s) => vec![s.as_str()],
            Odin::List(items) => items
                .iter()
                .filter_map(|o| match o {
                    Odin::Str(s) => Some(s.as_str()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// Parse an ODIN document into its top-level [`Node`] (a map of the document's
/// attribute pairs).
///
/// # Errors
/// Returns [`OdinError`] on malformed input.
pub fn parse(input: &str) -> Result<Node, OdinError> {
    let toks = tokenize(input)?;
    let mut p = Parser { toks, i: 0 };
    let pairs = p.parse_pairs()?;
    if !matches!(p.peek(), Tok::Eof) {
        return Err(OdinError(format!(
            "trailing tokens after top-level document (near {:?})",
            p.peek()
        )));
    }
    Ok(Node {
        type_name: None,
        value: Odin::Map(pairs),
    })
}

// ── Tokenizer ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    LAngle,
    RAngle,
    LParen,
    RParen,
    LBrack,
    RBrack,
    Eq,
    Comma,
    Ellipsis,
    Str(String),
    Int(i64),
    Real(f64),
    Ident(String),
    Bool(bool),
    Interval(String),
    Eof,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, OdinError> {
    let cs: Vec<char> = s.chars().collect();
    let n = cs.len();
    let mut i = 0;
    let mut out = Vec::new();
    while i < n {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // `--` line comment (ODIN)
        if c == '-' && i + 1 < n && cs[i + 1] == '-' {
            while i < n && cs[i] != '\n' {
                i += 1;
            }
            continue;
        }
        match c {
            '<' => {
                out.push(Tok::LAngle);
                i += 1;
            }
            '>' => {
                out.push(Tok::RAngle);
                i += 1;
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            '[' => {
                out.push(Tok::LBrack);
                i += 1;
            }
            ']' => {
                out.push(Tok::RBrack);
                i += 1;
            }
            '=' => {
                out.push(Tok::Eq);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '"' => {
                i += 1;
                let mut buf = String::new();
                let mut closed = false;
                while i < n {
                    let d = cs[i];
                    if d == '\\' && i + 1 < n {
                        match cs[i + 1] {
                            '"' => buf.push('"'),
                            '\\' => buf.push('\\'),
                            'n' => buf.push('\n'),
                            't' => buf.push('\t'),
                            'r' => buf.push('\r'),
                            other => {
                                buf.push('\\');
                                buf.push(other);
                            }
                        }
                        i += 2;
                        continue;
                    }
                    if d == '"' {
                        i += 1;
                        closed = true;
                        break;
                    }
                    buf.push(d);
                    i += 1;
                }
                if !closed {
                    return Err(OdinError("unterminated string literal".into()));
                }
                out.push(Tok::Str(buf));
            }
            '|' => {
                i += 1;
                let mut buf = String::new();
                while i < n && cs[i] != '|' {
                    buf.push(cs[i]);
                    i += 1;
                }
                if i >= n {
                    return Err(OdinError("unterminated interval `|..|`".into()));
                }
                i += 1; // closing '|'
                out.push(Tok::Interval(buf.trim().to_string()));
            }
            '.' => {
                if i + 2 < n && cs[i + 1] == '.' && cs[i + 2] == '.' {
                    out.push(Tok::Ellipsis);
                    i += 3;
                } else {
                    return Err(OdinError("stray `.` (expected `...`)".into()));
                }
            }
            _ => {
                if c == '-' || c.is_ascii_digit() {
                    let start = i;
                    if cs[i] == '-' {
                        i += 1;
                    }
                    while i < n && cs[i].is_ascii_digit() {
                        i += 1;
                    }
                    // fractional part: `.` followed by a digit (distinct from `...`)
                    let mut is_real = false;
                    if i + 1 < n && cs[i] == '.' && cs[i + 1].is_ascii_digit() {
                        is_real = true;
                        i += 1;
                        while i < n && cs[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                    let num: String = cs[start..i].iter().collect();
                    if is_real {
                        match num.parse::<f64>() {
                            Ok(v) => out.push(Tok::Real(v)),
                            Err(_) => out.push(Tok::Ident(num)),
                        }
                    } else {
                        match num.parse::<i64>() {
                            Ok(v) => out.push(Tok::Int(v)),
                            Err(_) => out.push(Tok::Ident(num)),
                        }
                    }
                } else if c.is_alphabetic() || c == '_' {
                    let start = i;
                    while i < n && (cs[i].is_alphanumeric() || cs[i] == '_') {
                        i += 1;
                    }
                    let id: String = cs[start..i].iter().collect();
                    match id.as_str() {
                        "True" => out.push(Tok::Bool(true)),
                        "False" => out.push(Tok::Bool(false)),
                        _ => out.push(Tok::Ident(id)),
                    }
                } else {
                    return Err(OdinError(format!("unexpected character {c:?}")));
                }
            }
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

// ── Parser ───────────────────────────────────────────────────────────────────

struct Parser {
    toks: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        self.toks.get(self.i).unwrap_or(&Tok::Eof)
    }
    fn peek2(&self) -> &Tok {
        self.toks.get(self.i + 1).unwrap_or(&Tok::Eof)
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks.get(self.i).cloned().unwrap_or(Tok::Eof);
        self.i += 1;
        t
    }
    fn expect(&mut self, want: &Tok) -> Result<(), OdinError> {
        if self.peek() == want {
            self.i += 1;
            Ok(())
        } else {
            Err(OdinError(format!(
                "expected {want:?}, found {:?}",
                self.peek()
            )))
        }
    }

    /// `pair := IDENT '=' node` — collected while the lookahead is `IDENT =`.
    fn parse_pairs(&mut self) -> Result<Vec<(String, Node)>, OdinError> {
        let mut v = Vec::new();
        while matches!(self.peek(), Tok::Ident(_)) && matches!(self.peek2(), Tok::Eq) {
            let key = match self.bump() {
                Tok::Ident(s) => s,
                _ => unreachable!(),
            };
            self.expect(&Tok::Eq)?;
            v.push((key, self.parse_node()?));
        }
        Ok(v)
    }

    /// `hash_entry := '[' STRING ']' '=' node`.
    fn parse_hash(&mut self) -> Result<Vec<(String, Node)>, OdinError> {
        let mut v = Vec::new();
        while matches!(self.peek(), Tok::LBrack) {
            self.bump();
            let key = match self.bump() {
                Tok::Str(s) => s,
                other => {
                    return Err(OdinError(format!(
                        "expected hash key string, found {other:?}"
                    )));
                }
            };
            self.expect(&Tok::RBrack)?;
            self.expect(&Tok::Eq)?;
            v.push((key, self.parse_node()?));
        }
        Ok(v)
    }

    /// `node := ('(' IDENT ')')? '<' body '>'`.
    fn parse_node(&mut self) -> Result<Node, OdinError> {
        let type_name = if matches!(self.peek(), Tok::LParen) {
            self.bump();
            let t = match self.bump() {
                Tok::Ident(s) => s,
                other => return Err(OdinError(format!("expected type name, found {other:?}"))),
            };
            self.expect(&Tok::RParen)?;
            Some(t)
        } else {
            None
        };
        self.expect(&Tok::LAngle)?;
        let value = self.parse_body()?;
        self.expect(&Tok::RAngle)?;
        Ok(Node { type_name, value })
    }

    fn parse_body(&mut self) -> Result<Odin, OdinError> {
        match self.peek() {
            Tok::RAngle => Ok(Odin::Empty),
            Tok::LBrack => Ok(Odin::Map(self.parse_hash()?)),
            Tok::Ident(_) if matches!(self.peek2(), Tok::Eq) => Ok(Odin::Map(self.parse_pairs()?)),
            _ => self.parse_scalar_or_list(),
        }
    }

    fn parse_scalar_or_list(&mut self) -> Result<Odin, OdinError> {
        let first = self.parse_scalar()?;
        if !matches!(self.peek(), Tok::Comma) {
            return Ok(first);
        }
        let mut items = vec![first];
        while matches!(self.peek(), Tok::Comma) {
            self.bump();
            match self.peek() {
                Tok::Ellipsis => {
                    self.bump();
                }
                Tok::RAngle => break, // trailing comma
                _ => items.push(self.parse_scalar()?),
            }
        }
        Ok(Odin::List(items))
    }

    fn parse_scalar(&mut self) -> Result<Odin, OdinError> {
        match self.bump() {
            Tok::Str(s) => Ok(Odin::Str(s)),
            Tok::Int(v) => Ok(Odin::Int(v)),
            Tok::Real(v) => Ok(Odin::Real(v)),
            Tok::Bool(b) => Ok(Odin::Bool(b)),
            Tok::Interval(s) => Ok(Odin::Interval(s)),
            Tok::Ident(s) => Ok(Odin::Str(s)),
            other => Err(OdinError(format!("expected a scalar, found {other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNIPPET: &str = r#"
        schema_name = <"rm">
        bmm_version = <"2.4">
        includes = <
            ["openehr_base_1.2.0"] = <
                id = <"openehr_base_1.2.0">
            >
        >
        class_definitions = <
            ["DV_QUANTITY"] = <
                name = <"DV_QUANTITY">
                documentation = <"A quantity with a \"unit\".">
                ancestors = <"DV_AMOUNT", ...>
                properties = <
                    ["magnitude"] = (P_BMM_SINGLE_PROPERTY) <
                        name = <"magnitude">
                        is_mandatory = <True>
                        type = <"Real">
                    >
                    ["precision"] = (P_BMM_SINGLE_PROPERTY) <
                        name = <"precision">
                        type = <"Integer">
                    >
                    ["other_reference_ranges"] = (P_BMM_CONTAINER_PROPERTY) <
                        name = <"other_reference_ranges">
                        cardinality = <|>=0|>
                        type_def = <
                            container_type = <"List">
                            type_def = (P_BMM_GENERIC_TYPE) <
                                root_type = <"REFERENCE_RANGE">
                                generic_parameters = <"DV_QUANTITY", ...>
                            >
                        >
                    >
                >
            >
        >
    "#;

    #[test]
    fn parses_header_scalars() {
        let doc = parse(SNIPPET).expect("parse");
        assert_eq!(doc.get("schema_name").and_then(Node::as_str), Some("rm"));
        assert_eq!(doc.get("bmm_version").and_then(Node::as_str), Some("2.4"));
    }

    #[test]
    fn parses_includes_hash() {
        let doc = parse(SNIPPET).unwrap();
        let inc = doc.get("includes").unwrap();
        assert_eq!(
            inc.get("openehr_base_1.2.0")
                .and_then(|n| n.get("id"))
                .and_then(Node::as_str),
            Some("openehr_base_1.2.0")
        );
    }

    #[test]
    fn parses_class_with_properties_and_types() {
        let doc = parse(SNIPPET).unwrap();
        let class = doc
            .get("class_definitions")
            .unwrap()
            .get("DV_QUANTITY")
            .unwrap();
        assert_eq!(
            class.get("name").and_then(Node::as_str),
            Some("DV_QUANTITY")
        );
        // escaped quote survived
        assert_eq!(
            class.get("documentation").and_then(Node::as_str),
            Some("A quantity with a \"unit\".")
        );
        // ancestors list with the `...` continuation dropped
        assert_eq!(
            class.get("ancestors").unwrap().str_list(),
            vec!["DV_AMOUNT"]
        );

        let props = class.get("properties").unwrap();
        let mag = props.get("magnitude").unwrap();
        assert_eq!(mag.type_name.as_deref(), Some("P_BMM_SINGLE_PROPERTY"));
        assert_eq!(mag.get("is_mandatory").and_then(Node::as_bool), Some(true));
        assert_eq!(mag.get("type").and_then(Node::as_str), Some("Real"));
        // precision has no is_mandatory (=> optional downstream)
        assert!(
            props
                .get("precision")
                .unwrap()
                .get("is_mandatory")
                .is_none()
        );

        // container + nested generic type_def + interval cardinality
        let orr = props.get("other_reference_ranges").unwrap();
        assert_eq!(orr.type_name.as_deref(), Some("P_BMM_CONTAINER_PROPERTY"));
        assert_eq!(
            orr.get("cardinality").and_then(Node::as_interval),
            Some(">=0")
        );
        let td = orr.get("type_def").unwrap();
        assert_eq!(
            td.get("container_type").and_then(Node::as_str),
            Some("List")
        );
        let inner = td.get("type_def").unwrap();
        assert_eq!(inner.type_name.as_deref(), Some("P_BMM_GENERIC_TYPE"));
        assert_eq!(
            inner.get("root_type").and_then(Node::as_str),
            Some("REFERENCE_RANGE")
        );
        assert_eq!(
            inner.get("generic_parameters").unwrap().str_list(),
            vec!["DV_QUANTITY"]
        );
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: openEHR LANG 1.0.0 ODIN grammar (specifications-BASE computable/grammar/odin.g4)
//   source_loc: n/a
//   confidence: medium
//   todos: 0
//   note: focused reader for the BMM ODIN subset; grows to full ODIN at P8/P9.
// ─────────────────────────────────────────────

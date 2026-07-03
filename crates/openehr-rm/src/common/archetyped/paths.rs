//! openEHR path syntax and evaluation machinery backing `PATHABLE`.
//!
//! Not a spec class — this module is the shared implementation behind the
//! five `PATHABLE` path functions (`item_at_path`, `items_at_path`,
//! `path_exists`, `path_unique`, `path_of_item`) declared in
//! [`super::pathable::PathableApi`]. The path notation itself is defined by
//! the openEHR Architecture Overview ("Paths and Locators") and used
//! throughout RM 1.1.0 `common.archetyped`
//! (docs/research/spec-cache/RM-1.1.0/common/master03-archetyped_package.adoc):
//! an X-path-like sequence of attribute-name segments, each optionally
//! qualified by a predicate carrying an archetype node id and/or a runtime
//! name, e.g.
//!
//! ```text
//! /content[openEHR-EHR-OBSERVATION.bp.v1]/data[at0001]/events[at0006]/data[at0003]/items[at0004, 'Systolic']
//! ```
//!
//! # Grammar (the deliberately tiny subset implemented here)
//!
//! ```text
//! path       = [ '/' ] [ segment { '/' segment } ]
//! segment    = attribute [ predicate ]
//! attribute  = ( ALPHA | '_' ) { ALPHA | DIGIT | '_' }
//! predicate  = '[' ( node_id [ ',' name ] | name ) ']'
//! node_id    = any run of characters other than ',' ']' or a quote
//!              (covers at-codes `at0001`, id-codes, and archetype ids
//!              `openEHR-EHR-OBSERVATION.bp.v1`)
//! name       = "'" { any character except "'" } "'"
//! ```
//!
//! A bare `/` (or a path of zero segments) denotes the node the path is
//! evaluated against, mirroring how the spec treats a path as *relative to
//! the current item*.
//!
//! PORT NOTE: this evaluator is deliberately self-contained and minimal —
//! it exists so the `PATHABLE` functions have real, testable semantics at
//! the RM layer. The full AQL path grammar (P12, `openehr-aql`) is a
//! separate, much larger machine: it additionally handles standard
//! predicates with comparison operators, `name/value` shorthand, numeric
//! positional predicates, node-id disjunction, and semantic path analysis
//! against Web Templates. Nothing here is a substitute for it, and it must
//! not grow toward it.
//!
//! TODO(port): quoted-name escape sequences (a literal `'` inside a name
//! predicate) and numeric positional predicates are not part of this tiny
//! grammar; they arrive with the full AQL path parser at P12.

use std::fmt;

use super::pathable::PathableApi;

/// Errors from parsing an openEHR path string.
///
/// Byte offsets refer to the *trimmed* input handed to [`RmPath::parse`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathParseError {
    /// The path string was empty (or all whitespace). The spec's own
    /// precondition on `path_exists` is `not a_path.is_empty`.
    #[error("openEHR path is empty")]
    Empty,
    /// A `/` was not followed by an attribute name (e.g. `/data/` or
    /// `//items`).
    #[error("expected an attribute name at byte {0}")]
    ExpectedAttribute(usize),
    /// An attribute name was followed by something other than a
    /// predicate, `/`, or end of input.
    #[error("unexpected character {ch:?} at byte {pos}")]
    UnexpectedChar {
        /// The offending character.
        ch: char,
        /// Its byte offset in the trimmed input.
        pos: usize,
    },
    /// A `[` predicate was never closed by `]`.
    #[error("unterminated predicate starting at byte {0}")]
    UnterminatedPredicate(usize),
    /// A `'` quoted name was never closed by a matching `'`.
    #[error("unterminated quoted name starting at byte {0}")]
    UnterminatedName(usize),
    /// A predicate was empty (`[]`) or carried an empty component
    /// (`[at0001, ]`).
    #[error("empty predicate component at byte {0}")]
    EmptyPredicate(usize),
}

/// The node predicate inside one path segment's `[..]` brackets: an
/// archetype node id, a quoted runtime name, or both
/// (`[at0004, 'Systolic']`).
///
/// At least one of the two fields is always `Some` — [`RmPath::parse`]
/// rejects an empty predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePredicate {
    /// The archetype node id to match against
    /// `LOCATABLE.archetype_node_id`: an at-code (`at0004`) or, at an
    /// archetype root point, a full archetype id
    /// (`openEHR-EHR-OBSERVATION.bp.v1`).
    pub archetype_node_id: Option<String>,
    /// The runtime name to match against `LOCATABLE.name.value`, without
    /// its quotes.
    pub name: Option<String>,
}

impl NodePredicate {
    /// True if `node` satisfies every component this predicate carries.
    ///
    /// A node that exposes no node id / name (a bare `PATHABLE` such as
    /// `EVENT_CONTEXT`) can never match a predicate demanding one.
    pub fn matches(&self, node: &dyn PathableApi) -> bool {
        if let Some(id) = &self.archetype_node_id
            && node.path_node_id() != Some(id.as_str())
        {
            return false;
        }
        if let Some(name) = &self.name
            && node.path_node_name() != Some(name.as_str())
        {
            return false;
        }
        true
    }
}

impl fmt::Display for NodePredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.archetype_node_id, &self.name) {
            (Some(id), Some(name)) => write!(f, "{id}, '{name}'"),
            (Some(id), None) => write!(f, "{id}"),
            (None, Some(name)) => write!(f, "'{name}'"),
            // Unreachable by construction (parse rejects empty predicates),
            // but Display must not panic.
            (None, None) => Ok(()),
        }
    }
}

/// One `/`-delimited path segment: an attribute name plus an optional
/// node predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    /// The RM attribute name this segment descends through
    /// (`content`, `data`, `events`, `items`, ...).
    pub attribute: String,
    /// The optional `[..]` predicate filtering the children reached via
    /// `attribute`.
    pub predicate: Option<NodePredicate>,
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.attribute)?;
        if let Some(pred) = &self.predicate {
            write!(f, "[{pred}]")?;
        }
        Ok(())
    }
}

/// A parsed openEHR path: the sequence of segments to walk from the node
/// the path is evaluated against. Zero segments (`"/"`) denotes that node
/// itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RmPath {
    /// The segments, in root-to-leaf order.
    pub segments: Vec<PathSegment>,
}

impl RmPath {
    /// Parse a path string per the module-level grammar.
    ///
    /// Leading/trailing whitespace is trimmed; a leading `/` is optional
    /// (both `/data/events` and `data/events` parse to the same path,
    /// since evaluation is always relative to the current item anyway).
    pub fn parse(input: &str) -> Result<Self, PathParseError> {
        let src = input.trim();
        if src.is_empty() {
            return Err(PathParseError::Empty);
        }

        let mut scanner = Scanner::new(src);
        // Optional single leading '/'.
        scanner.eat('/');

        let mut segments = Vec::new();
        while !scanner.is_at_end() {
            segments.push(parse_segment(&mut scanner)?);
            if scanner.is_at_end() {
                break;
            }
            match scanner.peek() {
                Some('/') => {
                    scanner.bump();
                    if scanner.is_at_end() {
                        // Trailing '/' after a segment: dangling separator.
                        return Err(PathParseError::ExpectedAttribute(scanner.pos()));
                    }
                }
                Some(ch) => {
                    return Err(PathParseError::UnexpectedChar {
                        ch,
                        pos: scanner.pos(),
                    });
                }
                None => break,
            }
        }

        Ok(RmPath { segments })
    }

    /// True if this path denotes the node it is evaluated against
    /// (a bare `/`).
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }
}

impl fmt::Display for RmPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            return write!(f, "/");
        }
        for segment in &self.segments {
            write!(f, "/{segment}")?;
        }
        Ok(())
    }
}

/// Minimal cursor over the trimmed path string.
struct Scanner<'s> {
    src: &'s str,
    pos: usize,
}

impl<'s> Scanner<'s> {
    fn new(src: &'s str) -> Self {
        Scanner { src, pos: 0 }
    }

    fn pos(&self) -> usize {
        self.pos
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Consume `expected` if it is the next character; report success.
    fn eat(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    /// Consume characters while `keep` holds, returning the consumed slice.
    fn take_while(&mut self, keep: impl Fn(char) -> bool) -> &'s str {
        let start = self.pos;
        while self.peek().is_some_and(&keep) {
            self.bump();
        }
        &self.src[start..self.pos]
    }
}

/// `segment = attribute [ predicate ]`.
fn parse_segment(scanner: &mut Scanner<'_>) -> Result<PathSegment, PathParseError> {
    let start = scanner.pos();
    let attribute = scanner.take_while(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if attribute.is_empty() {
        return Err(PathParseError::ExpectedAttribute(start));
    }
    if attribute.starts_with(|ch: char| ch.is_ascii_digit()) {
        return Err(PathParseError::UnexpectedChar {
            // Report the digit that illegally opens the attribute name.
            // `starts_with` above guarantees a first char exists; avoid
            // unwrap outside tests by falling back to '0' (unreachable).
            ch: attribute.chars().next().unwrap_or('0'),
            pos: start,
        });
    }

    let predicate = if scanner.peek() == Some('[') {
        Some(parse_predicate(scanner)?)
    } else {
        None
    };

    Ok(PathSegment {
        attribute: attribute.to_string(),
        predicate,
    })
}

/// `predicate = '[' ( node_id [ ',' name ] | name ) ']'`.
fn parse_predicate(scanner: &mut Scanner<'_>) -> Result<NodePredicate, PathParseError> {
    let open = scanner.pos();
    scanner.bump(); // consume '['
    scanner.skip_whitespace();

    let mut archetype_node_id = None;
    let mut name = None;

    if scanner.peek() == Some('\'') {
        name = Some(parse_quoted_name(scanner)?);
    } else {
        let id_start = scanner.pos();
        let raw = scanner.take_while(|ch| ch != ',' && ch != ']' && ch != '\'');
        let id = raw.trim();
        if id.is_empty() {
            return Err(PathParseError::EmptyPredicate(id_start));
        }
        archetype_node_id = Some(id.to_string());
        scanner.skip_whitespace();
        if scanner.eat(',') {
            scanner.skip_whitespace();
            if scanner.peek() == Some('\'') {
                name = Some(parse_quoted_name(scanner)?);
            } else {
                return Err(PathParseError::EmptyPredicate(scanner.pos()));
            }
        }
    }

    scanner.skip_whitespace();
    match scanner.peek() {
        Some(']') => {
            scanner.bump();
            Ok(NodePredicate {
                archetype_node_id,
                name,
            })
        }
        Some(ch) => Err(PathParseError::UnexpectedChar {
            ch,
            pos: scanner.pos(),
        }),
        None => Err(PathParseError::UnterminatedPredicate(open)),
    }
}

/// `name = "'" { any character except "'" } "'"`, quotes stripped.
fn parse_quoted_name(scanner: &mut Scanner<'_>) -> Result<String, PathParseError> {
    let open = scanner.pos();
    scanner.bump(); // consume opening '
    let value = scanner.take_while(|ch| ch != '\'');
    if !scanner.eat('\'') {
        return Err(PathParseError::UnterminatedName(open));
    }
    Ok(value.to_string())
}

// ── Evaluation ───────────────────────────────────────────────────────────

/// Resolve `path` against `root`, returning every node it reaches.
///
/// Breadth-per-segment walk: each segment maps the current node set to the
/// union of the children reached via the segment's attribute (through
/// [`PathableApi::path_child_nodes`]) that satisfy its predicate. A root
/// path (`"/"`) resolves to `root` itself.
pub fn resolve<'a>(root: &'a dyn PathableApi, path: &RmPath) -> Vec<&'a dyn PathableApi> {
    let mut current: Vec<&'a dyn PathableApi> = vec![root];
    for segment in &path.segments {
        let mut next: Vec<&'a dyn PathableApi> = Vec::new();
        for node in current {
            for child in node.path_child_nodes(&segment.attribute) {
                let keep = segment
                    .predicate
                    .as_ref()
                    .is_none_or(|pred| pred.matches(child));
                if keep {
                    next.push(child);
                }
            }
        }
        if next.is_empty() {
            return next;
        }
        current = next;
    }
    current
}

/// Compute the path of `target` relative to `root` (`PATHABLE.path_of_item`
/// semantics), or `None` if `target` is not `root` itself nor reachable
/// from it through [`PathableApi::path_child_nodes`].
///
/// Identity is by address ([`std::ptr::addr_eq`]) — the RM containment
/// tree owns each node exactly once, so "the same node" means "the same
/// allocation", never structural equality (two sibling `ELEMENT`s can be
/// value-equal yet have distinct paths).
pub fn path_of_descendant(root: &dyn PathableApi, target: &dyn PathableApi) -> Option<String> {
    if node_addr_eq(root, target) {
        return Some("/".to_string());
    }
    let mut segments: Vec<String> = Vec::new();
    if dfs(root, target, &mut segments) {
        let mut path = String::new();
        for segment in &segments {
            path.push('/');
            path.push_str(segment);
        }
        Some(path)
    } else {
        None
    }
}

/// Address-identity over trait objects, ignoring vtable metadata.
fn node_addr_eq(a: &dyn PathableApi, b: &dyn PathableApi) -> bool {
    std::ptr::addr_eq(a as *const dyn PathableApi, b as *const dyn PathableApi)
}

/// Depth-first search for `target` below `node`, accumulating rendered
/// segments in `out`. On success `out` holds the full segment list.
fn dfs(node: &dyn PathableApi, target: &dyn PathableApi, out: &mut Vec<String>) -> bool {
    for attribute in node.path_attribute_names() {
        let children = node.path_child_nodes(attribute);
        for child in children.iter().copied() {
            out.push(render_segment(attribute, child, &children));
            if node_addr_eq(child, target) || dfs(child, target, out) {
                return true;
            }
            out.pop();
        }
    }
    false
}

/// Render one path segment for `child` reached via `attribute`, choosing
/// the smallest predicate that identifies it among `siblings`:
///
/// * node id present and unique among siblings → `attribute[id]`;
/// * node id present but shared → `attribute[id, 'name']` when a name is
///   available (the runtime-name disambiguation the spec's path examples
///   use), else `attribute[id]` (ambiguous — the data itself carries no
///   distinguishing feature this evaluator knows about);
/// * no node id, multiple siblings, name available → `attribute['name']`;
/// * otherwise the bare attribute (single-valued attribute).
fn render_segment(
    attribute: &str,
    child: &dyn PathableApi,
    siblings: &[&dyn PathableApi],
) -> String {
    match (child.path_node_id(), child.path_node_name()) {
        (Some(id), name) => {
            let id_is_unique = siblings
                .iter()
                .filter(|sibling| sibling.path_node_id() == Some(id))
                .count()
                <= 1;
            match name {
                Some(name) if !id_is_unique => format!("{attribute}[{id}, '{name}']"),
                _ => format!("{attribute}[{id}]"),
            }
        }
        (None, Some(name)) if siblings.len() > 1 => format!("{attribute}['{name}']"),
        _ => attribute.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(attribute: &str, id: Option<&str>, name: Option<&str>) -> PathSegment {
        PathSegment {
            attribute: attribute.to_string(),
            predicate: if id.is_none() && name.is_none() {
                None
            } else {
                Some(NodePredicate {
                    archetype_node_id: id.map(str::to_string),
                    name: name.map(str::to_string),
                })
            },
        }
    }

    #[test]
    fn parses_root_path() {
        let path = RmPath::parse("/").unwrap();
        assert!(path.is_root());
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn parses_bare_attributes() {
        let path = RmPath::parse("/data/events").unwrap();
        assert_eq!(
            path.segments,
            vec![seg("data", None, None), seg("events", None, None)]
        );
    }

    #[test]
    fn leading_slash_is_optional() {
        assert_eq!(
            RmPath::parse("data/events").unwrap(),
            RmPath::parse("/data/events").unwrap()
        );
    }

    #[test]
    fn parses_at_code_predicate() {
        let path = RmPath::parse("/data[at0001]").unwrap();
        assert_eq!(path.segments, vec![seg("data", Some("at0001"), None)]);
    }

    #[test]
    fn parses_archetype_id_predicate() {
        let path = RmPath::parse("/content[openEHR-EHR-OBSERVATION.bp.v1]").unwrap();
        assert_eq!(
            path.segments,
            vec![seg("content", Some("openEHR-EHR-OBSERVATION.bp.v1"), None)]
        );
    }

    #[test]
    fn parses_name_only_predicate() {
        let path = RmPath::parse("/items['Systolic']").unwrap();
        assert_eq!(path.segments, vec![seg("items", None, Some("Systolic"))]);
    }

    #[test]
    fn parses_combined_predicate_with_and_without_space() {
        let spaced = RmPath::parse("/items[at0004, 'Systolic']").unwrap();
        let tight = RmPath::parse("/items[at0004,'Systolic']").unwrap();
        assert_eq!(spaced, tight);
        assert_eq!(
            spaced.segments,
            vec![seg("items", Some("at0004"), Some("Systolic"))]
        );
    }

    #[test]
    fn parses_deep_path() {
        let text = "/data[at0001]/events[at0006]/data[at0003]/items[at0004, 'Systolic']";
        let path = RmPath::parse(text).unwrap();
        assert_eq!(path.segments.len(), 4);
        assert_eq!(path.to_string(), text);
    }

    #[test]
    fn display_round_trips() {
        for text in [
            "/",
            "/data",
            "/data[at0001]/events[at0006, 'Any event']",
            "/items['Systolic']",
        ] {
            let path = RmPath::parse(text).unwrap();
            assert_eq!(path.to_string(), text);
            assert_eq!(RmPath::parse(&path.to_string()).unwrap(), path);
        }
    }

    #[test]
    fn rejects_empty_and_malformed_paths() {
        assert_eq!(RmPath::parse(""), Err(PathParseError::Empty));
        assert_eq!(RmPath::parse("   "), Err(PathParseError::Empty));
        assert!(matches!(
            RmPath::parse("//items"),
            Err(PathParseError::ExpectedAttribute(_))
        ));
        assert!(matches!(
            RmPath::parse("/data/"),
            Err(PathParseError::ExpectedAttribute(_))
        ));
        assert!(matches!(
            RmPath::parse("/data[]"),
            Err(PathParseError::EmptyPredicate(_))
        ));
        assert!(matches!(
            RmPath::parse("/data[at0001"),
            Err(PathParseError::UnterminatedPredicate(_))
        ));
        assert!(matches!(
            RmPath::parse("/items['Systolic]"),
            Err(PathParseError::UnterminatedName(_))
        ));
        assert!(matches!(
            RmPath::parse("/items[at0004, ]"),
            Err(PathParseError::EmptyPredicate(_))
        ));
        assert!(matches!(
            RmPath::parse("/da ta"),
            Err(PathParseError::UnexpectedChar { .. })
        ));
        assert!(matches!(
            RmPath::parse("/9data"),
            Err(PathParseError::UnexpectedChar { .. })
        ));
    }
}

// ─────────────────────────────────────────────
// PORT STATUS
//   source: RM 1.1.0 common.archetyped path semantics — docs/research/spec-cache/RM-1.1.0/common/master03-archetyped_package.adoc §The PATHABLE Class (path notation per the openEHR Architecture Overview "Paths and Locators")
//   source_loc: n/a
//   confidence: high
//   todos: 1
//   note: Helper module, not a spec class — tiny path grammar (attribute + at-code/name predicates) + resolve/path_of_descendant evaluators behind PathableApi's default methods. Deliberately NOT the AQL path engine (P12); escape sequences and positional predicates deferred there.
// ─────────────────────────────────────────────

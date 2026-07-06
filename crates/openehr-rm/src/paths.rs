//! openEHR path machinery (ADR-003 spec behaviour; hand-written, preserved
//! across `openehr-codegen` regeneration like `validate.rs`).
//!
//! Implements the RM `PATHABLE` pathing functions —
//! `item_at_path` / `items_at_path` / `path_exists` / `path_unique` /
//! `path_of_item` / `parent` — over the **canonical-JSON value tree**
//! (`serde_json::Value`), which is the repo's uniform RM representation (the
//! node codec, the P15 validator, and the FLAT converters all navigate it).
//!
//! Spec:
//! - RM 1.2.0 `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.pathable.adoc`
//!   (the function signatures + parent semantics).
//! - BASE `docs/specs/openehr/BASE/docs/architecture_overview/master11-paths.adoc`
//!   (the path syntax: `/`-separated attribute segments, `[atNNNN]` /
//!   `[archetype-id]` predicates, the `[atNNNN,'name']` name shortcut, and the
//!   explicit `[atNNNN and name/value='x']` form).
//!
//! Design notes (recorded per F-12-02):
//! - `PATHABLE.parent()` is a back-reference; per the repo convention (no
//!   owning back-refs) it is realised as a root-anchored lookup
//!   ([`parent_of`]), not a stored pointer.
//! - The typed-`enum` RM tree is *not* walked directly: a second, typed
//!   visitor over ~130 generated structs would duplicate this logic for no
//!   wire gain — every consumer already holds the canonical JSON form.
//! - Predicates carrying general comparison expressions (e.g.
//!   `[at0007 and time >= '...']`) are rejected as
//!   [`PathError::UnsupportedPredicate`] — they belong to AQL (P16), not the
//!   PATHABLE navigation primitive; the accept-set here is the archetype-id /
//!   name subset the spec's own top-level-structure examples use.

use serde_json::Value;
use std::fmt;
use std::str::FromStr;

/// Error raised when parsing an openEHR path expression.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    /// The path string was empty.
    #[error("empty path")]
    Empty,
    /// A path segment had no attribute name (e.g. `//`, or a leading `[`).
    #[error("path segment {0} has no attribute name")]
    MissingAttribute(usize),
    /// A `[` predicate was not terminated by `]`.
    #[error("unterminated predicate in segment {0}")]
    UnterminatedPredicate(usize),
    /// A predicate uses a construct outside the supported archetype-id /
    /// name/value subset (general comparisons belong to AQL, not PATHABLE).
    #[error("unsupported predicate expression: {0:?}")]
    UnsupportedPredicate(String),
}

/// The predicate of one path segment: an optional `archetype_node_id` match
/// (`[at0003]` / `[openEHR-EHR-OBSERVATION.x.v1]`, i.e. the shortcut for
/// `[@archetype_node_id = '...']`) and an optional `name/value` match
/// (`[at0003,'name']` / `[at0003 and name/value='name']`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Predicate {
    /// Required `archetype_node_id` (or, at archetype roots, archetype id).
    pub archetype_node_id: Option<String>,
    /// Required `name/value`.
    pub name_value: Option<String>,
}

impl Predicate {
    /// Whether this predicate constrains nothing (matches every node).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.archetype_node_id.is_none() && self.name_value.is_none()
    }

    /// Whether a canonical-JSON RM node satisfies this predicate.
    ///
    /// The `archetype_node_id` conjunct matches `LOCATABLE.archetype_node_id`
    /// (the `[atNNNN]` / `[archetype-id]` shortcut); the `name_value` conjunct
    /// matches `LOCATABLE.name.value` (a `DV_TEXT`) by exact, case-sensitive
    /// string comparison — the Xpath `name/value='…'` semantics of BASE
    /// `master11-paths` §"Name-based Predicate". Both are ANDed.
    #[must_use]
    pub fn matches(&self, node: &Value) -> bool {
        if let Some(id) = self.archetype_node_id.as_deref()
            && node.get("archetype_node_id").and_then(Value::as_str) != Some(id)
        {
            return false;
        }
        if let Some(name) = self.name_value.as_deref()
            && node
                .get("name")
                .and_then(|n| n.get("value"))
                .and_then(Value::as_str)
                != Some(name)
        {
            return false;
        }
        true
    }
}

/// One `attribute[predicate]` segment of an openEHR path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment {
    /// The RM attribute name (`content`, `data`, `events`, `items`, …).
    pub attribute: String,
    /// The optional `[...]` predicate.
    pub predicate: Predicate,
}

/// A parsed openEHR path: `/`-separated [`PathSegment`]s, absolute (leading
/// `/`) or relative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RmPath {
    /// `true` for an absolute path (leading `/`): the starting point is the
    /// top of the structure. Navigation from a given root treats both forms
    /// identically (the root *is* the starting point).
    pub absolute: bool,
    /// The segments in order.
    pub segments: Vec<PathSegment>,
}

impl FromStr for RmPath {
    type Err = PathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(PathError::Empty);
        }
        let absolute = s.starts_with('/');
        let body = if absolute { &s[1..] } else { s };
        let mut segments = Vec::new();
        // Split on top-level '/' only — slashes inside [...] predicates (e.g.
        // name/value=...) are not separators.
        let bytes = body.as_bytes();
        let (mut i, n) = (0, bytes.len());
        let mut index = 0usize;
        while i < n {
            let start = i;
            let mut depth = 0usize;
            while i < n {
                match bytes[i] {
                    b'[' => depth += 1,
                    b']' => depth = depth.saturating_sub(1),
                    b'/' if depth == 0 => break,
                    _ => {}
                }
                i += 1;
            }
            if depth > 0 {
                return Err(PathError::UnterminatedPredicate(index));
            }
            let seg = &body[start..i];
            if seg.is_empty() {
                return Err(PathError::MissingAttribute(index));
            }
            segments.push(parse_segment(seg, index)?);
            index += 1;
            i += 1; // skip the '/'
        }
        Ok(Self { absolute, segments })
    }
}

impl fmt::Display for RmPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.absolute && self.segments.is_empty() {
            return f.write_str("/");
        }
        for (i, seg) in self.segments.iter().enumerate() {
            if self.absolute || i > 0 {
                f.write_str("/")?;
            }
            f.write_str(&seg.attribute)?;
            if !seg.predicate.is_empty() {
                f.write_str("[")?;
                if let Some(id) = &seg.predicate.archetype_node_id {
                    f.write_str(id)?;
                    if let Some(name) = &seg.predicate.name_value {
                        write!(f, ",'{name}'")?;
                    }
                } else if let Some(name) = &seg.predicate.name_value {
                    write!(f, "name/value='{name}'")?;
                }
                f.write_str("]")?;
            }
        }
        Ok(())
    }
}

/// Parse one `attribute[predicate]` segment.
fn parse_segment(seg: &str, index: usize) -> Result<PathSegment, PathError> {
    let Some(open) = seg.find('[') else {
        return Ok(PathSegment {
            attribute: seg.to_owned(),
            predicate: Predicate::default(),
        });
    };
    let attribute = seg[..open].to_owned();
    if attribute.is_empty() {
        return Err(PathError::MissingAttribute(index));
    }
    let Some(inner) = seg[open..]
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
    else {
        return Err(PathError::UnterminatedPredicate(index));
    };
    Ok(PathSegment {
        attribute,
        predicate: parse_predicate(inner)?,
    })
}

/// Parse the inside of a `[...]` predicate: the archetype-id shortcut, the
/// `,'name'` shortcut, explicit `name/value='...'` conjuncts, and the
/// explicit `@archetype_node_id='...'` form.
fn parse_predicate(inner: &str) -> Result<Predicate, PathError> {
    let mut predicate = Predicate::default();
    // `[at0001, 'standing']` — the comma shortcut binds the whole remainder.
    let (head, comma_name) = match inner.split_once(',') {
        Some((h, rest)) => (h.trim(), Some(rest.trim())),
        None => (inner.trim(), None),
    };
    if let Some(name) = comma_name {
        let name = name
            .strip_prefix('\'')
            .and_then(|n| n.strip_suffix('\''))
            .ok_or_else(|| PathError::UnsupportedPredicate(inner.to_owned()))?;
        predicate.name_value = Some(name.to_owned());
        apply_conjunct(head, &mut predicate)?;
        return Ok(predicate);
    }
    // Split on ` and ` / ` AND ` conjunctions.
    let mut rest = head;
    while let Some(pos) = rest.find(" and ").or_else(|| rest.find(" AND ")) {
        apply_conjunct(rest[..pos].trim(), &mut predicate)?;
        rest = rest[pos + 5..].trim();
    }
    apply_conjunct(rest.trim(), &mut predicate)?;
    Ok(predicate)
}

/// Interpret one predicate conjunct.
fn apply_conjunct(c: &str, predicate: &mut Predicate) -> Result<(), PathError> {
    if c.is_empty() {
        return Ok(());
    }
    // Explicit name/value='...'.
    if let Some(rest) = c.strip_prefix("name/value") {
        let value = rest
            .trim_start()
            .strip_prefix('=')
            .map(str::trim)
            .and_then(|v| v.strip_prefix('\''))
            .and_then(|v| v.strip_suffix('\''))
            .ok_or_else(|| PathError::UnsupportedPredicate(c.to_owned()))?;
        predicate.name_value = Some(value.to_owned());
        return Ok(());
    }
    // Explicit @archetype_node_id = '...'.
    if let Some(rest) = c.strip_prefix("@archetype_node_id") {
        let value = rest
            .trim_start()
            .strip_prefix('=')
            .map(str::trim)
            .and_then(|v| v.strip_prefix('\''))
            .and_then(|v| v.strip_suffix('\''))
            .ok_or_else(|| PathError::UnsupportedPredicate(c.to_owned()))?;
        predicate.archetype_node_id = Some(value.to_owned());
        return Ok(());
    }
    // The bare shortcut: an at-code / id-code or an archetype id. Anything
    // containing comparison syntax is out of the PATHABLE subset.
    if c.contains(['=', '<', '>', '\'']) {
        return Err(PathError::UnsupportedPredicate(c.to_owned()));
    }
    predicate.archetype_node_id = Some(c.to_owned());
    Ok(())
}

// ── navigation over the canonical-JSON RM tree ───────────────────────────────

/// One navigation step: from `node`, follow `segment` and append every
/// matching child to `out`.
fn step<'a>(node: &'a Value, segment: &PathSegment, out: &mut Vec<&'a Value>) {
    let Some(child) = node.get(&segment.attribute) else {
        return;
    };
    match child {
        Value::Array(items) => {
            out.extend(items.iter().filter(|v| segment.predicate.matches(v)));
        }
        v => {
            if segment.predicate.is_empty() || segment.predicate.matches(v) {
                out.push(v);
            }
        }
    }
}

/// One RM path step over the canonical-JSON tree: every value under
/// `segment.attribute` on `node` that satisfies `segment.predicate`, in
/// document order.
///
/// A single-valued attribute yields at most one value (kept only if the
/// predicate matches — a predicate constrains a node regardless of the
/// attribute's cardinality, BASE `master11-paths` §"Predicate Expressions");
/// a container attribute yields each matching element. This is the primitive
/// [`items_at_path`] iterates, exposed so path-walking consumers (the FLAT
/// converters, the composition validator) compose it instead of re-deriving a
/// walker.
#[must_use]
pub fn select_children<'a>(node: &'a Value, segment: &PathSegment) -> Vec<&'a Value> {
    let mut out = Vec::new();
    step(node, segment, &mut out);
    out
}

/// RM `PATHABLE.items_at_path`: every item the path resolves to, relative to
/// `root` (document order preserved). An unresolvable path yields an empty
/// list.
#[must_use]
pub fn items_at_path<'a>(root: &'a Value, path: &RmPath) -> Vec<&'a Value> {
    let mut current = vec![root];
    for segment in &path.segments {
        let mut next = Vec::new();
        for node in current {
            step(node, segment, &mut next);
        }
        if next.is_empty() {
            return Vec::new();
        }
        current = next;
    }
    current
}

/// RM `PATHABLE.item_at_path`: the item at a path. The spec precondition is
/// `path_unique(a_path)`; for a non-unique path the *first* item in document
/// order is returned (use [`items_at_path`] to see all), and `None` when the
/// path does not exist.
#[must_use]
pub fn item_at_path<'a>(root: &'a Value, path: &RmPath) -> Option<&'a Value> {
    items_at_path(root, path).into_iter().next()
}

/// RM `PATHABLE.path_exists`: `true` if the path resolves to at least one
/// item relative to `root`.
#[must_use]
pub fn path_exists(root: &Value, path: &RmPath) -> bool {
    !items_at_path(root, path).is_empty()
}

/// RM `PATHABLE.path_unique`: `true` if the path resolves to exactly one item
/// relative to `root`.
#[must_use]
pub fn path_unique(root: &Value, path: &RmPath) -> bool {
    items_at_path(root, path).len() == 1
}

/// RM `PATHABLE.path_of_item`: the path of `item` relative to `root`, located
/// by pointer identity (`item` must be a node *inside* `root`'s tree).
/// Container steps carry the `[archetype_node_id,'name']` predicates needed
/// to make the returned path as selective as the data allows.
#[must_use]
pub fn path_of_item(root: &Value, item: &Value) -> Option<String> {
    if std::ptr::eq(root, item) {
        return Some("/".to_owned());
    }
    let mut segments = Vec::new();
    if find_path(root, item, &mut segments) {
        let path = RmPath {
            absolute: true,
            segments,
        };
        Some(path.to_string())
    } else {
        None
    }
}

/// RM `PATHABLE.parent`: the parent RM node (nearest enclosing JSON object,
/// skipping the container array) of `item` inside `root`'s tree, or `None`
/// for the root itself / a node not in the tree. Realised as a root-anchored
/// search — no owning back-references are stored (repo convention).
#[must_use]
pub fn parent_of<'a>(root: &'a Value, item: &Value) -> Option<&'a Value> {
    if std::ptr::eq(root, item) {
        return None;
    }
    walk_for_parent(root, item)
}

/// Recursive worker for [`parent_of`]: the enclosing JSON *object* is the RM
/// parent; array elements report the object holding the array.
fn walk_for_parent<'a>(node: &'a Value, item: &Value) -> Option<&'a Value> {
    match node {
        Value::Object(map) => {
            for child in map.values() {
                if std::ptr::eq(child, item) {
                    return Some(node);
                }
                if let Value::Array(items) = child {
                    if items.iter().any(|v| std::ptr::eq(v, item)) {
                        return Some(node);
                    }
                    for v in items {
                        if let Some(found) = walk_for_parent(v, item) {
                            return Some(found);
                        }
                    }
                } else if let Some(found) = walk_for_parent(child, item) {
                    return Some(found);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(|v| walk_for_parent(v, item)),
        _ => None,
    }
}

/// Depth-first search for `item` (pointer identity), accumulating the
/// attribute segments from `node` down to it.
fn find_path(node: &Value, item: &Value, segments: &mut Vec<PathSegment>) -> bool {
    let Value::Object(map) = node else {
        return false;
    };
    for (attr, child) in map {
        let make_segment = |v: &Value| PathSegment {
            attribute: attr.clone(),
            predicate: predicate_for(v),
        };
        match child {
            Value::Array(items) => {
                for v in items {
                    segments.push(make_segment(v));
                    if std::ptr::eq(v, item) || find_path(v, item, segments) {
                        return true;
                    }
                    segments.pop();
                }
            }
            v => {
                segments.push(PathSegment {
                    attribute: attr.clone(),
                    predicate: Predicate::default(),
                });
                if std::ptr::eq(v, item) || find_path(v, item, segments) {
                    return true;
                }
                segments.pop();
            }
        }
    }
    false
}

/// The identifying predicate for a container element: its
/// `archetype_node_id` and `name/value`, when present.
fn predicate_for(v: &Value) -> Predicate {
    Predicate {
        archetype_node_id: v
            .get("archetype_node_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        name_value: v
            .get("name")
            .and_then(|n| n.get("value"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn observation() -> Value {
        json!({
            "_type": "OBSERVATION",
            "archetype_node_id": "openEHR-EHR-OBSERVATION.blood_pressure.v2",
            "name": {"_type": "DV_TEXT", "value": "Blood pressure"},
            "data": {
                "_type": "HISTORY",
                "archetype_node_id": "at0001",
                "name": {"value": "history"},
                "events": [
                    {
                        "_type": "POINT_EVENT",
                        "archetype_node_id": "at0006",
                        "name": {"value": "any event"},
                        "data": {
                            "_type": "ITEM_TREE",
                            "archetype_node_id": "at0003",
                            "name": {"value": "blood pressure"},
                            "items": [
                                {
                                    "_type": "ELEMENT",
                                    "archetype_node_id": "at0004",
                                    "name": {"value": "Systolic"},
                                    "value": {"_type": "DV_QUANTITY", "magnitude": 120.0, "units": "mm[Hg]"}
                                },
                                {
                                    "_type": "ELEMENT",
                                    "archetype_node_id": "at0005",
                                    "name": {"value": "Diastolic"},
                                    "value": {"_type": "DV_QUANTITY", "magnitude": 80.0, "units": "mm[Hg]"}
                                }
                            ]
                        }
                    }
                ]
            }
        })
    }

    fn path(s: &str) -> RmPath {
        s.parse().unwrap()
    }

    #[test]
    fn parse_forms() {
        let p = path("/data/events[at0006]/data/items[at0004,'Systolic']/value");
        assert!(p.absolute);
        assert_eq!(p.segments.len(), 5);
        assert_eq!(
            p.segments[1].predicate.archetype_node_id.as_deref(),
            Some("at0006")
        );
        assert_eq!(
            p.segments[3].predicate.name_value.as_deref(),
            Some("Systolic")
        );

        // Explicit conjunction form.
        let q = path("/data/events[at0006 and name/value='any event']");
        assert_eq!(
            q.segments[1].predicate.archetype_node_id.as_deref(),
            Some("at0006")
        );
        assert_eq!(
            q.segments[1].predicate.name_value.as_deref(),
            Some("any event")
        );

        // Relative path.
        assert!(!path("items[at0004]/value").absolute);
    }

    #[test]
    fn parse_rejections() {
        assert_eq!("".parse::<RmPath>(), Err(PathError::Empty));
        assert!(matches!(
            "/data/events[at0006".parse::<RmPath>(),
            Err(PathError::UnterminatedPredicate(1))
        ));
        // A general comparison predicate is AQL, not PATHABLE.
        assert!(matches!(
            "/data/events[at0007 and time >= '2005-06-24T09:30:00']".parse::<RmPath>(),
            Err(PathError::UnsupportedPredicate(_))
        ));
    }

    #[test]
    fn display_round_trips() {
        for s in [
            "/data/events[at0006]/data/items[at0004,'Systolic']/value",
            "/content[openEHR-EHR-SECTION.vital_signs.v1,'Vital signs']",
            "items/value",
        ] {
            assert_eq!(path(s).to_string(), s);
        }
    }

    #[test]
    fn navigation_by_archetype_node_id() {
        let obs = observation();
        let p = path("/data/events[at0006]/data/items[at0004]/value/magnitude");
        let v = item_at_path(&obs, &p).unwrap();
        assert_eq!(v.as_f64(), Some(120.0));
        assert!(path_exists(&obs, &p));
        assert!(path_unique(&obs, &p));
    }

    #[test]
    fn navigation_by_name_predicate() {
        let obs = observation();
        let p = path("/data/events[at0006]/data/items[at0005,'Diastolic']/value/magnitude");
        assert_eq!(item_at_path(&obs, &p).unwrap().as_f64(), Some(80.0));
        // Wrong name → no match.
        let q = path("/data/events[at0006]/data/items[at0005,'Systolic']");
        assert!(!path_exists(&obs, &q));
    }

    #[test]
    fn unpredicated_container_selects_all() {
        let obs = observation();
        let p = path("/data/events[at0006]/data/items");
        let items = items_at_path(&obs, &p);
        assert_eq!(items.len(), 2);
        assert!(!path_unique(&obs, &p));
        assert!(path_exists(&obs, &p));
    }

    #[test]
    fn missing_path_resolves_to_nothing() {
        let obs = observation();
        let p = path("/data/events[at0099]/data");
        assert!(!path_exists(&obs, &p));
        assert!(item_at_path(&obs, &p).is_none());
        assert!(items_at_path(&obs, &p).is_empty());
    }

    #[test]
    fn path_of_item_reconstructs_predicated_path() {
        let obs = observation();
        let p = path("/data/events[at0006]/data/items[at0005]/value");
        let target = item_at_path(&obs, &p).unwrap();
        let found = path_of_item(&obs, target).unwrap();
        assert_eq!(
            found,
            "/data/events[at0006,'any event']/data/items[at0005,'Diastolic']/value"
        );
        // The reconstructed path resolves back to the same node.
        assert!(std::ptr::eq(
            item_at_path(&obs, &found.parse().unwrap()).unwrap(),
            target
        ));
        // The root's own path is "/".
        assert_eq!(path_of_item(&obs, &obs).as_deref(), Some("/"));
    }

    #[test]
    fn parent_of_walks_up_one_rm_node() {
        let obs = observation();
        let systolic_value =
            item_at_path(&obs, &path("/data/events[at0006]/data/items[at0004]/value")).unwrap();
        let element = parent_of(&obs, systolic_value).unwrap();
        assert_eq!(
            element.get("archetype_node_id").and_then(Value::as_str),
            Some("at0004")
        );
        // Array elements report the object holding the array.
        let event = item_at_path(&obs, &path("/data/events[at0006]")).unwrap();
        let history = parent_of(&obs, event).unwrap();
        assert_eq!(
            history.get("_type").and_then(Value::as_str),
            Some("HISTORY")
        );
        // The root has no parent.
        assert!(parent_of(&obs, &obs).is_none());
    }
}

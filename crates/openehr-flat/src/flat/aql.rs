//! AQL-path parsing + RM-JSON navigation shared by the FLAT converters.
//!
//! A [`WebTemplateNode`](crate::webtemplate::WebTemplateNode)'s `aqlPath` is the
//! full RM path from the versioned-object root (compacted intermediates kept),
//! so the relative path between a parent web-template node and one of its
//! children is the sequence of RM attribute steps (`/attr[predicate]`) that
//! locates the child's RM value inside the parent's RM value — including the
//! structural nodes (`HISTORY`, `ITEM_TREE`, a single `EVENT`, the `ELEMENT`
//! wrapper) the web-template compacted away.

use serde_json::Value;

/// One `/attr[predicate]` step of an AQL path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AqlSeg {
    /// The RM attribute name (`content`, `items`, `data`, `events`, `value`, …).
    pub attr: String,
    /// The `archetype_node_id` predicate (`at0004`, `openEHR-EHR-OBSERVATION.x.v1`), if any.
    pub node_id: Option<String>,
    /// The `name/value` predicate (`[at0004,'Systolic']`), if any.
    pub name: Option<String>,
}

/// Split an AQL path into its `/attr[pred]` segments (top-level `/` only —
/// slashes inside a `[...]` predicate are not separators).
pub(crate) fn parse_path(path: &str) -> Vec<AqlSeg> {
    let mut segs = Vec::new();
    let bytes = path.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        // Skip the leading '/'.
        if bytes[i] == b'/' {
            i += 1;
        }
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
        if i > start {
            segs.push(parse_seg(&path[start..i]));
        }
    }
    segs
}

fn parse_seg(seg: &str) -> AqlSeg {
    let Some(open) = seg.find('[') else {
        return AqlSeg {
            attr: seg.to_owned(),
            node_id: None,
            name: None,
        };
    };
    let attr = seg[..open].to_owned();
    let inner = seg[open + 1..seg.rfind(']').unwrap_or(seg.len())].to_owned();
    // Predicate is `node_id` or `node_id,'name'`.
    match inner.split_once(',') {
        Some((id, name)) => {
            let name = name.trim().trim_matches('\'').to_owned();
            AqlSeg {
                attr,
                node_id: non_empty(id.trim()),
                name: Some(name),
            }
        }
        None => AqlSeg {
            attr,
            node_id: non_empty(inner.trim()),
            name: None,
        },
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_owned())
    }
}

/// The relative segments from `parent_aql` to `child_aql` (child extends parent).
pub(crate) fn relative(parent_aql: &str, child_aql: &str) -> Vec<AqlSeg> {
    let rel = child_aql.strip_prefix(parent_aql).unwrap_or(child_aql);
    parse_path(rel)
}

/// Does an RM array element satisfy a segment's predicate?
fn matches_pred(elem: &Value, seg: &AqlSeg) -> bool {
    if let Some(id) = &seg.node_id
        && elem.get("archetype_node_id").and_then(Value::as_str) != Some(id.as_str())
    {
        return false;
    }
    if let Some(name) = &seg.name {
        let rm_name = elem
            .get("name")
            .and_then(|n| n.get("value"))
            .and_then(Value::as_str);
        if rm_name != Some(name.as_str()) {
            return false;
        }
    }
    true
}

/// Resolve the RM value(s) reached by following `segs` from `rm`.
///
/// An array attribute branches (filtered by the segment predicate); an object
/// attribute is descended (its 1..1 predicate is not re-checked); the terminal
/// value(s) are returned in document order.
pub(crate) fn resolve<'a>(rm: &'a Value, segs: &[AqlSeg]) -> Vec<&'a Value> {
    let Some((seg, rest)) = segs.split_first() else {
        return vec![rm];
    };
    match rm.get(&seg.attr) {
        Some(Value::Array(items)) => items
            .iter()
            .filter(|e| matches_pred(e, seg))
            .flat_map(|e| resolve(e, rest))
            .collect(),
        Some(child) => resolve(child, rest),
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_segments() {
        let segs = parse_path(
            "/content[openEHR-EHR-SECTION.ispek_dialog.v1,'Vitals']/items[openEHR-EHR-OBSERVATION.lab_test-hba1c.v1]/data[at0001]/value",
        );
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].attr, "content");
        assert_eq!(
            segs[0].node_id.as_deref(),
            Some("openEHR-EHR-SECTION.ispek_dialog.v1")
        );
        assert_eq!(segs[0].name.as_deref(), Some("Vitals"));
        assert_eq!(segs[2].attr, "data");
        assert_eq!(segs[2].node_id.as_deref(), Some("at0001"));
        assert_eq!(segs[3].attr, "value");
        assert_eq!(segs[3].node_id, None);
    }

    #[test]
    fn relative_strips_parent() {
        let rel = relative(
            "/content[openEHR-EHR-OBSERVATION.x.v1]",
            "/content[openEHR-EHR-OBSERVATION.x.v1]/data[at0001]/events[at0002]",
        );
        assert_eq!(rel.len(), 2);
        assert_eq!(rel[0].attr, "data");
        assert_eq!(rel[1].attr, "events");
    }

    #[test]
    fn resolves_through_arrays_and_objects() {
        let rm = serde_json::json!({
            "data": {
                "events": [
                    {"archetype_node_id": "at0002", "value": {"_type": "DV_TEXT", "value": "a"}},
                    {"archetype_node_id": "at0009", "value": {"_type": "DV_TEXT", "value": "b"}}
                ]
            }
        });
        let segs = parse_path("/data/events[at0002]/value");
        let found = resolve(&rm, &segs);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["value"], "a");
    }
}

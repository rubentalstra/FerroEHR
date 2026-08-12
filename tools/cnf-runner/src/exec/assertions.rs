// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! The pure assertion evaluators — verdict logic over canonical-JSON
//! values, shared by the live driver and the transcript player so any two
//! conformant runners compute identical verdicts.
//!
//! Wire-dependent assertion families (`version`, `signature`,
//! `instance_of`) need reads the driver performs; their FACT comparison
//! still happens here so the judgement stays pure.

#![expect(
    clippy::disallowed_types,
    reason = "dev/verification tooling over JSON artifacts (the catalogue, results, wire \
              exchanges) — not the application (#1694)"
)]

use serde_json::Value;
use std::collections::BTreeMap;

use crate::exec::resultset;
use crate::exec::state::{Captured, VarStore};
use crate::model::assertion::{Assertion, IgnoreSpec};
use crate::refgrammar::{Segment, Template, ValueRef};
use crate::vocab::IgnoreSetName;

/// One assertion failure (stable, human-readable — lands in the outcome
/// record and the report).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionFailure(pub String);

/// Resolve an RM path segment sequence (`context/setting`,
/// `content[0]/data/events[0]/...`) over a canonical-JSON value.
///
/// Supported addressing: object attributes and `[<index>]` list positions —
/// the subset the catalogue's field assertions use.
#[must_use]
pub fn resolve_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for raw in path.split('/').filter(|s| !s.is_empty()) {
        let (attr, index) = match raw.split_once('[') {
            Some((attr, rest)) => {
                let index: usize = rest.strip_suffix(']')?.parse().ok()?;
                (attr, Some(index))
            }
            None => (raw, None),
        };
        if !attr.is_empty() {
            current = current.get(attr)?;
        }
        if let Some(i) = index {
            current = current.get(i)?;
        }
    }
    Some(current)
}

/// Render a template against the store (captures become their scalar
/// values; other reference kinds must have been pre-resolved by the driver).
///
/// # Errors
/// A message when a capture is unbound or non-scalar.
pub fn render_template(template: &Template, vars: &VarStore) -> Result<String, String> {
    let mut out = String::new();
    for segment in template.segments() {
        match segment {
            Segment::Lit(s) => out.push_str(s),
            Segment::Ref(ValueRef::Capture { name, .. }) => match vars.get(name) {
                Some(Captured::Scalar(s)) => out.push_str(s),
                Some(_) => return Err(format!("capture {name} is not scalar")),
                None => return Err(format!("capture {name} is not bound")),
            },
            Segment::Ref(other) => {
                return Err(format!("reference {other} must be resolved by the driver"));
            }
        }
    }
    Ok(out)
}

/// Strip the named ignore-sets and explicit paths from a value (top-level
/// path removal; nested server-assigned paths use `/`-separated forms).
///
/// A `**` segment matches zero or more intervening attribute steps, so
/// `**/uid` names one attribute wherever it occurs in a recursive RM
/// structure. Recursive containment is an RM shape, not a fixture depth:
/// `FOLDER.folders` is `List<FOLDER>` (RM common `folder.adoc`), so an
/// ignore-set that could only be written per depth would silently
/// under-cover a deeper tree.
#[must_use]
pub fn strip_ignored(value: &Value, ignored_paths: &[String]) -> Value {
    fn remove(value: &mut Value, segments: &[&str]) {
        let Some((head, rest)) = segments.split_first() else {
            return;
        };
        if *head == "**" {
            // Zero intervening steps: apply the remainder here …
            remove(value, rest);
            // … or one-or-more: descend and retry the same pattern.
            match value {
                Value::Object(map) => {
                    for child in map.values_mut() {
                        remove(child, segments);
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        remove(item, segments);
                    }
                }
                _ => {}
            }
            return;
        }
        match value {
            Value::Object(map) => {
                if rest.is_empty() {
                    map.remove(*head);
                } else if let Some(next) = map.get_mut(*head) {
                    remove(next, rest);
                }
            }
            Value::Array(items) => {
                for item in items {
                    remove(item, segments);
                }
            }
            _ => {}
        }
    }
    let mut out = value.clone();
    for path in ignored_paths {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        remove(&mut out, &segments);
    }
    out
}

/// A FLAT body: a one-level object whose keys are path-formed strings.
fn is_flat_map(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            !map.is_empty()
                && map.keys().any(|k| k.contains('/'))
                && map.values().all(|v| !matches!(v, Value::Object(_)))
        }
        _ => false,
    }
}

/// Fold a committed FLAT document's `ctx/*` input-convenience keys onto the
/// RM-tree flat paths a read-back expresses them at, per ITS-REST
/// `simplified_formats` master06 §Context Information: `ctx/participation_*`
/// → `{root}/context/_participation:{i}|*`, `ctx/health_care_facility|*` →
/// `{root}/context/_health_care_facility|*`, and the declared
/// `ctx/id_namespace`/`ctx/id_scheme` defaults expand onto every folded
/// party that carries an `|id` ("default namespace/scheme for external
/// references" — master06 §ID Namespace and Scheme). Pure default-setters
/// whose targets the ignore-set covers (`ctx/time`, `ctx/setting`,
/// composer) pass through untouched for the ignore pass.
fn fold_flat_ctx(committed: &BTreeMap<String, Value>, root: &str) -> BTreeMap<String, Value> {
    let id_namespace = committed.get("ctx/id_namespace").cloned();
    let id_scheme = committed.get("ctx/id_scheme").cloned();
    let mut out: BTreeMap<String, Value> = BTreeMap::new();
    let mut folded_id_carriers: Vec<String> = Vec::new();
    for (key, value) in committed {
        if let Some(rest) = key.strip_prefix("ctx/participation_") {
            // participation_<field>[:i] — default index 0.
            let (field, index) = match rest.split_once(':') {
                Some((f, i)) => (f, i),
                None => (rest, "0"),
            };
            let target = format!("{root}/context/_participation:{index}|{field}");
            if field == "id" {
                folded_id_carriers.push(format!("{root}/context/_participation:{index}"));
            }
            out.insert(target, value.clone());
        } else if let Some(rest) = key.strip_prefix("ctx/health_care_facility|") {
            if rest == "id" {
                folded_id_carriers.push(format!("{root}/context/_health_care_facility"));
            }
            out.insert(
                format!("{root}/context/_health_care_facility|{rest}"),
                value.clone(),
            );
        } else if key == "ctx/id_namespace" || key == "ctx/id_scheme" {
            // consumed as qualifiers below
        } else {
            out.insert(key.clone(), value.clone());
        }
    }
    for carrier in folded_id_carriers {
        if let Some(ns) = &id_namespace {
            out.insert(format!("{carrier}|id_namespace"), ns.clone());
        }
        if let Some(scheme) = &id_scheme {
            out.insert(format!("{carrier}|id_scheme"), scheme.clone());
        }
    }
    out
}

/// Whether a FLAT key falls under an ignore path: the `ctx/*` default-setter
/// spellings map to their master06 targets (`ctx/time` →
/// `context/start_time`, `ctx/setting` → `context/setting`, `ctx/composer_*`
/// → `composer`); any other key matches when its post-root path starts with
/// the ignore path (`uid` also matches the flat `_uid` spelling).
fn flat_key_ignored(key: &str, ignored_paths: &[String]) -> bool {
    let effective: &str = match key {
        "ctx/time" => "context/start_time",
        "ctx/end_time" => "context/end_time",
        "ctx/setting" => "context/setting",
        k if k.starts_with("ctx/composer") => "composer",
        k => k.split_once('/').map_or(k, |(_, rest)| rest),
    };
    let effective = effective.replace("/_uid", "/uid");
    let effective = effective.strip_prefix('_').unwrap_or(&effective);
    ignored_paths.iter().any(|p| {
        effective == p.as_str()
            || effective.starts_with(&format!("{p}/"))
            || effective.starts_with(&format!("{p}|"))
    })
}

/// Canonicalize a FLAT key by eliding every `:0` first-element index
/// (`a:0/b`, `a:0|x`, trailing `a:0`) — the index is optional on the wire
/// (`simplified_formats` master04 §Field Identifiers), so both spellings
/// name the same datum.
fn dezero(key: &str) -> String {
    let inner = key.replace(":0/", "/").replace(":0|", "|");
    inner
        .strip_suffix(":0")
        .map_or(inner.clone(), ToOwned::to_owned)
}

/// Flatten a STRUCTURED body into its FLAT key form per the
/// `simplified_formats` master04 STRUCTURED->FLAT algorithm: object keys
/// join with `/`, array elements append `:{i}` to their segment,
/// `|`-prefixed attribute keys append without a separator, and the
/// empty-string key is the element's main value.
fn flatten_structured(value: &Value, prefix: &str, out: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next = if key.is_empty() {
                    prefix.to_owned()
                } else if let Some(attr) = key.strip_prefix('|') {
                    format!("{prefix}|{attr}")
                } else if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}/{key}")
                };
                flatten_structured(child, &next, out);
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                flatten_structured(item, &format!("{prefix}:{i}"), out);
            }
        }
        leaf => {
            out.insert(prefix.to_owned(), leaf.clone());
        }
    }
}

/// STRUCTURED is recognized by its attribute-key shape: `|`-prefixed or
/// empty-string keys somewhere in the tree (master04 §Structured format) —
/// canonical JSON never carries either, so a canonical body is never
/// misread as simplified.
fn has_simplified_leaf_keys(value: &Value) -> bool {
    match value {
        Value::Object(map) => map
            .iter()
            .any(|(k, v)| k.is_empty() || k.starts_with('|') || has_simplified_leaf_keys(v)),
        Value::Array(items) => items.iter().any(has_simplified_leaf_keys),
        _ => false,
    }
}

/// A simplified body in either wire form, as a FLAT key map: a FLAT body
/// verbatim, a STRUCTURED body via the master04 flattening. `None` for
/// canonical/other bodies (a canonical COMPOSITION carries `_type`).
fn simplified_as_flat(value: &Value) -> Option<BTreeMap<String, Value>> {
    let Value::Object(map) = value else {
        return None;
    };
    if is_flat_map(value) {
        return Some(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
    }
    if !map.is_empty()
        && map.keys().all(|k| !k.contains('/'))
        && !map.contains_key("_type")
        && (map.contains_key("ctx") || has_simplified_leaf_keys(value))
    {
        let mut out = BTreeMap::new();
        flatten_structured(value, "", &mut out);
        return Some(out);
    }
    None
}

/// FLAT round-trip equivalence: fold the committed side's ctx keys onto the
/// read-back's RM-path forms (master06), drop ignore-set keys from both
/// sides, then require every committed entry to appear in the read-back
/// with an equal value (keys compared in their `:0`-elided canonical form,
/// master04 §Field Identifiers). Read-back surplus (template-derived
/// defaults such as `category|*`, terminology qualifiers, server-assigned
/// ids) is tolerated: the export is the full RM projection of the committed
/// data (`simplified_formats` master04), so the round-trip guarantee is
/// that no committed datum is lost or altered. (No openEHR spec defines a
/// round-trip comparator — our own design over the master04/master06
/// semantics.)
fn flat_equivalent(
    actual: &BTreeMap<String, Value>,
    committed: &BTreeMap<String, Value>,
    ignored_paths: &[String],
) -> bool {
    let root = actual
        .keys()
        .find(|k| !k.starts_with("ctx/"))
        .and_then(|k| k.split(['/', ':']).next())
        .unwrap_or_default()
        .to_owned();
    let normalized: BTreeMap<String, &Value> = actual.iter().map(|(k, v)| (dezero(k), v)).collect();
    let folded = fold_flat_ctx(committed, &root);
    folded
        .iter()
        .filter(|(k, _)| !flat_key_ignored(k, ignored_paths))
        .all(|(k, want)| {
            normalized
                .get(&dezero(k))
                .is_some_and(|got| resultset::cells_equal(got, want))
        })
}

/// The `equivalent` comparison.
///
/// Structural equality after stripping the
/// resolved ignore paths from BOTH sides (numeric leaves by value, via the
/// result-set cell rule — canonical JSON carries RM numbers), with canonical
/// `_type` self-tag PRESENCE normalized (see this module's `rm_cells_equal`). FLAT bodies
/// take the master06-aware round-trip rule (`flat_equivalent`).
#[must_use]
pub fn equivalent(actual: &Value, expected: &Value, ignored_paths: &[String]) -> bool {
    if let (Some(a), Some(e)) = (simplified_as_flat(actual), simplified_as_flat(expected)) {
        return flat_equivalent(&a, &e, ignored_paths);
    }
    let a = strip_ignored(actual, ignored_paths);
    let b = strip_ignored(expected, ignored_paths);
    rm_cells_equal(&a, &b)
}

/// Canonical-RM structural equality for `equivalent`: the result-set cell
/// rule everywhere, EXCEPT that a `_type` self-tag present on only one side
/// of an object is not a content difference. ITS-REST overview Resources.md
/// §JSON Format makes `_type` presence conditional ("should be used to
/// specify the RM type whenever polymorphism is involved, or when the
/// underlying definition in RM type is abstract") while the MUST governs its
/// VALUE — so a decode→re-encode path that self-tags every object (the
/// canonical codec) and a sparsely-tagged committed twin describe the same
/// RM content. Present on BOTH sides, the tags must be equal (a genuine
/// polymorphic-type substitution stays detectable).
fn rm_cells_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let keys: std::collections::BTreeSet<&str> = x
                .keys()
                .chain(y.keys())
                .map(String::as_str)
                .filter(|k| *k != "_type")
                .collect();
            let type_tags_agree = match (x.get("_type"), y.get("_type")) {
                (Some(ta), Some(tb)) => ta == tb,
                _ => true,
            };
            type_tags_agree
                && keys.iter().all(|k| match (x.get(*k), y.get(*k)) {
                    (Some(va), Some(vb)) => rm_cells_equal(va, vb),
                    _ => false,
                })
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(va, vb)| rm_cells_equal(va, vb))
        }
        _ => resultset::cells_equal(a, b),
    }
}

/// Resolve the `ignoring:` list into concrete paths: named sets come from
/// the binding (`server_assigned`) and the selectors vocabulary
/// (`ctx_defaults`); explicit paths pass through.
#[must_use]
pub fn resolve_ignore_sets(
    specs: &[IgnoreSpec],
    server_assigned: &[String],
    ctx_defaults: &[String],
) -> Vec<String> {
    let mut paths = Vec::new();
    for spec in specs {
        match spec {
            IgnoreSpec::Named(IgnoreSetName::ServerAssigned) => {
                paths.extend(server_assigned.iter().cloned());
            }
            IgnoreSpec::Named(IgnoreSetName::CtxDefaults) => {
                paths.extend(ctx_defaults.iter().cloned());
            }
            IgnoreSpec::Path(p) => paths.push(p.clone()),
        }
    }
    paths
}

/// Evaluate a `field` assertion over a response body.
///
/// # Errors
/// [`AssertionFailure`] describing the violated predicate.
pub fn eval_field(
    body: &Value,
    path: &str,
    equals: Option<&Value>,
    not_equals: Option<&Value>,
    exists: Option<bool>,
    absent: Option<bool>,
    matches: Option<&str>,
) -> Result<(), AssertionFailure> {
    let found = resolve_path(body, path);
    if let Some(true) = exists {
        return found
            .map(|_| ())
            .ok_or_else(|| AssertionFailure(format!("{path}: expected present, is absent")));
    }
    if let Some(true) = absent {
        return match found {
            None => Ok(()),
            Some(v) => Err(AssertionFailure(format!(
                "{path}: expected absent, found {v}"
            ))),
        };
    }
    let Some(actual) = found else {
        return Err(AssertionFailure(format!(
            "{path}: path resolves to nothing"
        )));
    };
    if let Some(want) = equals {
        if resultset::cells_equal(actual, want) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "{path}: {actual} != expected {want}"
        )));
    }
    if let Some(reject) = not_equals {
        if resultset::cells_equal(actual, reject) {
            return Err(AssertionFailure(format!(
                "{path}: equals the client-supplied value {reject} (must be server-set)"
            )));
        }
        return Ok(());
    }
    if let Some(pattern) = matches {
        let re = regex::Regex::new(pattern)
            .map_err(|e| AssertionFailure(format!("{path}: pattern does not compile: {e}")))?;
        let text = match actual {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        if re.is_match(&text) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "{path}: {text:?} does not match {pattern:?}"
        )));
    }
    Ok(())
}

/// Evaluate the aggregate `unique` assertion over the per-row stores
/// (law e: collected across all rows, evaluated once).
///
/// # Errors
/// [`AssertionFailure`] naming the duplicate value.
pub fn eval_unique(
    over: &crate::ids::CaptureName,
    all_rows: &[VarStore],
) -> Result<(), AssertionFailure> {
    let mut seen: Vec<&str> = Vec::new();
    for (row, store) in all_rows.iter().enumerate() {
        let Some(value) = store.scalar(over) else {
            continue; // rows that never bound the capture do not participate
        };
        if seen.contains(&value) {
            return Err(AssertionFailure(format!(
                "unique over ${{{over}}}: value {value:?} repeats at row {row}"
            )));
        }
        seen.push(value);
    }
    Ok(())
}

/// Evaluate a `returns` assertion against a (scalar-shaped) body.
///
/// # Errors
/// [`AssertionFailure`] describing the mismatch.
pub fn eval_returns(
    body: &Value,
    equals: Option<&Value>,
    matches: Option<&str>,
    omits: Option<&str>,
) -> Result<(), AssertionFailure> {
    if let Some(want) = equals {
        if resultset::cells_equal(body, want) {
            return Ok(());
        }
        return Err(AssertionFailure(format!(
            "returns: {body} != expected {want}"
        )));
    }
    let text = match body {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if let Some(pattern) = matches {
        let re = regex::Regex::new(pattern)
            .map_err(|e| AssertionFailure(format!("returns pattern does not compile: {e}")))?;
        if !re.is_match(&text) {
            return Err(AssertionFailure(format!(
                "returns: {text:?} does not match {pattern:?}"
            )));
        }
    }
    if let Some(pattern) = omits {
        let re = regex::Regex::new(pattern).map_err(|e| {
            AssertionFailure(format!("returns omits pattern does not compile: {e}"))
        })?;
        if re.is_match(&text) {
            return Err(AssertionFailure(format!(
                "returns: {text:?} matches {pattern:?} but must omit it"
            )));
        }
    }
    Ok(())
}

/// The XML Schema instance namespace, whose `type` attribute selects the
/// concrete type of an element declared with an abstract one
/// (<https://www.w3.org/TR/xmlschema-1/#xsi_type>).
const XSI_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// The judged facts of an XML document entity's ROOT element.
#[derive(Debug, Clone, PartialEq, Eq)]
struct XmlRootElement {
    /// The root's LOCAL name.
    local: String,
    /// The namespace URI the root resolves to, when the document binds one.
    namespace: Option<String>,
    /// The root's `xsi:type`, when present: the `QName`'s LOCAL part and the
    /// namespace URI its prefix resolves to (absent when the `QName` is
    /// unprefixed and the document binds no default namespace).
    xsi_type: Option<(String, Option<String>)>,
}

/// The `xsi:type` an element carries, resolved: the `QName`'s LOCAL part and the
/// namespace URI its prefix resolves to.
///
/// The attribute is identified by its resolved NAME (the XML Schema instance
/// namespace + local `type`), never by the literal prefix `xsi`, which a
/// document is free to spell any way it binds it. Its VALUE is a `QName` and
/// resolves by the `QName`-in-content rule — an unprefixed `QName` takes the
/// document's DEFAULT namespace — which `resolve_element` implements.
///
/// # Errors
/// A message when an attribute is malformed or its value is not valid UTF-8,
/// or when the `xsi:type` `QName` carries a prefix the document never bound.
fn root_xsi_type(
    reader: &mut quick_xml::NsReader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
) -> Result<Option<(String, Option<String>)>, AssertionFailure> {
    for attribute in start.attributes() {
        let attribute = attribute
            .map_err(|e| AssertionFailure(format!("xml_root: body is not well-formed XML: {e}")))?;
        let (attribute_ns, attribute_local) =
            reader.resolver_mut().resolve_attribute(attribute.key);
        let is_xsi_type = attribute_local.as_ref() == b"type"
            && matches!(
                attribute_ns,
                quick_xml::name::ResolveResult::Bound(ns) if ns.as_ref() == XSI_NAMESPACE.as_bytes()
            );
        if !is_xsi_type {
            continue;
        }
        // Attribute-value normalization per XML 1.0 §3.3.3 (the version every
        // published ITS-XML schema and canonical openEHR document is written
        // in — `<?xml version="1.0"?>`): entity references resolved, tab/CR/LF
        // folded to spaces, before the value is read as a QName.
        let value = attribute
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|e| AssertionFailure(format!("xml_root: xsi:type is not readable: {e}")))?;
        let (type_ns, type_local) = reader
            .resolver_mut()
            .resolve_element(quick_xml::name::QName(value.as_bytes()));
        let namespace = match type_ns {
            quick_xml::name::ResolveResult::Bound(ns) => {
                Some(String::from_utf8_lossy(ns.as_ref()).into_owned())
            }
            quick_xml::name::ResolveResult::Unbound => None,
            quick_xml::name::ResolveResult::Unknown(prefix) => {
                return Err(AssertionFailure(format!(
                    "xml_root: the xsi:type QName's prefix `{}` is not bound to any namespace",
                    String::from_utf8_lossy(&prefix)
                )));
            }
        };
        return Ok(Some((
            String::from_utf8_lossy(type_local.as_ref()).into_owned(),
            namespace,
        )));
    }
    Ok(None)
}

/// The root element of an XML document entity: its LOCAL name, the namespace
/// URI it resolves to when the document binds one, and its `xsi:type` when it
/// carries one.
///
/// Namespace resolution is delegated to `quick_xml::NsReader` rather than
/// pattern-matched out of the text: a conforming document may bind the
/// namespace with any prefix (or as the default `xmlns`), and only a real
/// resolver relates the root's prefix to the URI in scope for it. The
/// `xsi:type` VALUE is a `QName` too and resolves by the `QName`-in-content rule —
/// an unprefixed `QName` takes the DEFAULT namespace — which is what
/// `resolve_element` implements.
///
/// The whole document is read to end-of-input, not just its first tag: a
/// payload that is not well-formed cannot be valid against any schema either,
/// so the same §"XML Format" MUST that fixes the root also rules it out.
///
/// # Errors
/// A message when the payload is not a well-formed XML document entity.
fn xml_root_element(text: &str) -> Result<XmlRootElement, AssertionFailure> {
    let mut reader = quick_xml::NsReader::from_str(text);
    let mut root: Option<XmlRootElement> = None;
    // Element balance, tracked here rather than left to the reader's
    // configuration: at end of input an unclosed element is a truncated
    // document, which no schema can validate.
    let mut depth: i64 = 0;
    loop {
        let (resolved, event) = reader
            .read_resolved_event()
            .map_err(|e| AssertionFailure(format!("xml_root: body is not well-formed XML: {e}")))?;
        if matches!(event, quick_xml::events::Event::Start(_)) {
            depth += 1;
        } else if matches!(event, quick_xml::events::Event::End(_)) {
            depth -= 1;
        }
        match event {
            quick_xml::events::Event::Eof => break,
            quick_xml::events::Event::Start(e) | quick_xml::events::Event::Empty(e)
                if root.is_none() =>
            {
                let local = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let namespace = match resolved {
                    quick_xml::name::ResolveResult::Bound(ns) => {
                        Some(String::from_utf8_lossy(ns.as_ref()).into_owned())
                    }
                    quick_xml::name::ResolveResult::Unbound => None,
                    quick_xml::name::ResolveResult::Unknown(prefix) => {
                        return Err(AssertionFailure(format!(
                            "xml_root: the root element's prefix `{}` is not bound to any namespace",
                            String::from_utf8_lossy(&prefix)
                        )));
                    }
                };
                let xsi_type = root_xsi_type(&mut reader, &e)?;
                root = Some(XmlRootElement {
                    local,
                    namespace,
                    xsi_type,
                });
            }
            // The prolog (declaration, doctype, comments, whitespace) before
            // the root, and the whole content after it: read through, so an
            // ill-formed document fails rather than passing on its first tag.
            _ => {}
        }
    }
    if depth != 0 {
        return Err(AssertionFailure(
            "xml_root: body is not well-formed XML: the document ends with unclosed elements"
                .to_owned(),
        ));
    }
    root.ok_or_else(|| AssertionFailure("xml_root: body carries no XML element at all".to_owned()))
}

/// Evaluate an `xml_root` assertion over a canonical-XML response body.
///
/// # Errors
/// [`AssertionFailure`] when the body is not an XML document entity, when the
/// root's local name differs, when its namespace is not the expected published
/// openEHR ITS-XML target namespace, or when an expected `xsi_type` is absent
/// or names another type.
pub fn eval_xml_root(
    body: &Value,
    name: &str,
    namespace: Option<crate::vocab::XmlNamespace>,
    xsi_type: Option<&str>,
) -> Result<(), AssertionFailure> {
    let Value::String(text) = body else {
        return Err(AssertionFailure(format!(
            "xml_root: expected a canonical-XML document body, got {}",
            match body {
                Value::Null => "no body".to_owned(),
                other => other.to_string().chars().take(80).collect::<String>(),
            }
        )));
    };
    let root = xml_root_element(text)?;
    let local = root.local;
    if local != name {
        return Err(AssertionFailure(format!(
            "xml_root: document root is `{local}`, expected the published document element `{name}`"
        )));
    }
    if let Some(expected) = namespace {
        match root.namespace.as_deref() {
            Some(uri) if expected.accepts(uri) => {}
            Some(uri) => {
                return Err(AssertionFailure(format!(
                    "xml_root: root `{local}` is in namespace {uri:?}, expected {}",
                    expected.token()
                )));
            }
            None => {
                return Err(AssertionFailure(format!(
                    "xml_root: root `{local}` is in NO namespace, expected {} — every published \
                     ITS-XML schema declares elementFormDefault=\"qualified\" over its \
                     targetNamespace, so a conforming document's root is namespace-qualified",
                    expected.token()
                )));
            }
        }
    }
    let Some(expected_type) = xsi_type else {
        return Ok(());
    };
    // An element whose XSD-declared type is abstract MUST name a non-abstract
    // derived type with `xsi:type` (XML Schema Part 1 §2.6.1 + §3.4.6,
    // <https://www.w3.org/TR/xmlschema-1/#xsi_type>).
    let Some((type_local, type_uri)) = root.xsi_type else {
        return Err(AssertionFailure(format!(
            "xml_root: root `{local}` carries no xsi:type, expected `{expected_type}` — the \
             published element's declared type is abstract, and an instance may not use an \
             abstract type directly"
        )));
    };
    if type_local != expected_type {
        return Err(AssertionFailure(format!(
            "xml_root: root `{local}` names concrete type `{type_local}`, expected \
             `{expected_type}`"
        )));
    }
    // The ITS-XML complexTypes are declared in each schema's own
    // `targetNamespace`, so a type QName in another namespace names another
    // schema's type — judged by the same expectation as the root.
    if let Some(expected) = namespace {
        match type_uri.as_deref() {
            Some(uri) if expected.accepts(uri) => {}
            Some(uri) => {
                return Err(AssertionFailure(format!(
                    "xml_root: xsi:type `{type_local}` is in namespace {uri:?}, expected {}",
                    expected.token()
                )));
            }
            None => {
                return Err(AssertionFailure(format!(
                    "xml_root: xsi:type `{type_local}` resolves to NO namespace, expected {} — \
                     the ITS-XML complexTypes are declared in each schema's targetNamespace",
                    expected.token()
                )));
            }
        }
    }
    Ok(())
}

/// Whether an assertion is wire-dependent (its facts need a driver read:
/// versioned-object reads for `version`/`signature`, schema validation for
/// `instance_of`). The pure evaluators above handle the rest.
#[must_use]
pub fn is_wire_dependent(assertion: &Assertion) -> bool {
    matches!(
        assertion,
        Assertion::Version { .. } | Assertion::Signature { .. } | Assertion::InstanceOf { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn path_resolution_addresses_objects_and_lists() {
        let body = json!({
            "context": { "setting": { "value": "other care" } },
            "content": [ { "data": { "events": [ { "time": "t0" } ] } } ]
        });
        assert_eq!(
            resolve_path(&body, "context/setting/value").unwrap(),
            &json!("other care")
        );
        assert_eq!(
            resolve_path(&body, "content[0]/data/events[0]/time").unwrap(),
            &json!("t0")
        );
        assert!(resolve_path(&body, "content[1]").is_none());
    }

    #[test]
    fn recursive_ignore_segment_strips_every_depth() {
        // A FOLDER tree (RM common `folder.adoc`: `folders: List<FOLDER>`),
        // with a uid on the root and on each nested node.
        let tree = json!({
            "_type": "FOLDER",
            "uid": { "_type": "OBJECT_VERSION_ID", "value": "r::s::1" },
            "folders": [
                {
                    "_type": "FOLDER",
                    "uid": { "_type": "HIER_OBJECT_ID", "value": "a" },
                    "name": { "value": "emergency" },
                    "folders": [
                        { "_type": "FOLDER", "uid": { "value": "b" }, "name": { "value": "episode" } }
                    ]
                }
            ]
        });
        // A depth-anchored path reaches only the root.
        let shallow = strip_ignored(&tree, &["uid".to_owned()]);
        assert!(shallow.get("uid").is_none());
        assert!(shallow["folders"][0].get("uid").is_some());
        // `**/uid` reaches the root and every nested node.
        let deep = strip_ignored(&tree, &["**/uid".to_owned()]);
        assert!(deep.get("uid").is_none());
        assert!(deep["folders"][0].get("uid").is_none());
        assert!(deep["folders"][0]["folders"][0].get("uid").is_none());
        // Nothing else is touched.
        assert_eq!(deep["folders"][0]["name"]["value"], json!("emergency"));
        assert_eq!(
            deep["folders"][0]["folders"][0]["name"]["value"],
            json!("episode")
        );
    }

    #[test]
    fn field_predicates() {
        let body =
            json!({ "is_queryable": true, "audit": { "time_committed": "2026-07-21T10:00:00Z" } });
        assert!(
            eval_field(
                &body,
                "is_queryable",
                Some(&json!(true)),
                None,
                None,
                None,
                None
            )
            .is_ok()
        );
        assert!(eval_field(&body, "is_queryable", None, None, Some(true), None, None).is_ok());
        assert!(eval_field(&body, "subject", None, None, None, Some(true), None).is_ok());
        // the server-set predicate: stored time must differ from the client value
        assert!(
            eval_field(
                &body,
                "audit/time_committed",
                None,
                Some(&json!("1990-01-01T00:00:00Z")),
                None,
                None,
                None
            )
            .is_ok()
        );
        assert!(
            eval_field(
                &body,
                "audit/time_committed",
                None,
                Some(&json!("2026-07-21T10:00:00Z")),
                None,
                None,
                None
            )
            .is_err()
        );
    }

    #[test]
    fn equivalence_strips_normative_ignore_sets_only() {
        let committed = json!({ "name": { "value": "v1" }, "content": [{"x": 1}] });
        let served = json!({
            "uid": { "value": "generated::sut::1" },
            "name": { "value": "v1" },
            "content": [{"x": 1}]
        });
        assert!(equivalent(&served, &committed, &["uid".to_owned()]));
        assert!(!equivalent(&served, &committed, &[])); // nothing stripped -> uid differs
    }

    #[test]
    fn xml_root_judges_the_published_element_and_its_namespace() {
        use crate::vocab::XmlNamespace;

        let v1 = Value::String(
            r#"<?xml version="1.0" encoding="UTF-8"?>
               <composition xmlns="http://schemas.openehr.org/v1"><name/></composition>"#
                .to_owned(),
        );
        assert!(eval_xml_root(&v1, "composition", Some(XmlNamespace::Published), None).is_ok());
        assert!(eval_xml_root(&v1, "composition", Some(XmlNamespace::V1), None).is_ok());
        assert!(eval_xml_root(&v1, "composition", Some(XmlNamespace::V2), None).is_err());

        // A prefix binding is equally conforming — only the URI is asserted.
        let prefixed = Value::String(
            r#"<oe:composition xmlns:oe="http://schemas.openehr.org/v2"/>"#.to_owned(),
        );
        assert!(
            eval_xml_root(
                &prefixed,
                "composition",
                Some(XmlNamespace::Published),
                None
            )
            .is_ok()
        );

        // The defect this assertion exists for: a root in NO namespace, against
        // schemas that are elementFormDefault="qualified" over a targetNamespace.
        let unqualified = Value::String(r#"<composition archetype_node_id="x"/>"#.to_owned());
        let failure = eval_xml_root(
            &unqualified,
            "composition",
            Some(XmlNamespace::Published),
            None,
        )
        .expect_err("an unqualified root must fail");
        assert!(failure.0.contains("NO namespace"), "{failure:?}");
        // …and the name-only row still passes it, which is why the namespace
        // fact needs its own assertion rather than a regex over the body.
        assert!(eval_xml_root(&unqualified, "composition", None, None).is_ok());

        let wrong_name =
            Value::String(r#"<folder xmlns="http://schemas.openehr.org/v1"/>"#.to_owned());
        assert!(eval_xml_root(&wrong_name, "composition", None, None).is_err());

        // A JSON body is not an XML document entity.
        assert!(
            eval_xml_root(
                &json!({ "_type": "COMPOSITION" }),
                "composition",
                None,
                None
            )
            .is_err()
        );
        assert!(eval_xml_root(&Value::Null, "composition", None, None).is_err());
        // Malformed XML fails loudly rather than silently passing.
        let malformed = Value::String("<composition>".to_owned());
        assert!(eval_xml_root(&malformed, "composition", None, None).is_err());
    }

    /// The abstract-root half: `ALL/Version.xsd` publishes
    /// `<xs:element name="version" type="VERSION"/>` over
    /// `<xs:complexType name="VERSION" abstract="true">`, and XML Schema Part 1
    /// §2.6.1 + §3.4.6 (<https://www.w3.org/TR/xmlschema-1/#xsi_type>) forbid an
    /// instance from using an abstract type directly — it must select a
    /// non-abstract derived type with `xsi:type`. So on that root the concrete
    /// class is a judged fact, and it is what tells an `ORIGINAL_VERSION`
    /// response apart from an `IMPORTED_VERSION` one.
    #[test]
    fn xml_root_judges_the_concrete_type_of_an_abstract_root() {
        use crate::vocab::XmlNamespace;

        let original = Value::String(
            r#"<version xmlns="http://schemas.openehr.org/v1"
                        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                        xsi:type="ORIGINAL_VERSION"><uid/></version>"#
                .to_owned(),
        );
        assert!(
            eval_xml_root(
                &original,
                "version",
                Some(XmlNamespace::Published),
                Some("ORIGINAL_VERSION")
            )
            .is_ok()
        );
        // The discrimination the attribute exists for.
        let failure = eval_xml_root(
            &original,
            "version",
            Some(XmlNamespace::Published),
            Some("IMPORTED_VERSION"),
        )
        .expect_err("a different concrete type must fail");
        assert!(failure.0.contains("ORIGINAL_VERSION"), "{failure:?}");

        // The prefix is the document's own choice, on the attribute NAME and
        // inside the QName VALUE alike; both resolve, neither is matched.
        let prefixed = Value::String(
            r#"<oe:version xmlns:oe="http://schemas.openehr.org/v1"
                           xmlns:i="http://www.w3.org/2001/XMLSchema-instance"
                           i:type="oe:IMPORTED_VERSION"/>"#
                .to_owned(),
        );
        assert!(
            eval_xml_root(
                &prefixed,
                "version",
                Some(XmlNamespace::Published),
                Some("IMPORTED_VERSION")
            )
            .is_ok()
        );

        // An abstract root with no xsi:type at all is invalid against the
        // published schema, and says so.
        let bare = Value::String(r#"<version xmlns="http://schemas.openehr.org/v1"/>"#.to_owned());
        let failure = eval_xml_root(
            &bare,
            "version",
            Some(XmlNamespace::Published),
            Some("ORIGINAL_VERSION"),
        )
        .expect_err("an abstract root must name its concrete type");
        assert!(failure.0.contains("no xsi:type"), "{failure:?}");
        // …and a row that does not assert the type still passes it, which is
        // why the concrete class needs its own field.
        assert!(eval_xml_root(&bare, "version", Some(XmlNamespace::Published), None).is_ok());

        // A type QName from a foreign namespace names another schema's type.
        let foreign = Value::String(
            r#"<version xmlns="http://schemas.openehr.org/v1"
                        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                        xmlns:x="http://example.org/other"
                        xsi:type="x:ORIGINAL_VERSION"/>"#
                .to_owned(),
        );
        let failure = eval_xml_root(
            &foreign,
            "version",
            Some(XmlNamespace::Published),
            Some("ORIGINAL_VERSION"),
        )
        .expect_err("a foreign type namespace must fail");
        assert!(failure.0.contains("example.org"), "{failure:?}");

        // An unbound prefix on the QName is a defect, not a silent local name.
        let unbound = Value::String(
            r#"<version xmlns="http://schemas.openehr.org/v1"
                        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
                        xsi:type="nope:ORIGINAL_VERSION"/>"#
                .to_owned(),
        );
        assert!(
            eval_xml_root(
                &unbound,
                "version",
                Some(XmlNamespace::Published),
                Some("ORIGINAL_VERSION")
            )
            .is_err()
        );
    }

    #[test]
    fn unique_is_aggregate_across_rows() {
        let name = crate::ids::CaptureName::parse("new_ehr_id").unwrap();
        let mut a = VarStore::default();
        a.set(name.clone(), Captured::Scalar("id-1".into()));
        let mut b = VarStore::default();
        b.set(name.clone(), Captured::Scalar("id-2".into()));
        assert!(eval_unique(&name, &[a.clone(), b]).is_ok());
        let mut c = VarStore::default();
        c.set(name.clone(), Captured::Scalar("id-1".into()));
        assert!(eval_unique(&name, &[a, c]).is_err());
    }
}

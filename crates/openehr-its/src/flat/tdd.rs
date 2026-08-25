// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-FileCopyrightText: openEHR Foundation
// SPDX-License-Identifier: MIT AND Apache-2.0

//! TDD (Ocean/Marand **Template Data Document**) XML → canonical `COMPOSITION`
//! JSON, guided by the operational template's [`WebTemplate`].
//!
//! A TDD is a *template-namespaced* XML instance of a COMPOSITION
//! (`docs/specs/openehr/CNF/tests/platform/robot/_resources/test_data_sets/compositions/TDD/`):
//! the root element is named after the operational template and carries a
//! `template_id`, its structural nodes use the **template node names**, and its
//! leaves are `rm:`-namespaced canonical RM value fragments. It carries **no**
//! `archetype_node_id`s, no RM `_type`/`xsi:type` on most nodes, and omits the
//! RM wrapper structure (`HISTORY`/`EVENT`/`ITEM_TREE`/`ELEMENT`) — all of which
//! come from the operational template.
//!
//! The format's own account is
//! `docs/specs/openehr/ITS-REST/docs/simplified_formats/master03-design_rationale.adoc`
//! §Historical Formats, which describes the Template Data Schema (TDS) as an
//! XSD generated per template — flattening RM structures and turning at-coded
//! object nodes into element names — and a TDD as an instance document of that
//! schema. The service seam is
//! `docs/specs/openehr/SM/docs/UML/classes/i_tdd_service.adoc` (`I_TDD_SERVICE`,
//! `import_tdd`/`import_tdds`).
//!
//! NOTE: that account fixes neither a TDD grammar nor a mapping back to
//! canonical form, so the matching rule below is our own design/extension.
//!
//! # The matching rule (derived from the vendored corpus + `WebTemplate`)
//!
//! The [`WebTemplate`] tree
//! ([`build_web_template`](crate::flat::webtemplate::builder::build_web_template)) is the
//! identity oracle. Each web-template node's `aqlPath` is the full RM path
//! from the versioned-object root **with the compacted wrapper node-ids kept**
//! (e.g. `…/data[at0001]/events[at0002]/data[at0003]/items[at0004]/value`), so it
//! supplies every `archetype_node_id`, the concrete leaf RM type, and the wrapper
//! chain to re-materialise. This converter therefore drives the build from the
//! `WebTemplate` (the reverse of the FLAT builder
//! [`composition_from_flat`](crate::flat::convert::composition_from_flat) — same
//! `place`/wrapper-materialisation model) and sources each node's data from
//! the TDD:
//!
//! * A TDD element **matches** a web-template node when the element's local name
//!   (spaces ↔ `_`, and dropping an Ocean `…_as_<ConcreteType>` polymorphic
//!   suffix) equals the node's `name` (its localised term). The child is located
//!   by a **scoped search** of the parent element's subtree, so the compaction
//!   difference between the TDD (some wrappers explicit, e.g. `HISTORY`/`EVENT`;
//!   some omitted, e.g. an `ITEM_TREE` `description`) and the `WebTemplate` is
//!   absorbed — the intermediate TDD wrapper elements are simply skipped, and the
//!   canonical wrapper chain is rebuilt from the node's `aqlPath`.
//! * The **composition/entry context** the `WebTemplate` does not model as tree
//!   nodes (`COMPOSITION` `name`/`language`/`territory`/`composer`/`context`/
//!   `links`; `ENTRY` `language`/`encoding`/`subject`/`other_participations`/
//!   `narrative`/…) is read **directly** from the TDD elements and parsed through
//!   the canonical-XML [`FromXml`](crate::xml::runtime::FromXml) codec (fully typed:
//!   nested `_type`s, `xsi:type` dispatch, numeric coercion) — faithful to the
//!   instance.
//! * A **leaf** node's `DATA_VALUE` is the leaf element's `<value>` fragment,
//!   re-serialised to canonical XML with the web-template-declared concrete type
//!   as `xsi:type` and parsed as an [`openehr_rm::prelude::DataValue`].
//!
//! # Scope this wave (`// NOTE`s below)
//!
//! The vendored corpus pairs (`persistent_minimal.en.v1`, `nested.en.v1`)
//! convert and validate. Documented limits, none of which affect canonical
//! validity of the produced COMPOSITION:
//!
//! * **Compacted-wrapper instance data is not carried.** Where the TDD spells out
//!   a wrapper the `WebTemplate` compacted (`HISTORY.origin`, `EVENT.time`/`name`),
//!   the re-materialised node takes the RM-mandatory default
//!   (`DEFAULT_TIME`-equivalent), matching the FLAT reverse
//!   converter — the *leaf data values* and the full composition/entry context
//!   are faithful. Driving from the (uncompacted) OPT definition tree to recover
//!   them is future work (the same BMM-RM-model dependency `from_flat` carries).
//! * The multi-valued RM-attribute set used to re-materialise arrays is derived
//!   from the generated BMM RM attribute model (`is_multiple_attr`) — the same
//!   single source of truth the FLAT builder
//!   ([`composition_from_flat`](crate::flat::convert::composition_from_flat)) delegates
//!   to; no hard-coded list.
//! * A construct outside the corpus (e.g. an archetyped `other_context`, choice
//!   leaves) is handled on a best-effort basis; the SM `import_tdd` envelope
//!   rejects an unconvertible TDD with a typed error rather than committing a
//!   partial COMPOSITION.

#![expect(
    clippy::disallowed_types,
    reason = "canonical JSON / Simplified Formats operate on the wire value by definition \
              (ITS-JSON; ITS-REST Simplified Formats) — serde_json::Value IS the subject matter \
              (#1694)"
)]

use std::collections::HashSet;
use std::sync::LazyLock;

use openehr_rm::v1_2::paths::{PathSegment, is_archetype_root_node_id};
use serde_json::{Map, Value, json};

use crate::flat::error::FlatError;
use crate::flat::rmpath;
use crate::flat::webtemplate::model::{WebTemplate, WebTemplateNode};

/// The Ocean/Marand template-data XML namespace (the default `xmlns` on a TDD
/// root). Kept here for callers that only need the constant.
pub const TDD_TEMPLATE_NS: &str = "http://schemas.oceanehr.com/templates";

/// openEHR reference-model release stamped into a rebuilt `ARCHETYPED.rm_version`
/// (the workspace RM pin). Shared meaning with the FLAT
/// converter's `RM_VERSION`.
const RM_VERSION: &str = "1.2.0";

/// RM-mandatory temporal default for wrapper nodes whose instance value the TDD
/// leaves in a compacted wrapper this wave does not carry (see the module
/// `NOTE`). Matches the FLAT converter's `DEFAULT_TIME`.
const DEFAULT_TIME: &str = "1970-01-01T00:00:00Z";

// ── generic XML tree ─────────────────────────────────────────────────────────

/// A parsed TDD element: local name (namespace prefix stripped), attributes
/// (raw), direct text, and child elements in document order.
struct El {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<El>,
}

impl El {
    /// The `xsi:type` discriminator with any namespace prefix on the value
    /// stripped (`rm:PARTY_IDENTIFIED` → `PARTY_IDENTIFIED`), matching the
    /// canonical-XML runtime's dispatch key.
    fn xsi_type(&self) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == "xsi:type" || (k.ends_with(":type") && k.contains("xsi")))
            .map(|(_, v)| v.rsplit(':').next().unwrap_or(v))
    }

    /// The first direct child element named `name` (local-name compare).
    fn child(&self, name: &str) -> Option<&El> {
        self.children.iter().find(|c| c.name == name)
    }
}

/// Local element name with the namespace prefix stripped (`rm:value` → `value`).
fn local_name(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(raw);
    s.rsplit(':').next().unwrap_or(&s).to_string()
}

/// Parse a TDD document into the generic [`El`] tree.
///
/// NOTE: the reader diagnostics below stay flattened into the message rather
/// than carried as a source (RFC 0201) — a TDD is CLIENT content, and this
/// message is what the `400`/`422` body shows the caller.
fn parse_tree(xml: &str) -> Result<El, FlatError> {
    use quick_xml::Reader;
    use quick_xml::events::Event;

    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<El> = Vec::new();
    let mut root: Option<El> = None;

    let start_el = |e: &quick_xml::events::BytesStart<'_>| -> Result<El, FlatError> {
        let mut attrs = Vec::new();
        for a in e.attributes() {
            let a =
                a.map_err(|err| FlatError::Conversion(format!("malformed TDD attribute: {err}")))?;
            let key = String::from_utf8_lossy(a.key.as_ref()).into_owned();
            let val = String::from_utf8_lossy(&a.value).into_owned();
            attrs.push((key, val));
        }
        Ok(El {
            name: local_name(e.name().as_ref()),
            attrs,
            text: String::new(),
            children: Vec::new(),
        })
    };

    let close = |stack: &mut Vec<El>, root: &mut Option<El>| {
        if let Some(done) = stack.pop() {
            match stack.last_mut() {
                Some(parent) => parent.children.push(done),
                None => *root = Some(done),
            }
        }
    };

    loop {
        match reader
            .read_event()
            .map_err(|e| FlatError::Conversion(format!("TDD is not well-formed XML: {e}")))?
        {
            Event::Start(e) => stack.push(start_el(&e)?),
            Event::Empty(e) => {
                stack.push(start_el(&e)?);
                close(&mut stack, &mut root);
            }
            Event::End(_) => close(&mut stack, &mut root),
            Event::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    let txt = t
                        .decode()
                        .map_err(|e| FlatError::Conversion(format!("TDD text decode: {e}")))?;
                    top.text.push_str(txt.trim());
                }
            }
            Event::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    let bytes = t.into_inner();
                    top.text.push_str(&String::from_utf8_lossy(&bytes));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    root.ok_or_else(|| FlatError::Conversion("TDD has no root element".into()))
}

/// Re-serialise an [`El`] subtree to a standalone canonical-XML document so the
/// `openehr-its` [`FromXml`](crate::xml::runtime::FromXml) codec can parse it into a
/// typed RM value. The root carries the openEHR v1 namespace and, when a
/// `type_hint` is given and the element lacks its own `xsi:type`, an injected
/// `xsi:type` (the polymorphic slot's concrete type from the `WebTemplate`).
fn to_canonical_xml(el: &El, type_hint: Option<&str>) -> String {
    let mut out = String::new();
    write_el(el, true, type_hint, &mut out);
    out
}

fn write_el(el: &El, is_root: bool, type_hint: Option<&str>, out: &mut String) {
    out.push('<');
    out.push_str(&el.name);
    if is_root {
        out.push_str(" xmlns=\"http://schemas.openehr.org/v1\"");
        out.push_str(" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"");
    }
    let mut has_xsi = false;
    for (k, v) in &el.attrs {
        if k.starts_with("xmlns") {
            continue;
        }
        if k == "xsi:type" || (k.ends_with(":type") && k.contains("xsi")) {
            has_xsi = true;
            let v = v.rsplit(':').next().unwrap_or(v);
            out.push_str(" xsi:type=\"");
            out.push_str(&xml_escape(v));
            out.push('"');
        }
        // Other attributes are not part of the canonical RM wire shape and are
        // dropped (the codec keys only on element names + xsi:type).
    }
    if is_root
        && !has_xsi
        && let Some(h) = type_hint
    {
        out.push_str(" xsi:type=\"");
        out.push_str(&xml_escape(h));
        out.push('"');
    }
    if el.children.is_empty() && el.text.is_empty() {
        out.push_str("/>");
        return;
    }
    out.push('>');
    if !el.text.is_empty() {
        out.push_str(&xml_escape(&el.text));
    }
    for c in &el.children {
        write_el(c, false, None, out);
    }
    out.push_str("</");
    out.push_str(&el.name);
    out.push('>');
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── typed parsing via the canonical-XML codec ────────────────────────────────

/// Parse a TDD element as the RM type named by `type_name` (the concrete type
/// resolved from the element's own `xsi:type` or a `WebTemplate` hint), returning
/// its canonical JSON. Polymorphic slots (`PARTY_PROXY`, `DATA_VALUE`, …) are
/// parsed as the closed-set enum, which dispatches on `xsi:type`.
///
/// NOTE: the codec's refusal stays flattened into the message rather than
/// carried as a source (RFC 0201) — it names the offending path in CLIENT
/// content, which is what the `400`/`422` body must show the caller.
fn parse_typed(el: &El, type_name: &str) -> Result<Value, FlatError> {
    use crate::xml::from_canonical_xml as fx;
    use openehr_rm::prelude::{
        CodePhrase, DataValue, DvBoolean, DvCodedText, DvCount, DvDate, DvDateTime, DvDuration,
        DvEhrUri, DvIdentifier, DvMultimedia, DvOrdinal, DvParagraph, DvParsable, DvProportion,
        DvQuantity, DvScale, DvState, DvText, DvTime, DvUri, EventContext, IsmTransition, Link,
        Participation, PartyProxy,
    };

    let resolved = el.xsi_type().unwrap_or(type_name);
    let xml = to_canonical_xml(el, Some(resolved));
    macro_rules! p {
        ($t:ty) => {{
            let v: $t = fx(&xml)
                .map_err(|e| FlatError::Conversion(format!("TDD {}: {e}", stringify!($t))))?;
            crate::json::to_canonical_value(&v)
        }};
    }
    let value = match resolved {
        "DV_TEXT" => p!(DvText),
        "DV_PARAGRAPH" => p!(DvParagraph),
        "DV_CODED_TEXT" => p!(DvCodedText),
        "DV_STATE" => p!(DvState),
        "CODE_PHRASE" => p!(CodePhrase),
        "DV_DATE_TIME" => p!(DvDateTime),
        "DV_DATE" => p!(DvDate),
        "DV_TIME" => p!(DvTime),
        "DV_DURATION" => p!(DvDuration),
        "DV_QUANTITY" => p!(DvQuantity),
        "DV_COUNT" => p!(DvCount),
        "DV_PROPORTION" => p!(DvProportion),
        "DV_ORDINAL" => p!(DvOrdinal),
        "DV_SCALE" => p!(DvScale),
        "DV_BOOLEAN" => p!(DvBoolean),
        "DV_IDENTIFIER" => p!(DvIdentifier),
        "DV_MULTIMEDIA" => p!(DvMultimedia),
        "DV_URI" => p!(DvUri),
        "DV_EHR_URI" => p!(DvEhrUri),
        "DV_PARSABLE" => p!(DvParsable),
        "PARTY_PROXY" | "PARTY_IDENTIFIED" | "PARTY_SELF" | "PARTY_RELATED" => p!(PartyProxy),
        "PARTICIPATION" => p!(Participation),
        "EVENT_CONTEXT" => p!(EventContext),
        "LINK" => p!(Link),
        "ISM_TRANSITION" => p!(IsmTransition),
        // Any DV slot with an unknown/absent concrete hint dispatches through the
        // DATA_VALUE enum on the element's own xsi:type.
        _ => p!(DataValue),
    };
    Ok(value)
}

// ── build-tables (bounded; shared shape with `from_flat`) ────────────────────

/// The concrete RM type + JSON attribute-key for a *simple* RM attribute of
/// `rm_type` (parsed directly from a TDD element via the canonical-XML codec).
/// `Text` denotes a plain-string RM field (`ACTIVITY.action_archetype_id`).
enum Simple {
    Typed(&'static str),
    Text,
}

fn simple_attr(rm_type: &str, attr: &str) -> Option<Simple> {
    use Simple::{Text, Typed};
    let comp = rm_type == "COMPOSITION";
    let entry = entry_family(rm_type);
    Some(match attr {
        // Every LOCATABLE (and the ENTRY family) carries `name`.
        "name" if comp || entry || matches!(rm_type, "SECTION" | "CLUSTER" | "ACTIVITY") => {
            Typed("DV_TEXT")
        }
        // COMPOSITION in-context attributes (category is a `WebTemplate` node).
        "language" if comp || entry => Typed("CODE_PHRASE"),
        "territory" if comp => Typed("CODE_PHRASE"),
        "composer" if comp => Typed("PARTY_PROXY"),
        "context" if comp => Typed("EVENT_CONTEXT"),
        "links" if comp => Typed("LINK"),
        // ENTRY family (OBSERVATION/EVALUATION/INSTRUCTION/ACTION/ADMIN_ENTRY/…).
        "encoding" if entry => Typed("CODE_PHRASE"),
        "subject" | "provider" if entry => Typed("PARTY_PROXY"),
        "other_participations" if entry => Typed("PARTICIPATION"),
        "narrative" if entry => Typed("DV_TEXT"),
        "expiry_time" | "time" if entry => Typed("DV_DATE_TIME"),
        "ism_transition" if entry => Typed("ISM_TRANSITION"),
        // ACTIVITY.
        "timing" if rm_type == "ACTIVITY" => Typed("DV_PARSABLE"),
        "action_archetype_id" if rm_type == "ACTIVITY" => Text,
        _ => return None,
    })
}

/// Multi-valued RM attributes reachable as *simple* attributes (each occurrence
/// is one array member). The structural multi-valued set
/// (`content`/`items`/`events`/`activities`) is handled by [`place`].
fn is_multi_simple(attr: &str) -> bool {
    matches!(attr, "links" | "other_participations")
}

fn entry_family(rm_type: &str) -> bool {
    matches!(
        rm_type,
        "OBSERVATION" | "EVALUATION" | "INSTRUCTION" | "ACTION" | "ADMIN_ENTRY" | "GENERIC_ENTRY"
    )
}

fn is_locatable(rm_type: &str) -> bool {
    !(rm_type.starts_with("DV_")
        || matches!(
            rm_type,
            "CODE_PHRASE" | "EVENT_CONTEXT" | "ISM_TRANSITION" | "PARTY_PROXY" | "PARTICIPATION"
        ))
}

// ── the `WebTemplate`-driven build ─────────────────────────────────────────────

/// Convert a TDD XML instance to a canonical-JSON `COMPOSITION`, guided by `wt`.
///
/// # Errors
/// [`FlatError::Conversion`] if the TDD is not well-formed XML, its root does not
/// match the template, or a leaf/attribute fragment cannot be parsed as its RM
/// type.
pub fn from_tdd(tdd_xml: &str, wt: &WebTemplate) -> Result<Value, FlatError> {
    let root_el = parse_tree(tdd_xml)?;
    let root_id = &wt.tree.id;

    // The TDD root element must match the template root node (name, spaces↔_).
    if !name_matches(&root_el.name, node_display(&wt.tree)) {
        return Err(FlatError::Conversion(format!(
            "TDD root element <{}> does not match template root {:?} ({})",
            root_el.name, root_id, wt.template_id
        )));
    }

    // Conformance: a TDD conforms to the template-derived TDS ("a kind of
    // XSD" — AM OPT2 master02-overview.adoc §Purpose of the OPT), so an
    // element the template defines no node for is nonconforming content —
    // rejected, never silently absorbed (the wrapper-skip in the matching
    // rule is for elements ON THE PATH to a template node, not for content
    // the template does not know).
    check_conformance(&root_el, &wt.tree, "COMPOSITION")?;

    let mut comp = build_node(&root_el, &wt.tree, "COMPOSITION", "", true)?;
    if let Value::Object(m) = &mut comp {
        ensure_template_id(m, &wt.tree, &wt.template_id);
        ensure_category(m);
    }
    complete_tree(&mut comp);
    Ok(comp)
}

/// Reject TDD content the template defines no node for.
///
/// Mirrors [`build_node`]'s matching walk and classifies every element of
/// `el`'s subtree: an element is CONFORMANT when it is a simple in-context RM
/// attribute of its structural parent ([`simple_attr`] — its subtree is the
/// typed fragment), a match of a web-template child (a leaf's subtree is the
/// datum fragment; an interior match recurses), or a WRAPPER on the path to
/// at least one match (the compaction rule in the module doc). Anything else
/// is content the template-derived TDS does not define — a conversion error
/// naming the offending element, so the refusal localizes.
/// Compacted-wrapper instance metadata (the module doc's documented limit): a
/// TDD may spell out RM fields of a wrapper the `WebTemplate` compacted
/// (`HISTORY.origin`, `EVENT.time`/`name`) plus universal LOCATABLE metadata —
/// RM attribute names, never template content, deliberately not carried by
/// the build. Everything else is content the TDS does not define.
const WRAPPER_METADATA: [&str; 6] = ["name", "uid", "links", "feeder_audit", "origin", "time"];

#[expect(
    clippy::indexing_slicing,
    reason = "`simple`, `matched` and `wrapper` are all sized `el.children.len()` and every index into them is either a `match_indices(el, ..)` position over those same children or the `enumerate()` index of the same iteration, so all are in bounds by construction"
)]
fn check_conformance(el: &El, wt: &WebTemplateNode, rm_type: &str) -> Result<(), FlatError> {
    // Direct children consumed as simple in-context attributes.
    let simple: Vec<bool> = el
        .children
        .iter()
        .map(|c| simple_attr(rm_type, &c.name).is_some())
        .collect();

    // Children matched by some web-template child (with the matched
    // template node, to recurse), at any depth of the scoped search.
    let mut matched: Vec<Option<&WebTemplateNode>> = vec![None; el.children.len()];
    let mut wrapper: Vec<bool> = vec![false; el.children.len()];
    for wc in &wt.children {
        for (idx, is_direct) in match_indices(el, node_display(wc)) {
            if is_direct {
                matched[idx] = Some(wc);
            } else {
                // The match sits deeper: the direct child at `idx` is a
                // wrapper on the path to it. The deeper levels are checked
                // by the recursion below only for DIRECT matches; wrapper
                // interiors are re-checked against the SAME context.
                wrapper[idx] = true;
            }
        }
    }

    for (idx, child) in el.children.iter().enumerate() {
        if simple[idx] {
            continue;
        }
        if let Some(wc) = matched[idx] {
            if !wc.has_input() {
                check_conformance(child, wc, concrete_type(&wc.rm_type))?;
            }
            continue;
        }
        if wrapper[idx] {
            // A wrapper is transparent: its children are checked against the
            // same template node so a junk sibling BESIDE the real match is
            // still caught.
            check_conformance(child, wt, rm_type)?;
            continue;
        }
        if WRAPPER_METADATA.contains(&child.name.as_str()) {
            continue;
        }
        return Err(FlatError::Conversion(format!(
            "TDD element <{}> (under <{}>) matches no node of the operational template — \
             the document does not conform to the template-derived TDS",
            child.name, el.name
        )));
    }
    Ok(())
}

/// For each shallowest match of `display` in `parent`'s subtree (the same
/// pruned search as [`find_matches`]), the index of the DIRECT child of
/// `parent` the match sits under, and whether the match IS that direct child.
fn match_indices(parent: &El, display: &str) -> Vec<(usize, bool)> {
    let mut out = Vec::new();
    for (idx, c) in parent.children.iter().enumerate() {
        if name_matches(&c.name, display) {
            out.push((idx, true));
        } else if !find_matches(c, display).is_empty() {
            out.push((idx, false));
        }
    }
    out
}

/// Build a LOCATABLE RM node from its TDD element `el` and web-template node `wt`.
fn build_node(
    el: &El,
    wt: &WebTemplateNode,
    rm_type: &str,
    path: &str,
    is_root: bool,
) -> Result<Value, FlatError> {
    let mut obj = Map::new();
    obj.insert("_type".into(), json!(rm_type));

    // Identity: archetype_node_id (the last path segment's node-id, or the
    // web-template node's own for the root) + archetype_details for archetype
    // roots (the root's `template_id` is added by `ensure_template_id`).
    let node_id = if is_root {
        wt.node_id.clone()
    } else {
        last_node_id_str(path)
            .map(str::to_owned)
            .or_else(|| wt.node_id.clone())
    };
    if is_locatable(rm_type)
        && let Some(nid) = &node_id
    {
        obj.insert("archetype_node_id".into(), json!(nid));
        if is_archetype_root_node_id(nid) {
            obj.insert("archetype_details".into(), archetyped(nid, None));
        }
    }

    // Simple in-context RM attributes — read directly from the TDD (faithful).
    let mut i = 0;
    while let Some(child) = el.children.get(i) {
        i += 1;
        let Some(hint) = simple_attr(rm_type, &child.name) else {
            continue;
        };
        match hint {
            Simple::Text => {
                obj.insert(child.name.clone(), json!(child.text));
            }
            Simple::Typed(ty) => {
                if is_multi_simple(&child.name) {
                    let arr = obj
                        .entry(child.name.clone())
                        .or_insert_with(|| json!([]))
                        .as_array_mut();
                    if let Some(arr) = arr {
                        arr.push(parse_typed(child, ty)?);
                    }
                } else if !obj.contains_key(&child.name) {
                    obj.insert(child.name.clone(), parse_typed(child, ty)?);
                }
            }
        }
    }

    // Content children — driven by the web-template node tree. Each web-template
    // child is located in the TDD by name (scoped) and placed at its relative
    // aqlPath, re-materialising the wrapper chain the TDD/template compacted.
    for wc in &wt.children {
        let rel = rmpath::relative(path, &wc.aql_path);
        if rel.is_empty() {
            continue;
        }
        // A node the WebTemplate synthesizes for a *simple* RM in-context
        // attribute (COMPOSITION context/language/territory/composer, ENTRY
        // language/encoding/subject — `ITS-REST simplified_formats master04`
        // §"Web Template Metadata", the `inContext` marker) is already built
        // above from the TDD element by `simple_attr`; walking it again would
        // rebuild it partially (e.g. an EVENT_CONTEXT without its mandatory
        // `start_time`) and overwrite the faithful value. `category` and the
        // per-EVENT `time` are real tree data and still build through the walk.
        if (wc.in_context == Some(true) || wc.rm_type == "EVENT_CONTEXT")
            && rel
                .last()
                .is_some_and(|s| simple_attr(rm_type, &s.attribute).is_some())
        {
            continue;
        }
        let matches = find_matches(el, node_display(wc));
        for cel in matches {
            let child_value = if wc.has_input() {
                // A leaf: the ELEMENT wrapper is materialised by `place`; the
                // datum is the leaf element's `<value>` fragment.
                leaf_value(cel, wc)?
            } else {
                build_node(cel, wc, concrete_type(&wc.rm_type), &wc.aql_path, false)?
            };
            place(&mut obj, &rel, child_value, wc);
        }
    }

    Ok(Value::Object(obj))
}

/// The `DATA_VALUE` of a leaf, parsed as the web-template-declared concrete type.
///
/// Two shapes: an `ELEMENT.value` leaf (`aqlPath` ends `…/value`) is held in the
/// matched `ELEMENT` element's `<value>` child; a leaf that is a composition-/
/// entry-level `DATA_VALUE` attribute (e.g. `COMPOSITION.category`, `aqlPath`
/// `/category`) *is* the matched element.
fn leaf_value(el: &El, wc: &WebTemplateNode) -> Result<Value, FlatError> {
    let dv_type = concrete_type(&wc.rm_type);
    let frag = if wc.aql_path.ends_with("/value") {
        el.child("value").unwrap_or(el)
    } else {
        el
    };
    parse_typed(frag, dv_type)
}

// ── placement + wrapper re-materialisation (shape shared with `from_flat`) ────

/// RM attributes that are arrays (needed to re-materialise compacted structure),
/// derived from the generated BMM RM attribute model — the single source of
/// truth shared with the FLAT builder
/// ([`composition_from_flat`](crate::flat::convert::composition_from_flat), which
/// delegates here). See [`is_multiple_attr`] for the derivation.
fn is_multiple(attr: &str) -> bool {
    is_multiple_attr(attr)
}

/// The set of RM attribute names whose value is a multi-valued *structural*
/// container reachable from a versioned-object root, computed once from the
/// generated BMM RM attribute model ([`openehr_rm::v1_2::model`]) instead of a
/// hard-coded list.
///
/// "Structural" means the attribute's container is a `List`/`Set`/`Hash` **and**
/// its declared element type resolves to a model class — so leaf/primitive byte
/// arrays such as `DV_MULTIMEDIA.data : Array<Octet>` are excluded (they are
/// values, not wrapper nodes the FLAT/TDD builders re-materialise), which keeps
/// single-valued structural attributes such as `OBSERVATION.data : HISTORY`
/// correctly single. The reachability walk starts at the versioned-object roots
/// (`COMPOSITION`, `EHR_STATUS`, `FOLDER`) and follows every class-typed
/// attribute through the RM abstract→concrete descendant sets. Correctness
/// reference: the attribute cardinalities in openEHR RM `common`, `composition`,
/// `ehr`, and `data_structures`.
static MULTIVALUED_ATTRS: LazyLock<HashSet<&str>> = LazyLock::new(|| {
    let mut set: HashSet<&'static str> = HashSet::new();
    let mut seen: HashSet<&'static str> = HashSet::new();
    let mut queue: Vec<&'static str> = vec!["COMPOSITION", "EHR_STATUS", "FOLDER"];
    while let Some(cls) = queue.pop() {
        if !seen.insert(cls) {
            continue;
        }
        for attr in openehr_rm::v1_2::model::attributes(cls) {
            // Only follow (and count) attributes whose value is itself an RM
            // class; a primitive/foundation element type (e.g. `Octet`) is not a
            // structural wrapper.
            if openehr_rm::v1_2::model::class(attr.declared_type).is_none() {
                continue;
            }
            if attr.container != openehr_rm::v1_2::model::Container::None {
                set.insert(attr.name);
            }
            queue.push(attr.declared_type);
            queue.extend(
                openehr_rm::v1_2::model::descendants(attr.declared_type)
                    .iter()
                    .copied(),
            );
        }
    }
    set
});

/// Whether `attr` is a multi-valued structural RM attribute — the model-driven
/// membership test shared by the TDD and FLAT composition builders.
pub(crate) fn is_multiple_attr(attr: &str) -> bool {
    MULTIVALUED_ATTRS.contains(attr)
}

/// Insert `child_value` into `parent` at relative path `rel`, materialising the
/// compacted RM structural nodes it passes through (mirrors the FLAT builder's
/// `place`).
fn place(
    parent: &mut Map<String, Value>,
    rel: &[PathSegment],
    child_value: Value,
    wc: &WebTemplateNode,
) {
    if rel.is_empty() {
        return;
    }
    let id_idx = rel.iter().rposition(|s| is_multiple(&s.attribute));
    place_rec(parent, rel, 0, id_idx, child_value, wc);
}

#[expect(
    clippy::indexing_slicing,
    reason = "`i` is a cursor into `rel` that the recursion only advances while `i + 1 < rel.len()` (the `last` guard), so `rel[i]` and `rel[i + 1..]` are in bounds at every call"
)]
fn place_rec(
    cur: &mut Map<String, Value>,
    rel: &[PathSegment],
    i: usize,
    id_idx: Option<usize>,
    child_value: Value,
    wc: &WebTemplateNode,
) {
    let seg = &rel[i];
    let node_id = seg.predicate.archetype_node_id.as_deref();
    let last = i + 1 == rel.len();

    if Some(i) == id_idx {
        let Some(arr) = cur
            .entry(seg.attribute.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut()
        else {
            return;
        };
        if last {
            let mut el = child_value;
            set_node_id(&mut el, node_id);
            arr.push(el);
        } else {
            let mut el = new_struct(seg, rel.get(i + 1), wc.name.as_deref());
            if let Value::Object(m) = &mut el {
                place(m, &rel[i + 1..], child_value, wc);
            }
            arr.push(el);
        }
        return;
    }

    if is_multiple(&seg.attribute) {
        let Some(arr) = cur
            .entry(seg.attribute.clone())
            .or_insert_with(|| json!([]))
            .as_array_mut()
        else {
            return;
        };
        let pos = arr
            .iter()
            .position(|e| e.get("archetype_node_id").and_then(Value::as_str) == node_id);
        let idx = pos.unwrap_or_else(|| {
            arr.push(new_struct(seg, rel.get(i + 1), None));
            arr.len() - 1
        });
        if let Some(Value::Object(m)) = arr.get_mut(idx) {
            place_rec(m, rel, i + 1, id_idx, child_value, wc);
        }
        return;
    }

    if last {
        cur.insert(seg.attribute.clone(), child_value);
        return;
    }
    if !cur.contains_key(&seg.attribute) {
        cur.insert(seg.attribute.clone(), new_struct(seg, rel.get(i + 1), None));
    }
    if let Some(Value::Object(m)) = cur.get_mut(&seg.attribute) {
        place_rec(m, rel, i + 1, id_idx, child_value, wc);
    }
}

/// Create a compacted structural RM node for `seg` with its RM-mandatory fields
/// filled (mirrors the FLAT builder's `new_struct`, plus `archetype_details` for an
/// archetyped wrapper root the TDD omitted, e.g. an `ITEM_TREE` `description`).
fn new_struct(seg: &PathSegment, next: Option<&PathSegment>, name: Option<&str>) -> Value {
    let rm_type = infer_type(&seg.attribute, next.map(|s| s.attribute.as_str()));
    let node_id = seg.predicate.archetype_node_id.as_deref();
    let mut o = Map::new();
    o.insert("_type".into(), json!(rm_type));
    let display = name.or(node_id).unwrap_or(rm_type).to_owned();
    o.insert("name".into(), json!({"_type": "DV_TEXT", "value": display}));
    if let Some(nid) = node_id {
        o.insert("archetype_node_id".into(), json!(nid));
        if is_archetype_root_node_id(nid) {
            o.insert("archetype_details".into(), archetyped(nid, None));
        }
    }
    match rm_type {
        "HISTORY" => {
            o.insert(
                "origin".into(),
                json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}),
            );
            o.insert("events".into(), json!([]));
        }
        "POINT_EVENT" | "EVENT" | "INTERVAL_EVENT" => {
            o.insert(
                "time".into(),
                json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME}),
            );
        }
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" => {
            o.insert("items".into(), json!([]));
        }
        _ => {}
    }
    Value::Object(o)
}

fn infer_type(attr: &str, next: Option<&str>) -> &'static str {
    match (attr, next) {
        ("data", Some("events")) => "HISTORY",
        ("events", _) => "POINT_EVENT",
        ("items", _) => "ELEMENT",
        ("activities", _) => "ACTIVITY",
        _ => "ITEM_TREE",
    }
}

fn set_node_id(v: &mut Value, node_id: Option<&str>) {
    if let (Value::Object(m), Some(nid)) = (v, node_id) {
        m.entry("archetype_node_id".to_owned())
            .or_insert_with(|| json!(nid));
    }
}

// ── mandatory-field completion (safety net; mirrors `from_flat`) ─────────────

/// Recursively fill RM-mandatory structural fields the TDD/template did not
/// surface (event `data`, item `items`, ism `current_state`, …) so the result
/// deserialises as an `openehr-rm` `Composition`. `or_insert_with` never
/// overwrites a value the build produced.
fn complete_tree(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if let Some(ty) = m.get("_type").and_then(Value::as_str).map(str::to_owned) {
                fill_structural_mandatory(m, &ty);
            }
            for child in m.values_mut() {
                complete_tree(child);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(complete_tree),
        _ => {}
    }
}

fn fill_structural_mandatory(obj: &mut Map<String, Value>, rm_type: &str) {
    let dt = || json!({"_type": "DV_DATE_TIME", "value": DEFAULT_TIME});
    let empty_tree = || json!({"_type": "ITEM_TREE", "name": {"_type": "DV_TEXT", "value": "Tree"}, "items": []});
    match rm_type {
        "HISTORY" => {
            obj.entry("origin".to_owned()).or_insert_with(dt);
            obj.entry("events".to_owned()).or_insert_with(|| json!([]));
        }
        "POINT_EVENT" | "EVENT" => {
            obj.entry("time".to_owned()).or_insert_with(dt);
            obj.entry("data".to_owned()).or_insert_with(empty_tree);
        }
        "INTERVAL_EVENT" => {
            obj.entry("time".to_owned()).or_insert_with(dt);
            obj.entry("data".to_owned()).or_insert_with(empty_tree);
            obj.entry("width".to_owned())
                .or_insert_with(|| json!({"_type": "DV_DURATION", "value": "P0D"}));
            obj.entry("math_function".to_owned()).or_insert_with(|| {
                json!({"_type": "DV_CODED_TEXT", "value": "mean",
                       "defining_code": code_phrase("openehr", "146")})
            });
        }
        "ITEM_TREE" | "ITEM_LIST" | "ITEM_SINGLE" | "ITEM_TABLE" | "CLUSTER" => {
            obj.entry("items".to_owned()).or_insert_with(|| json!([]));
        }
        "ACTIVITY" => {
            obj.entry("action_archetype_id".to_owned())
                .or_insert_with(|| json!("/.*/"));
            obj.entry("description".to_owned())
                .or_insert_with(empty_tree);
        }
        "ISM_TRANSITION" => {
            obj.entry("current_state".to_owned())
                .or_insert_with(|| dv_coded_text("initial", "openehr", "524"));
        }
        _ => {}
    }
}

// ── small canonical-JSON builders ────────────────────────────────────────────

fn code_phrase(terminology: &str, code: &str) -> Value {
    json!({
        "_type": "CODE_PHRASE",
        "terminology_id": {"_type": "TERMINOLOGY_ID", "value": terminology},
        "code_string": code,
    })
}

fn dv_coded_text(value: &str, terminology: &str, code: &str) -> Value {
    json!({"_type": "DV_CODED_TEXT", "value": value, "defining_code": code_phrase(terminology, code)})
}

fn archetyped(archetype_id: &str, template_id: Option<&str>) -> Value {
    let mut a = Map::new();
    a.insert("_type".into(), json!("ARCHETYPED"));
    a.insert(
        "archetype_id".into(),
        json!({"_type": "ARCHETYPE_ID", "value": archetype_id}),
    );
    if let Some(t) = template_id {
        a.insert(
            "template_id".into(),
            json!({"_type": "TEMPLATE_ID", "value": t}),
        );
    }
    a.insert("rm_version".into(), json!(RM_VERSION));
    Value::Object(a)
}

/// Ensure the composition's `archetype_details` carries `archetype_id` +
/// `template_id` (a composition must be self-describing for its template).
fn ensure_template_id(comp: &mut Map<String, Value>, root: &WebTemplateNode, template_id: &str) {
    let ad = comp
        .entry("archetype_details".to_owned())
        .or_insert_with(|| {
            json!({"_type": "ARCHETYPED",
               "archetype_id": {"_type": "ARCHETYPE_ID", "value": root.node_id},
               "rm_version": RM_VERSION})
        });
    if let Value::Object(ad) = ad {
        ad.entry("template_id".to_owned())
            .or_insert_with(|| json!({"_type": "TEMPLATE_ID", "value": template_id}));
    }
}

/// `COMPOSITION.category` is mandatory; default to `event` (openEHR 433) when the
/// template did not carry it as a tree node.
fn ensure_category(comp: &mut Map<String, Value>) {
    comp.entry("category".to_owned())
        .or_insert_with(|| dv_coded_text("event", "openehr", "433"));
}

// ── name matching + path helpers ─────────────────────────────────────────────

/// The web-template node's display name for TDD element matching (its localised
/// `name`, falling back to its json `id`).
fn node_display(wc: &WebTemplateNode) -> &str {
    wc.name.as_deref().unwrap_or(&wc.id)
}

/// A TDD element local name matches a web-template display name when they are
/// equal after `' '`→`'_'` normalisation and dropping an Ocean `…_as_<Type>`
/// polymorphic suffix from the element name.
fn name_matches(el_name: &str, display: &str) -> bool {
    let base = el_name.split("_as_").next().unwrap_or(el_name);
    normalise(base) == normalise(display)
}

fn normalise(s: &str) -> String {
    s.replace(' ', "_")
}

/// All shallowest descendant elements of `parent` matching `display` (pruning at
/// each match, so a match's own subtree is not searched). Scoped so the TDD's
/// explicit wrapper elements between a parent and its template child are skipped.
fn find_matches<'a>(parent: &'a El, display: &str) -> Vec<&'a El> {
    let mut out = Vec::new();
    for c in &parent.children {
        if name_matches(&c.name, display) {
            out.push(c);
        } else {
            out.extend(find_matches(c, display));
        }
    }
    out
}

/// Map a (possibly abstract/generic) web-template rm type to the concrete RM
/// type to instantiate (`EVENT` → `POINT_EVENT`; generics stripped).
fn concrete_type(rm_type: &str) -> &str {
    match rm_type.split('<').next().unwrap_or(rm_type) {
        "EVENT" => "POINT_EVENT",
        other => other,
    }
}

/// String-level extraction of the node-id in the last `[node_id]` predicate of
/// an aqlPath (`…/items[at0004]/value` → `at0004`).
fn last_node_id_str(path: &str) -> Option<&str> {
    let close = path.rfind(']')?;
    let open = path.get(..close)?.rfind('[')?;
    path.get(open + 1..close)
}

#[cfg(test)]
mod multiplicity_tests {
    use super::is_multiple_attr;

    /// Completeness guard: the model-driven set must cover every member of the
    /// former hard-coded COMPOSITION/HISTORY/ITEM/ENTRY envelope, so replacing
    /// the list with the RM-model lookup does not lose coverage.
    #[test]
    fn covers_legacy_hardcoded_set() {
        for attr in ["content", "items", "events", "activities"] {
            assert!(
                is_multiple_attr(attr),
                "model-driven multiplicity set must include the legacy member `{attr}`",
            );
        }
    }

    /// Behaviour guard: `data` must stay single. Its only container use in the RM
    /// is `DV_MULTIMEDIA.data : Array<Octet>` (a byte array, not a structural
    /// wrapper); the structural `data` attributes (`OBSERVATION.data : HISTORY`,
    /// `ADMIN_ENTRY.data : ITEM_STRUCTURE`) are single-valued and must not be
    /// re-materialised as arrays.
    #[test]
    fn data_stays_single() {
        assert!(
            !is_multiple_attr("data"),
            "`data` must not be treated as multi-valued (OBSERVATION.data is a single HISTORY)",
        );
    }

    /// The derivation now reaches genuinely multi-valued structural attributes
    /// beyond the legacy four.
    #[test]
    fn covers_reachable_multi_valued_attributes() {
        assert!(is_multiple_attr("other_participations"));
    }

    /// An unknown / non-container attribute is not multi-valued.
    #[test]
    fn rejects_single_and_unknown() {
        assert!(!is_multiple_attr("value"));
        assert!(!is_multiple_attr("not_a_real_attribute"));
    }
}

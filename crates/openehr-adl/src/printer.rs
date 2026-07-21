//! The ADL2 serializer (phase A3c): render an assembled
//! `openehr_am::am24::aom2` [`Archetype`] back to ADL2 source text.
//!
//! The printer is the inverse of [`crate::assemble::parse_artefact`] at the
//! model level: `parse_artefact(print(a))` reconstructs a structurally-equal
//! [`Archetype`]. It emits the differential source form with text keyword
//! operators (`matches`), the canonical section order (identification →
//! specialise → language → description → definition → rules → terminology →
//! annotations → `rm_overlay` / `component_terminologies`), ODIN for the ODIN
//! sections, cADL for the definition (every construct [`crate::cadl`] parses),
//! and the `rules` via the stored expression tree.
//!
//! Section order is example-derived (`ADL2/master07.04`; the vendored grammar
//! has no top-level ordering production). NOTE: no openEHR spec governs the
//! exact whitespace layout — our own design/extension, chosen so the output
//! re-lexes 1:1.

use std::collections::BTreeMap;
use std::fmt::Write;

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::archetype::archetype_hrid::ArchetypeHrid;
use openehr_am::am24::aom2::archetype::authored_archetype::AuthoredArchetype;
use openehr_am::am24::aom2::constraint_model::archetype_slot::ArchetypeSlot;
use openehr_am::am24::aom2::constraint_model::c_archetype_root::CArchetypeRoot;
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_attribute_tuple::CAttributeTuple;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::constraint_model::c_complex_object_proxy::CComplexObjectProxy;
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
use openehr_am::am24::aom2::constraint_model::c_primitive_tuple::CPrimitiveTuple;
use openehr_am::am24::aom2::constraint_model::primitive::c_terminology_code::CTerminologyCode;
use openehr_am::am24::aom2::constraint_model::primitive::constraint_status::ConstraintStatus;
use openehr_am::am24::aom2::constraint_model::sibling_order::SiblingOrder;
use openehr_am::am24::aom2::rm_overlay::rm_overlay::RmOverlay;
use openehr_am::am24::aom2::rules::expr_constraint::ExprConstraint;
use openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::am24::beom::core::expr_value::ExprValue;
use openehr_am::am24::beom::core::expr_value_ref::ExprValueRef;
use openehr_am::am24::beom::core::expression::Expression;
use openehr_am::am24::beom::core::statement::Statement;
use openehr_am::am24::beom::core::statement_set::StatementSet;
use openehr_am::am24::resource::resource_description::ResourceDescription;
use openehr_base::prelude::{
    Cardinality, Interval, MultiplicityInterval, PointInterval, ProperInterval,
    ResourceAnnotations, ResourceDescriptionItem, TerminologyCode, TranslationDetails, Uuid,
};

/// Serialize an assembled [`Archetype`] to ADL2 source text.
///
/// `parse_artefact(&print(a))` reconstructs an [`Archetype`] structurally equal
/// to `a` (the round-trip gate).
#[must_use]
pub fn print(archetype: &Archetype) -> String {
    let mut p = Printer { out: String::new() };
    p.archetype(archetype);
    p.out
}

/// Reconstruct an [`ArchetypeHrid`] string
/// (`[ns::]publisher-package-class.concept.vMAJOR.MINOR.PATCH[-status.build]`;
/// `master07.05`).
#[must_use]
pub fn hrid_to_string(h: &ArchetypeHrid) -> String {
    let mut s = String::new();
    if let Some(ns) = &h.namespace {
        let _ = write!(s, "{ns}::");
    }
    let _ = write!(
        s,
        "{}-{}-{}.{}.v{}",
        h.rm_publisher, h.rm_package, h.rm_class, h.concept_id, h.release_version
    );
    let status = h.version_status.as_str();
    if !status.is_empty() {
        let _ = write!(s, "-{status}");
        if !h.build_count.is_empty() {
            let _ = write!(s, ".{}", h.build_count);
        }
    }
    s
}

struct Printer {
    out: String,
}

impl Printer {
    fn line(&mut self, depth: usize, s: &str) {
        for _ in 0..depth {
            self.out.push('\t');
        }
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    // ── artefact ──────────────────────────────────────────────────────────
    fn archetype(&mut self, a: &Archetype) {
        let parts = Parts::of(a);
        self.identification(&parts);
        if let Some(parent) = parts.parent_archetype_id {
            self.blank();
            self.line(0, "specialize");
            self.line(1, parent);
        }
        if !parts.is_overlay {
            self.language(parts.original_language, parts.translations);
            self.description(parts.description);
        }
        self.blank();
        self.line(0, "definition");
        self.definition(parts.definition);
        if !parts.rules.is_empty() {
            self.blank();
            self.line(0, "rules");
            for set in parts.rules {
                self.rules(set);
            }
        }
        self.terminology_section(parts.terminology);
        if let Some(ann) = parts.annotations {
            self.annotations(ann);
        }
        if let Some(rm) = parts.rm_overlay {
            self.rm_overlay(rm);
        }
        if let Some(ct) = parts.component_terminologies {
            self.component_terminologies(ct);
        }
        for overlay in parts.overlays {
            self.blank();
            self.line(
                0,
                "----------------------------------------------------------------",
            );
            self.blank();
            self.archetype(&Archetype::TemplateOverlay(Box::new(overlay.clone())));
        }
    }

    fn identification(&mut self, parts: &Parts<'_>) {
        // A flattened artefact prints with the `flat` keyword prefix
        // (`ADL2/master07.04` §Artefact declaration: "The flattened form … starts
        // with the keyword 'flat' followed by the artefact type").
        let keyword_owned;
        let keyword: &str = if parts.flat {
            keyword_owned = format!("flat {}", parts.keyword);
            &keyword_owned
        } else {
            parts.keyword
        };
        let mut meta = String::new();
        if let Some(adl) = parts.adl_version {
            let _ = write!(meta, "adl_version={adl}");
        }
        if let Some(rm) = parts.rm_release
            && !rm.is_empty()
        {
            push_meta(&mut meta, &format!("rm_release={rm}"));
        }
        if let Some(uid) = parts.uid
            && !is_nil(uid)
        {
            push_meta(&mut meta, &format!("uid={}", uid.value));
        }
        if let Some(build) = parts.build_uid
            && !is_nil(build)
        {
            push_meta(&mut meta, &format!("build_uid={}", build.value));
        }
        if parts.is_generated {
            push_meta(&mut meta, "generated");
        }
        if let Some(controlled) = parts.is_controlled {
            push_meta(
                &mut meta,
                if controlled {
                    "controlled"
                } else {
                    "uncontrolled"
                },
            );
        }
        if let Some(other) = parts.other_meta_data {
            for (k, v) in other {
                push_meta(&mut meta, &format!("{k}={v}"));
            }
        }
        if meta.is_empty() {
            self.line(0, keyword);
        } else {
            self.line(0, &format!("{keyword} ({meta})"));
        }
        self.line(1, &hrid_to_string(parts.archetype_id));
    }

    // ── language (master07.07) ──────────────────────────────────────────────
    fn language(&mut self, original: &TerminologyCode, translations: Option<&Translations>) {
        self.blank();
        self.line(0, "language");
        self.line(
            1,
            &format!("original_language = <{}>", term_code_str(original)),
        );
        if let Some(tr) = translations
            && !tr.is_empty()
        {
            self.line(1, "translations = <");
            for (lang, td) in tr {
                self.translation(lang, td, 2);
            }
            self.line(1, ">");
        }
    }

    fn translation(&mut self, lang: &str, td: &TranslationDetails, depth: usize) {
        self.line(depth, &format!("[{}] = <", quoted(lang)));
        self.line(
            depth + 1,
            &format!("language = <{}>", term_code_str(&td.language)),
        );
        self.odin_string_map(depth + 1, "author", &td.author);
        if let Some(a) = &td.accreditation {
            self.line(depth + 1, &format!("accreditation = <{}>", quoted(a)));
        }
        if let Some(v) = &td.version_last_translated {
            self.line(
                depth + 1,
                &format!("version_last_translated = <{}>", quoted(v)),
            );
        }
        self.odin_string_list(depth + 1, "other_contributors", &td.other_contributors);
        if let Some(od) = &td.other_details {
            self.odin_string_map(depth + 1, "other_details", od);
        }
        self.line(depth, ">");
    }

    // ── description (master07.08) ───────────────────────────────────────────
    fn description(&mut self, d: Option<&ResourceDescription>) {
        let Some(d) = d else { return };
        self.blank();
        self.line(0, "description");
        if let Some(t) = &d.title {
            self.line(1, &format!("title = <{}>", quoted(t)));
        }
        self.odin_string_map(1, "original_author", &d.original_author);
        self.opt_string(1, "original_namespace", d.original_namespace.as_deref());
        self.opt_string(1, "original_publisher", d.original_publisher.as_deref());
        self.odin_string_list(1, "other_contributors", &d.other_contributors);
        self.line(
            1,
            &format!("lifecycle_state = <{}>", quoted(&d.lifecycle_state)),
        );
        self.opt_string(1, "custodian_namespace", d.custodian_namespace.as_deref());
        self.opt_string(
            1,
            "custodian_organisation",
            d.custodian_organisation.as_deref(),
        );
        self.opt_string(1, "copyright", d.copyright.as_deref());
        self.opt_string(1, "licence", d.licence.as_deref());
        self.opt_string(1, "resource_package_uri", d.resource_package_uri.as_deref());
        if let Some(m) = &d.ip_acknowledgements {
            self.odin_string_map(1, "ip_acknowledgements", m);
        }
        if let Some(m) = &d.references {
            self.odin_string_map(1, "references", m);
        }
        if let Some(m) = &d.conversion_details {
            self.odin_string_map(1, "conversion_details", m);
        }
        if let Some(details) = &d.details {
            self.line(1, "details = <");
            for (lang, item) in details {
                self.description_item(lang, item, 2);
            }
            self.line(1, ">");
        }
        if let Some(m) = &d.other_details {
            self.odin_string_map(1, "other_details", m);
        }
    }

    fn description_item(&mut self, lang: &str, item: &ResourceDescriptionItem, depth: usize) {
        self.line(depth, &format!("[{}] = <", quoted(lang)));
        self.line(
            depth + 1,
            &format!("language = <{}>", term_code_str(&item.language)),
        );
        self.line(depth + 1, &format!("purpose = <{}>", quoted(&item.purpose)));
        self.odin_string_list(depth + 1, "keywords", &item.keywords);
        if let Some(u) = &item.use_ {
            self.line(depth + 1, &format!("use = <{}>", quoted(u)));
        }
        if let Some(m) = &item.misuse {
            self.line(depth + 1, &format!("misuse = <{}>", quoted(m)));
        }
        if let Some(m) = &item.original_resource_uri {
            self.odin_string_map(depth + 1, "original_resource_uri", m);
        }
        if let Some(m) = &item.other_details {
            self.odin_string_map(depth + 1, "other_details", m);
        }
        self.line(depth, ">");
    }

    // ── terminology (master07.13) ──────────────────────────────────────────
    fn terminology_section(&mut self, t: &ArchetypeTerminology) {
        self.blank();
        self.line(0, "terminology");
        self.terminology_body(t, 1);
    }

    fn terminology_body(&mut self, t: &ArchetypeTerminology, depth: usize) {
        self.line(depth, "term_definitions = <");
        for (lang, codes) in &t.term_definitions {
            self.line(depth + 1, &format!("[{}] = <", quoted(lang)));
            for (code, term) in codes {
                self.line(depth + 2, &format!("[{}] = <", quoted(code)));
                self.line(depth + 3, &format!("text = <{}>", quoted(&term.text)));
                self.line(
                    depth + 3,
                    &format!("description = <{}>", quoted(&term.description)),
                );
                if let Some(items) = &term.other_items {
                    for (k, v) in items {
                        self.line(depth + 3, &format!("{k} = <{}>", quoted(v)));
                    }
                }
                self.line(depth + 2, ">");
            }
            self.line(depth + 1, ">");
        }
        self.line(depth, ">");
        if let Some(bindings) = &t.term_bindings {
            self.line(depth, "term_bindings = <");
            for (terminology, map) in bindings {
                self.line(depth + 1, &format!("[{}] = <", quoted(terminology)));
                for (key, uri) in map {
                    self.line(depth + 2, &format!("[{}] = <{uri}>", quoted(key)));
                }
                self.line(depth + 1, ">");
            }
            self.line(depth, ">");
        }
        if let Some(value_sets) = &t.value_sets {
            self.line(depth, "value_sets = <");
            for (id, vs) in value_sets {
                self.line(depth + 1, &format!("[{}] = <", quoted(id)));
                self.line(depth + 2, &format!("id = <{}>", quoted(&vs.id)));
                let members = vs
                    .members
                    .iter()
                    .map(|m| quoted(m))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.line(depth + 2, &format!("members = <{members}>"));
                self.line(depth + 1, ">");
            }
            self.line(depth, ">");
        }
    }

    // ── annotations (master07.14) + rm_overlay (master07.12) ────────────────
    fn annotations(&mut self, a: &ResourceAnnotations) {
        if a.documentation.is_empty() {
            return;
        }
        self.blank();
        self.line(0, "annotations");
        self.line(1, "documentation = <");
        for (lang, paths) in &a.documentation {
            self.line(2, &format!("[{}] = <", quoted(lang)));
            for (path, tags) in paths {
                self.line(3, &format!("[{}] = <", quoted(path)));
                for (tag, value) in tags {
                    self.line(4, &format!("[{}] = <{}>", quoted(tag), quoted(value)));
                }
                self.line(3, ">");
            }
            self.line(2, ">");
        }
        self.line(1, ">");
    }

    fn rm_overlay(&mut self, rm: &RmOverlay) {
        let Some(vis) = &rm.rm_visibility else { return };
        if vis.is_empty() {
            return;
        }
        self.blank();
        self.line(0, "rm_overlay");
        self.line(1, "rm_visibility = <");
        for (path, v) in vis {
            self.line(2, &format!("[{}] = <", quoted(path)));
            if let Some(visibility) = &v.visibility {
                self.line(
                    3,
                    &format!("visibility = <{}>", quoted(visibility.as_str())),
                );
            }
            if let Some(alias) = &v.alias {
                self.line(3, &format!("alias = <{}>", term_code_str(alias)));
            }
            self.line(2, ">");
        }
        self.line(1, ">");
    }

    fn component_terminologies(&mut self, ct: &BTreeMap<String, ArchetypeTerminology>) {
        self.blank();
        self.line(0, "component_terminologies");
        // A bare ODIN keyed-list block (no `attr =`), keyed by archetype id
        // (`master10`; the OPT section holds `id → ARCHETYPE_TERMINOLOGY`).
        self.line(1, "<");
        for (id, term) in ct {
            self.line(2, &format!("[{}] = <", quoted(id)));
            self.terminology_body(term, 3);
            self.line(2, ">");
        }
        self.line(1, ">");
    }

    // ── ODIN helpers ────────────────────────────────────────────────────────
    fn opt_string(&mut self, depth: usize, key: &str, v: Option<&str>) {
        if let Some(v) = v {
            self.line(depth, &format!("{key} = <{}>", quoted(v)));
        }
    }

    /// Emit a `C_DEFINED_OBJECT.default_value` as the `_default` pseudo-attribute
    /// (`master06-default_values.adoc` §Syntax): `_default = (RM_TYPE) < … >`
    /// with the canonical-JSON intermediate rendered as ODIN — the inverse of
    /// the cADL parser's `_default` handling (`odin_to_json`). Scalar and
    /// object shapes round-trip through print → parse exactly; a JSON array
    /// of objects re-parses as a keyed object (see the [`Self::odin_json_entry`]
    /// NOTE) — the ODIN text is the durable form either way.
    fn default_value(&mut self, v: &serde_json::Value, depth: usize) {
        match v {
            serde_json::Value::Object(m) => {
                let head = match m.get("_type").and_then(serde_json::Value::as_str) {
                    Some(t) => format!("_default = ({t}) <"),
                    None => "_default = <".to_owned(),
                };
                self.line(depth, &head);
                for (k, val) in m {
                    if k == "_type" {
                        continue;
                    }
                    self.odin_json_entry(k, val, depth + 1);
                }
                self.line(depth, ">");
            }
            other => self.line(depth, &format!("_default = <{}>", odin_scalar(other))),
        }
    }

    /// One ODIN attribute line (or block) for a canonical-JSON member.
    ///
    /// NOTE: a JSON array of objects has no positional ODIN form — ODIN
    /// containers are keyed lists — so it renders as `["1"] = <…>` entries;
    /// re-parsing yields a `"1"`-keyed object rather than an array. The ODIN
    /// text is the durable ADL2 form; the JSON intermediate carries no
    /// spec-mandated shape (no openEHR spec governs it — our own design,
    /// matching the parser's `odin_to_json`).
    fn odin_json_entry(&mut self, key: &str, v: &serde_json::Value, depth: usize) {
        match v {
            serde_json::Value::Null => {}
            serde_json::Value::Object(m) => {
                let head = match m.get("_type").and_then(serde_json::Value::as_str) {
                    Some(t) => format!("{key} = ({t}) <"),
                    None => format!("{key} = <"),
                };
                self.line(depth, &head);
                for (k, val) in m {
                    if k == "_type" {
                        continue;
                    }
                    self.odin_json_entry(k, val, depth + 1);
                }
                self.line(depth, ">");
            }
            serde_json::Value::Array(items) if items.iter().all(is_json_scalar) => {
                let joined = items.iter().map(odin_scalar).collect::<Vec<_>>().join(", ");
                self.line(depth, &format!("{key} = <{joined}>"));
            }
            serde_json::Value::Array(items) => {
                self.line(depth, &format!("{key} = <"));
                for (i, item) in items.iter().enumerate() {
                    self.odin_json_entry(
                        &format!("[{}]", quoted(&(i + 1).to_string())),
                        item,
                        depth + 1,
                    );
                }
                self.line(depth, ">");
            }
            scalar => self.line(depth, &format!("{key} = <{}>", odin_scalar(scalar))),
        }
    }

    fn odin_string_map(&mut self, depth: usize, key: &str, m: &BTreeMap<String, String>) {
        if m.is_empty() {
            return;
        }
        self.line(depth, &format!("{key} = <"));
        for (k, v) in m {
            self.line(depth + 1, &format!("[{}] = <{}>", quoted(k), quoted(v)));
        }
        self.line(depth, ">");
    }

    fn odin_string_list(&mut self, depth: usize, key: &str, list: &[String]) {
        if list.is_empty() {
            return;
        }
        let joined = list
            .iter()
            .map(|s| quoted(s))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(depth, &format!("{key} = <{joined}>"));
    }

    // ── definition (cADL) ──────────────────────────────────────────────────
    fn definition(&mut self, def: &CComplexObject) {
        // The definition root is a `C_COMPLEX_OBJECT` (plain or `C_ARCHETYPE_ROOT`),
        // both dispatched by `object`.
        let obj = CObject::CComplexObject(def.clone());
        self.object(&obj, 1);
    }

    fn object(&mut self, obj: &CObject, depth: usize) {
        let mut head = String::new();
        if let Some(so) = sibling_order_of(obj) {
            head.push_str(&sibling_str(so));
            head.push(' ');
        }
        match obj {
            CObject::CComplexObject(CComplexObject::CComplexObject(d)) => {
                let _ = write!(head, "{}{}", d.rm_type_name, node_bracket(&d.node_id));
                head.push_str(&occ_suffix(d.occurrences.as_ref()));
                let has_body = !d.attributes.is_empty()
                    || !d.attribute_tuples.is_empty()
                    || d.default_value.is_some();
                if has_body {
                    self.line(depth, &format!("{head} matches {{"));
                    for a in &d.attributes {
                        self.attribute(a, depth + 1);
                    }
                    for t in &d.attribute_tuples {
                        self.attribute_tuple(t, depth + 1);
                    }
                    if let Some(dv) = &d.default_value {
                        self.default_value(dv, depth + 1);
                    }
                    self.line(depth, "}");
                } else {
                    self.line(depth, &head);
                }
            }
            CObject::CComplexObject(CComplexObject::CArchetypeRoot(r)) => {
                self.archetype_root(&head, r, depth);
            }
            CObject::CComplexObjectProxy(pr) => self.proxy(&head, pr, depth),
            CObject::ArchetypeSlot(s) => self.slot(&head, s, depth),
            // Primitives with a real node id are regular primitive objects.
            other => {
                if let Some(prim) = cobject_as_primitive(other) {
                    let (ty, node_id) = prim_type_and_node(other);
                    if node_id == "Primitive_node_id" {
                        // Inline primitive (only reached inside an attribute body).
                        self.line(depth, &format!("{head}{}", primitive_inline(&prim)));
                    } else {
                        let value = primitive_inline(&prim);
                        if value.is_empty() {
                            self.line(depth, &format!("{head}{ty}{}", node_bracket(&node_id)));
                        } else {
                            self.line(
                                depth,
                                &format!(
                                    "{head}{ty}{} matches {{{value}}}",
                                    node_bracket(&node_id)
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    fn attribute(&mut self, a: &CAttribute, depth: usize) {
        let name = match &a.differential_path {
            Some(path) => format!("{path}/{}", a.rm_attribute_name),
            None => a.rm_attribute_name.clone(),
        };
        let mut head = name;
        if let Some(ex) = &a.existence {
            let _ = write!(head, " existence matches {}", mult_braces(ex));
        }
        if let Some(card) = &a.cardinality {
            let _ = write!(head, " cardinality matches {}", card_braces(card));
        }
        if a.children.is_empty() {
            self.line(depth, &head);
            return;
        }
        // A single C_STRING regex child came from the `attr matches {/re/}`
        // contained-regexp shortcut (`cadl2.g4`); re-emit that form.
        if let [CObject::CString(cs)] = a.children.as_slice()
            && let Some(regex) = regex_of(&cs.constraint)
        {
            let mut body = regex.to_owned();
            if let Some(assumed) = &cs.assumed_value {
                let _ = write!(body, "; {}", quoted(assumed));
            }
            self.line(depth, &format!("{head} matches {{{body}}}"));
            return;
        }
        // A single inline primitive child prints inline; regular objects nest.
        if let [child] = a.children.as_slice()
            && let Some(prim) = cobject_as_primitive(child)
            && prim_type_and_node(child).1 == "Primitive_node_id"
        {
            self.line(
                depth,
                &format!("{head} matches {{{}}}", primitive_inline(&prim)),
            );
            return;
        }
        self.line(depth, &format!("{head} matches {{"));
        for child in &a.children {
            self.object(child, depth + 1);
        }
        self.line(depth, "}");
    }

    fn attribute_tuple(&mut self, t: &CAttributeTuple, depth: usize) {
        let members = t
            .members
            .iter()
            .map(|m| m.rm_attribute_name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        self.line(depth, &format!("[{members}] matches {{"));
        // Tuple rows are comma-separated (`cadl2.g4` `c_primitive_tuple
        // (',' c_primitive_tuple)*`); emit a trailing comma on all but the last.
        let last = t.tuples.len().saturating_sub(1);
        for (idx, row) in t.tuples.iter().enumerate() {
            self.tuple_row(row, depth + 1, idx != last);
        }
        self.line(depth, "}");
    }

    fn tuple_row(&mut self, row: &CPrimitiveTuple, depth: usize, comma: bool) {
        let items = row
            .members
            .iter()
            .map(|m| format!("{{{}}}", primitive_inline(m)))
            .collect::<Vec<_>>()
            .join(", ");
        let sep = if comma { "," } else { "" };
        self.line(depth, &format!("[{items}]{sep}"));
    }

    fn archetype_root(&mut self, head: &str, r: &CArchetypeRoot, depth: usize) {
        let node = if r.node_id.is_empty() {
            format!("[{}]", r.archetype_ref)
        } else {
            format!("[{}, {}]", r.node_id, r.archetype_ref)
        };
        let occ = occ_suffix(r.occurrences.as_ref());
        // An OPT-inlined root carries the flattened filler structure in
        // `attributes`/`attribute_tuples` (OPT2 master03 §Flattening); it prints
        // as a plain object head `TYPE[id, ref] occ matches { … }` (no
        // `use_archetype` keyword), which the cADL parser reads back as a
        // `C_ARCHETYPE_ROOT`. A source-form external reference / slot filler has
        // Void children and prints with the `use_archetype` keyword
        // (`cadl2.g4` c_archetype_root).
        if r.attributes.is_empty() && r.attribute_tuples.is_empty() {
            self.line(
                depth,
                &format!("{head}use_archetype {}{node}{occ}", r.rm_type_name),
            );
            return;
        }
        self.line(
            depth,
            &format!("{head}{}{node}{occ} matches {{", r.rm_type_name),
        );
        for a in &r.attributes {
            self.attribute(a, depth + 1);
        }
        for t in &r.attribute_tuples {
            self.attribute_tuple(t, depth + 1);
        }
        self.line(depth, "}");
    }

    fn proxy(&mut self, head: &str, pr: &CComplexObjectProxy, depth: usize) {
        let occ = occ_suffix(pr.occurrences.as_ref());
        self.line(
            depth,
            &format!(
                "{head}use_node {}{}{occ} {}",
                pr.rm_type_name,
                node_bracket(&pr.node_id),
                pr.target_path
            ),
        );
    }

    fn slot(&mut self, head: &str, s: &ArchetypeSlot, depth: usize) {
        let base = format!(
            "{head}allow_archetype {}{}",
            s.rm_type_name,
            node_bracket(&s.node_id)
        );
        if s.is_closed {
            self.line(depth, &format!("{base} closed"));
            return;
        }
        let occ = occ_suffix(s.occurrences.as_ref());
        if s.includes.is_empty() && s.excludes.is_empty() {
            self.line(depth, &format!("{base}{occ}"));
            return;
        }
        self.line(depth, &format!("{base}{occ} matches {{"));
        for inc in &s.includes {
            self.line(depth + 1, "include");
            self.line(depth + 2, &assertion_str(inc));
        }
        for exc in &s.excludes {
            self.line(depth + 1, "exclude");
            self.line(depth + 2, &assertion_str(exc));
        }
        self.line(depth, "}");
    }

    // ── rules (master07.11; BEL) ───────────────────────────────────────────
    fn rules(&mut self, set: &StatementSet) {
        for stmt in &set.statement {
            match stmt {
                Statement::Assertion(a) => {
                    let expr = expression_str(&a.expression);
                    match &a.tag {
                        Some(tag) => self.line(1, &format!("{tag}: {expr}")),
                        None => self.line(1, &expr),
                    }
                }
                Statement::Assignment(a) => {
                    self.line(
                        1,
                        &format!("${} = {}", a.target.name, expr_value_str(&a.source)),
                    );
                }
                Statement::VariableDeclaration(v) => {
                    self.line(1, &format!("${} : {}", v.name, type_def_name(&v.r#type)));
                }
            }
        }
    }
}

// ── the artefact-kind projection ──────────────────────────────────────────

/// A uniform view of the fields the printer needs, over the four
/// [`Archetype`] variants.
struct Parts<'a> {
    keyword: &'a str,
    /// True for a flattened artefact (prints the `flat` keyword prefix,
    /// `ADL2/master07.04` §Artefact declaration): a specialised archetype whose
    /// `is_differential` flag is cleared by the flattener.
    flat: bool,
    is_overlay: bool,
    archetype_id: &'a ArchetypeHrid,
    parent_archetype_id: Option<&'a str>,
    adl_version: Option<&'a str>,
    rm_release: Option<&'a str>,
    uid: Option<&'a Uuid>,
    build_uid: Option<&'a Uuid>,
    is_generated: bool,
    is_controlled: Option<bool>,
    other_meta_data: Option<&'a BTreeMap<String, String>>,
    original_language: &'a TerminologyCode,
    translations: Option<&'a Translations>,
    description: Option<&'a ResourceDescription>,
    definition: &'a CComplexObject,
    rules: &'a [StatementSet],
    terminology: &'a ArchetypeTerminology,
    annotations: Option<&'a ResourceAnnotations>,
    rm_overlay: Option<&'a RmOverlay>,
    component_terminologies: Option<&'a BTreeMap<String, ArchetypeTerminology>>,
    overlays: &'a [openehr_am::am24::aom2::archetype::template_overlay::TemplateOverlay],
}

type Translations = BTreeMap<String, TranslationDetails>;

/// A placeholder language for a `TEMPLATE_OVERLAY` (which has no language of its
/// own — it inherits the owner's); the printer never emits it.
static OVERLAY_LANG: std::sync::LazyLock<TerminologyCode> =
    std::sync::LazyLock::new(|| TerminologyCode {
        terminology_id: "ISO_639-1".to_owned(),
        terminology_version: None,
        code_string: "en".to_owned(),
        uri: None,
    });

const NO_OVERLAYS: &[openehr_am::am24::aom2::archetype::template_overlay::TemplateOverlay] = &[];

impl<'a> Parts<'a> {
    fn of(a: &'a Archetype) -> Self {
        match a {
            Archetype::TemplateOverlay(o) => Parts {
                keyword: "template_overlay",
                flat: false,
                is_overlay: true,
                archetype_id: &o.archetype_id,
                parent_archetype_id: o.parent_archetype_id.as_deref(),
                adl_version: None,
                rm_release: None,
                uid: None,
                build_uid: None,
                is_generated: false,
                is_controlled: None,
                other_meta_data: None,
                original_language: &OVERLAY_LANG,
                translations: None,
                description: None,
                definition: &o.definition,
                rules: &o.rules,
                terminology: &o.terminology,
                annotations: None,
                rm_overlay: o.rm_overlay.as_ref(),
                component_terminologies: None,
                overlays: NO_OVERLAYS,
            },
            Archetype::AuthoredArchetype(inner) => match inner.as_ref() {
                AuthoredArchetype::AuthoredArchetype(d) => Parts {
                    keyword: "archetype",
                    flat: d.parent_archetype_id.is_some() && !d.is_differential,
                    is_overlay: false,
                    archetype_id: &d.archetype_id,
                    parent_archetype_id: d.parent_archetype_id.as_deref(),
                    adl_version: d.adl_version.as_deref(),
                    rm_release: Some(&d.rm_release),
                    uid: d.uid.as_ref(),
                    build_uid: Some(&d.build_uid),
                    is_generated: d.is_generated,
                    is_controlled: d.is_controlled,
                    other_meta_data: Some(&d.other_meta_data),
                    original_language: &d.original_language,
                    translations: d.translations.as_ref(),
                    description: d.description.as_deref(),
                    definition: &d.definition,
                    rules: &d.rules,
                    terminology: &d.terminology,
                    annotations: d.annotations.as_ref(),
                    rm_overlay: d.rm_overlay.as_ref(),
                    component_terminologies: None,
                    overlays: NO_OVERLAYS,
                },
                AuthoredArchetype::Template(t) => Parts {
                    keyword: "template",
                    flat: t.parent_archetype_id.is_some() && !t.is_differential,
                    is_overlay: false,
                    archetype_id: &t.archetype_id,
                    parent_archetype_id: t.parent_archetype_id.as_deref(),
                    adl_version: t.adl_version.as_deref(),
                    rm_release: Some(&t.rm_release),
                    uid: t.uid.as_ref(),
                    build_uid: Some(&t.build_uid),
                    is_generated: t.is_generated,
                    is_controlled: t.is_controlled,
                    other_meta_data: Some(&t.other_meta_data),
                    original_language: &t.original_language,
                    translations: t.translations.as_ref(),
                    description: t.description.as_ref(),
                    definition: &t.definition,
                    rules: &t.rules,
                    terminology: &t.terminology,
                    annotations: t.annotations.as_ref(),
                    rm_overlay: t.rm_overlay.as_ref(),
                    component_terminologies: None,
                    overlays: &t.overlays,
                },
                AuthoredArchetype::OperationalTemplate(o) => Parts {
                    keyword: "operational_template",
                    flat: false,
                    is_overlay: false,
                    archetype_id: &o.archetype_id,
                    parent_archetype_id: o.parent_archetype_id.as_deref(),
                    adl_version: o.adl_version.as_deref(),
                    rm_release: Some(&o.rm_release),
                    uid: o.uid.as_ref(),
                    build_uid: Some(&o.build_uid),
                    is_generated: o.is_generated,
                    is_controlled: o.is_controlled,
                    other_meta_data: Some(&o.other_meta_data),
                    original_language: &o.original_language,
                    translations: o.translations.as_ref(),
                    description: o.description.as_ref(),
                    definition: &o.definition,
                    rules: &o.rules,
                    terminology: &o.terminology,
                    annotations: o.annotations.as_ref(),
                    rm_overlay: o.rm_overlay.as_ref(),
                    component_terminologies: o.component_terminologies.as_ref(),
                    overlays: NO_OVERLAYS,
                },
            },
        }
    }
}

// ── free helpers ──────────────────────────────────────────────────────────

fn push_meta(meta: &mut String, item: &str) {
    if !meta.is_empty() {
        meta.push_str("; ");
    }
    meta.push_str(item);
}

fn is_nil(u: &Uuid) -> bool {
    u.value.is_nil()
}

fn node_bracket(node_id: &str) -> String {
    if node_id.is_empty() {
        String::new()
    } else {
        format!("[{node_id}]")
    }
}

fn occ_suffix(occ: Option<&MultiplicityInterval>) -> String {
    occ.map(|m| format!(" occurrences matches {}", mult_braces(m)))
        .unwrap_or_default()
}

fn mult_braces(m: &MultiplicityInterval) -> String {
    let lo = m.lower.unwrap_or(0);
    if m.upper_unbounded {
        return format!("{{{lo}..*}}");
    }
    let hi = m.upper.unwrap_or(lo);
    if lo == hi {
        format!("{{{lo}}}")
    } else {
        format!("{{{lo}..{hi}}}")
    }
}

fn card_braces(c: &Cardinality) -> String {
    let inner = {
        let m = &c.interval;
        let lo = m.lower.unwrap_or(0);
        if m.upper_unbounded {
            format!("{lo}..*")
        } else {
            let hi = m.upper.unwrap_or(lo);
            if lo == hi {
                format!("{lo}")
            } else {
                format!("{lo}..{hi}")
            }
        }
    };
    let mut s = format!("{{{inner}");
    if !c.is_ordered {
        s.push_str("; unordered");
    }
    if c.is_unique {
        s.push_str("; unique");
    }
    s.push('}');
    s
}

fn sibling_order_of(obj: &CObject) -> Option<&SiblingOrder> {
    match obj {
        CObject::CComplexObject(CComplexObject::CComplexObject(d)) => d.sibling_order.as_ref(),
        CObject::CComplexObject(CComplexObject::CArchetypeRoot(r)) => r.sibling_order.as_ref(),
        CObject::CComplexObjectProxy(p) => p.sibling_order.as_ref(),
        CObject::ArchetypeSlot(s) => s.sibling_order.as_ref(),
        CObject::CBoolean(c) => c.sibling_order.as_ref(),
        CObject::CDate(c) => c.sibling_order.as_ref(),
        CObject::CDateTime(c) => c.sibling_order.as_ref(),
        CObject::CDuration(c) => c.sibling_order.as_ref(),
        CObject::CInteger(c) => c.sibling_order.as_ref(),
        CObject::CReal(c) => c.sibling_order.as_ref(),
        CObject::CString(c) => c.sibling_order.as_ref(),
        CObject::CTerminologyCode(c) => c.sibling_order.as_ref(),
        CObject::CTime(c) => c.sibling_order.as_ref(),
    }
}

fn sibling_str(so: &SiblingOrder) -> String {
    let kw = if so.is_before { "before" } else { "after" };
    format!("{kw}[{}]", so.sibling_node_id)
}

fn prim_type_and_node(obj: &CObject) -> (String, String) {
    match obj {
        CObject::CBoolean(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CDate(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CDateTime(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CDuration(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CInteger(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CReal(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CString(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CTerminologyCode(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        CObject::CTime(c) => (c.rm_type_name.clone(), c.node_id.clone()),
        _ => (String::new(), String::new()),
    }
}

fn cobject_as_primitive(obj: &CObject) -> Option<CPrimitiveObject> {
    Some(match obj {
        CObject::CBoolean(c) => CPrimitiveObject::CBoolean(c.clone()),
        CObject::CDate(c) => CPrimitiveObject::CDate(c.clone()),
        CObject::CDateTime(c) => CPrimitiveObject::CDateTime(c.clone()),
        CObject::CDuration(c) => CPrimitiveObject::CDuration(c.clone()),
        CObject::CInteger(c) => CPrimitiveObject::CInteger(c.clone()),
        CObject::CReal(c) => CPrimitiveObject::CReal(c.clone()),
        CObject::CString(c) => CPrimitiveObject::CString(c.clone()),
        CObject::CTerminologyCode(c) => CPrimitiveObject::CTerminologyCode(c.clone()),
        CObject::CTime(c) => CPrimitiveObject::CTime(c.clone()),
        _ => return None,
    })
}

/// True if a `C_STRING` constraint is a single delimited regex (`/re/` or
/// `^re^`) — the `cadl2.g4` `CONTAINED_REGEXP` form the parser stores verbatim.
/// A plain string *value* that merely starts with `/` (e.g. a unit `"/min"`) is
/// not a regex — the delimiter must close, so both ends are required.
/// The single delimited regex a `C_STRING` constraint carries, if any.
fn regex_of(constraint: &[String]) -> Option<&str> {
    match constraint {
        [one] if is_delimited_regex(one) => Some(one),
        _ => None,
    }
}

fn is_delimited_regex(s: &str) -> bool {
    (s.len() >= 2 && s.starts_with('/') && s.ends_with('/'))
        || (s.len() >= 2 && s.starts_with('^') && s.ends_with('^'))
}

/// The inline value text of a primitive constraint (`55`, `|0..100|`,
/// `"x", "y"; "z"`, `yyyy-mm-??`, `[ac1]`, …) — the body a `matches { … }`
/// wraps. Mirrors `cadl2_primitives.g4`.
fn primitive_inline(prim: &CPrimitiveObject) -> String {
    match prim {
        CPrimitiveObject::CBoolean(c) => {
            let mut s = c
                .constraint
                .iter()
                .map(|b| bool_str(*b))
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(a) = c.assumed_value {
                let _ = write!(s, "; {}", bool_str(a));
            }
            s
        }
        CPrimitiveObject::CString(c) => {
            if let Some(regex) = regex_of(&c.constraint) {
                let mut s = regex.to_owned();
                if let Some(a) = &c.assumed_value {
                    let _ = write!(s, "; {}", quoted(a));
                }
                return s;
            }
            let mut s = c
                .constraint
                .iter()
                .map(|v| quoted(v))
                .collect::<Vec<_>>()
                .join(", ");
            if let Some(a) = &c.assumed_value {
                let _ = write!(s, "; {}", quoted(a));
            }
            s
        }
        CPrimitiveObject::CInteger(c) => {
            let mut s = int_list(&c.constraint);
            if let Some(a) = c.assumed_value {
                // The integer assumed value is stored as a whole `f64`
                // (`C_INTEGER.assumed_value`); `Display` renders it without a
                // decimal point so it re-lexes as an integer.
                let _ = write!(s, "; {a}");
            }
            s
        }
        CPrimitiveObject::CReal(c) => {
            let mut s = real_list(&c.constraint);
            if let Some(a) = c.assumed_value {
                let _ = write!(s, "; {}", real_str(a));
            }
            s
        }
        CPrimitiveObject::CDate(c) => temporal(
            c.pattern_constraint.as_deref(),
            &c.constraint,
            c.assumed_value.as_ref().map(|v| v.value.as_str()),
        ),
        CPrimitiveObject::CTime(c) => temporal(
            c.pattern_constraint.as_deref(),
            &c.constraint,
            c.assumed_value.as_ref().map(|v| v.value.as_str()),
        ),
        CPrimitiveObject::CDateTime(c) => temporal(
            c.pattern_constraint.as_deref(),
            &c.constraint,
            c.assumed_value.as_ref().map(|v| v.value.as_str()),
        ),
        CPrimitiveObject::CDuration(c) => temporal(
            c.pattern_constraint.as_deref(),
            &c.constraint,
            c.assumed_value.as_ref().map(|v| v.value.as_str()),
        ),
        CPrimitiveObject::CTerminologyCode(c) => terminology_code_inline(c),
    }
}

fn terminology_code_inline(c: &CTerminologyCode) -> String {
    // A fully unconstrained terminology code renders as nothing — the caller
    // prints the bare (any-allowed) node instead of an unparseable `{[]}`.
    if c.constraint.is_empty() && c.assumed_value.is_none() && c.constraint_status.is_none() {
        return String::new();
    }
    let mut s = String::new();
    if let Some(status) = &c.constraint_status {
        s.push_str(strength_keyword(*status));
        s.push(' ');
    }
    let _ = write!(s, "[{}]", c.constraint);
    if let Some(assumed) = &c.assumed_value {
        // A `[ac…; at…]` assumed value re-uses the bracket form.
        let inner = format!("{}; {}", c.constraint, assumed.code_string);
        s = String::new();
        if let Some(status) = &c.constraint_status {
            s.push_str(strength_keyword(*status));
            s.push(' ');
        }
        let _ = write!(s, "[{inner}]");
    }
    s
}

fn strength_keyword(status: ConstraintStatus) -> &'static str {
    match status {
        ConstraintStatus::Extensible => "extensible",
        ConstraintStatus::Preferred => "preferred",
        ConstraintStatus::Example => "example",
        ConstraintStatus::Required | ConstraintStatus::Other(_) => "required",
    }
}

fn temporal(
    pattern: Option<&str>,
    constraint: &[Interval<impl TemporalValue>],
    assumed: Option<&str>,
) -> String {
    let mut s = String::new();
    match pattern {
        Some(p) => {
            s.push_str(p);
            // `pattern/interval` mixed form (`PWD/|P0W..P50W|`).
            if let Some(first) = constraint.first() {
                let _ = write!(s, "/{}", interval_str(first, temporal_str));
            }
        }
        None => {
            s = constraint
                .iter()
                .map(|iv| interval_str(iv, temporal_str))
                .collect::<Vec<_>>()
                .join(", ");
        }
    }
    if let Some(a) = assumed {
        let _ = write!(s, "; {a}");
    }
    s
}

fn int_list(constraint: &[Interval<i32>]) -> String {
    constraint
        .iter()
        .map(|iv| interval_str(iv, std::string::ToString::to_string))
        .collect::<Vec<_>>()
        .join(", ")
}

fn real_list(constraint: &[Interval<f64>]) -> String {
    constraint
        .iter()
        .map(|iv| interval_str(iv, |v| real_str(*v)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render one `Interval<T>` in cADL form: a point interval as the bare value, a
/// proper interval as `|lo..hi|` with relational-operator prefixes for
/// exclusivity/unboundedness (`master04.5`).
fn interval_str<T: Clone, F: Fn(&T) -> String>(iv: &Interval<T>, f: F) -> String {
    match iv {
        Interval::PointInterval(PointInterval { lower: Some(v), .. }) => f(v),
        Interval::ProperInterval(ProperInterval::ProperInterval(p)) => proper_str(p, &f),
        // An unbounded point interval, or the `MultiplicityInterval` proper
        // variant — the cADL primitive parser produces neither for a value
        // constraint (the latter is the occurrences/cardinality shape, printed
        // separately) — renders as nothing.
        Interval::PointInterval(_)
        | Interval::ProperInterval(ProperInterval::MultiplicityInterval(_)) => String::new(),
    }
}

fn proper_str<T, F: Fn(&T) -> String>(
    p: &openehr_base::prelude::ProperIntervalData<T>,
    f: &F,
) -> String {
    let two_sided = p.lower.is_some() && p.upper.is_some();
    if two_sided {
        let lo = p.lower.as_ref().map(f).unwrap_or_default();
        let hi = p.upper.as_ref().map(f).unwrap_or_default();
        let lp = if p.lower_included { "" } else { ">" };
        let hp = if p.upper_included { "" } else { "<" };
        format!("|{lp}{lo}..{hp}{hi}|")
    } else if let Some(lo) = &p.lower {
        let op = if p.lower_included { ">=" } else { ">" };
        format!("|{op}{}|", f(lo))
    } else if let Some(hi) = &p.upper {
        let op = if p.upper_included { "<=" } else { "<" };
        format!("|{op}{}|", f(hi))
    } else {
        String::new()
    }
}

fn bool_str(b: bool) -> &'static str {
    if b { "True" } else { "False" }
}

/// Format an `f64` so it always re-lexes as a `Real` (a decimal point present).
fn real_str(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        s
    } else {
        format!("{s}.0")
    }
}

/// A double-quoted, `master03`-escaped string literal.
/// Whether a canonical-JSON value renders as one ODIN scalar token.
fn is_json_scalar(v: &serde_json::Value) -> bool {
    matches!(
        v,
        serde_json::Value::String(_) | serde_json::Value::Number(_) | serde_json::Value::Bool(_)
    )
}

/// Render a canonical-JSON scalar as its ODIN literal (the inverse of the
/// cADL parser's `odin_to_json` scalar arms).
fn odin_scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => quoted(s),
        serde_json::Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        serde_json::Value::Number(n) => n.to_string(),
        // Null/containers never reach here (`is_json_scalar` gates the list
        // form; containers take the block form).
        other => other.to_string(),
    }
}

fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// `[terminology::code]` (with optional `(version)`), the ODIN `TERM_CODE_REF`
/// form (`master07-leaf_data`).
fn term_code_str(t: &TerminologyCode) -> String {
    let id = match &t.terminology_version {
        Some(v) => format!("{}({v})", t.terminology_id),
        None => t.terminology_id.clone(),
    };
    format!("[{id}::{}]", t.code_string)
}

// ── rules expression printing (full parenthesization) ──────────────────────

/// A single slot include/exclude assertion — the verbatim `string_expression`
/// is the round-trip carrier (`master04.6`); otherwise the expression tree.
fn assertion_str(a: &openehr_am::am24::beom::core::assertion::Assertion) -> String {
    if let Some(s) = &a.string_expression {
        return s.clone();
    }
    expression_str(&a.expression)
}

/// Render an [`Expression`] with full parenthesization so it re-parses to the
/// identical tree regardless of operator precedence (the BEL parser drops
/// redundant parentheses — `bel::parser::parse_primary` — so extra parens never
/// change the built tree).
fn expression_str(e: &Expression) -> String {
    match e {
        Expression::ExprLiteral(l) => literal_str(&l.item),
        Expression::ExprVariableRef(v) => format!("${}", v.item.name),
        Expression::ExprValueRef(r) => value_ref_str(r),
        Expression::ExprBinaryOperator(b) => {
            let sym = b.symbol.as_deref().unwrap_or(b.operator.as_str());
            let left = expression_str(&b.left_operand);
            if sym == "matches" {
                format!("({left} matches {})", constraint_rhs(&b.right_operand))
            } else {
                format!("({left} {sym} {})", expression_str(&b.right_operand))
            }
        }
        Expression::ExprUnaryOperator(u) => {
            let sym = u.symbol.as_deref().unwrap_or(u.operator.as_str());
            if sym == "exists" {
                // `exists` binds a bare reference leaf (no parentheses; the BEL
                // grammar's `parse_ref_leaf`).
                format!("exists {}", ref_leaf_str(&u.operand))
            } else {
                format!("{sym} ({})", expression_str(&u.operand))
            }
        }
        Expression::ExprFunctionCall(f) => {
            let name = f.item.as_ref().and_then(|v| v.as_str()).unwrap_or_default();
            let args = f
                .arguments
                .iter()
                .map(expression_str)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}({args})")
        }
        Expression::ExprForAll(fa) => {
            let collection = value_ref_str(&fa.operand);
            let cond = expression_str(&fa.condition.expression);
            format!("for_all $x : {collection} | {cond}")
        }
        Expression::ExprConstraint(c) => constraint_leaf_str(c),
    }
}

/// The RHS of a `matches` operator: the cADL primitive/regex, wrapped in braces.
fn constraint_rhs(e: &Expression) -> String {
    match e {
        Expression::ExprConstraint(c) => constraint_leaf_str(c),
        other => expression_str(other),
    }
}

fn constraint_leaf_str(c: &ExprConstraint) -> String {
    match c {
        ExprConstraint::ExprConstraint(d) => format!("{{{}}}", primitive_inline(&d.item)),
        ExprConstraint::ExprArchetypeIdConstraint(a) => {
            // A C_STRING regex matcher (`master04.3` §Archetype Slots).
            format!(
                "{{{}}}",
                a.item.constraint.first().cloned().unwrap_or_default()
            )
        }
    }
}

fn value_ref_str(r: &ExprValueRef) -> String {
    match r {
        ExprValueRef::ExprArchetypeRef(a) => a.path.clone(),
        ExprValueRef::ExprValueRef(v) => v
            .item
            .as_ref()
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned(),
    }
}

fn ref_leaf_str(e: &Expression) -> String {
    match e {
        Expression::ExprValueRef(r) => value_ref_str(r),
        Expression::ExprVariableRef(v) => format!("${}", v.item.name),
        other => expression_str(other),
    }
}

fn expr_value_str(v: &ExprValue) -> String {
    match v {
        ExprValue::ExprBinaryOperator(b) => {
            expression_str(&Expression::ExprBinaryOperator(Box::new(b.clone())))
        }
        ExprValue::ExprUnaryOperator(u) => {
            expression_str(&Expression::ExprUnaryOperator(Box::new(u.clone())))
        }
        ExprValue::ExprForAll(f) => expression_str(&Expression::ExprForAll(Box::new(f.clone()))),
        ExprValue::ExprFunctionCall(f) => expression_str(&Expression::ExprFunctionCall(f.clone())),
        ExprValue::ExprLiteral(l) => literal_str(&l.item),
        ExprValue::ExprValueRef(r) => value_ref_str(r),
        ExprValue::ExprVariableRef(v) => format!("${}", v.item.name),
        ExprValue::ExprConstraint(c) => constraint_leaf_str(c),
        ExprValue::ExternalQuery(_) => String::new(),
    }
}

fn literal_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Bool(b) => bool_str(*b).to_owned(),
        serde_json::Value::String(s) => quoted(s),
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn type_def_name(t: &openehr_lang::prelude::ExprTypeDef) -> String {
    match t {
        openehr_lang::prelude::ExprTypeDef::TypeDefObjectRef(r) => r.type_name.clone(),
        _ => "Any".to_owned(),
    }
}

// ── the temporal value trait ──────────────────────────────────────────────

/// A temporal primitive value whose verbatim ISO-8601 text the printer emits.
trait TemporalValue: Clone {
    fn text(&self) -> &str;
}

fn temporal_str<T: TemporalValue>(v: &T) -> String {
    v.text().to_owned()
}

impl TemporalValue for openehr_base::prelude::Iso8601Date {
    fn text(&self) -> &str {
        &self.value
    }
}
impl TemporalValue for openehr_base::prelude::Iso8601Time {
    fn text(&self) -> &str {
        &self.value
    }
}
impl TemporalValue for openehr_base::prelude::Iso8601DateTime {
    fn text(&self) -> &str {
        &self.value
    }
}
impl TemporalValue for openehr_base::prelude::Iso8601Duration {
    fn text(&self) -> &str {
        &self.value
    }
}

//! The ADL 1.4 → ADL 2 converter core.
//!
//! NOTE: no openEHR spec governs 1.4→2 conversion — the whole `adl14` module is
//! our own design (see the [`crate::adl14`] flag). Every heuristic below is
//! pinned by the paired `upgrade_from_14` corpus fixtures, never by a spec
//! clause; the comments cite the fixture behaviour, not an openEHR section.

use std::collections::BTreeMap;

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::archetype::authored_archetype::{
    AuthoredArchetype, AuthoredArchetypeData,
};
use openehr_am::am24::aom2::constraint_model::c_attribute::CAttribute;
use openehr_am::am24::aom2::constraint_model::c_complex_object::{
    CComplexObject, CComplexObjectData,
};
use openehr_am::am24::aom2::constraint_model::c_object::CObject;
use openehr_am::am24::aom2::terminology::archetype_term::ArchetypeTerm;
use openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::am24::aom2::terminology::value_set::ValueSet;

use crate::adl14::log::ConversionLog;
use crate::error::SyntaxError;

/// Configuration for a 1.4→2 conversion (all spec-silent — our own design).
#[derive(Debug, Clone)]
pub struct ConvertConfig {
    /// The `adl_version` to stamp on the converted artefact.
    pub adl_version: String,
    /// The `rm_release` to stamp (1.4 sources carry no `rm_release`).
    pub rm_release: String,
    /// Per-terminology URI templates for synthesised term bindings; `{code}` is
    /// replaced by the external code. A terminology with no template gets a
    /// flagged fabricated fallback (`urn:adl14:<terminology>:<code>`).
    pub binding_uri_templates: BTreeMap<String, String>,
}

impl Default for ConvertConfig {
    fn default() -> Self {
        let mut binding_uri_templates = BTreeMap::new();
        // The `upgrade_from_14` fixtures bind `openehr` codes to
        // `http://openehr.org/id/{code}`.
        binding_uri_templates.insert(
            "openehr".to_owned(),
            "http://openehr.org/id/{code}".to_owned(),
        );
        Self {
            adl_version: "2.0.6".to_owned(),
            rm_release: "1.0.3".to_owned(),
            binding_uri_templates,
        }
    }
}

/// A 1.4→2 conversion failure.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The 1.4 source did not parse.
    #[error("1.4 source did not parse: {0:?}")]
    Parse(Vec<SyntaxError>),
    /// The artefact kind is not a plain authored archetype (templates/OPTs are
    /// not yet converted here).
    #[error("1.4→2 conversion supports authored archetypes only: {0}")]
    Unsupported(String),
}

/// Parse a 1.4 `.adl` source and convert it to an ADL2 [`Archetype`].
///
/// # Errors
/// [`ConvertError::Parse`] if the source does not parse; [`ConvertError::Unsupported`]
/// for a non-archetype artefact kind.
pub fn parse_and_convert(
    src: &str,
    cfg: &ConvertConfig,
    log: &mut ConversionLog,
) -> Result<Archetype, ConvertError> {
    let art = crate::assemble::parse_artefact_adl14(src).map_err(ConvertError::Parse)?;
    convert(&art, cfg, log)
}

/// Convert a 1.4-shaped [`Archetype`] (from [`crate::assemble::parse_artefact_adl14`])
/// into a spec-valid ADL2 archetype. For a specialised source, this performs the
/// base conversion (renumbering against the child's own codes); re-differentialisation
/// against the converted+flattened parent is [`crate::adl14::differ::differentiate`].
///
/// # Errors
/// [`ConvertError::Unsupported`] for a non-archetype artefact kind.
pub fn convert(
    art: &Archetype,
    cfg: &ConvertConfig,
    log: &mut ConversionLog,
) -> Result<Archetype, ConvertError> {
    let data = match art {
        Archetype::AuthoredArchetype(b) => match b.as_ref() {
            AuthoredArchetype::AuthoredArchetype(d) => d.clone(),
            AuthoredArchetype::Template(_) => {
                return Err(ConvertError::Unsupported("template".to_owned()));
            }
            AuthoredArchetype::OperationalTemplate(_) => {
                return Err(ConvertError::Unsupported("operational_template".to_owned()));
            }
        },
        Archetype::TemplateOverlay(_) => {
            return Err(ConvertError::Unsupported("template_overlay".to_owned()));
        }
    };

    let mut cx = Converter::new(cfg, log);
    let converted = cx.run(data);
    Ok(Archetype::AuthoredArchetype(Box::new(
        AuthoredArchetype::AuthoredArchetype(converted),
    )))
}

/// A synthesised terminology entry the converter minted.
struct Synth {
    code: String,
    text: String,
    description: String,
    binding: Option<(String, String)>, // (terminology, uri)
    value_set_members: Option<Vec<String>>,
}

struct Converter<'a> {
    cfg: &'a ConvertConfig,
    log: &'a mut ConversionLog,
    /// old node at-code → new node id-code.
    node_map: BTreeMap<String, String>,
    /// Source at-codes referenced as a `local` *value* (they get an at-code
    /// term entry even when the same code is also a node id — a 1.4 at-code
    /// used as both an id and a value splits into an id-code + an at-code).
    value_at_codes: std::collections::BTreeSet<String>,
    /// Next free id-number for synthesised object nodes (top-level id space).
    next_id: i64,
    /// Next free at-number for synthesised external at-codes.
    next_at: i64,
    /// Next free ac-number for synthesised value sets.
    next_ac: i64,
    /// Synthesised terminology entries, in mint order.
    synth: Vec<Synth>,
    /// The converted root node id (the terminology `concept_code`).
    root_id: String,
}

impl<'a> Converter<'a> {
    fn new(cfg: &'a ConvertConfig, log: &'a mut ConversionLog) -> Self {
        Self {
            cfg,
            log,
            node_map: BTreeMap::new(),
            value_at_codes: std::collections::BTreeSet::new(),
            next_id: 1,
            next_at: 1,
            next_ac: 1,
            synth: Vec::new(),
            root_id: "id1".to_owned(),
        }
    }

    fn run(&mut self, mut data: AuthoredArchetypeData) -> AuthoredArchetypeData {
        // 1. Build the node-id map + synthesis counters from the definition.
        self.plan_codes(&data);
        // 2. Renumber node ids in document order (existing → mapped, empty →
        //    synthesised).
        self.renumber_nodes(&mut data.definition);
        // 3. Convert every terminology-code constraint (local/external/list).
        self.convert_constraints(&mut data.definition);
        // 4. Elide RM-default cardinality/occurrences.
        elide_multiplicity(&mut data.definition);
        // 5. Rebuild the terminology (renumber keys, drop @internal, add synth).
        data.terminology = Box::new(self.rebuild_terminology(&data.terminology));
        // 6. Header/meta + description + version.
        self.stamp_meta(&mut data);
        data.is_differential = data.parent_archetype_id.is_some();
        data
    }

    // ── code planning ──────────────────────────────────────────────────────

    fn plan_codes(&mut self, data: &AuthoredArchetypeData) {
        // Existing node at-codes → shifted id-codes; track max id-number.
        let mut max_id = 1i64;
        let mut max_at = 0i64;
        collect_node_codes(&data.definition, &mut |code| {
            if code.is_empty() {
                return;
            }
            let new = shift_code(code, "id");
            if let Some(n) = first_num(&new) {
                max_id = max_id.max(n);
            }
            self.node_map.insert(code.to_owned(), new);
        });
        // Value at-codes referenced in constraints (local single/list) →
        // shifted at-codes; track max at-number.
        let mut value_at_codes = std::collections::BTreeSet::new();
        collect_local_value_codes(&data.definition, &mut |code| {
            let new = shift_code(code, "at");
            if let Some(n) = first_num(&new) {
                max_at = max_at.max(n);
            }
            value_at_codes.insert(code.to_owned());
        });
        self.value_at_codes = value_at_codes;
        self.next_id = max_id + 1;
        self.next_at = max_at + 1;
    }

    fn alloc_id(&mut self) -> String {
        let c = format!("id{}", self.next_id);
        self.next_id += 1;
        c
    }

    fn alloc_at(&mut self) -> String {
        let c = format!("at{}", self.next_at);
        self.next_at += 1;
        c
    }

    fn alloc_ac(&mut self) -> String {
        let c = format!("ac{}", self.next_ac);
        self.next_ac += 1;
        c
    }

    // ── node-id renumbering ──────────────────────────────────────────────────

    fn renumber_nodes(&mut self, def: &mut CComplexObject) {
        renumber_cco(def, self);
        if let CComplexObject::CComplexObject(d) = def {
            self.root_id = d.node_id.clone();
        }
    }

    fn new_node_id(&mut self, old: &str) -> String {
        if old.is_empty() {
            return self.alloc_id();
        }
        self.node_map
            .get(old)
            .cloned()
            .unwrap_or_else(|| shift_code(old, "id"))
    }

    // ── terminology-constraint conversion ────────────────────────────────────

    fn convert_constraints(&mut self, def: &mut CComplexObject) {
        convert_constraints_cco(def, self, "");
    }

    /// Convert one 1.4 `C_TERMINOLOGY_CODE.constraint` encoding
    /// (`terminology::code[,code…][;assumed]`, or an already-ADL2 `at/ac` code)
    /// into an ADL2 constraint, minting synthesised codes as needed.
    /// `owner_text` is the enclosing element's rubric (for the `(synthesised)`
    /// value-set label). Returns `(constraint, assumed)`.
    fn convert_constraint(&mut self, raw: &str, owner_text: &str) -> (String, Option<String>) {
        // Split off an assumed value (`…;code`).
        let (body, assumed_raw) = match raw.split_once(';') {
            Some((b, a)) => (b, Some(a)),
            None => (raw, None),
        };
        // Not a qualified 1.4 code → already ADL2 (`at5`, `ac1`); pass through.
        let Some((terminology, codes_str)) = body.split_once("::") else {
            return (body.to_owned(), assumed_raw.map(str::to_owned));
        };
        let codes: Vec<&str> = codes_str
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        let is_local = terminology == "local";

        // Map each source code to its ADL2 at-code (shift a local at-code; mint
        // for an external code).
        let mut at_codes: Vec<String> = Vec::new();
        for code in &codes {
            let at = if is_local {
                shift_code(code, "at")
            } else {
                self.external_at_code(terminology, code)
            };
            at_codes.push(at);
        }

        let assumed = assumed_raw.map(|a| {
            if is_local {
                shift_code(a, "at")
            } else {
                self.external_at_code(terminology, a)
            }
        });

        if at_codes.len() == 1 {
            (at_codes.into_iter().next().unwrap_or_default(), assumed)
        } else {
            // A list → a synthesised ac-code value set.
            let signature = at_codes.join(",");
            let ac = if let Some(existing) = self.log.value_set(&signature) {
                existing.to_owned()
            } else {
                let ac = self.alloc_ac();
                self.log.record_value_set(&signature, &ac);
                let (text, description) = synth_value_set_rubric(owner_text);
                self.synth.push(Synth {
                    code: ac.clone(),
                    text,
                    description,
                    binding: None,
                    value_set_members: Some(at_codes.clone()),
                });
                ac
            };
            (ac, assumed)
        }
    }

    /// The synthesised at-code for an external `terminology::code`, minting +
    /// recording a binding on first sight (idempotent via the log).
    fn external_at_code(&mut self, terminology: &str, code: &str) -> String {
        let key = format!("{terminology}::{code}");
        if let Some(existing) = self.log.external_at_code(&key) {
            return existing.to_owned();
        }
        let at = self.alloc_at();
        self.log.record_external_at_code(&key, &at);
        let uri = match self.cfg.binding_uri_templates.get(terminology) {
            Some(tmpl) => tmpl.replace("{code}", code),
            // Fabricated fallback URI (flagged) where no template is configured.
            None => format!("urn:adl14:{terminology}:{code}"),
        };
        self.synth.push(Synth {
            code: at.clone(),
            // The external code as a placeholder rubric — the human name would
            // need the external terminology resolved (not available here).
            // TODO: resolve `openehr-term` property/state names for the rubric.
            text: code.to_owned(),
            description: code.to_owned(),
            binding: Some((terminology.to_owned(), uri)),
            value_set_members: None,
        });
        at
    }

    // ── terminology rebuild ──────────────────────────────────────────────────

    fn rebuild_terminology(&mut self, old: &ArchetypeTerminology) -> ArchetypeTerminology {
        // `@ internal @` node terms are dropped in every language (the marker is
        // authored in the original language; translations carry the openEHR
        // untranslated form `*@ internal @(<lang>)`). Determine the internal
        // node set once, from the original language.
        let internal_nodes: std::collections::BTreeSet<String> = old
            .term_definitions
            .get(&old.original_language)
            .into_iter()
            .flatten()
            .filter(|(code, term)| {
                self.node_map.contains_key(*code) && term.description.trim() == "@ internal @"
            })
            .map(|(code, _)| code.clone())
            .collect();

        let mut term_definitions: BTreeMap<String, BTreeMap<String, ArchetypeTerm>> =
            BTreeMap::new();
        for (lang, terms) in &old.term_definitions {
            let mut out: BTreeMap<String, ArchetypeTerm> = BTreeMap::new();
            for (code, term) in terms {
                let is_node = self.node_map.contains_key(code);
                // A 1.4 at-code used as a value (`[local::atX]`) always yields an
                // at-code term; used as a node id it yields an id-code term. A
                // code that is both splits into both entries.
                if self.value_at_codes.contains(code) || !is_node {
                    let at = shift_code(code, "at");
                    out.insert(at.clone(), term_with_code(term, at));
                }
                if is_node {
                    // Drop an `@ internal @` node term in every language
                    // (reference-converter behaviour; validates clean).
                    if internal_nodes.contains(code) {
                        continue;
                    }
                    let id = shift_code(code, "id");
                    out.insert(id.clone(), term_with_code(term, id));
                }
            }
            // Add synthesised terms to every language (the fixtures translate
            // them per language, appending the English `(synthesised)` suffix;
            // we mint one rubric for all languages).
            // TODO: per-language synthesised rubrics.
            for s in &self.synth {
                out.insert(
                    s.code.clone(),
                    ArchetypeTerm {
                        code: s.code.clone(),
                        text: s.text.clone(),
                        description: s.description.clone(),
                        other_items: None,
                    },
                );
            }
            term_definitions.insert(lang.clone(), out);
        }

        // Bindings: renumber existing binding keys, then add synthesised ones.
        let mut term_bindings: BTreeMap<String, BTreeMap<String, String>> = old
            .term_bindings
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|(t, m)| {
                let m2 = m
                    .into_iter()
                    .map(|(k, v)| (self.rename_binding_key(&k), v))
                    .collect();
                (t, m2)
            })
            .collect();
        for s in &self.synth {
            if let Some((terminology, uri)) = &s.binding {
                term_bindings
                    .entry(terminology.clone())
                    .or_default()
                    .insert(s.code.clone(), uri.clone());
            }
        }

        let mut value_sets: BTreeMap<String, ValueSet> = old.value_sets.clone().unwrap_or_default();
        for s in &self.synth {
            if let Some(members) = &s.value_set_members {
                value_sets.insert(
                    s.code.clone(),
                    ValueSet {
                        id: s.code.clone(),
                        members: members.clone(),
                    },
                );
            }
        }

        ArchetypeTerminology {
            is_differential: old.is_differential,
            original_language: old.original_language.clone(),
            concept_code: self.root_id.clone(),
            term_definitions,
            term_bindings: if term_bindings.is_empty() {
                None
            } else {
                Some(term_bindings)
            },
            value_sets: if value_sets.is_empty() {
                None
            } else {
                Some(value_sets)
            },
            terminology_extracts: old.terminology_extracts.clone(),
        }
    }

    fn rename_binding_key(&self, key: &str) -> String {
        // Binding keys are codes or code-terminated paths; rename a leading code.
        if let Some(id) = self.node_map.get(key) {
            id.clone()
        } else if crate::codes::is_at_code(key) {
            shift_code(key, "at")
        } else {
            key.to_owned()
        }
    }

    // ── meta / description / version ─────────────────────────────────────────

    fn stamp_meta(&mut self, data: &mut AuthoredArchetypeData) {
        data.adl_version = Some(self.cfg.adl_version.clone());
        self.cfg.rm_release.clone_into(&mut data.rm_release);
        data.is_generated = true;

        // Version from `other_details["revision"]` if present, else 1.0.0.
        let revision = data
            .description
            .as_ref()
            .and_then(|d| d.other_details.as_ref())
            .and_then(|o| o.get("revision"))
            .cloned();
        let version = revision.clone().unwrap_or_else(|| "1.0.0".to_owned());
        set_release_version(&mut data.archetype_id, &version);

        if let Some(desc) = data.description.as_mut() {
            transform_description(desc);
        }
    }
}

// ── description transform ────────────────────────────────────────────────────

fn transform_description(
    desc: &mut openehr_am::am24::resource::resource_description::ResourceDescription,
) {
    // All observed 1.4 lifecycle states convert to `unmanaged` (fixtures).
    // TODO: a finer 1.4→2 lifecycle-state map if a fixture needs one.
    "unmanaged".clone_into(&mut desc.lifecycle_state);

    // Hoist `details[lang].copyright` (if any) up to `description.copyright`.
    if desc.copyright.is_none()
        && let Some(details) = desc.details.as_ref()
        && let Some(cr) = details.values().find_map(|item| {
            item.other_details
                .as_ref()
                .and_then(|o| o.get("copyright"))
                .cloned()
        })
    {
        desc.copyright = Some(cr);
    }

    // Drop the consumed `revision` from other_details.
    if let Some(o) = desc.other_details.as_mut() {
        o.remove("revision");
    }
}

// ── free-function tree walks ─────────────────────────────────────────────────

/// The mutable `C_COMPLEX_OBJECT` data, if this is a plain complex object.
///
/// A 1.4 source definition never contains an inline `C_ARCHETYPE_ROOT` (only a
/// flattened OPT does), so that arm yields `None` and its walk is a no-op.
/// TODO: handle `C_ARCHETYPE_ROOT` if OPT-1.4 conversion feeds one.
fn cco_data_mut(cco: &mut CComplexObject) -> Option<&mut CComplexObjectData> {
    match cco {
        CComplexObject::CComplexObject(d) => Some(d),
        CComplexObject::CArchetypeRoot(_) => None,
    }
}

fn collect_node_codes(def: &CComplexObject, f: &mut impl FnMut(&str)) {
    if let CComplexObject::CComplexObject(d) = def {
        f(&d.node_id);
        for attr in &d.attributes {
            for child in &attr.children {
                collect_node_codes_obj(child, f);
            }
        }
        for tuple in &d.attribute_tuples {
            for member in &tuple.members {
                for child in &member.children {
                    collect_node_codes_obj(child, f);
                }
            }
        }
    }
}

fn collect_node_codes_obj(obj: &CObject, f: &mut impl FnMut(&str)) {
    match obj {
        CObject::CComplexObject(cco) => collect_node_codes(cco, f),
        CObject::CComplexObjectProxy(p) => f(&p.node_id),
        CObject::ArchetypeSlot(s) => f(&s.node_id),
        _ => {}
    }
}

fn collect_local_value_codes(def: &CComplexObject, f: &mut impl FnMut(&str)) {
    walk_constraints(def, &mut |raw, _| {
        if let Some((term, codes)) = raw.split_once("::")
            && term == "local"
        {
            for code in codes.split([',', ';']).map(str::trim) {
                if crate::codes::is_at_code(code) {
                    f(code);
                }
            }
        }
    });
}

/// Visit every `C_TERMINOLOGY_CODE.constraint` (with its enclosing element
/// rubric context — unused here, passed empty).
fn walk_constraints(def: &CComplexObject, f: &mut impl FnMut(&str, &str)) {
    if let CComplexObject::CComplexObject(d) = def {
        for attr in &d.attributes {
            for child in &attr.children {
                walk_constraints_obj(child, f);
            }
        }
        for tuple in &d.attribute_tuples {
            for member in &tuple.members {
                for child in &member.children {
                    walk_constraints_obj(child, f);
                }
            }
        }
    }
}

fn walk_constraints_obj(obj: &CObject, f: &mut impl FnMut(&str, &str)) {
    match obj {
        CObject::CTerminologyCode(tc) => f(&tc.constraint, ""),
        CObject::CComplexObject(cco) => walk_constraints(cco, f),
        _ => {}
    }
}

fn renumber_cco(cco: &mut CComplexObject, cx: &mut Converter<'_>) {
    let Some(d) = cco_data_mut(cco) else { return };
    d.node_id = cx.new_node_id(&d.node_id);
    for attr in &mut d.attributes {
        for child in &mut attr.children {
            renumber_obj(child, cx);
        }
    }
    for tuple in &mut d.attribute_tuples {
        for member in &mut tuple.members {
            for child in &mut member.children {
                renumber_obj(child, cx);
            }
        }
    }
}

fn renumber_obj(obj: &mut CObject, cx: &mut Converter<'_>) {
    match obj {
        CObject::CComplexObject(cco) => renumber_cco(cco, cx),
        CObject::CComplexObjectProxy(p) => {
            p.node_id = cx.new_node_id(&p.node_id);
            p.target_path = rewrite_path(&p.target_path, cx);
        }
        CObject::ArchetypeSlot(s) => s.node_id = cx.new_node_id(&s.node_id),
        _ => {}
    }
}

fn convert_constraints_cco(cco: &mut CComplexObject, cx: &mut Converter<'_>, owner_text: &str) {
    let Some(d) = cco_data_mut(cco) else { return };
    let node_text = owner_text.to_owned();
    for attr in &mut d.attributes {
        for child in &mut attr.children {
            convert_constraints_obj(child, cx, &node_text);
        }
    }
    for tuple in &mut d.attribute_tuples {
        for member in &mut tuple.members {
            for child in &mut member.children {
                convert_constraints_obj(child, cx, &node_text);
            }
        }
    }
}

fn convert_constraints_obj(obj: &mut CObject, cx: &mut Converter<'_>, owner_text: &str) {
    match obj {
        CObject::CTerminologyCode(tc) => {
            let (constraint, assumed) = cx.convert_constraint(&tc.constraint, owner_text);
            tc.constraint = constraint;
            if let Some(a) = assumed {
                tc.assumed_value = Some(openehr_base::prelude::TerminologyCode {
                    terminology_id: "local".to_owned(),
                    terminology_version: None,
                    code_string: a,
                    uri: None,
                });
            }
        }
        CObject::CComplexObject(cco) => convert_constraints_cco(cco, cx, owner_text),
        _ => {}
    }
}

fn rewrite_path(path: &str, cx: &Converter<'_>) -> String {
    // Rewrite `[atNNNN]` predicates in an ADL path to their converted codes.
    let mut out = String::with_capacity(path.len());
    let mut chars = path.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '[' {
            // Read to the matching ']'.
            let start = i + 1;
            let rest = &path[start..];
            if let Some(end) = rest.find(']') {
                let code = &rest[..end];
                let mapped = if let Some(id) = cx.node_map.get(code) {
                    id.clone()
                } else if crate::codes::is_at_code(code) {
                    shift_code(code, "id")
                } else {
                    code.to_owned()
                };
                out.push('[');
                out.push_str(&mapped);
                out.push(']');
                for _ in 0..=end {
                    chars.next();
                }
                continue;
            }
        }
        out.push(c);
    }
    out
}

fn elide_multiplicity(def: &mut CComplexObject) {
    elide_cco(def);
}

fn elide_cco(cco: &mut CComplexObject) {
    let Some(d) = cco_data_mut(cco) else { return };
    for attr in &mut d.attributes {
        elide_attr(attr);
        for child in &mut attr.children {
            if let CObject::CComplexObject(c) = child {
                elide_cco(c);
            }
        }
    }
}

fn elide_attr(attr: &mut CAttribute) {
    // Drop a container cardinality equal to the RM default `{0..*}` (the
    // fixtures elide `cardinality matches {0..*; unordered}`); keep any narrower
    // bound. Drop `occurrences matches {0..*}` on children likewise.
    if let Some(card) = &attr.cardinality
        && is_zero_unbounded(&card.interval)
    {
        attr.cardinality = None;
        attr.is_multiple = false;
    }
    for child in &mut attr.children {
        if let Some(occ) = child_occurrences(child)
            && is_zero_unbounded(occ)
        {
            clear_occurrences(child);
        }
    }
}

fn is_zero_unbounded(mi: &openehr_base::prelude::MultiplicityInterval) -> bool {
    mi.lower == Some(0) && mi.upper_unbounded
}

fn child_occurrences(obj: &CObject) -> Option<&openehr_base::prelude::MultiplicityInterval> {
    match obj {
        CObject::CComplexObject(CComplexObject::CComplexObject(d)) => d.occurrences.as_ref(),
        CObject::CComplexObject(CComplexObject::CArchetypeRoot(r)) => r.occurrences.as_ref(),
        CObject::CComplexObjectProxy(p) => p.occurrences.as_ref(),
        CObject::ArchetypeSlot(s) => s.occurrences.as_ref(),
        _ => None,
    }
}

fn clear_occurrences(obj: &mut CObject) {
    match obj {
        CObject::CComplexObject(CComplexObject::CComplexObject(d)) => d.occurrences = None,
        CObject::CComplexObject(CComplexObject::CArchetypeRoot(r)) => r.occurrences = None,
        CObject::CComplexObjectProxy(p) => p.occurrences = None,
        CObject::ArchetypeSlot(s) => s.occurrences = None,
        _ => {}
    }
}

// ── code shifting ────────────────────────────────────────────────────────────

/// Shift a 1.4 code to an ADL2 code with the given `prefix` (`"id"`/`"at"`).
/// The first segment's number is incremented by one (`at0000`→`id1`,
/// `at0003.1`→`id4.1`), except the specialisation new-code prefix `at0`
/// (`at0.89`→`id0.89` — number kept). No openEHR spec governs this — fixture-pinned.
fn shift_code(code: &str, prefix: &str) -> String {
    let bare = code
        .trim_start_matches("at")
        .trim_start_matches("id")
        .trim_start_matches("ac");
    let mut segs = bare.split('.');
    let Some(first) = segs.next() else {
        return code.to_owned();
    };
    let rest: Vec<&str> = segs.collect();
    let has_suffix = !rest.is_empty();
    let shifted_first = if first == "0" && has_suffix {
        // `at0.K` new-at-level code: keep the 0.
        "0".to_owned()
    } else {
        match first.parse::<i64>() {
            Ok(n) => (n + 1).to_string(),
            Err(_) => first.to_owned(),
        }
    };
    if has_suffix {
        format!("{prefix}{shifted_first}.{}", rest.join("."))
    } else {
        format!("{prefix}{shifted_first}")
    }
}

fn first_num(code: &str) -> Option<i64> {
    let bare = code
        .trim_start_matches("at")
        .trim_start_matches("id")
        .trim_start_matches("ac");
    bare.split('.').next()?.parse::<i64>().ok()
}

fn term_with_code(term: &ArchetypeTerm, code: String) -> ArchetypeTerm {
    ArchetypeTerm {
        code,
        text: term.text.clone(),
        description: term.description.clone(),
        other_items: term.other_items.clone(),
    }
}

fn synth_value_set_rubric(owner_text: &str) -> (String, String) {
    // The fixtures label a synthesised value set with the owning element's
    // rubric + " (synthesised)".
    let base = if owner_text.is_empty() {
        "Value set".to_owned()
    } else {
        owner_text.to_owned()
    };
    (
        format!("{base} (synthesised)"),
        format!("{base} (synthesised)"),
    )
}

fn set_release_version(
    hrid: &mut openehr_am::am24::aom2::archetype::archetype_hrid::ArchetypeHrid,
    version: &str,
) {
    // `version` may be `1.1.0` or `0.0.1-alpha`; split a `-status.build` tail.
    let (numeric, status, build) = split_version(version);
    hrid.release_version = numeric;
    hrid.version_status = openehr_base::prelude::VersionStatus::from_wire(status);
    hrid.build_count = build;
}

fn split_version(v: &str) -> (String, &'static str, String) {
    for (marker, status) in [("-rc", "rc"), ("-alpha", "alpha"), ("-beta", "beta")] {
        if let Some(idx) = v.find(marker) {
            let numeric = normalise_numeric(&v[..idx]);
            let build = v[idx + marker.len()..]
                .strip_prefix('.')
                .unwrap_or("")
                .to_owned();
            return (numeric, status, build);
        }
    }
    (normalise_numeric(v), "", String::new())
}

fn normalise_numeric(v: &str) -> String {
    let mut parts = v.split('.');
    let major = parts.next().unwrap_or("1");
    let minor = parts.next().unwrap_or("0");
    let patch = parts.next().unwrap_or("0");
    format!(
        "{}.{}.{}",
        if major.is_empty() { "1" } else { major },
        if minor.is_empty() { "0" } else { minor },
        if patch.is_empty() { "0" } else { patch }
    )
}

//! The ADL 1.4 → ADL 2 converter core.
//!
//! NOTE: no openEHR spec governs 1.4→2 conversion — the whole `adl14` module is
//! our own design (see the [`crate::adl14`] flag). The conversion HEURISTICS
//! are pinned by the paired `upgrade_from_14` corpus fixtures, never by a
//! spec clause; where a comment cites an AOM2/ADL2 section, it pins the
//! validity of the ADL2 OUTPUT form (which is spec-governed), not the
//! conversion procedure.
//!
//! The one exception is the standardised `description/other_details` meta-data
//! mapping (`build_uid` + the `RESOURCE_DESCRIPTION` governance items), which
//! `ADL1.4/masterAppB-extended_metadata.adoc` §Standardised Items governs
//! directly — those items are "intended to be implemented by any ADL 1.4 =>
//! ADL 2 conversion tool". It is applied by the description transform below.

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
use openehr_am::am24::aom2::constraint_model::c_primitive_object::CPrimitiveObject;
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
    /// Collapse specialised (dotted) codes to top level, producing a depth-0
    /// archetype. For a `-`-specialised root inlined standalone by a
    /// flattened OPT the differential lineage is unresolvable — the spec
    /// defines differential-form semantics only relative to a resolvable
    /// parent (`ADL2/master09.02` §Specialisation concepts), a specialised
    /// archetype must declare that parent (`AOM2/master03` §Validity Rules
    /// VASID/VACSD), and deriving the parent id from the 1.4 `-` naming is
    /// endorsed nowhere — so the standalone source is emitted UNSPECIALISED:
    /// every dotted node/value/constraint code is renumbered into the flat
    /// code space (terminology keys follow), satisfying VARCN/VATCD at depth
    /// 0. Off for plain ADL-source conversion, where the `specialise` clause
    /// carries the lineage. No openEHR spec governs 1.4→2 conversion — our
    /// own design/extension.
    pub collapse_specialised_codes: bool,
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
            collapse_specialised_codes: false,
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
    /// Collapsed-value remapping (dotted `local` value at-codes → fresh flat
    /// at-codes) — populated only under `collapse_specialised_codes`.
    value_map: BTreeMap<String, String>,
    /// Collapsed-constraint remapping (dotted ac-codes → fresh flat
    /// ac-codes) — populated only under `collapse_specialised_codes`.
    ac_map: BTreeMap<String, String>,
    /// New node ids already claimed by an occurrence — the VCOSU ground:
    /// ADL2 node ids are unique archetype-wide (`AOM2/master04.5` §Validity
    /// Rules: `C_OBJECT`), while 1.4 at-codes are only sibling-unique, so a
    /// reused code's second occurrence mints a fresh id.
    assigned_node_ids: std::collections::BTreeSet<String>,
    /// `(first-occurrence new id, freshly minted id)` per VCOSU re-mint — the
    /// terminology rebuild clones the definition/bindings of the first id
    /// under each minted one (VATDF: every used code must be defined).
    dup_mints: Vec<(String, String)>,
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
            value_map: BTreeMap::new(),
            ac_map: BTreeMap::new(),
            assigned_node_ids: std::collections::BTreeSet::new(),
            dup_mints: Vec::new(),
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
        // 4. Write out the ADL 1.4 default occurrences where ADL 2 would infer a
        //    different one, then elide the RM-default cardinality/occurrences.
        //    Order matters: the materialisation reads the 1.4 cardinality, which
        //    the elision may drop.
        materialise_adl14_occurrences(&mut data.definition);
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
        let collapse = self.cfg.collapse_specialised_codes;
        let root_code = match &data.definition {
            CComplexObject::CComplexObject(d) => d.node_id.clone(),
            CComplexObject::CArchetypeRoot(_) => String::new(),
        };
        // Existing node at-codes → shifted id-codes; track max id-number.
        // Under collapse, dotted (specialised) codes are deferred: they get
        // fresh flat ids above the max, in document order (the root always
        // takes `id1` — VARCN's depth-0 root form).
        let mut max_id = 1i64;
        let mut max_at = 0i64;
        let mut deferred_nodes: Vec<String> = Vec::new();
        collect_node_codes(&data.definition, &mut |code| {
            if code.is_empty() {
                return;
            }
            if collapse && code.contains('.') {
                if code == root_code {
                    self.node_map.insert(code.to_owned(), "id1".to_owned());
                } else if !self.node_map.contains_key(code)
                    && !deferred_nodes.iter().any(|c| c == code)
                {
                    deferred_nodes.push(code.to_owned());
                }
                return;
            }
            let new = shift_code(code, "id");
            if let Some(n) = first_num(&new) {
                max_id = max_id.max(n);
            }
            self.node_map.insert(code.to_owned(), new);
        });
        // Value at-codes referenced in constraints (local single/list) →
        // shifted at-codes; track max at-number. Dotted values collapse to
        // fresh flat at-codes likewise.
        let mut value_at_codes = std::collections::BTreeSet::new();
        let mut deferred_values: Vec<String> = Vec::new();
        collect_local_value_codes(&data.definition, &mut |code| {
            if collapse && code.contains('.') {
                if !deferred_values.iter().any(|c| c == code) {
                    deferred_values.push(code.to_owned());
                }
                value_at_codes.insert(code.to_owned());
                return;
            }
            let new = shift_code(code, "at");
            if let Some(n) = first_num(&new) {
                max_at = max_at.max(n);
            }
            value_at_codes.insert(code.to_owned());
        });
        self.value_at_codes = value_at_codes;
        // Existing ac codes (terminology entries + bare constraint refs):
        // synthesised/minted acs must allocate ABOVE the shifted range so a
        // fresh `acN` never collides with an existing `ac000(N-1)` → `acN`.
        let mut highest_ac = 0i64;
        let mut track_ac = |code: &str| {
            if crate::codes::is_ac_code(code)
                && !code.contains('.')
                && let Some(n) = first_num(&shift_code(code, "ac"))
            {
                highest_ac = highest_ac.max(n);
            }
        };
        for terms in data.terminology.term_definitions.values() {
            for code in terms.keys() {
                track_ac(code);
            }
        }
        walk_constraints(&data.definition, &mut |raw, _| {
            let body = raw.split_once(';').map_or(raw, |(b, _)| b);
            if !body.contains("::") {
                track_ac(body.trim());
            }
        });
        self.next_id = max_id + 1;
        self.next_at = max_at + 1;
        self.next_ac = highest_ac + 1;
        for code in deferred_nodes {
            let fresh = self.alloc_id();
            self.log.note(format!(
                "specialised node code {code} collapsed to {fresh} (standalone depth-0 emission)"
            ));
            self.node_map.insert(code, fresh);
        }
        for code in deferred_values {
            let fresh = self.alloc_at();
            self.log.note(format!(
                "specialised value code {code} collapsed to {fresh} (standalone depth-0 emission)"
            ));
            self.value_map.insert(code, fresh);
        }
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
            let fresh = self.alloc_id();
            self.assigned_node_ids.insert(fresh.clone());
            return fresh;
        }
        let mapped = self
            .node_map
            .get(old)
            .cloned()
            .unwrap_or_else(|| shift_code(old, "id"));
        if self.assigned_node_ids.insert(mapped.clone()) {
            return mapped;
        }
        // A second occurrence of a reused 1.4 code: 1.4 node ids are only
        // sibling-unique, ADL2 requires archetype-wide uniqueness (VCOSU,
        // `AOM2/master04.5` §Validity Rules: C_OBJECT) — mint a fresh id and
        // clone the first id's terminology in the rebuild.
        let fresh = self.alloc_id();
        self.assigned_node_ids.insert(fresh.clone());
        self.log.note(format!(
            "reused 1.4 node code {old} re-minted as {fresh} (archetype-wide id uniqueness)"
        ));
        self.dup_mints.push((mapped, fresh.clone()));
        fresh
    }

    /// The ADL2 at-code of a `local` value code: the collapse remap when one
    /// exists, else the ordinary shift.
    fn value_at(&self, code: &str) -> String {
        self.value_map
            .get(code)
            .cloned()
            .unwrap_or_else(|| shift_code(code, "at"))
    }

    /// The at-code for a terminology entry under the collapse: a dotted code
    /// with no planned remap yet (an UNUSED flattened-ontology entry — VTSD
    /// covers defined codes, not only used ones) mints a fresh flat at-code
    /// on first sight; everything else follows [`Self::value_at`].
    fn collapsed_value_at(&mut self, code: &str) -> String {
        if self.cfg.collapse_specialised_codes
            && code.contains('.')
            && !self.value_map.contains_key(code)
        {
            let fresh = self.alloc_at();
            self.log.note(format!(
                "specialised terminology code {code} collapsed to {fresh} (standalone depth-0 emission)"
            ));
            self.value_map.insert(code.to_owned(), fresh);
        }
        self.value_at(code)
    }

    /// The ac-code under the collapse: a dotted ac (specialised constraint
    /// definition) mints a fresh flat ac on first sight; else the ordinary
    /// shift.
    fn collapsed_ac(&mut self, code: &str) -> String {
        if self.cfg.collapse_specialised_codes && code.contains('.') {
            if let Some(existing) = self.ac_map.get(code) {
                return existing.clone();
            }
            let fresh = self.alloc_ac();
            self.log.note(format!(
                "specialised constraint code {code} collapsed to {fresh} (standalone depth-0 emission)"
            ));
            self.ac_map.insert(code.to_owned(), fresh.clone());
            return fresh;
        }
        shift_code(code, "ac")
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
        // Not a qualified 1.4 code: a bare ac-code (a 1.4 `CONSTRAINT_REF`
        // reference or a reference-set ac minted by the OPT front end) is
        // shifted like every other code so it stays aligned with its shifted
        // terminology entry (`ac0001`→`ac2`; the `ac0.K` specialisation form
        // keeps its number — see `shift_code`). Anything else (`at5`) is
        // already ADL2; pass through.
        let Some((terminology, codes_str)) = body.split_once("::") else {
            if crate::codes::is_ac_code(body) {
                let ac = self.collapsed_ac(body);
                let assumed = assumed_raw.map(|a| self.collapsed_value_at(a));
                return (ac, assumed);
            }
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
                self.collapsed_value_at(code)
            } else {
                self.external_at_code(terminology, code)
            };
            at_codes.push(at);
        }

        let assumed = assumed_raw.map(|a| {
            if is_local {
                self.collapsed_value_at(a)
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
            // NOTE: the synthesised rubric uses the external code itself as its
            // text/description. Resolving a human-readable name would require the
            // external terminology (SNOMED CT, LOINC, an openEHR terminology group)
            // to be resolved, which this network-free converter does not do. No
            // openEHR spec governs 1.4→2 conversion — our own design/extension.
            text: code.to_owned(),
            description: code.to_owned(),
            binding: Some((terminology.to_owned(), uri)),
            value_set_members: None,
        });
        at
    }

    // ── terminology rebuild ──────────────────────────────────────────────────

    #[expect(
        clippy::too_many_lines,
        reason = "one linear terminology rebuild — definitions, then bindings, then value sets; the steps are sequential, not extractable units"
    )]
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
                // An ac-code (a merged 1.4 `constraint_definitions` entry —
                // ADL2 merges that section into `term_definitions`,
                // `master07.13` §Terminology section) keeps its ac prefix,
                // shifted like every other code so the converted
                // `C_TERMINOLOGY_CODE` ac constraints still resolve (VACDF).
                if crate::codes::is_ac_code(code) {
                    let ac = self.collapsed_ac(code);
                    out.insert(ac.clone(), term_with_code(term, ac));
                    continue;
                }
                // A 1.4 at-code used as a value (`[local::atX]`) always yields an
                // at-code term; used as a node id it yields an id-code term. A
                // code that is both splits into both entries.
                if self.value_at_codes.contains(code) || !is_node {
                    let at = self.collapsed_value_at(code);
                    out.insert(at.clone(), term_with_code(term, at));
                }
                if is_node {
                    // Drop an `@ internal @` node term in every language
                    // (reference-converter behaviour; validates clean).
                    if internal_nodes.contains(code) {
                        continue;
                    }
                    // The planned mapping, never a re-shift: under the
                    // specialisation collapse a dotted code maps to a fresh
                    // flat id that a plain shift would miss.
                    let id = self
                        .node_map
                        .get(code)
                        .cloned()
                        .unwrap_or_else(|| shift_code(code, "id"));
                    out.insert(id.clone(), term_with_code(term, id));
                }
            }
            // VCOSU re-mints: each freshly minted node id reuses the first
            // occurrence's rubric (the 1.4 source defined ONE term for the
            // shared code; both ADL2 nodes must be defined — VATDF).
            for (first, fresh) in &self.dup_mints {
                if let Some(term) = out.get(first) {
                    out.insert(fresh.clone(), term_with_code(term, fresh.clone()));
                }
            }
            // Add synthesised terms to every language.
            //
            // NOTE: one rubric text is minted for all languages (a converter has
            // no translator to produce per-language rubrics; the reference
            // fixtures carry per-language translated text with a `(synthesised)`
            // suffix, which the structural conversion does not reproduce). No
            // openEHR spec governs 1.4→2 conversion — our own design/extension.
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
        // VCOSU re-mints inherit the first occurrence's binding, if any.
        for bindings in term_bindings.values_mut() {
            for (first, fresh) in &self.dup_mints {
                if let Some(uri) = bindings.get(first).cloned() {
                    bindings.insert(fresh.clone(), uri);
                }
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
            self.value_at(key)
        } else if crate::codes::is_ac_code(key) {
            // A merged 1.4 `constraint_bindings` key (ADL2 folds that section
            // into `term_bindings`, `master07.13` §Terminology section)
            // follows its definitions-rebuild remap (collapse map first,
            // else the ordinary ac shift), matching VTCBK.
            self.ac_map
                .get(key)
                .cloned()
                .unwrap_or_else(|| shift_code(key, "ac"))
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

        // `other_details["build_uid"]` → the archetype's `build_uid`
        // (`ADL1.4/masterAppB-extended_metadata.adoc` §Standardised Items:
        // "Guid string … See AOM2 spec, Machine Identifiers section"). Only a
        // well-formed GUID is consumed — a value violating the stated syntax
        // stays in `other_details` verbatim rather than being guessed at — and
        // an already-populated `build_uid` (a 1.4 header `build_uid=` meta
        // item) is never overwritten by the meta-data section.
        if data.build_uid.value.is_nil()
            && let Some(other) = data
                .description
                .as_mut()
                .and_then(|d| d.other_details.as_mut())
            && let Some(value) = other
                .get("build_uid")
                .and_then(|raw| uuid::Uuid::parse_str(raw.trim()).ok())
        {
            data.build_uid = openehr_base::prelude::Uuid { value };
            other.remove("build_uid");
        }

        if let Some(desc) = data.description.as_mut() {
            transform_description(desc);
            // Surface the conversion's non-mechanical decisions (collapse
            // remaps, VCOSU re-mints) as conversion provenance — the AOM2
            // home for it (`RESOURCE_DESCRIPTION.conversion_details`).
            if !self.log.notes.is_empty() {
                let details = desc.conversion_details.get_or_insert_with(BTreeMap::new);
                for (index, note) in self.log.notes.iter().enumerate() {
                    details.insert(format!("code_remap_{index:03}"), note.clone());
                }
            }
        }
    }
}

// ── description transform ────────────────────────────────────────────────────

fn transform_description(
    desc: &mut openehr_am::am24::resource::resource_description::ResourceDescription,
) {
    // NOTE: every 1.4 lifecycle state converts to `unmanaged`, matching the
    // conversion oracle — the vendored `upgrade_from_14` expected `.adls` all
    // carry `lifecycle_state = <"unmanaged">` regardless of the 1.4 source state
    // (AuthorDraft / CommitteeDraft / published). A finer state map would diverge
    // from that oracle. No openEHR spec governs 1.4→2 conversion — our own
    // design/extension.
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

    convert_standardised_meta_data(desc);

    // Drop the consumed `revision` from other_details.
    if let Some(o) = desc.other_details.as_mut() {
        o.remove("revision");
    }
}

/// Convert the ADL 1.4 standardised `description/other_details` meta-data items
/// to their AOM2 `RESOURCE_DESCRIPTION` homes, consuming each converted key.
///
/// `ADL1.4/masterAppB-extended_metadata.adoc` §Standardised Items is the one
/// part of 1.4→2 conversion the spec text governs directly: the items' "naming
/// and rules should be followed, and … are intended to be implemented by any
/// ADL 1.4 => ADL 2 conversion tool". The mapping applied here is the table's:
///
/// - `original_namespace`, `original_publisher`, `custodian_namespace`,
///   `custodian_organisation`, `licence` transfer verbatim to the same-named
///   `RESOURCE_DESCRIPTION` attributes. The table's `"name <URN>"` shapes are
///   DISPLAY conventions ("the use of the typical string for a person or
///   organisation of the form \"name \<URN\>\", which enables email addresses,
///   website URLs etc to be easily extracted", §Extended Meta-data Guide
///   preamble) — the AOM2 attributes are single strings, so the value is not
///   decomposed.
/// - `references` and `ip_acknowledgements` are "string with one LF (`\n`)
///   terminated line for each reference. Intervening LFs and leading and
///   trailing whitespace may be added for clarity, to be stripped on
///   conversion to ADL2" — so the value splits on LF, each line is trimmed,
///   and blank lines are dropped.
///
/// §Other Items (`MD5-CAM-1.0.1`, `current_contact`, `review_date`,
/// `responsible_organisation`) are reserved/display-only names with no
/// conversion mandated; they stay in `other_details` untouched, as does any
/// value that violates its item's stated syntax.
///
/// An AOM2 attribute already populated from elsewhere is never overwritten, and
/// its `other_details` key is then left in place (nothing was consumed).
fn convert_standardised_meta_data(
    desc: &mut openehr_am::am24::resource::resource_description::ResourceDescription,
) {
    let Some(other) = desc.other_details.as_mut() else {
        return;
    };
    take_verbatim(other, "original_namespace", &mut desc.original_namespace);
    take_verbatim(other, "original_publisher", &mut desc.original_publisher);
    take_verbatim(other, "custodian_namespace", &mut desc.custodian_namespace);
    take_verbatim(
        other,
        "custodian_organisation",
        &mut desc.custodian_organisation,
    );
    take_verbatim(other, "licence", &mut desc.licence);
    take_keyed_lines(other, "references", &mut desc.references);
    take_keyed_lines(other, "ip_acknowledgements", &mut desc.ip_acknowledgements);
}

/// Move `other[key]` verbatim into `target`, consuming the key; a no-op when
/// `target` is already populated or the key is absent.
fn take_verbatim(other: &mut BTreeMap<String, String>, key: &str, target: &mut Option<String>) {
    if target.is_some() {
        return;
    }
    if let Some(value) = other.remove(key) {
        *target = Some(value);
    }
}

/// Move the LF-separated lines of `other[key]` into `target` as a keyed list,
/// consuming the key; a no-op when `target` is already populated, the key is
/// absent, or no non-blank line survives the strip (nothing to convert — the
/// value stays in `other_details` rather than being dropped).
fn take_keyed_lines(
    other: &mut BTreeMap<String, String>,
    key: &str,
    target: &mut Option<BTreeMap<String, String>>,
) {
    if target.is_some() {
        return;
    }
    let Some(raw) = other.get(key) else {
        return;
    };
    let lines: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    if lines.is_empty() {
        return;
    }
    // NOTE: the appendix prescribes the LF-per-entry SOURCE syntax and the
    // AOM2 target ("a keyed list of strings"), but no key scheme for the
    // converted entries — no openEHR spec governs the key scheme, so this is
    // our own design: stable 1-based ordinals in source line order, matching
    // the `["1"]`/`["2"]` keys the vendored `upgrade_from_14` reference output
    // carries. (Ordinals are unpadded, so a list of ten or more entries sorts
    // lexicographically in the `BTreeMap` — a display order, not a semantic
    // one; the ordinal itself still names the source line.)
    *target = Some(
        lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| ((index + 1).to_string(), line))
            .collect(),
    );
    other.remove(key);
}

// ── free-function tree walks ─────────────────────────────────────────────────

/// The mutable `C_COMPLEX_OBJECT` data, if this is a plain complex object.
///
/// NOTE: a 1.4 *source archetype* never contains an inline `C_ARCHETYPE_ROOT`
/// (only a flattened OPT does), so the `CArchetypeRoot` arm yields `None` and its
/// walk is a no-op. Feeding a flattened OPT-1.4 through here is a separate
/// capability (OPT-1.4 → ADL2 conversion, not performed by this source-archetype
/// converter). No openEHR spec governs 1.4→2 conversion — our own
/// design/extension.
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
            // Tuple ROWS carry the actual primitive constraints (e.g. ordinal
            // `[value, symbol]` symbol codes) — visit their terminology codes
            // so value at-codes are planned and converted like attribute ones.
            for row in &tuple.tuples {
                for m in &row.members {
                    if let CPrimitiveObject::CTerminologyCode(tc) = m {
                        f(&tc.constraint, "");
                    }
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
        // Tuple ROWS carry the actual primitive constraints — convert their
        // terminology codes (ordinal symbols etc.) like attribute ones.
        for row in &mut tuple.tuples {
            for m in &mut row.members {
                if let CPrimitiveObject::CTerminologyCode(tc) = m {
                    let (constraint, assumed) = cx.convert_constraint(&tc.constraint, &node_text);
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
            let rest = path.get(start..).unwrap_or_default();
            if let Some(end) = rest.find(']')
                && let Some(code) = rest.get(..end)
            {
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

/// Materialise the ADL 1.4 default `occurrences` on container children before the
/// definition is emitted as ADL 2.
///
/// The two formalisms give an ABSENT `occurrences` different meanings, so the
/// default cannot be carried across implicitly:
///
/// - ADL 1.4 — `ADL1.4/master05-cadl.adoc` §Occurrences L316: "The default
///   occurrences, if none is mentioned, is `{1..1}`".
/// - ADL 2 — `AOM2/master04.5-constraint_model-class_definitions.adoc`
///   §Occurrences inferencing rules: an absent `occurrences` is inferred from the
///   owning attribute's cardinality upper (lower forced to 0), i.e.
///   `0..cardinality.upper`.
///
/// So an unstated 1.4 occurrences on a container child means "exactly once", and
/// leaving it unstated in the ADL 2 output would silently widen it to "none to
/// many". It is written out explicitly here.
///
/// Restricted to CONTAINER attributes because master05 L308 restricts the rule's
/// significance to them ("It only has significance for objects which are children
/// of a container attribute, since by definition, the occurrences of an object
/// which is the value of a single valued attribute can only be `0..1` or `1..1`,
/// and this is already defined by the attribute `existence`"). A `use_node`
/// internal reference is exempt: master05 L515 gives it the REFERENCED node's
/// occurrences, which is exactly what leaving it unstated means in ADL 2 once the
/// proxy is expanded.
///
/// NOTE: no openEHR spec governs 1.4→2 conversion — our own design (see the
/// module flag on [`crate::adl14`]); the two default rules it reconciles are the
/// spec-cited ones above.
fn materialise_adl14_occurrences(def: &mut CComplexObject) {
    let Some(d) = cco_data_mut(def) else { return };
    for attr in &mut d.attributes {
        let is_container = attr.cardinality.is_some();
        for child in &mut attr.children {
            if is_container
                && child_occurrences(child).is_none()
                && !matches!(child, CObject::CComplexObjectProxy(_))
            {
                set_occurrences(child, one_to_one());
            }
            if let CObject::CComplexObject(c) = child {
                materialise_adl14_occurrences(c);
            }
        }
    }
}

/// The ADL 1.4 default multiplicity `{1..1}` (`ADL1.4/master05-cadl.adoc`
/// §Occurrences L316).
fn one_to_one() -> openehr_base::prelude::MultiplicityInterval {
    openehr_base::prelude::MultiplicityInterval {
        lower: Some(1),
        upper: Some(1),
        lower_unbounded: false,
        upper_unbounded: false,
        lower_included: true,
        upper_included: true,
    }
}

fn set_occurrences(obj: &mut CObject, occ: openehr_base::prelude::MultiplicityInterval) {
    match obj {
        CObject::CComplexObject(CComplexObject::CComplexObject(d)) => d.occurrences = Some(occ),
        CObject::CComplexObject(CComplexObject::CArchetypeRoot(r)) => r.occurrences = Some(occ),
        CObject::CComplexObjectProxy(p) => p.occurrences = Some(occ),
        CObject::ArchetypeSlot(s) => s.occurrences = Some(occ),
        CObject::CBoolean(o) => o.occurrences = Some(occ),
        CObject::CInteger(o) => o.occurrences = Some(occ),
        CObject::CReal(o) => o.occurrences = Some(occ),
        CObject::CString(o) => o.occurrences = Some(occ),
        CObject::CTerminologyCode(o) => o.occurrences = Some(occ),
        CObject::CDate(o) => o.occurrences = Some(occ),
        CObject::CTime(o) => o.occurrences = Some(occ),
        CObject::CDateTime(o) => o.occurrences = Some(occ),
        CObject::CDuration(o) => o.occurrences = Some(occ),
    }
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
        if let Some((numeric, tail)) = v.split_once(marker) {
            let numeric = normalise_numeric(numeric);
            let build = tail.strip_prefix('.').unwrap_or("").to_owned();
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

//! The AOM2 validation catalogue.
//!
//! A [`Validator`] walks an assembled `openehr_am::am24::aom2` [`Archetype`]
//! and produces typed [`ValidationIssue`]s, each carrying a [`ValidationCode`]
//! (one variant per AOM2 validity code), a [`Severity`], a message, and — where
//! derivable — the archetype path the issue is anchored at.
//!
//! Phase 1 (basic integrity, standalone) is implemented in [`phase1`]
//! ([`validate_phase1`] / [`validate_source_phase1`]); the phase-2
//! reference-model checks are in [`rm`] ([`rm::validate_phase2_rm`]). The
//! phase-1 catalogue and its orchestration are defined in
//! `docs/specs/openehr/AM/docs/AOM2/master08-validation.adoc` §Phase 1 - Basic
//! Integrity, with the full rule texts in `master03-archetype_package.adoc`,
//! `master04.5-constraint_model-class_definitions.adoc`,
//! `master06-rm_overlay.adoc`, and `master07-terminology_package.adoc` (cited
//! per check in [`phase1`]); the phase-2 RM checks are `master08` §Phase 2 →
//! Validate Against Reference Model (cited per check in [`rm`]).
//!
//! Phase orchestration follows `master08` "multi-pass … more basic kinds of
//! errors being checked first": phase 2 runs only after phase 1 passes, which
//! [`validate`] / [`validate_source`] apply (`master08` §Overview). Parent-
//! dependent checks (VACSD's depth comparison, VASID, VALC, VTPL) take an
//! optional [`ArchetypeRepository`]; when a parent is not supplied they degrade
//! to the standalone half they can compute (or are skipped).
//!
//! The specialised-archetype-vs-flat-parent checks (VSON*/VSANC*/VSSM/VDSS*/
//! VARX*/…) live in [`phase2`] and run against the flat parent resolved via
//! [`resolve_flat_parent`]; the `master04.5` conformance functions they build on
//! are in [`conformance`]. Per `ADL2/master09.02` §Differential and Flat Forms
//! ("For a top-level archetype, the flat-form is the same as its differential
//! form") a level-0 parent is used as-is; a parent that is itself specialised
//! needs the full flattener before its DEEP flat form is available.
//!
//! TODO: build the specialisation flattener so [`FlatParent::NeedsFlattener`]
//! parents (specialised parents) can be flattened for the deep phase-2 checks,
//! and add the phase-3 flat-form checks (VUNP, VACMCO).

pub mod conformance;
mod phase1;
mod phase2;
pub mod rm;

use std::collections::HashMap;

use openehr_am::am24::aom2::archetype::archetype::Archetype;
use openehr_am::am24::aom2::archetype::archetype_hrid::ArchetypeHrid;
use openehr_am::am24::aom2::archetype::authored_archetype::AuthoredArchetype;
use openehr_am::am24::aom2::constraint_model::c_complex_object::CComplexObject;
use openehr_am::am24::aom2::rm_overlay::rm_overlay::RmOverlay;
use openehr_am::am24::aom2::terminology::archetype_terminology::ArchetypeTerminology;
use openehr_am::am24::resource::resource_description::ResourceDescription;
use openehr_base::prelude::{ResourceAnnotations, TerminologyCode};

use crate::error::SyntaxError;
use crate::source::{ArtefactKind, parse_source};

/// The severity of a [`ValidationIssue`].
///
/// The `W`-prefixed codes (WACMCL, WOUC) are warnings; every other code is an
/// error. `master08` assigns no explicit severity column, so this follows the
/// `V`/`W` naming convention (the `W` prefix = advisory "should"; see
/// `master04.5` WACMCL "should be" vs VACMCU "must").
///
/// NOTE: no openEHR spec states the `W`→Warning convention normatively; it is
/// inferred from the code naming (`master08-validation` is silent on severity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    /// A validity error — the archetype is invalid.
    Error,
    /// A validity warning — advisory, does not invalidate the archetype.
    Warning,
}

/// An AOM2 validation code (one typed variant per validity rule).
///
/// Each variant's doc comment names the spec file + section that defines it.
/// The catalogue is the phase-1 set of `docs/specs/openehr/AM/docs/AOM2/`
/// plus the two corpus-adjudicated additions (VRDLA, WOUC — archie parity, no
/// full vendored text, NOTE-flagged). Deferred variants (their check needs the
/// RM model, the flat parent, or an external terminology service) are present
/// as the vocabulary but not raised in phase 1 — see [`phase1`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidationCode {
    /// VARDT — archetype definition typename validity (`master03` §Validity Rules).
    Vardt,
    /// VARCN — archetype concept validity (`master03` §Validity Rules).
    Varcn,
    /// STCNT — missing mandatory part, e.g. terminology (`master08` §Phase 1; no
    /// full vendored text — NOTE-flagged).
    Stcnt,
    /// VACSD — archetype concept specialisation depth (`master03` §Validity Rules).
    Vacsd,
    /// VOLT — original language available in terminology (`master08` §Phase 1; no
    /// full vendored text — NOTE-flagged).
    Volt,
    /// VARAV — ADL version validity (`master03` §Validity Rules).
    Varav,
    /// VARRV — RM release validity (`master03` §Validity Rules).
    Varrv,
    /// VOTM — terminology translations validity (`master03` §Validity Rules).
    Votm,
    /// VDIFV — differential path only in specialised archetype (`master04.5`
    /// §`C_ATTRIBUTE`).
    Vdifv,
    /// VDIFP — differential path exists in flat parent (`master04.5` §`C_ATTRIBUTE`).
    ///
    /// TODO: needs the specialisation flattener (the flat parent) + the RM path
    /// walk before it can run.
    Vdifp,
    /// VCORM — object constraint type name exists in the RM (`master04.5`
    /// §`C_OBJECT`; checked in [`rm`]).
    Vcorm,
    /// VCORMT — object type conforms to the owning attribute's RM type
    /// (`master04.5` §`C_OBJECT`; checked in [`rm`]).
    Vcormt,
    /// VCARM — attribute name exists on the enclosing RM type (`master04.5`
    /// §`C_ATTRIBUTE`; checked in [`rm`]).
    Vcarm,
    /// VCAEX — attribute existence conforms to the RM existence (`master04.5`
    /// §`C_ATTRIBUTE`; checked in [`rm`]).
    Vcaex,
    /// VCACA — attribute cardinality conforms to the RM cardinality (`master04.5`
    /// §`C_ATTRIBUTE`; checked in [`rm`]).
    Vcaca,
    /// VCAM — attribute single/multiple arity matches the RM (`master04.5`
    /// §`C_ATTRIBUTE`; checked in [`rm`]).
    Vcam,
    /// VATCV — terminology code format validity (`master08` §Phase 1; no full
    /// vendored text — NOTE-flagged).
    Vatcv,
    /// VTSD — specialisation level of codes (`master07` §Validity Rules).
    Vtsd,
    /// VTLC — terminology language consistency (`master07` §Validity Rules).
    Vtlc,
    /// VTTBK — term binding key valid (`master07` §Validity Rules).
    Vttbk,
    /// VTCBK — constraint binding key valid (`master07` §Validity Rules).
    Vtcbk,
    /// VETDF — external term validity (`master03` §Validity Rules).
    /// TODO: check against an external terminology service.
    Vetdf,
    /// VTVSID — value-set id defined (`master07` §Validity Rules).
    Vtvsid,
    /// VTVSMD — value-set members defined (`master07` §Validity Rules).
    Vtvsmd,
    /// VTVSUQ — value-set members unique (`master07` §Validity Rules).
    Vtvsuq,
    /// VDSEV — slot 'exclude' constraint validity (`master04.5` §`ARCHETYPE_SLOT`).
    Vdsev,
    /// VDSIV — slot 'include' constraint validity (`master04.5` §`ARCHETYPE_SLOT`).
    Vdsiv,
    /// VARXRA — `C_ARCHETYPE_ROOT` validity set (`master08` §Phase 1; umbrella for
    /// VARXNC/VARXAV/VARXR — no full vendored text, NOTE-flagged).
    Varxra,
    /// VARXNC — `C_ARCHETYPE_ROOT` node-id conformance (`master08` §Phase 1).
    Varxnc,
    /// VARXAV — `C_ARCHETYPE_ROOT` archetype-ref validity (`master08` §Phase 1).
    Varxav,
    /// VARXR — external reference resolution (`master08` §Phase 2).
    /// TODO: resolve external references against the supplier repository.
    Varxr,
    /// VARXTV — `C_ARCHETYPE_ROOT` type validity (`master08` §Phase 1).
    Varxtv,
    /// VATID — all definition codes defined in terminology (`master08` §Phase 1;
    /// no full vendored text — NOTE-flagged).
    Vatid,
    /// VATCD — archetype code specialisation level validity (`master03` §Validity
    /// Rules).
    Vatcd,
    /// VATDF — value code (at-code) validity (`master03` §Validity Rules).
    /// TODO: check the flat-parent half for specialised archetypes (needs the
    /// flattener).
    Vatdf,
    /// VACDF — constraint code (ac-code) validity (`master03` §Validity Rules).
    Vacdf,
    /// VATDA — value-set assumed value code validity (`master03` §Validity Rules).
    Vatda,
    /// VRANP — annotation path valid (`master03` §Validity Rules; the RM-path half
    /// is a reference-model check, [`rm`]).
    Vranp,
    /// VOKU — object key unique in keyed lists (`master03` §Validity Rules).
    Voku,
    /// VARID — archetype identifier validity (`master03` §Validity Rules).
    Varid,
    /// VDEOL — original language specified (`master03` §Validity Rules).
    Vdeol,
    /// VARD — description specified (`master03` §Validity Rules).
    Vard,
    /// VASID — specialisation parent identifier validity (`master03` §Validity
    /// Rules; fires only when the parent is supplied).
    Vasid,
    /// VALC — archetype language conformance (`master03` §Validity Rules; fires
    /// only when the parent is supplied).
    Valc,
    /// VTPL — template/filler language consistency (`master03` §Validity Rules).
    /// TODO: check once template fillers are resolved.
    Vtpl,
    /// VRRLP — rule path valid (`master03` §Validity Rules; the RM-extension half
    /// is a reference-model check, [`rm`]).
    Vrrlp,
    /// VCOCD — object constraint definition validity (`master04.5` §`C_OBJECT`).
    Vcocd,
    /// VCOID — object node identifier present (`master04.5` §`C_OBJECT`).
    Vcoid,
    /// VCOSU — object node identifier unique (`master04.5` §`C_OBJECT`).
    Vcosu,
    /// VCATU — sibling attribute uniqueness (`master04.5` §`C_COMPLEX_OBJECT`).
    Vcatu,
    /// VDFAI — archetype id validity in slot definition (`master04.5`
    /// §`ARCHETYPE_SLOT`).
    Vdfai,
    /// VOBAV — object node assumed value validity (`master04.5`
    /// §`C_PRIMITIVE_OBJECT`).
    Vobav,
    /// VRMVP — RM-visibility path validity (`master06` §Validity).
    Vrmvp,
    /// VRMVAV — RM-visibility alias validity (`master06` §Validity).
    Vrmvav,
    /// VACSO — single-valued attribute child occurrences validity (`master04.5`
    /// §`C_ATTRIBUTE`).
    Vacso,
    /// VACMCU — cardinality/occurrences upper bound validity (`master04.5`
    /// §`C_ATTRIBUTE`).
    Vacmcu,
    /// VSONIF — object node identification validity in flat siblings (`master04.5`
    /// §`ARCHETYPE_SLOT`; refs undefined VACMI).
    /// TODO: check against the flattened parent siblings (needs the flattener).
    Vsonif,
    /// VRDLA — resource-description language-code consistency (archie parity; no
    /// openEHR spec governs this — our own design/extension, NOTE-flagged).
    Vrdla,
    /// WACMCL — cardinality/occurrences lower bound warning (`master04.5`
    /// §`C_ATTRIBUTE`; WARNING).
    Wacmcl,
    /// WOUC — defined terminology code unused in the definition (archie parity;
    /// no openEHR spec governs this — our own design/extension; WARNING).
    Wouc,
    // ── phase-2 specialisation-vs-flat-parent codes (`master04.5` §Validity
    //    Rules: `C_ATTRIBUTE` / `C_OBJECT` / `ARCHETYPE_SLOT` / `C_ARCHETYPE_ROOT` /
    //    `C_COMPLEX_OBJECT_PROXY`; `master08` §Phase 2 → Validate Specialised
    //    Definition). Raised by [`phase2`] against the flat parent.
    /// VSANCE — specialised attribute node existence conformance (`master04.5`
    /// §`C_ATTRIBUTE`).
    Vsance,
    /// VSANCC — specialised attribute node cardinality conformance (`master04.5`
    /// §`C_ATTRIBUTE`).
    Vsancc,
    /// VSAM — specialised attribute multiplicity conformance (`master04.5`
    /// §`C_ATTRIBUTE`).
    Vsam,
    /// VSONIN — new object node identifier validity (`master04.5` §`C_OBJECT`).
    Vsonin,
    /// VSSM — specialised sibling order validity (`master04.5` §`C_OBJECT`).
    Vssm,
    /// VSONT — specialised object node meta-type conformance (`master04.5`
    /// §`C_OBJECT`).
    Vsont,
    /// VSONCT — specialised object node reference type conformance (`master04.5`
    /// §`C_OBJECT`).
    Vsonct,
    /// VSONCO — specialised object node occurrences redefinition validity
    /// (`master04.5` §`C_OBJECT` — the collective-occurrences rule).
    Vsonco,
    /// VSONPT — prohibited object node AOM type validity (`master04.5` §`C_OBJECT`).
    Vsonpt,
    /// VSONPI — prohibited object node node-id validity (`master04.5` §`C_OBJECT`).
    Vsonpi,
    /// VSONPO — new object node prohibited occurrences validity (`master04.5`
    /// §`C_OBJECT`).
    Vsonpo,
    /// VSONI — _deprecated_ redefined object node identifier validity (`master04.5`
    /// §`C_OBJECT`; recognise, do not enforce).
    Vsoni,
    /// VSONIR — _deprecated_ redefined object node identifier condition
    /// (`master04.5` §`C_OBJECT`; recognise, do not enforce).
    Vsonir,
    /// VSUNT — `use_node` specialisation parent validity (`master04.5`
    /// §`C_COMPLEX_OBJECT_PROXY`).
    Vsunt,
    /// VUNT — `use_node` reference model type validity (`master04.5`
    /// §`C_COMPLEX_OBJECT_PROXY`).
    Vunt,
    /// VDSSID — slot redefinition child node id (`master04.5` §`ARCHETYPE_SLOT`).
    Vdssid,
    /// VDSSM — specialised slot definition match validity (`master04.5`
    /// §`ARCHETYPE_SLOT`).
    Vdssm,
    /// VDSSP — specialised slot definition parent validity (`master04.5`
    /// §`ARCHETYPE_SLOT`).
    Vdssp,
    /// VDSSC — specialised slot definition closed validity (`master04.5`
    /// §`ARCHETYPE_SLOT`).
    Vdssc,
    /// VARXS — external reference slot conformance (`master04.5`
    /// §`C_ARCHETYPE_ROOT`).
    Varxs,
    /// VARXID — external reference slot filling id validity (`master04.5`
    /// §`C_ARCHETYPE_ROOT`).
    Varxid,
    /// VPOV — invalid leaf object value redefinition (`master08` §Phase 2; no full
    /// vendored text — implemented from the gloss via `c_value_conforms_to`,
    /// NOTE-flagged).
    Vpov,
    /// VUNK — invalid leaf object value redefinition (`master08` §Phase 2; no full
    /// vendored text — NOTE-flagged).
    Vunk,
    /// VTPNC — tuple non-conformance to the parent node (`master08` §Phase 2; no
    /// full vendored text — NOTE-flagged).
    Vtpnc,
    /// VTPIN — tuple invalidity against the parent node (`master08` §Phase 2; no
    /// full vendored text — NOTE-flagged).
    Vtpin,
}

impl ValidationCode {
    /// The bare mnemonic (e.g. `"VARDT"`), as used in the spec catalogue and
    /// the conformance-corpus `regression` tags.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one arm per catalogue code
    pub fn mnemonic(self) -> &'static str {
        match self {
            Self::Vardt => "VARDT",
            Self::Varcn => "VARCN",
            Self::Stcnt => "STCNT",
            Self::Vacsd => "VACSD",
            Self::Volt => "VOLT",
            Self::Varav => "VARAV",
            Self::Varrv => "VARRV",
            Self::Votm => "VOTM",
            Self::Vdifv => "VDIFV",
            Self::Vdifp => "VDIFP",
            Self::Vcorm => "VCORM",
            Self::Vcormt => "VCORMT",
            Self::Vcarm => "VCARM",
            Self::Vcaex => "VCAEX",
            Self::Vcaca => "VCACA",
            Self::Vcam => "VCAM",
            Self::Vatcv => "VATCV",
            Self::Vtsd => "VTSD",
            Self::Vtlc => "VTLC",
            Self::Vttbk => "VTTBK",
            Self::Vtcbk => "VTCBK",
            Self::Vetdf => "VETDF",
            Self::Vtvsid => "VTVSID",
            Self::Vtvsmd => "VTVSMD",
            Self::Vtvsuq => "VTVSUQ",
            Self::Vdsev => "VDSEV",
            Self::Vdsiv => "VDSIV",
            Self::Varxra => "VARXRA",
            Self::Varxnc => "VARXNC",
            Self::Varxav => "VARXAV",
            Self::Varxr => "VARXR",
            Self::Varxtv => "VARXTV",
            Self::Vatid => "VATID",
            Self::Vatcd => "VATCD",
            Self::Vatdf => "VATDF",
            Self::Vacdf => "VACDF",
            Self::Vatda => "VATDA",
            Self::Vranp => "VRANP",
            Self::Voku => "VOKU",
            Self::Varid => "VARID",
            Self::Vdeol => "VDEOL",
            Self::Vard => "VARD",
            Self::Vasid => "VASID",
            Self::Valc => "VALC",
            Self::Vtpl => "VTPL",
            Self::Vrrlp => "VRRLP",
            Self::Vcocd => "VCOCD",
            Self::Vcoid => "VCOID",
            Self::Vcosu => "VCOSU",
            Self::Vcatu => "VCATU",
            Self::Vdfai => "VDFAI",
            Self::Vobav => "VOBAV",
            Self::Vrmvp => "VRMVP",
            Self::Vrmvav => "VRMVAV",
            Self::Vacso => "VACSO",
            Self::Vacmcu => "VACMCU",
            Self::Vsonif => "VSONIF",
            Self::Vrdla => "VRDLA",
            Self::Wacmcl => "WACMCL",
            Self::Wouc => "WOUC",
            Self::Vsance => "VSANCE",
            Self::Vsancc => "VSANCC",
            Self::Vsam => "VSAM",
            Self::Vsonin => "VSONIN",
            Self::Vssm => "VSSM",
            Self::Vsont => "VSONT",
            Self::Vsonct => "VSONCT",
            Self::Vsonco => "VSONCO",
            Self::Vsonpt => "VSONPT",
            Self::Vsonpi => "VSONPI",
            Self::Vsonpo => "VSONPO",
            Self::Vsoni => "VSONI",
            Self::Vsonir => "VSONIR",
            Self::Vsunt => "VSUNT",
            Self::Vunt => "VUNT",
            Self::Vdssid => "VDSSID",
            Self::Vdssm => "VDSSM",
            Self::Vdssp => "VDSSP",
            Self::Vdssc => "VDSSC",
            Self::Varxs => "VARXS",
            Self::Varxid => "VARXID",
            Self::Vpov => "VPOV",
            Self::Vunk => "VUNK",
            Self::Vtpnc => "VTPNC",
            Self::Vtpin => "VTPIN",
        }
    }

    /// The severity of this code: [`Severity::Warning`] for the `W`-prefixed
    /// codes, [`Severity::Error`] otherwise (see [`Severity`]).
    #[must_use]
    pub fn severity(self) -> Severity {
        match self {
            Self::Wacmcl | Self::Wouc => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

impl std::fmt::Display for ValidationCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.mnemonic())
    }
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// The catalogue code.
    pub code: ValidationCode,
    /// The severity (mirrors [`ValidationCode::severity`]).
    pub severity: Severity,
    /// A concrete, human-readable description of the specific violation.
    pub message: String,
    /// The archetype path the issue is anchored at, where derivable.
    pub path: Option<String>,
    /// A source byte span, for source-level checks (e.g. VOKU) where derivable.
    pub span: Option<std::ops::Range<usize>>,
}

impl ValidationIssue {
    /// Build an issue for `code` with `message`, deriving the severity.
    pub(crate) fn new(code: ValidationCode, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: code.severity(),
            message: message.into(),
            path: None,
            span: None,
        }
    }

    /// Attach an archetype path.
    #[must_use]
    pub(crate) fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// A minimal in-memory archetype repository — the parent/supplier seam the
/// specialisation-aware checks resolve against.
///
/// Keyed on the `publisher-package-class.concept` portion of the HRID (version
/// family and namespace are ignored for lookup), so a child's
/// `parent_archetype_id` (`…redefine_occurrences.v1`) resolves to the parsed
/// parent (`…redefine_occurrences.v1.0.0`).
#[derive(Debug, Default)]
pub struct ArchetypeRepository {
    by_id: HashMap<String, Archetype>,
}

impl ArchetypeRepository {
    /// A new, empty repository.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a parsed archetype under its HRID key.
    pub fn insert(&mut self, archetype: Archetype) {
        let key = hrid_lookup_key(view(&archetype).archetype_id);
        self.by_id.insert(key, archetype);
    }

    /// Resolve a raw archetype-id reference (as it appears in a
    /// `parent_archetype_id` / external ref) to a registered archetype.
    #[must_use]
    pub fn get(&self, raw_id: &str) -> Option<&Archetype> {
        self.by_id.get(&raw_id_lookup_key(raw_id))
    }
}

/// The outcome of resolving a specialised archetype's flat parent — the input
/// the phase-2 specialisation checks validate against.
///
/// A level-0 (non-specialised) parent is its own flat form, so it is returned
/// [`Available`](FlatParent::Available) directly (`ADL2/master09.02`
/// §Differential and Flat Forms: "For a top-level archetype, the flat-form is
/// the same as its differential form"). A parent that is itself specialised
/// needs the deep flat form the flattener is not yet built to produce, and is
/// reported [`NeedsFlattener`](FlatParent::NeedsFlattener) rather than validated
/// against a wrong (un-flattened) parent.
#[derive(Debug, Clone, Copy)]
pub enum FlatParent<'a> {
    /// The archetype is not specialised — the phase-2 specialisation checks do
    /// not apply.
    NotSpecialised,
    /// The flat parent is available (a level-0 parent used as-is).
    Available(&'a Archetype),
    /// The declared parent is registered but is itself specialised, so its deep
    /// flat form needs the flattener.
    NeedsFlattener,
    /// The declared parent could not be resolved in the repository.
    NotFound,
}

/// Resolve `child`'s flat parent from `repo` for the phase-2 specialisation
/// checks.
///
/// Returns [`FlatParent::NotSpecialised`] for a non-specialised archetype,
/// [`FlatParent::NotFound`] when the declared parent is absent from `repo`,
/// [`FlatParent::NeedsFlattener`] when the parent is itself specialised (its
/// deep flat form is not yet computable), and
/// [`FlatParent::Available`] for a level-0 parent (its own flat form).
#[must_use]
pub fn resolve_flat_parent<'a>(child: &Archetype, repo: &'a ArchetypeRepository) -> FlatParent<'a> {
    let Some(parent_id) = view(child).parent_archetype_id else {
        return FlatParent::NotSpecialised;
    };
    let Some(parent) = repo.get(parent_id) else {
        return FlatParent::NotFound;
    };
    if view(parent).is_specialised() {
        // TODO: flatten the specialised parent to obtain its deep flat form.
        return FlatParent::NeedsFlattener;
    }
    FlatParent::Available(parent)
}

/// The `publisher-package-class.concept` lookup key of an [`ArchetypeHrid`].
fn hrid_lookup_key(h: &ArchetypeHrid) -> String {
    format!(
        "{}-{}-{}.{}",
        h.rm_publisher, h.rm_package, h.rm_class, h.concept_id
    )
}

/// The lookup key of a raw archetype-id string (strips an optional `ns::`
/// namespace prefix and the trailing `.vN…` version).
fn raw_id_lookup_key(raw: &str) -> String {
    let no_ns = raw.rsplit("::").next().unwrap_or(raw);
    match version_marker(no_ns) {
        Some(idx) => no_ns.get(..idx).unwrap_or(no_ns).to_owned(),
        None => no_ns.to_owned(),
    }
}

/// The byte index of the version marker in an archetype id — the first `.v`
/// immediately followed by a digit (so a concept id starting with `v`, e.g.
/// `…ENTRY.valc_parent…`, is not mistaken for the version).
fn version_marker(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    (0..bytes.len().saturating_sub(2))
        .find(|&i| bytes[i] == b'.' && bytes[i + 1] == b'v' && bytes[i + 2].is_ascii_digit())
}

/// Validate an assembled [`Archetype`] against the AOM2 **phase-1** catalogue.
///
/// `repo`, when supplied, resolves the archetype's parent (and suppliers) so
/// the parent-dependent phase-1 checks (VACSD depth comparison, VASID, VALC,
/// VTPL) can run; when `None`, those checks compute only their standalone half
/// or are skipped. Source-level checks that need the raw ODIN text (VOKU) are
/// not run here — use [`validate_source_phase1`] for those.
#[must_use]
pub fn validate_phase1(
    archetype: &Archetype,
    repo: Option<&ArchetypeRepository>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    phase1::run(&view(archetype), repo, None, &mut issues);
    issues
}

/// Parse ADL2 `src` and validate it against the AOM2 **phase-1** catalogue,
/// including the source-level checks (VOKU keyed-list uniqueness) that need the
/// raw ODIN text.
///
/// # Errors
/// Returns the parse [`SyntaxError`]s if `src` does not parse into an
/// [`Archetype`]; validation runs only on a successful parse.
pub fn validate_source_phase1(
    src: &str,
    repo: Option<&ArchetypeRepository>,
) -> Result<Vec<ValidationIssue>, Vec<SyntaxError>> {
    let source = parse_source(src)?;
    let archetype = crate::assemble::assemble(&source, src)?;
    let mut issues = Vec::new();
    phase1::run(&view(&archetype), repo, Some((&source, src)), &mut issues);
    Ok(issues)
}

/// Validate an assembled [`Archetype`] against phase 1 and — only if phase 1
/// raised no [`Severity::Error`] — the phase-2 reference-model checks against
/// `rm`.
///
/// This is the `master08` §Overview phase gate ("more basic kinds of errors
/// being checked first"): the RM pass runs on a structurally-sound archetype
/// only. Source-level phase-1 checks (VOKU) are not included — use
/// [`validate_source`] for those.
#[must_use]
pub fn validate(
    archetype: &Archetype,
    repo: Option<&ArchetypeRepository>,
    rm: &dyn rm::RmModel,
) -> Vec<ValidationIssue> {
    let mut issues = validate_phase1(archetype, repo);
    if issues.iter().all(|i| i.severity != Severity::Error) {
        issues.extend(rm::validate_phase2_rm(archetype, rm));
    }
    run_phase2_spec(archetype, repo, rm, &mut issues);
    issues
}

/// Run the phase-2 specialisation checks against the resolved flat parent, gated
/// on a supplied repository and a still-clean issue list (`master08` §Overview
/// phase gate). A non-specialised archetype, an unresolved parent, or a parent
/// that itself needs the flattener silently skips the checks (never a wrong
/// answer against an un-flattened parent).
fn run_phase2_spec(
    archetype: &Archetype,
    repo: Option<&ArchetypeRepository>,
    rm: &dyn rm::RmModel,
    issues: &mut Vec<ValidationIssue>,
) {
    if issues.iter().any(|i| i.severity == Severity::Error) {
        return;
    }
    let Some(repo) = repo else {
        return;
    };
    if let FlatParent::Available(parent) = resolve_flat_parent(archetype, repo) {
        issues.extend(phase2::validate_phase2_spec(archetype, parent, rm, repo));
    }
}

/// Parse and validate ADL2 `src` against phase 1 (including the source-level
/// checks) and — only if phase 1 is error-free — the phase-2 reference-model
/// checks against `rm` (`master08` §Overview phase gate).
///
/// # Errors
/// Returns the parse [`SyntaxError`]s if `src` does not parse into an
/// [`Archetype`]; validation runs only on a successful parse.
pub fn validate_source(
    src: &str,
    repo: Option<&ArchetypeRepository>,
    rm: &dyn rm::RmModel,
) -> Result<Vec<ValidationIssue>, Vec<SyntaxError>> {
    let source = parse_source(src)?;
    let archetype = crate::assemble::assemble(&source, src)?;
    let mut issues = Vec::new();
    phase1::run(&view(&archetype), repo, Some((&source, src)), &mut issues);
    if issues.iter().all(|i| i.severity != Severity::Error) {
        issues.extend(rm::validate_phase2_rm(&archetype, rm));
    }
    run_phase2_spec(&archetype, repo, rm, &mut issues);
    Ok(issues)
}

/// A borrowed, artefact-kind-agnostic view of an [`Archetype`]'s common fields
/// — the single access point the checks read through.
pub(crate) struct ArchetypeView<'a> {
    pub(crate) kind: ArtefactKind,
    pub(crate) archetype_id: &'a ArchetypeHrid,
    pub(crate) parent_archetype_id: Option<&'a str>,
    pub(crate) definition: &'a CComplexObject,
    pub(crate) terminology: &'a ArchetypeTerminology,
    pub(crate) rm_overlay: Option<&'a RmOverlay>,
    pub(crate) original_language: Option<&'a TerminologyCode>,
    pub(crate) description: Option<&'a ResourceDescription>,
    pub(crate) translations:
        Option<&'a std::collections::BTreeMap<String, openehr_base::prelude::TranslationDetails>>,
    pub(crate) annotations: Option<&'a ResourceAnnotations>,
    pub(crate) adl_version: Option<&'a str>,
    pub(crate) rm_release: &'a str,
    pub(crate) is_differential: bool,
}

impl ArchetypeView<'_> {
    /// True if this archetype specialises a parent.
    pub(crate) fn is_specialised(&self) -> bool {
        self.parent_archetype_id.is_some()
    }

    /// The archetype's specialisation level = the specialisation depth of its
    /// root node id (`master07` §Specialisation Depth; VARCN).
    pub(crate) fn specialisation_level(&self) -> usize {
        crate::codes::specialisation_depth(crate::paths::complex_node_id(self.definition))
            .unwrap_or(0)
    }
}

/// Build an [`ArchetypeView`] over any [`Archetype`] variant.
pub(crate) fn view(archetype: &Archetype) -> ArchetypeView<'_> {
    match archetype {
        Archetype::AuthoredArchetype(a) => match a.as_ref() {
            AuthoredArchetype::AuthoredArchetype(d) => ArchetypeView {
                kind: ArtefactKind::Archetype,
                archetype_id: &d.archetype_id,
                parent_archetype_id: d.parent_archetype_id.as_deref(),
                definition: &d.definition,
                terminology: &d.terminology,
                rm_overlay: d.rm_overlay.as_ref(),
                original_language: Some(&d.original_language),
                description: d.description.as_deref(),
                translations: d.translations.as_ref(),
                annotations: d.annotations.as_ref(),
                adl_version: d.adl_version.as_deref(),
                rm_release: &d.rm_release,
                is_differential: d.is_differential,
            },
            AuthoredArchetype::Template(t) => ArchetypeView {
                kind: ArtefactKind::Template,
                archetype_id: &t.archetype_id,
                parent_archetype_id: t.parent_archetype_id.as_deref(),
                definition: &t.definition,
                terminology: &t.terminology,
                rm_overlay: t.rm_overlay.as_ref(),
                original_language: Some(&t.original_language),
                description: t.description.as_ref(),
                translations: t.translations.as_ref(),
                annotations: t.annotations.as_ref(),
                adl_version: t.adl_version.as_deref(),
                rm_release: &t.rm_release,
                is_differential: t.is_differential,
            },
            AuthoredArchetype::OperationalTemplate(o) => ArchetypeView {
                kind: ArtefactKind::OperationalTemplate,
                archetype_id: &o.archetype_id,
                parent_archetype_id: o.parent_archetype_id.as_deref(),
                definition: &o.definition,
                terminology: &o.terminology,
                rm_overlay: o.rm_overlay.as_ref(),
                original_language: Some(&o.original_language),
                description: o.description.as_ref(),
                translations: o.translations.as_ref(),
                annotations: o.annotations.as_ref(),
                adl_version: o.adl_version.as_deref(),
                rm_release: &o.rm_release,
                is_differential: o.is_differential,
            },
        },
        Archetype::TemplateOverlay(t) => ArchetypeView {
            kind: ArtefactKind::TemplateOverlay,
            archetype_id: &t.archetype_id,
            parent_archetype_id: t.parent_archetype_id.as_deref(),
            definition: &t.definition,
            terminology: &t.terminology,
            rm_overlay: t.rm_overlay.as_ref(),
            original_language: None,
            description: None,
            translations: None,
            annotations: None,
            adl_version: None,
            rm_release: "",
            is_differential: t.is_differential,
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_code_has_a_unique_mnemonic_and_severity() {
        let all = [
            ValidationCode::Vardt,
            ValidationCode::Varcn,
            ValidationCode::Stcnt,
            ValidationCode::Vacsd,
            ValidationCode::Volt,
            ValidationCode::Varav,
            ValidationCode::Varrv,
            ValidationCode::Votm,
            ValidationCode::Vdifv,
            ValidationCode::Vdifp,
            ValidationCode::Vcorm,
            ValidationCode::Vcormt,
            ValidationCode::Vcarm,
            ValidationCode::Vcaex,
            ValidationCode::Vcaca,
            ValidationCode::Vcam,
            ValidationCode::Vatcv,
            ValidationCode::Vtsd,
            ValidationCode::Vtlc,
            ValidationCode::Vttbk,
            ValidationCode::Vtcbk,
            ValidationCode::Vetdf,
            ValidationCode::Vtvsid,
            ValidationCode::Vtvsmd,
            ValidationCode::Vtvsuq,
            ValidationCode::Vdsev,
            ValidationCode::Vdsiv,
            ValidationCode::Varxra,
            ValidationCode::Varxnc,
            ValidationCode::Varxav,
            ValidationCode::Varxr,
            ValidationCode::Varxtv,
            ValidationCode::Vatid,
            ValidationCode::Vatcd,
            ValidationCode::Vatdf,
            ValidationCode::Vacdf,
            ValidationCode::Vatda,
            ValidationCode::Vranp,
            ValidationCode::Voku,
            ValidationCode::Varid,
            ValidationCode::Vdeol,
            ValidationCode::Vard,
            ValidationCode::Vasid,
            ValidationCode::Valc,
            ValidationCode::Vtpl,
            ValidationCode::Vrrlp,
            ValidationCode::Vcocd,
            ValidationCode::Vcoid,
            ValidationCode::Vcosu,
            ValidationCode::Vcatu,
            ValidationCode::Vdfai,
            ValidationCode::Vobav,
            ValidationCode::Vrmvp,
            ValidationCode::Vrmvav,
            ValidationCode::Vacso,
            ValidationCode::Vacmcu,
            ValidationCode::Vsonif,
            ValidationCode::Vrdla,
            ValidationCode::Wacmcl,
            ValidationCode::Wouc,
            ValidationCode::Vsance,
            ValidationCode::Vsancc,
            ValidationCode::Vsam,
            ValidationCode::Vsonin,
            ValidationCode::Vssm,
            ValidationCode::Vsont,
            ValidationCode::Vsonct,
            ValidationCode::Vsonco,
            ValidationCode::Vsonpt,
            ValidationCode::Vsonpi,
            ValidationCode::Vsonpo,
            ValidationCode::Vsoni,
            ValidationCode::Vsonir,
            ValidationCode::Vsunt,
            ValidationCode::Vunt,
            ValidationCode::Vdssid,
            ValidationCode::Vdssm,
            ValidationCode::Vdssp,
            ValidationCode::Vdssc,
            ValidationCode::Varxs,
            ValidationCode::Varxid,
            ValidationCode::Vpov,
            ValidationCode::Vunk,
            ValidationCode::Vtpnc,
            ValidationCode::Vtpin,
        ];
        let mut seen = std::collections::HashSet::new();
        for c in all {
            assert!(seen.insert(c.mnemonic()), "duplicate mnemonic {c}");
            let expected = if c.mnemonic().starts_with('W') {
                Severity::Warning
            } else {
                Severity::Error
            };
            assert_eq!(c.severity(), expected, "{c} severity");
        }
        assert_eq!(seen.len(), 85);
    }
}

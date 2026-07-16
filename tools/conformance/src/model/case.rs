//! The case model: [`CaseMeta`] and its classification vocabulary.
//!
//! Every case is one of **our own** ECC cases. The metadata is what the
//! report, Statement, and Certificate are scoped from — the claim is a
//! function of the run, never hand-asserted. Two W-10 additions make the
//! derivation square of `CNF/docs/guide/master04-framework.adoc` §From
//! Specifications to Runnable Tests machine-readable on every case:
//!
//! - [`ScheduleTrace`] — the abstract schedule test case this case
//! concretizes (or an explicit `EccOriginal` marker where the schedule
//! chapter is a stub — a stub-derived case is never presented as
//! schedule-conformant; registers 02/07/08/09/10).
//! - [`Binding`] — the ITS-REST concretization, or the explicit
//! `NoRestBinding` fact for SM operations the REST contract never bound
//! (schedule master04 `delete_opt`, master05 `list_queries`, master08
//! `list_contributions`, Messaging, native-only Admin ops): those cases
//! skip-with-reason or probe, they never fabricate a URL and never book a
//! failure.

use serde::Serialize;

use crate::model::catalog::Area;

/// Static metadata for one conformance test case.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CaseMeta {
    /// The registration key: a stable, descriptive slug
    /// `<area-slug>/<case-slug>`, lowercase kebab-case, e.g.
    /// `"ehr/create-default-sets"`. Bound to an ECC number in the committed
    /// catalogue; keys carried from the pre-W-10 instrument keep their
    /// numbers so the baseline delta stays per-case explainable.
    pub id: &'static str,
    /// The human title.
    pub title: &'static str,
    /// The catalogue area (the category axis of the ECC id).
    pub area: Area,
    /// The capability the case exercises (profiles master03 matrix).
    pub capability: Capability,
    /// The wire formats the case runs under (profiles master03 §Other
    /// Non-Functional: external data formats are XML + JSON; a case runs
    /// once per claimed format where the payload is format-sensitive).
    pub formats: &'static [Format],
    /// The spec grounding: CNF schedule file + § plus the ITS-REST/RM
    /// sections the assertion enforces (spec citations only — never ADRs).
    pub citation: &'static str,
    /// The schedule trace (the abstract test case, or the honest
    /// ECC-original marker).
    pub schedule: ScheduleTrace,
    /// The ITS-REST binding this case drives.
    pub binding: Binding,
    /// The payload comparison mode the case's content assertions use.
    pub compare: Compare,
}

/// The abstract-schedule trace of a case — the (3)→(4) edge of the guide's
/// derivation square (`guide/master04-framework.adoc` §From Specifications to
/// Runnable Tests).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum ScheduleTrace {
    /// The case concretizes a schedule test case: the official
    /// `<SERVICE_COMPONENT>.<operation>-<id>` form + chapter locus, e.g.
    /// `"I_EHR_SERVICE.create_ehr-no_status (master06 §create_ehr)"`.
    Schedule(&'static str),
    /// The case is ECC-original: no normative schedule backing exists. The
    /// string states why, e.g. `"schedule stub (master11 is TBD); derived
    /// from AQL 1.1 + golden corpus"` or `"extension: item tags"`.
    EccOriginal(&'static str),
}

/// The ITS-REST concretization of the case — the (1)→(2) edge of the
/// derivation square.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Binding {
    /// The case drives a bound ITS-REST resource; the string names the
    /// operation(s), e.g. `"POST /ehr"` or `"PUT /ehr/{ehr_id}/composition/{uid}"`.
    Rest(&'static str),
    /// The SM operation has **no** ITS-REST binding (the schedule is
    /// SM-based and wider than the REST contract). The string cites the SM
    /// operation; the case must resolve to a documented skip or a
    /// negative-space probe (405/404-when-unbound), never a fabricated URL.
    NoRestBinding(&'static str),
    /// Native-API-only capability (e.g. Messaging): evidenced off-wire by
    /// named integration tests; always skip-with-reason on the wire runner.
    NativeApiOnly(&'static str),
}

/// A conformance profile (`profiles/master03-profiles.adoc`): claims are made
/// per profile, composed of capabilities. CORE/STANDARD require **all**
/// mentioned capabilities; OPTIONS is obtained if **any** optional capability
/// passes (per-capability reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// CORE — minimal functional openEHR platform (storage + retrieval).
    Core,
    /// STANDARD — CORE plus AQL querying and logging.
    Standard,
    /// OPTIONS — optional capabilities, reported individually (any-of).
    Options,
}

/// A capability — the unit a profile requires. Names track
/// `profiles/master03-profiles.adoc` (functional + non-functional tables);
/// the profile membership lives in [`crate::model::profile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// EHR create/get/has operations.
    EhrOperations,
    /// `EHR_STATUS` get/set operations.
    EhrStatus,
    /// COMPOSITION create/update/delete/get operations.
    CompositionOps,
    /// CONTRIBUTION change-set commits (profiles "Change sets").
    ChangeSets,
    /// Version reads: history, at-time, by-id.
    Versioning,
    /// Validation of committed data against semantic models (profiles
    /// "Archetype Validation"; the master15–17 content suites).
    ArchetypeValidation,
    /// ADL 1.4 archetype provisioning.
    Adl14ArchetypeProvisioning,
    /// ADL 1.4 OPT provisioning.
    Adl14OptProvisioning,
    /// ADL 2 archetype + OPT provisioning (OPTIONS).
    Adl2Provisioning,
    /// Stored-query provisioning (STANDARD "Query provisioning").
    QueryProvisioning,
    /// DIRECTORY (FOLDER) operations (STANDARD).
    DirectoryOps,
    /// Basic AQL execution (STANDARD "AQL basic").
    AqlBasic,
    /// Advanced AQL (OPTIONS).
    AqlAdvanced,
    /// AQL & terminology (OPTIONS).
    AqlTerminology,
    /// Demographic Party operations (OPTIONS).
    PartyOperations,
    /// Demographic Party-Relationship operations (OPTIONS).
    PartyRelationshipOperations,
    /// Admin — Activity Report (OPTIONS).
    AdminActivityReport,
    /// Admin — Physical Deletion (OPTIONS).
    AdminPhysicalDeletion,
    /// Admin — EHR Dump/Load (OPTIONS).
    AdminEhrDumpLoad,
    /// Admin — Bulk EHR load (OPTIONS).
    AdminBulkEhrLoad,
    /// Admin — EHR Archive (OPTIONS).
    AdminEhrArchive,
    /// Admin — Demographic Archive (OPTIONS).
    AdminDemographicArchive,
    /// Messaging — EHR Extract (OPTIONS; native-API-only on the wire).
    MessagingEhrExtract,
    /// Messaging — TDS/TDD (OPTIONS; native-API-only on the wire).
    MessagingTds,
    /// Version signing (non-functional, STANDARD).
    Signing,
    /// Anonymous (subject-less) EHRs (non-functional, CORE).
    AnonymousEhrs,
    /// Authentication enforcement (out-of-band per the SM; reported, never
    /// profile-gating).
    Authentication,
    /// Terminology-server integration (ehrbase-rs extension + generic
    /// FHIR-tx cases; reported, never profile-gating).
    Terminology,
}

/// A wire format a case runs under (profiles master03 §Other Non-Functional).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// Canonical openEHR JSON.
    Json,
    /// Canonical openEHR XML.
    Xml,
}

impl Format {
    /// The IANA media type for this format's canonical openEHR payloads.
    #[must_use]
    pub const fn media_type(self) -> &'static str {
        match self {
            Format::Json => "application/json",
            Format::Xml => "application/xml",
        }
    }
}

/// The payload comparison mode a case's content assertions use (the
/// retrieved-equals-committed checks the schedule mandates on every read —
/// register 04 G-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compare {
    /// The response must equal the expected payload exactly.
    Exact,
    /// The response may carry more than the expected payload (subset match).
    Superset,
    /// Diff ignoring the declared server-assigned ignore-set (uids, audit
    /// timestamps, `_type` defaults) — the mode content round-trips use.
    IgnoreSet,
    /// No content comparison (status/header-only case).
    None,
}

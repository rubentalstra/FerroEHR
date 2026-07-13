//! The case model (design §4.2): [`CaseMeta`] and the classification enums.
//!
//! Every case is one of **our own** ECC cases: it carries its ECC registration
//! key ([`CaseMeta::id`] — a descriptive `<area>/<case>` slug), a human
//! [`CaseMeta::title`], its catalogue [`Area`], the capability it exercises
//! ([`Capability`]), which profiles require that capability ([`Profile`]), the
//! wire formats it runs under ([`Format`]), a spec [`CaseMeta::citation`], and
//! the payload comparison mode ([`Compare`]). This metadata is what the
//! generated report and Conformance Statement are scoped from — the claim is a
//! function of the run, never hand-asserted. There is no runtime mapping to the
//! legacy CNF corpus: the vendored corpus was design-time reading only.

use serde::Serialize;

use crate::catalog::Area;

/// Static metadata for one conformance test case (design §4.2).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CaseMeta {
    /// The registration key: a stable, descriptive slug `<area-slug>/<case-slug>`,
    /// lowercase kebab-case, e.g. `"ehr/create-default-sets"`, `"val/dv-count-range"`.
    pub id: &'static str,
    /// The human title shown in `CATALOG.md`, e.g.
    /// `"Create EHR with default EHR_STATUS"`.
    pub title: &'static str,
    /// The catalogue area (explicit — the category axis of the ECC id).
    pub area: Area,
    /// The capability the case exercises (design §8 matrix).
    pub capability: Capability,
    /// Which profiles require this capability.
    pub profiles: &'static [Profile],
    /// The wire formats the case runs under (a case runs once per claimed format
    /// where the payload is format-sensitive).
    pub formats: &'static [Format],
    /// The spec grounding: file/section citation(s), e.g.
    /// `"ITS-REST 1.0.3 EHR §create_ehr; RM 1.2.0 ehr §EHR_STATUS"`.
    pub citation: &'static str,
    /// The payload comparison mode this case's assertion uses (jsonlib semantics,
    /// §2.2a).
    pub compare: Compare,
    /// An optional CNF-schedule trace reference: the official
    /// `<SERVICE_COMPONENT>.<operation>` id (with its schedule locus) this case
    /// directly concretizes, e.g.
    /// `"I_EHR_DIRECTORY.get_versioned_directory (CNF master09:670)"`. `None`
    /// where the case maps to no single CNF schedule test-case id (most
    /// ECC-original cases). This is the field the catalogue's "trace references
    /// carried in metadata" claim needs to be literally true (R2 caveat,
    /// `docs/blueprint/07-cnf.md`); set it best-effort where a module already
    /// cites a `master NN` schedule id, `None` otherwise. Threaded onto a case
    /// with [`crate::registry::CaseEntry::with_schedule_ref`].
    pub schedule_ref: Option<&'static str>,
}

/// A conformance profile (design §2.1, master03-profiles): claims are made per
/// profile, composed of capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// CORE — the all-or-nothing baseline (ADL1.4 + OPT provisioning, EHR /
    /// `EHR_STATUS` / COMPOSITION / change sets / versioning / archetype
    /// validation, anonymous EHRs).
    Core,
    /// STANDARD — CORE plus query provisioning, directory, AQL basic, and
    /// Signing.
    Standard,
    /// OPTIONS — any optional capability, reported individually.
    Options,
}

/// A capability (design §2.1): the unit a profile requires. Names track the
/// master03 profiles document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// EHR create/get/has operations.
    EhrOperations,
    /// `EHR_STATUS` get/update operations.
    EhrStatus,
    /// COMPOSITION create/update/delete/get operations.
    CompositionOps,
    /// CONTRIBUTION change-set commits.
    ChangeSets,
    /// Version reads: history, at-time, by-id, `ALL_VERSIONS`.
    Versioning,
    /// Archetype (RM invariant + terminology) validation of committed content.
    ArchetypeValidation,
    /// ADL 1.4 archetype provisioning.
    Adl14ArchetypeProvisioning,
    /// OPT 1.4 operational-template provisioning.
    Adl14OptProvisioning,
    /// Stored-query (AQL) provisioning.
    QueryProvisioning,
    /// DIRECTORY (FOLDER) operations.
    DirectoryOps,
    /// Basic AQL execution.
    AqlBasic,
    /// Version signing (STANDARD; runner-defined SIGN-* cases, §4.6).
    Signing,
    /// Anonymous (subject-less) EHRs.
    AnonymousEhrs,
    /// ADMIN API (OPTIONS) — the ITS-REST admin REST surface (physical delete),
    /// `master03-profiles.adoc` §Functional/REST APIs.
    AdminApi,
    /// DEMOGRAPHIC API (OPTIONS).
    DemographicApi,
    /// ADL 2 archetype + OPT provisioning (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Definitions. Modeled for the full OPTIONS surface (D4); ADL 2
    /// is not implemented, so it reports **not evidenced** until dedicated cases
    /// exist.
    Adl2Provisioning,
    /// AQL advanced querying (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Querying. STANDARD covers [`Capability::AqlBasic`]; the
    /// advanced surface is modeled here for the OPTIONS report (D4) and reports
    /// **not evidenced** until advanced-AQL cases are tagged to it.
    AqlAdvanced,
    /// Admin — Activity Report (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Admin. Modeled for the OPTIONS surface (D4); unimplemented →
    /// not evidenced.
    AdminActivityReport,
    /// Admin — Physical Deletion (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Admin. The physical cascade delete is exercised over the
    /// ADMIN API and tagged [`Capability::AdminApi`]; this functional
    /// sub-capability is modeled for the OPTIONS surface (D4) and reports
    /// not-evidenced-directly (the evidence rides `AdminApi`).
    AdminPhysicalDeletion,
    /// Admin — EHR Dump/Load (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Admin. Modeled (D4); unimplemented → not evidenced.
    AdminEhrDumpLoad,
    /// Admin — Bulk EHR load (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Admin. Modeled (D4); unimplemented → not evidenced.
    AdminBulkEhrLoad,
    /// Admin — EHR Archive (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Admin. Modeled (D4); unimplemented → not evidenced.
    AdminEhrArchive,
    /// Admin — Demographic Archive (OPTIONS) — `master03-profiles.adoc`
    /// §Functional/Admin. Modeled (D4); unimplemented → not evidenced.
    AdminDemographicArchive,
    /// Messaging — EHR Extract export/import + TDD import (OPTIONS). SM-5
    /// realizes it as a **native-API-only** capability: openEHR Messaging is an
    /// OPTIONS-profile feature with no ITS-REST 1.0.3 binding, so the
    /// HTTP-driven ECC cannot exercise it over the wire and its cases report
    /// `SKIPPED(NativeApiOnly)`, citing the `ehrbase` integration tests that do
    /// exercise it. It is therefore *not* in [`crate::profile::required_capabilities`]
    /// (a wire-tested SUT must not be denied OPTIONS for a capability the wire
    /// cannot reach) — it is reported individually.
    Messaging,
    /// Terminology-server integration (OPTIONS) — the AQL `TERMINOLOGY('expand',
    /// …)` family (B4). The in-process `openehr-term` bundle
    /// (`service_api = "openehr"`) is wire-exercisable and its cases pass against
    /// any SUT; the external FHIR-tx cases depend on the SUT carrying a
    /// configured FHIR terminology provider and report `SKIPPED(SutConfig)`
    /// otherwise. Like [`Capability::Messaging`] it is *not* in
    /// [`crate::profile::required_capabilities`] (an optional, partly
    /// config-gated capability is reported individually, never blocking a
    /// profile).
    Terminology,
    /// Authentication / authorization enforcement (`SEC` area) — the auth surface
    /// mirrored from the CNF `SECURITY_TESTS/I_OAuth2_Keycloak` Robot suite
    /// (unauthenticated access is refused; role-privileged routes distinguish
    /// roles). openEHR models security as **Non-Functional** attributes (Signing,
    /// Anonymous EHRs) in `master03-profiles.adoc`, *not* as a gated
    /// profile-Functional capability — so like [`Capability::Messaging`] /
    /// [`Capability::Terminology`], `Authentication` is **not** in
    /// [`crate::profile::required_capabilities`] and never blocks a profile; its
    /// `SEC` cases are reported in the area matrix + catalogue and skip-with-reason
    /// when the SUT's auth mode cannot be determined from the wire.
    Authentication,
}

/// A wire format a case runs under.
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

/// The payload comparison mode a case's assertion uses — the upstream `jsonlib`
/// semantics (design §2.2a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compare {
    /// The response must equal the expected payload exactly.
    Exact,
    /// The response may carry more than the expected payload (a subset match).
    Superset,
    /// Diff ignoring the RM `_type`/metadata/path ignore-set.
    IgnoreSet,
}

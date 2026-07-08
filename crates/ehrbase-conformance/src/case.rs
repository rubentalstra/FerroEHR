//! The case model (design §4.2): [`CaseMeta`] and the classification enums.
//!
//! Every identified case in the openEHR Platform Conformance Test Schedule maps
//! to exactly one [`CaseMeta`] — its book provenance ([`Chapter`]), the
//! capability it exercises ([`Capability`]), which profiles require that
//! capability ([`Profile`]), the wire formats it runs under ([`Format`]), where
//! the assertion authority comes from ([`Provenance`]), the exact schedule
//! reference, the upstream Robot-suite stability tags, and the payload
//! comparison mode ([`Compare`]). This metadata is what the generated report and
//! Conformance Statement are scoped from — the claim is a function of the run,
//! never hand-asserted.

use serde::Serialize;

/// Static metadata for one conformance test case (design §4.2).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CaseMeta {
    /// The schedule's own case id, e.g. `"I_EHR_SERVICE.create_ehr-main"` or
    /// `"CONT-DV_ORDINAL-validate_open"`.
    pub id: &'static str,
    /// The schedule chapter (book) the case is defined in.
    pub chapter: Chapter,
    /// The capability the case exercises (from the CNF profiles doc, master03).
    pub capability: Capability,
    /// Which profiles require this capability.
    pub profiles: &'static [Profile],
    /// The wire formats the case runs under (a case runs once per claimed format
    /// where the payload is format-sensitive).
    pub formats: &'static [Format],
    /// Where the case's assertion authority comes from.
    pub provenance: Provenance,
    /// The exact schedule reference, e.g.
    /// `"master06-func_tc_ehr.adoc §Test Case I_EHR_SERVICE.create_ehr-main"`.
    pub schedule_ref: &'static str,
    /// The upstream Robot-suite tags where one exists (`"future"`, `"not-ready"`,
    /// …) — upstream's own stability signal, reportable and filterable (§2.2a).
    pub upstream_tags: &'static [&'static str],
    /// The payload comparison mode this case's assertion uses (jsonlib semantics,
    /// §2.2a).
    pub compare: Compare,
}

/// A schedule chapter (book). Each variant maps 1:1 to a source `.adoc` file
/// under `docs/specs/openehr/CNF/docs/platform_test_schedule/`.
///
/// There is deliberately no `Master14`: the schedule skips it (design §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Chapter {
    /// master04 — DEFINITION: ADL 1.4 archetype + OPT provisioning.
    Master04,
    /// master05 — DEFINITION: stored query provisioning.
    Master05,
    /// master06 — EHR + `EHR_STATUS` operations.
    Master06,
    /// master07 — COMPOSITION operations.
    Master07,
    /// master08 — CONTRIBUTION / change sets.
    Master08,
    /// master09 — DIRECTORY (FOLDER) operations.
    Master09,
    /// master10 — DEMOGRAPHIC API (OPTIONS).
    Master10,
    /// master11 — QUERY (AQL) — mostly TBD stubs upstream.
    Master11,
    /// master12 — ADMIN API (OPTIONS).
    Master12,
    /// master13 — MESSAGING (OPTIONS, not implemented).
    Master13,
    /// master15 — content: COMPOSITION validation truth tables.
    Master15,
    /// master16 — content: ENTRY validation truth tables.
    Master16,
    /// master17.1 — content: data types — basic.
    Master17_1,
    /// master17.2 — content: data types — text.
    Master17_2,
    /// master17.3 — content: data types — quantity.
    Master17_3,
    /// master17.4 — content: data types — date/time.
    Master17_4,
    /// master17.5 — content: data types — time specification (empty upstream).
    Master17_5,
    /// master17.6 — content: data types — encapsulated.
    Master17_6,
    /// master17.7 — content: data types — URI.
    Master17_7,
    /// A non-schedule pseudo-chapter for the runner-defined `SIGN-*` capability
    /// cases (design §4.6): upstream ships zero Signing test material, so these
    /// sit outside the 322-case schedule inventory. Deliberately **absent from
    /// [`Chapter::ALL`]** (which is the schedule chapters only) — it maps to no
    /// `.adoc` file and never appears in the parsed inventory.
    Signing,
}

impl Chapter {
    /// Every chapter, in schedule order.
    pub const ALL: [Chapter; 19] = [
        Chapter::Master04,
        Chapter::Master05,
        Chapter::Master06,
        Chapter::Master07,
        Chapter::Master08,
        Chapter::Master09,
        Chapter::Master10,
        Chapter::Master11,
        Chapter::Master12,
        Chapter::Master13,
        Chapter::Master15,
        Chapter::Master16,
        Chapter::Master17_1,
        Chapter::Master17_2,
        Chapter::Master17_3,
        Chapter::Master17_4,
        Chapter::Master17_5,
        Chapter::Master17_6,
        Chapter::Master17_7,
    ];

    /// The source `.adoc` file name (with extension) this chapter is defined in.
    #[must_use]
    pub const fn source_file(self) -> &'static str {
        match self {
            Chapter::Master04 => "master04-func_tc_definition_adl.adoc",
            Chapter::Master05 => "master05-func_tc_definition_query.adoc",
            Chapter::Master06 => "master06-func_tc_ehr.adoc",
            Chapter::Master07 => "master07-func_tc_ehr_composition.adoc",
            Chapter::Master08 => "master08-func_tc_ehr_contribution.adoc",
            Chapter::Master09 => "master09-func_tc_ehr_directory.adoc",
            Chapter::Master10 => "master10-func_tc_demographic.adoc",
            Chapter::Master11 => "master11-func_tc_querying.adoc",
            Chapter::Master12 => "master12-func_tc_admin.adoc",
            Chapter::Master13 => "master13-func_tc_messaging.adoc",
            Chapter::Master15 => "master15-content_tc_composition.adoc",
            Chapter::Master16 => "master16-content_tc_entry.adoc",
            Chapter::Master17_1 => "master17.1-content_tc_data_types-basic.adoc",
            Chapter::Master17_2 => "master17.2-content_tc_data_types-text.adoc",
            Chapter::Master17_3 => "master17.3-content_tc_data_types-quantity.adoc",
            Chapter::Master17_4 => "master17.4-content_tc_data_types-date_time.adoc",
            Chapter::Master17_5 => "master17.5-content_tc_data_types-time_specification.adoc",
            Chapter::Master17_6 => "master17.6-content_tc_data_types-encapsulated.adoc",
            Chapter::Master17_7 => "master17.7-content_tc_data_types-uri.adoc",
            // A sentinel that matches no vendored `.adoc`, so `from_source_file`
            // never yields this pseudo-chapter (runner-defined SIGN-* cases only).
            Chapter::Signing => "__runner_defined__signing",
        }
    }

    /// The chapter whose [`Chapter::source_file`] equals `file`, if any. Files
    /// that are not a test-schedule chapter (`master00`–`master03`, the manifest)
    /// map to `None`.
    #[must_use]
    pub fn from_source_file(file: &str) -> Option<Chapter> {
        Chapter::ALL.into_iter().find(|c| c.source_file() == file)
    }

    /// A short, stable label for reports and synthesized ids (the file stem up to
    /// the first `-`), e.g. `"master06"`, `"master17.3"`.
    #[must_use]
    pub fn label(self) -> &'static str {
        if matches!(self, Chapter::Signing) {
            return "signing";
        }
        // The prefix before the first '-' of the source file name.
        let file = self.source_file();
        match file.split_once('-') {
            Some((stem, _)) => stem,
            None => file,
        }
    }
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
    /// ADMIN API (OPTIONS).
    AdminApi,
    /// DEMOGRAPHIC API (OPTIONS).
    DemographicApi,
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

/// Where a case's assertion authority comes from (design §3.4, §4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Transcribed directly from a normative schedule test case.
    Schedule,
    /// Derived from the vendored fixture corpus where the schedule chapter is a
    /// TBD stub (e.g. the `QUERY-FIXTURE-*` cases).
    FixtureDerived,
    /// Defined by the runner against implemented behaviour where upstream ships
    /// no test material (the `SIGN-*` cases).
    RunnerDefined,
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

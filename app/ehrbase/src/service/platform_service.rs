//! `PLATFORM_SERVICE` (`master03-common_package.adoc` §Overview: "an
//! enumeration of the available services, used in various interfaces";
//! `platform_service.adoc`).

/// `PLATFORM_SERVICE` — "Enumeration of platform service names"
/// (`platform_service.adoc`).
///
/// The ADMIN statistics calls (`i_admin_service.adoc` `list_contributions` /
/// `contribution_count` / `versioned_composition_count` /
/// `composition_version_count`) each take a `PLATFORM_SERVICE` naming the
/// versioned-content service whose contributions/versions to count.
///
/// NOTE (spec defect): `platform_service.adoc` is named by the master03
/// §Overview but **not** `include::`d in its §Class Definitions, and its
/// enumeration lists exactly these eight members, omitting `Terminology` and
/// `Subject_proxy` (the SM defines both interfaces but the enum forgot their
/// members). This type carries the eight vendored members verbatim; the two
/// missing services are not versioned-content services and would count zero
/// regardless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformService {
    /// `Admin` — the administrative service.
    Admin,
    /// `Definitions` — the DEFINITION (templates/archetypes/queries) service.
    Definitions,
    /// `Ehr` — the EHR (clinical, EHR-scoped) service.
    Ehr,
    /// `Ehr_index` — the EHR Index (subject↔EHR) service.
    EhrIndex,
    /// `Demographic` — the demographic (ehr-less party) service.
    Demographic,
    /// `Message` — the messaging service.
    Message,
    /// `Query` — the querying (AQL) service.
    Query,
    /// `System_log` — the system-log service.
    SystemLog,
}

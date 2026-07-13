//! Identity law for archetype & template identifiers.
//!
//! Spec:
//! `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
//!
//! - §Archetype Identifiers — `ARCHETYPE_ID` is a multi-axial *immutable*
//!   identifier (`rm_originator-rm_name-rm_entity.domain_concept.vN`); the
//!   version segment is part of identity, so two versions are two distinct
//!   archetypes.
//! - `TEMPLATE_ID` (§Class Descriptions) — "Identifier for templates. Lexical
//!   form to be determined." The generated [`openehr_base::prelude::TemplateId`]
//!   carries only the opaque `value`; we therefore treat a `TEMPLATE_ID` as an
//!   opaque composite identifier governed by the case rule below, and do **not**
//!   attempt a multi-axial decomposition the spec has not yet fixed.
//! - **§Composite Identifiers and Case** — the case law that governs G-T04:
//!   composite identifiers are *case-preserving* (stored verbatim) **and**
//!   *case-insensitive* (two ids differing only in case are the **same** id).
//!
//! Coordination note: the SM `I_DEFINITION_ADL14` provisioning surface
//! (`service/definition/adl14.rs`) enforces the same rule at its SQL boundary
//! with `lower(<column>) = lower($1)` (its G-05-14). This module is the
//! in-process side of the identical law: [`canonical_key`] is the comparison
//! form used for the derived-runtime cache key, so case variants of one stored
//! template resolve to a single cache entry — while the persisted `template_id`
//! stays case-preserved. Storage lookups likewise compare case-insensitively in
//! SQL (see [`crate::templates::store`]).

/// The §Composite Identifiers and Case *comparison* form of an identifier: the
/// value with ASCII case folded away. Two ids are the same identifier iff their
/// canonical keys are equal.
///
/// Case-**preserving**: this is only the comparison/keying form — the original
/// string is what is stored and returned on the wire. Archetype/template ids are
/// ASCII by grammar (§Archetype Identifiers), so ASCII case folding is exact and
/// matches `PostgreSQL` `lower()` on the same values.
#[must_use]
pub(crate) fn canonical_key(id: &str) -> String {
    id.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::canonical_key;

    /// Two identifiers denote the *same* id under §Composite Identifiers and
    /// Case iff their canonical keys are equal (the comparison the store SQL
    /// boundary and the runtime cache key both apply — G-T04).
    fn same_id(a: &str, b: &str) -> bool {
        canonical_key(a) == canonical_key(b)
    }

    #[test]
    fn case_insensitive_equality() {
        // §Composite Identifiers and Case: differ only in case → same id.
        assert!(same_id(
            "openEHR-EHR-COMPOSITION.encounter.v1",
            "OPENEHR-EHR-COMPOSITION.ENCOUNTER.V1",
        ));
        assert!(same_id("Vital signs.v1", "vital signs.v1"));
    }

    #[test]
    fn distinct_ids_stay_distinct() {
        // The version segment is part of identity (§Archetype Identifiers).
        assert!(!same_id("diagnosis.v1", "diagnosis.v2"));
        assert!(!same_id("encounter.v1", "vitals.v1"));
    }

    #[test]
    fn canonical_key_folds_case_and_trims() {
        assert_eq!(canonical_key("  Encounter.V1 "), "encounter.v1");
        // Idempotent — canonicalising a canonical key is a no-op.
        let once = canonical_key("MixedCase.V2");
        assert_eq!(canonical_key(&once), once);
    }
}

// SPDX-FileCopyrightText: FerroEHR contributors
// SPDX-License-Identifier: MIT

//! Which RM `_type`s become their own `node` row, and archetype-id part
//! extraction for the promoted subsumption columns.
//!
//! No openEHR spec governs the decomposition granularity — it is our own
//! storage design. The *composition-content*
//! structure set is NOT hand-maintained here: it is delegated to the single
//! BMM-generated oracle [`openehr_rm::v1_2::model::is_structure_root`], which the
//! codegen keeps in lockstep with this codec — never a local duplicate
//! constant. The only local addition is the five demographic **party roots**,
//! which are versioned objects of their own but are deliberately outside the
//! composition-content set (the RM model excludes the demographic LOCATABLE
//! hierarchy, since a party is never composition content).

use openehr_base::prelude::ArchetypeId;

/// The five concrete demographic **party roots** (RM demographic). Each is a
/// standalone versioned object that reuses the `node`/`vo_version` machinery
/// (with a NULL `ehr_id`), so it must be accepted as a decomposition root — yet
/// it is intentionally NOT part of the composition-content structure set the RM
/// model tracks. The demographic *container* classes nested inside a party
/// (`PARTY_IDENTITY`, `CONTACT`, `ADDRESS`, `CAPABILITY`, `PARTY_RELATIONSHIP`)
/// are NOT structure types: they are reached only through non-structure array
/// attributes (`identities`, `contacts`, `relationships`, `capabilities`) and
/// stay inline verbatim in the party's fragment (see the codec's
/// `prune_children`), which is lossless and needs no per-container row.
/// (The delta is pinned against [`crate::versioning::Kind`] by
/// `tests::demographic_party_roots_mirror_the_versioning_kinds` — the
/// versioned-object domain is the owner of "which RM types are party roots";
/// this list only records which of them the node codec also splits into rows.)
const DEMOGRAPHIC_PARTY_ROOTS: [&str; 5] = ["PERSON", "ORGANISATION", "GROUP", "AGENT", "ROLE"];

/// Whether an RM `_type` gets its own `node` row: the BMM-generated
/// composition-content structure set, plus the demographic party roots.
#[must_use]
pub fn is_structure_type(rm_type: &str) -> bool {
    openehr_rm::v1_2::model::is_structure_root(rm_type)
        || DEMOGRAPHIC_PARTY_ROOTS.contains(&rm_type)
}

/// Whether an RM `_type` may be the **root** of a versioned object handed to
/// [`crate::storage::codec::decompose`].
///
/// This is [`is_structure_type`] plus `PARTY_RELATIONSHIP`: a relationship is
/// a standalone versioned object with its own `node`/`vo_version` rows, yet
/// it is deliberately **not** a structure type for child-pruning purposes — a
/// `PARTY_RELATIONSHIP` nested inside a party's `relationships` attribute
/// must stay inline. Splitting the two predicates gives both behaviours from
/// one codec.
#[must_use]
pub fn is_versioned_root_type(rm_type: &str) -> bool {
    is_structure_type(rm_type) || rm_type == crate::versioning::Kind::PartyRelationship.as_str()
}

/// Parses a full archetype HRID `archetype_node_id` into its identifying parts.
///
/// Yields the `(qualified_rm_entity, domain_concept, major)` parts, lowercased
/// for case-insensitive comparison (BASE `base_types` master05 §Archetype
/// Identifiers and §"Composite Identifiers and Case"). Reuses the shared
/// [`ArchetypeId`] parser (never a hand-rolled regex); returns `None` for
/// at/id-codes and any value that is not a full HRID with a numeric major.
#[must_use]
pub fn archetype_parts(node_id: &str) -> Option<(String, String, i32)> {
    let id: ArchetypeId = node_id.parse().ok()?;
    let major: i32 = id.major_version().parse().ok()?;
    Some((
        id.qualified_rm_entity().to_ascii_lowercase(),
        id.domain_concept().to_ascii_lowercase(),
        major,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The party-root delta is exactly the party kinds of the versioned-object
    /// domain — a kind added to [`crate::versioning::Kind`] without a row here
    /// fails this test.
    #[test]
    fn demographic_party_roots_mirror_the_versioning_kinds() {
        let mut from_kinds: Vec<&str> = crate::versioning::Kind::ALL
            .iter()
            .filter(|k| k.is_party())
            .map(|k| k.as_str())
            .collect();
        from_kinds.sort_unstable();
        let mut listed: Vec<&str> = DEMOGRAPHIC_PARTY_ROOTS.to_vec();
        listed.sort_unstable();
        assert_eq!(from_kinds, listed, "party roots must mirror `Kind`");
    }

    #[test]
    fn party_roots_are_structure_roots() {
        for t in DEMOGRAPHIC_PARTY_ROOTS {
            assert!(is_structure_type(t), "{t} must be a structure root");
            assert!(is_versioned_root_type(t));
        }
    }

    #[test]
    fn composition_content_delegates_to_the_rm_model() {
        // Every type the BMM-generated oracle calls a structure root is a
        // storage structure type — the single source of truth, no local dup.
        for t in [
            "COMPOSITION",
            "OBSERVATION",
            "SECTION",
            "CLUSTER",
            "ELEMENT",
            "EVENT_CONTEXT",
            "FEEDER_AUDIT",
            "ITEM_TREE",
        ] {
            assert!(openehr_rm::v1_2::model::is_structure_root(t));
            assert!(is_structure_type(t), "{t}");
        }
    }

    #[test]
    fn party_roots_are_the_only_delta_from_the_rm_model() {
        // The storage set = the RM-model structure set ⊎ the party roots, and
        // nothing else diverges: a party root is NOT in the RM model's set (the
        // model excludes the demographic hierarchy), confirming the split.
        for t in DEMOGRAPHIC_PARTY_ROOTS {
            assert!(
                !openehr_rm::v1_2::model::is_structure_root(t),
                "{t} unexpectedly in the RM-model structure set"
            );
        }
    }

    #[test]
    fn demographic_containers_are_not_structure_types() {
        for t in [
            "PARTY_IDENTITY",
            "CONTACT",
            "ADDRESS",
            "CAPABILITY",
            "PARTY_RELATIONSHIP",
        ] {
            assert!(!is_structure_type(t), "{t} must stay inline");
        }
        // …though a PARTY_RELATIONSHIP is still a valid versioned-object root.
        assert!(is_versioned_root_type("PARTY_RELATIONSHIP"));
    }

    #[test]
    fn parses_full_hrid_lowercased() {
        let (entity, concept, major) =
            archetype_parts("openEHR-EHR-OBSERVATION.laboratory-glucose.v2").unwrap();
        assert_eq!(entity, "openehr-ehr-observation");
        assert_eq!(concept, "laboratory-glucose");
        assert_eq!(major, 2);
    }

    #[test]
    fn at_codes_have_no_parts() {
        assert!(archetype_parts("at0001").is_none());
    }
}

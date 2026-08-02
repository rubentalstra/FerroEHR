---
name: rm-common-master06-change-control-overview
description: Verified findings for RM common master06 §6.1 (change_control Overview) — the VERSIONED_PARTY owner_id three-way divergence (code vs served OpenAPI vs AMB-69), the unregistered PARTY_RELATIONSHIP-as-top-level-structure RM-vs-SM tension, and the §-heading list for citation checking
metadata:
  type: feedback
---

Verified 2026-08-02 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
lines 3–18 (§6.1 = TWO paragraphs + two figures; §6.2 Basic Semantics starts
line 19).

**§6.1's whole enforceable content:** (a) change control exists for
consistency/indelibility/traceability/distributed sharing (delegated to BASE
`architecture_overview/master08-versioning.adoc`, which IS vendored);
(b) `VERSIONED_OBJECT<T>` = a "version container" for ONE versioned item;
(c) versioning is "limited to 'top-level structures', such as EHR Compositions
and Party objects in a demographic system"; (d) **physical containment of
Versions by a Versioned object "is only one possible implementation"** — our
decomposed `vo_version`/`node` storage is EXPLICITLY sanctioned here, so cite
§Overview for it rather than the weaker "no openEHR spec governs this" flag.

**CONFORMANT, do not re-check:** the container wire builders
(`app/ferroehr/src/versioning/wire.rs:175-240` — EHR side owner_id
`{local, EHR, ehr_id}` matches AMB-69 + the CNF container_shape cases);
`VERSIONED_OBJECT` is NOT abstract in RM (only `VERSION<T>` is), so serving
`_type: VERSIONED_OBJECT` for a kind with no dedicated binding is legal;
the versioned-kind set is CHECK-constrained to 10 top-level structures
(`migrations/ehr/0001_baseline.sql:362`); indelibility holds — the ONLY
`DELETE FROM vo_version` is the ITS-REST ADMIN path
(`service/admin/delete.rs:452`) and the only `UPDATE vo_version` closes
`sys_period` (`storage/version_repo/commit.rs:261`); container type
homogeneity is enforced on the one path that could break it (import kind
mismatch, `versioning/import.rs:258`).

**VERIFIED DEFECTS:**
- **VERSIONED_PARTY `owner_id` is a THREE-WAY divergence.** Code emits
  `{_type: OBJECT_REF, namespace: "demographic", type: "PARTY", id: the
  party's OWN vo_id}` (`service/demographic/support.rs:259-265`); the served
  OpenAPI documents + exemplifies `{_type: PARTY_REF, namespace: "local",
  type: "SYSTEM", …}` and asserts "`owner_id` is emitted in the shape that
  schema's own example uses" (`ferroehr-rest/src/api/demographic/
  openapi_routes.rs:5373,5398`); register **AMB-69 states "This server emits
  that shape"** (local/SYSTEM) — false. Released ground:
  `VersionedParty.owner_id` = `ObjectRefOfHierObjectId`, example
  `{namespace: local, type: SYSTEM}` (vendored
  `rest-oas/demographic-html.openapi.yaml`). Undetected because the
  container_shape CNF case DELIBERATELY does not assert owner_id (AMB-69).
  Three more owner_id shapes coexist: X_VERSIONED_PARTY export
  `{demographic, SYSTEM, system_id}` (`service/message/export.rs:481`) and
  demographic ITEM_TAG `{local, SYSTEM, system_id}` (`demographic/tags.rs:219`).
- **PARTY_RELATIONSHIP-as-its-own-version-container is unregistered spec
  tension.** RM demographic master02 §Versioning Semantics: "A Version of a
  PARTY includes all the compositional parts, such as … Party relationships of
  which it is the source"; §Party Relationships: "stored as part of the data of
  the PARTY designated as the source"; `PARTY.relationships` is an inline
  `List<PARTY_RELATIONSHIP>`. SM `i_party_relationship.adoc` contradicts it —
  `a_versioned_party_rel_id: UUID`, "Causes server-side creation of a new
  ORIGINAL_VERSION and CONTRIBUTION", `versioned_object_does_not_exist`. We do
  BOTH (own `kind='PARTY_RELATIONSHIP'` container + inline validation in
  `demographic/validate.rs:167`). AMB-32 covers only the missing WIRE; AMB-130
  only the defective invariants — neither asks whether a relationship is a
  §6.1 "top-level structure".
- **Dangling master06 headings:** `§Version tree` (17 sites, 14 files — real
  heading is "The 'Virtual Version Tree'"), `§Version subtypes` (2, real:
  "Version and its Subtypes"), plus the already-known `§Change Control` and
  `§CONTRIBUTION`.
- `{diagrams_uri}` PNGs still un-vendored (172 refs) — §6.1's
  `version_control_structures.png` is one, and the prose interprets it.
  Disclosed in each PROVENANCE.md, so an honest boundary, not a violation.

**master06 heading list (for citation checking):** Overview · Basic Semantics
(Typing · Versioned Objects · Version and its Subtypes · The 'Virtual Version
Tree' · Contributions · Committal and Audits · Digital Signature ·
Attestation) · Versioning Semantics (Version Lifecycle [Incomplete Content ·
Abandoned and Inactive States] · Logical Deletion · Version Identification
[Local Versioning · Distributed Versioning]) · Semantics in Distributed
Systems (Copying [The Copy Operation · Subsequent Local Modifications] ·
Version Merging · Disjoint Merging · Moving Version Containers) · Class
Descriptions. Nothing else exists.

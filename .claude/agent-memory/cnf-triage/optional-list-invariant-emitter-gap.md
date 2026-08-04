---
name: optional-list-invariant-emitter-gap
description: APP defect class — "present implies non-empty" RM invariants enforced only by Option<NonEmptyVec>; ROLE.capabilities + PARTY.relationships still emit Option<Vec> and are accepted empty
metadata:
  type: project
---

RM has a family of optional lists with a "present ⇒ non-empty" invariant:
`PARTY.Contacts_valid`, `PARTY.Relationships_validity` (first arm),
`ACTOR.Roles_valid`, `ROLE.Capabilities_valid`
(`RM/docs/UML/classes/org.openehr.rm.demographic.{party,actor,role}.adoc`
§Invariants). The project enforces them BY CONSTRUCTION with
`Option<NonEmptyVec<T>>` (#1873) and DELETED the raw-body checks that used to do
it (`app/ferroehr/src/service/demographic/validate.rs` now carries only a NOTE
claiming they "hold by construction").

**The closure is incomplete** (confirmed 2026-08-04 wire capture):
`crates/openehr-rm/src/demographic/role.rs:60` emits
`capabilities: Option<Vec<Capability>>` and `:54`
`relationships: Option<Vec<PartyRelationship>>`, while `contacts`/`roles` DID get
`NonEmptyVec`. Result: `POST /demographic/role` with `"capabilities": []`
returns **201 Created** — an RM-invalid instance committed
(`I_DEMOGRAPHIC_SERVICE.create_party-capabilities_present_empty`), while the
`contacts_present_empty` / `roles_present_empty` siblings pass. No case exists for
an empty `relationships` list.

**How to apply:** when an emptiness refusal is MISSING (not over-firing), check
the emitted field type first — an `Option<Vec<…>>` on an invariant-bearing list is
the whole bug, and the fix is the `openehr-codegen` emitter + regeneration, never
a consumer-side check and never a hand-edit. Cross-check
`crates/openehr-rm/src/validate/generated.rs`: a register line still naming
`app/ferroehr/.../validate.rs` as the enforcer is stale.
Related: [[nonempty-1star-containers]] (the mandatory 1..* twin).

---
paths: ["crates/openehr-rm/**", "crates/openehr-base/**", "crates/openehr-foundation/**", "crates/openehr-terminology/**"]
---

# RM / BASE / Terminology transcription rules

These four crates have **no Java to port** — EHRbase pulled the Reference
Model, Base Types, and Terminology from the external `archie`/openEHR-SDK
libraries, which are not in this repo. Everything here is transcribed
literally from the published openEHR specifications (PORT_MASTER_PLAN.md
Sections 7 and 14.4), not reinterpreted.

## One class, one type

- One RM/BASE class → one Rust struct or enum, named identically in
  PascalCase (`DV_TEXT` → `DvText`, `HIER_OBJECT_ID` → `HierObjectId`). Keep
  the openEHR class name visible via a doc comment and a serde rename to the
  canonical `_type` discriminator string (uppercase, e.g.
  `#[serde(rename = "DV_TEXT")]`).
- An abstract RM class with attributes becomes a struct the concrete types
  embed by composition (not inherit), plus a marker trait if behaviour is
  shared across the hierarchy.
- A closed subtype set (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`,
  `VERSION<T>`) becomes a closed Rust `enum`. Trait objects only for
  genuinely archetype-driven runtime polymorphism.
- A constrained generic (`DV_INTERVAL<T: DV_ORDERED>`,
  `REFERENCE_RANGE<T: DV_ORDERED>`, `HISTORY<T: ITEM_STRUCTURE>`,
  `VERSIONED_OBJECT<T>`) becomes a Rust generic with a matching trait bound.
- Covariant redefinition (`LOCATABLE_REF.id` narrowed from `OBJECT_ID` to
  `UID_BASED_ID`; `DV_COUNT.magnitude` as `Integer` not `Real`) is encoded
  directly on the concrete struct with the narrowed type; document the
  override in a doc comment.
- Multiple inheritance (`Ordered_Numeric`, `Iso8601_type`, `DV_DURATION`, the
  `EXTERNAL_ENVIRONMENT_ACCESS` mixin) is composition of fields from all
  parents plus one trait per parent behaviour.
- `PATHABLE.parent()` and any other reverse pointer use `Weak<..>` or a
  path-index — never an owning back-reference.
- Recursive containment (`FOLDER`, `CLUSTER`, `ITEM_TREE`, `SECTION`,
  `DV_MULTIMEDIA.thumbnail`) is boxed.
- Symbolic operators (`++`, `and then`, `∀`) become named methods, not
  operator overloads.

## Known hazards (do not relitigate — these are settled)

- `EVENT_CONTEXT`, `INSTRUCTION_DETAILS`, and `ISM_TRANSITION` inherit
  `PATHABLE`, **not** `LOCATABLE`. Do not give them `LOCATABLE` fields
  (`uid`, `archetype_details`, etc.).
- Terminology-service interfaces live in `rm.support`, not BASE.
- `ACCESS_GROUP_REF` was **not** migrated to BASE 1.2.0 — implement it only
  if legacy data actually needs it, and note why if you do.
- The type is `Octet`, not "Byte".
- The TERM 3.x XML has an `id=532` dual-rubric quirk (`complete` vs
  `completed`) — preserve both rubrics verbatim; do not normalize to one.

## Invariants

Model RM invariants with a `Validate` trait (context + path + error
accumulator), layered under `garde` for the outer request-DTO surface. Where
an invariant is not yet implemented, leave `// TODO(port):` on the impl
rather than skipping the trait entirely.

## Crate boundaries

`openehr-foundation` (primitives, `Interval<T>`, ISO 8601 temporals) has no
internal deps. `openehr-base` depends on it. `openehr-terminology` depends on
`openehr-base`. `openehr-rm` depends on both `openehr-base` and
`openehr-terminology`. Never point a dependency arrow upward.

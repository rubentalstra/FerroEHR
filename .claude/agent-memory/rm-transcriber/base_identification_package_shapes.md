---
name: base-identification-package-shapes
description: How the BASE 1.2.0 base_types.identification package (UID/OBJECT_ID hierarchies, OBJECT_REF/PARTY_REF/LOCATABLE_REF) was shaped in crates/openehr-base/src/identification/ — reusable pattern for abstract classes used both as embeddable parents and as polymorphic field types.
metadata:
  type: project
---

Transcribed 2026-07-02 into `crates/openehr-base/src/identification/` (16
files, one per spec class, all unwired — no `mod identification;` added to
`lib.rs` yet, per the P1-P16 Phase A convention). Source:
`docs/research/spec-cache/BASE-1.2.0/base_types/master05-identification_package.adoc`
+ `docs/research/spec-cache/BASE-1.2.0/uml_classes/*.adoc`, Release-1.2.0 @
commit 9064413.

**The recurring shape problem:** several abstract BASE classes (`UID`,
`OBJECT_ID`, `UID_BASED_ID`) are declared abstract with attributes (ADR-001
§3 territory — embed by composition) *and* are used polymorphically as the
declared type of another class's field (`OBJECT_REF.id: OBJECT_ID`,
`LOCATABLE_REF.id: UID_BASED_ID` — ADR-001 §4 territory — closed enum). Both
ADR-001 rules apply to the same class simultaneously. The pattern that
resolved this, reusable for any future RM abstract class in the same
position:

1. Give the abstract class an `XxxData` struct (e.g. `UidData`,
   `ObjectIdData`, `UidBasedIdData`) holding just its own attributes — this
   is the embeddable-by-composition half.
2. Give the abstract class an `Xxx` enum (e.g. `Uid`, `ObjectId`,
   `UidBasedId`) with one variant per *concrete* descendant — this is the
   polymorphic-field-type half.
3. Give the abstract class an `XxxApi` trait (e.g. `UidApi`, `ObjectIdApi`,
   `UidBasedIdApi`) with the shared accessor/behaviour methods, implemented
   once on the enum (dispatching via `match`) and once per concrete struct
   (reading its embedded `XxxData`).
4. Nest narrower enums inside wider ones rather than flattening, so a field
   genuinely typed at the narrower abstract level (`UID_BASED_ID`) can use
   the narrower enum directly. Concretely: `ObjectId::UidBased(UidBasedId)`
   nests, it does not flatten `HierObjectId`/`ObjectVersionId` as direct
   `ObjectId` variants. This is what makes `LocatableRef.id: UidBasedId`
   (the ADR-001 §6 covariant-redefinition case) work without generics.

**Ambiguities flagged in the transcription report, not silently resolved:**
- `PARTY_REF`'s `Type_validity` invariant constrains `OBJECT_REF.type` to
  one of 7 string values, but the spec table does *not* mark `type`
  `(redefined)` on `PARTY_REF` (unlike `LOCATABLE_REF.id`, which is marked
  `(redefined)`). Resolved by keeping `type: String` and adding a
  `VALID_TYPES` const + `is_type_valid()` check, not narrowing to an enum —
  because ADR-001 §6 is specifically about spec-marked redefinitions, and
  this isn't one.
- `VERSION_TREE_ID.branch_number()`/`branch_version()` are typed `String`
  (1..1, non-optional) in the Functions table but the invariant text treats
  them as possibly `Void`. Resolved with empty-string-for-absent (matching
  `UID_BASED_ID.extension()`'s own convention), not `Option<String>`.
- `UID_BASED_ID.root()` and `OBJECT_VERSION_ID.object_id()`/
  `creating_system_id()` all need a format-sniffing sub-parser to decide
  which `Uid` variant (`IsoOid`/`Uuid`/`InternetId`) a substring represents
  — left as `todo!()`, not guessed.
- `UUID` (spec class) was named `Uuid` and deliberately NOT backed by the
  external `uuid` crate in this pass — see [[serde-not-yet-wired]] sibling
  note on why external deps aren't added mid-transcription; this is the
  same caution applied to a second dependency, not just serde.

Excluded per settled hazard: `ACCESS_GROUP_REF` was not migrated to BASE
1.2.0 and was not transcribed (see `.claude/rules/rm-transcription.md`).

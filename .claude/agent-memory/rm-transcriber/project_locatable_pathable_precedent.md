---
name: project-locatable-pathable-precedent
description: Precedent decisions set while transcribing RM 1.1.0 common.archetyped (PATHABLE, LOCATABLE, ARCHETYPED, LINK, FEEDER_AUDIT, FEEDER_AUDIT_DETAILS) and common.generic (PARTY_PROXY, PARTY_SELF, PARTY_IDENTIFIED, PARTY_RELATED, PARTICIPATION, AUDIT_DETAILS, ATTESTATION, REVISION_HISTORY, REVISION_HISTORY_ITEM) into openehr-rm/src/common/{archetyped,generic}/ — the PATHABLE.parent() reverse-pointer reference implementation and the LOCATABLE embedding pattern every later concrete RM class must reuse.
metadata:
  type: project
---

Transcribed RM 1.1.0 `common.archetyped` and `common.generic` into
`crates/openehr-rm/src/common/{archetyped,generic}/` (15 files total, 6 +
9). This is the **first RM-proper transcription pass** landed in
`openehr-rm/src` (the crate's `src/` had only `lib.rs` before this) and sets
two load-bearing precedents downstream RM transcribers must reuse rather
than re-derive.

**Why this matters:** `LOCATABLE` is inherited (directly or via an
intermediate abstract class) by the overwhelming majority of concrete RM
classes across every later package (`ehr`, `demographic`, `integration`).
Any downstream transcriber writing `ENTRY`, `SECTION`, `COMPOSITION`,
`CLUSTER`, `ELEMENT`, `FOLDER`, `EHR_STATUS`, `PARTY`, etc. needs to embed
the same `LocatableData` shape this pass created, not invent a fresh one.

**How to apply:** any later RM class whose spec table has `Inherit:
LOCATABLE` (or a chain ending in it) should:
1. add a field of type `openehr_rm::common::archetyped::locatable::LocatableData`
   (conventionally named `locatable`)
2. implement `LocatableApi` (which requires `PathableApi` as supertrait) by
   providing `locatable_data(&self) -> &LocatableData` and letting the
   trait's default methods do the rest
3. NOT re-declare `name`/`archetype_node_id`/`uid`/`links`/
   `archetype_details`/`feeder_audit` as its own fields

Key decisions:

1. **`PATHABLE.parent()` reverse-pointer hazard — settled and
   reference-implemented in `pathable.rs`.** The trait method signature is
   `fn parent(&self) -> Option<Weak<dyn PathableApi>>`. Three deliberate
   layers, each independently justified in the file's module doc (do not
   re-litigate any of them without a strong reason):
   - `Weak<..>` (never owning) — the standard reverse-pointer rule.
   - `dyn PathableApi` (not `Weak<Self>` or a concrete type) — because the
     spec's own return type is the abstract `PATHABLE` supertype, and the
     realistic set of `PATHABLE` implementors spans every RM package, so a
     closed enum would force every RM package to depend on every other one
     just to name a variant. **This is the one deliberate open-trait-object
     exception to the "closed enum over trait object" rule (ADR-001 §4) in
     the entire RM common package** — flagged prominently so a reviewer
     doesn't mistake it for an accidental departure.
   - Outer `Option<..>` — distinct from `Weak::upgrade() == None` (parent
     dropped); models "this node is the root of its own tree, never had a
     parent to report" as a structurally different state.
   All five `PathableApi` methods (`parent`, `item_at_path`,
   `items_at_path`, `path_exists`, `path_unique`, `path_of_item`) are
   `todo!()`-bodied by design — `PATHABLE` itself has no state, so no real
   path-resolution logic is possible until concrete `LOCATABLE` descendants
   with actual child collections exist (RM data_structures/ehr phase).

2. **`LOCATABLE` — the load-bearing composition pattern, described above.**
   `LocatableData` (embeddable struct: `name`, `archetype_node_id`, `uid`,
   `links`, `archetype_details`, `feeder_audit`, plus a **non-spec**
   `parent: Option<Weak<dyn LocatableApi>>` field) + `LocatableApi:
   PathableApi` (behaviour trait, requires `locatable_data()` accessor,
   provides all six spec attribute-getters plus `concept()` and
   `is_archetype_root()` as default methods). The `parent` field on
   `LocatableData` is **not** a spec attribute — `LOCATABLE`'s own
   attribute table has no `parent` row, since the spec models `parent()`
   purely as an inherited `PATHABLE` function — but concrete state has to
   live somewhere for `PathableApi::parent()` to have something to read
   from, per the settled `Weak` pattern. Flagged as a `PORT NOTE`, not
   silently added.
   **Known unresolved seam, left for the first concrete `LOCATABLE`
   descendant to close:** no blanket `impl<T: LocatableApi> PathableApi
   for T` is provided. The reason is real, not laziness: `LocatableData`
   stores `Weak<dyn LocatableApi>` (the narrower trait, so a
   `LOCATABLE`-aware caller doesn't need a downcast to call `LocatableApi`
   methods on the parent), but `PathableApi::parent()`'s signature demands
   `Weak<dyn PathableApi>` (the wider trait, matching the spec's abstract
   return type) — and `Weak<dyn LocatableApi>` → `Weak<dyn PathableApi>` is
   not an automatic unsizing coercion in stable Rust behind a `Weak`. The
   first concrete `LOCATABLE` descendant (RM data_structures or ehr phase)
   has to write that upgrade-and-rewrap glue in its own
   `impl PathableApi for ConcreteType`. Don't try to solve this
   preemptively in `locatable.rs` again — there's no implementor yet to
   validate the shape against.
   `EVENT_CONTEXT`, `INSTRUCTION_DETAILS`, `ISM_TRANSITION` inherit
   `PATHABLE` directly, **not** `LOCATABLE` — repeated the warning inside
   `locatable.rs` itself (not just the rules file) because that's exactly
   where a future transcriber reaching for "just embed `LocatableData`"
   out of habit would make the mistake.

3. **`PARTY_PROXY` closed-enum triple** (`PartyProxyData` + `PartyProxy`
   enum + `PartyProxyApi` trait) is ADR-001 §4's own named example — three
   variants `PartySelf`/`PartyIdentified`/`PartyRelated`, flattened as
   direct siblings rather than nesting `PartyRelated` inside a
   `PartyIdentified`-shaped indirection (contrast the `ObjectId`/
   `UidBasedId` nesting precedent in `openehr-base`, which nests because
   `UID_BASED_ID` is itself somebody else's declared field type elsewhere
   in the spec — no RM/BASE attribute anywhere is typed narrowly as
   `PARTY_IDENTIFIED`, so there's no forcing reason to nest here).
   **Reusable pattern:** flatten a closed-enum's transitively-inheriting
   variant as a direct sibling unless some other spec attribute's declared
   type specifically needs the narrower intermediate type on its own.

4. **Composition chains stay unflattened, one hop per file.**
   `PartyRelated` embeds `PartyIdentified` (not a flattened copy of
   `PartyIdentified`'s fields), which itself embeds `PartyProxyData`.
   `Attestation` embeds `AuditDetails`. Each file only ever embeds its
   *immediate* parent's struct — never reaches through to flatten a
   grandparent's fields directly — so a change to any one ancestor's
   attribute set only requires touching that ancestor's own file.

5. **Terminology-bound invariants that need a live `TerminologyService`
   are `todo!()`-bodied methods that take `&TerminologyService` as a
   parameter, not bare boolean stubs.** E.g. `PartyRelated::
   is_relationship_valid`, `Participation::is_function_valid`/
   `is_mode_valid`, `AuditDetails::is_change_type_valid`, `Attestation::
   is_reason_valid`. Distinguish these from **self-contained** invariants
   (e.g. `System_id_valid: not system_id.is_empty`), which get an actual
   working boolean method now (`is_system_id_valid(&self) -> bool`), no
   `todo!()` needed — the difference is whether the check only reads the
   struct's own fields or needs external service state.
   **A further wrinkle worth repeating:** several invariants
   (`Participation.Function_valid`, `Attestation.Reason_valid`) are
   *conditional* — they only apply when a `DV_TEXT`-typed field happens at
   runtime to actually be a `DV_CODED_TEXT` (`generating_type.is_equal
   ("DV_CODED_TEXT")`). This needs runtime-type discrimination of a
   not-yet-transcribed closed enum (`DvText`/`DvCodedText`, `data_types`
   package) in addition to the terminology service — flagged as a second,
   compounding blocker in the doc comment, not silently conflated with the
   simpler terminology-only case.

6. **Two genuine spec-text ambiguities found and flagged loudly (not
   silently resolved) — worth a reviewer's attention:**
   - `REVISION_HISTORY_ITEM.audits` is typed `List<AUDIT_DETAILS>` in the
     attribute table, but the class's own description says an entry "may
     itself be an `ATTESTATION`" — since `ATTESTATION` *embeds* (not
     `is-a`, in Rust terms) `AuditDetails`, a `Vec<AuditDetails>` cannot
     literally hold an `Attestation` value losslessly. Transcribed
     literally as `Vec<AuditDetails>` per the table's stated type, with
     the field doc explaining the mismatch rather than inventing an enum
     the spec doesn't declare at that attribute.
   - `REVISION_HISTORY`'s class-level Purpose text says "most-recent-
     **first** order" while the `items` attribute's own row says
     "most-recent-**last** order" — directly contradictory within the same
     cached spec file. Resolved in favor of most-recent-**last** because
     both of the class's own derived functions
     (`most_recent_version`/`most_recent_version_time_committed`) have
     postconditions reading `items.last` — which is only meaningful under
     the last-order reading. Documented the contradiction and the
     resolution rationale directly on the struct, not just in the
     transcription report, so it survives independently of this memory
     file.
   **Pattern to keep applying:** when a spec table is internally
   inconsistent, resolve toward whichever reading is *structurally forced*
   by an adjacent, unambiguous part of the same class (a postcondition, an
   invariant, a sibling attribute's own wording) — and always leave the
   contradiction visible in the file's own doc comments, not just in a
   session report that won't travel with the code.

Depends on forward-references to `DvText`, `DvCodedText`, `DvIdentifier`,
`DvDateTime`, `DvInterval<T>`, `DvMultimedia`, `DvEhrUri`, `DvEncapsulated`
(all RM `data_types`, a sibling agent's concurrent territory in the same
phase, not yet landed at transcription time) and `ItemStructure` (RM
`data_structures`, likewise). All forward-references point at their
*eventual* module paths under `crate::data_types::*`/
`crate::data_structures::*` and are marked `TODO(port):` — Phase A, nothing
is expected to compile yet.

See `docs/ADRs/ADR-001-spec-transcription-shapes.md` §§1,3,4,8 for the ADR
text this cluster either follows or (for `PATHABLE.parent()`'s trait-object
return type) documents as a deliberate, reasoned exception to. Ground truth:
`docs/research/spec-cache/RM-1.1.0/common/master03-archetyped_package.adoc`,
`master04-generic_package.adoc`, plus the 15 per-class
`docs/research/spec-cache/RM-1.1.0/uml_classes/{pathable,locatable,
archetyped,link,feeder_audit,feeder_audit_details,party_proxy,party_self,
party_identified,party_related,participation,audit_details,attestation,
revision_history,revision_history_item}.adoc` tables (Release-1.1.0 @
3cbd85b).

**Environment note for future sessions in an isolated worktree:** at
transcription time, this worktree's own copy of
`docs/research/spec-cache/RM-1.1.0/` was missing the `common/` and
`uml_classes/` subdirectories present in the outer/main checkout (only
`support/` had synced across) — a stale worktree snapshot relative to
concurrent sibling-agent work populating spec-cache elsewhere. Copied the 17
needed `.adoc` files in read-only from the outer checkout path
(`/Users/rubentalstra/RustroverProjects/ehrbase-rs/docs/research/spec-cache/...`)
before transcribing, verified byte-identical via `diff`. If a future session
in a worktree hits a similar "file exists at the outer path but not the
worktree path" gap for spec-cache material specifically (read-only
reference, not a file this task type ever writes to), the same copy-in
approach is safe and appropriate — just verify identity with `diff` first
and don't assume every path under the repo root is mirrored 1:1 into every
worktree.

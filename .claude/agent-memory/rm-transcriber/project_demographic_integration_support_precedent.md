---
name: project-demographic-integration-support-precedent
description: Precedent decisions from transcribing RM 1.1.0 demographic (PARTY/ACTOR/ROLE/PERSON/ORGANISATION/GROUP/AGENT/PARTY_RELATIONSHIP/PARTY_IDENTITY/CONTACT/ADDRESS/CAPABILITY/VERSIONED_PARTY), integration (GENERIC_ENTRY), and support (MEASUREMENT_SERVICE/EXTERNAL_ENVIRONMENT_ACCESS/terminology mapping-note) packages into crates/openehr-rm/src/{demographic,integration,support}/ — 17 files, 1062 lines.
metadata:
  type: project
---

Transcribed RM 1.1.0's `demographic`, `integration`, and remaining `support`
packages into `crates/openehr-rm/src/{demographic,integration,support}/` (17
files). Spec cache: `docs/research/spec-cache/RM-1.1.0/uml_classes/*.adoc`
(Release-1.1.0 @ 3cbd85b) — note these packages' class tables live in the
**shared** `RM-1.1.0/uml_classes/` directory, not per-package subdirectories;
the per-package `masterNN-*.adoc` files only `include::` them by path. Always
check the shared dir first before concluding a class table is missing.

**Why this matters for future transcribers:** two structural judgment calls
were made here (nested enums, and a flagged-not-resolved MI mismatch) that
are reusable patterns, not one-off choices, plus one factual discovery about
where `rm.support.terminology` actually landed.

**How to apply:** any future class with a two-level closed-hierarchy spec
inheritance chain, or a mixin that inherits across a trait/struct boundary
established in an earlier phase, should follow these precedents rather than
re-deriving a shape from scratch.

1. **Two-level spec hierarchy → nested enums, not one flat enum.**
   `PARTY` → `ACTOR` → `{PERSON, ORGANISATION, GROUP, AGENT}` and `PARTY` →
   `ROLE` was modelled as `Party { Actor(Actor), Role(Role) }` with `Actor`
   itself a separate four-variant enum (`actor.rs`), rather than flattening
   into one five-variant `Party` enum. This mirrors the spec's own two-step
   `Inherit` chain exactly (`ACTOR` inherits `PARTY`; `PERSON` inherits
   `ACTOR`) and keeps `ActorApi: PartyApi` a genuine supertrait relationship
   matching the spec's inheritance depth. **Apply this whenever a spec
   abstract class has its own abstract sub-hierarchy before reaching
   concrete leaves** — do not collapse multi-level `Inherit` chains into a
   single enum just because all the leaves end up disjoint; nest one enum
   per abstraction level instead, matching `docs/ADRs/ADR-001-spec-transcription-shapes.md`
   §4 applied recursively.

2. **Constants-class multiple inheritance keeps composing, all the way up
   the mixin chain — this is now a *third* confirmed use of the
   `Time_Definitions` pattern** (see
   `[[project-time-types-precedent]]` item 2 for the original). Both
   `TERMINOLOGY_SERVICE`'s spec `Inherit: OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS,
   OPENEHR_CODE_SET_IDENTIFIERS` (already realised this way in P2 by
   `openehr-terminology`, confirmed by reading that crate's source directly
   — not re-derived) and this session's own
   `EXTERNAL_ENVIRONMENT_ACCESS: TERMINOLOGY_SERVICE, MEASUREMENT_SERVICE`
   follow the identical "constants classes are never supertraits" rule.
   Every future terminology/support class inheriting one of these two
   identifier classes should also use direct qualified calls, never a
   supertrait bound.

3. **NEW, more serious pattern than #2: a mixin that spec-inherits a class
   whose *shape* (trait vs. concrete struct) is not under the current
   transcription's control cannot always be resolved as a clean supertrait
   — and the honest move is to flag it unresolved, not force a fit.**
   `EXTERNAL_ENVIRONMENT_ACCESS: TERMINOLOGY_SERVICE, MEASUREMENT_SERVICE`
   is the concrete case: `MEASUREMENT_SERVICE` (transcribed this session,
   interface-only, ADR-001 §1) became a plain `trait`, but
   `TERMINOLOGY_SERVICE` was already transcribed in **P2** directly into
   `openehr-terminology::TerminologyService` as a **concrete struct**
   (bundled-asset-backed, inherent methods, `LazyLock` singleton — verified
   by reading `crates/openehr-terminology/src/terminology_service.rs`
   directly, not assumed). A struct cannot be a Rust supertrait bound, so
   `trait ExternalEnvironmentAccess: TerminologyService + MeasurementService`
   is not well-formed. Two resolutions exist (retrofit `TerminologyService`
   behind an extracted trait; or model the mixin asymmetrically with a
   trait bound plus a concrete field/accessor) and **neither was applied** —
   the task explicitly said "flag the mismatch, do not invent a
   resolution," and this precedent confirms that was the right call, not
   just an instruction to follow blindly. **Apply this whenever a spec
   class's declared multiple inheritance crosses a trait/struct boundary
   set by an earlier phase**: document both resolutions in the file's doc
   comment (so the next agent that touches it has the tradeoff already
   laid out), pick neither, and leave a `TODO(port)` naming the decision
   explicitly rather than silently picking the shape that happens to
   compile.

4. **`rm.support.terminology`'s five classes (`TERMINOLOGY_SERVICE`,
   `TERMINOLOGY_ACCESS`, `CODE_SET_ACCESS`,
   `OPENEHR_TERMINOLOGY_GROUP_IDENTIFIERS`, `OPENEHR_CODE_SET_IDENTIFIERS`)
   already fully exist, compiling and tested, in `openehr-terminology`
   (P2) — do not re-transcribe them under `openehr-rm/src/support/` even
   though their spec package is `rm.support`.** This was a deliberate
   earlier-phase decision (terminology is a dependency leaf, already wired
   unlike the rest of Phase-A `openehr-rm`), confirmed by reading
   `openehr-terminology`'s actual file tree and `lib.rs` re-exports rather
   than assumed from the plan document. A pure doc-only mapping-note file
   (`support/terminology.rs`) records this fact plus a table of which spec
   class landed as which concrete Rust path, and defers a possible
   `pub use openehr_terminology::{...}` re-export to P17 as a crate-API
   surface decision, not a transcription one. **Apply this whenever a
   package's own spec chapter names classes that provably already exist
   elsewhere in the workspace** — write the pointer file, do not
   re-implement, and do not silently skip without a trace either.

5. **`support.assumed_types` requires zero transcription in `openehr-rm`.**
   The spec's own text says so explicitly ("These sections have been
   removed to a separate specification in the BASE component") — its three
   subsections map 1:1 to BASE Foundation Types' Primitive Types, Structure
   Types, and Time Types sections, i.e. **already-done P1 work** in
   `openehr-foundation`. Recorded as a paragraph in the same
   `support/terminology.rs` doc-only file rather than a separate file,
   since the task grouped this note there too.

6. **Genuine spec-table ambiguities found and flagged, not silently
   resolved (do not silently "fix" these on a future pass without
   re-reading the spec first):**
   - `PARTY`'s own `Uid_mandatory` invariant (`uid /= Void`) makes `uid`
     effectively required, but the inherited `LOCATABLE.uid` field type is
     unconditionally `[0..1]` everywhere else in the RM. Kept the field
     `Option<UidBasedId>` (not narrowed to non-`Option`) since the spec
     narrows *optionality via invariant*, not the declared cardinality —
     changing the field type would be a structural deviation the spec
     itself does not make on every other `LOCATABLE`-inheriting class.
   - `PARTY.Reverse_relationships_validity` and
     `PARTY_RELATIONSHIP.Source_valid`/`Target_valid` all reference a
     `repository("demographics")`-style construct or dereference through
     another Party's own `relationships`/`reverse_relationships` list —
     both presume a resolvable object graph/repository service with no
     analogue in a spec-transcription crate. Left as `TODO(port)` rather
     than inventing a repository abstraction the spec class itself does
     not define.
   - `CAPABILITY`'s own spec table has **no** `Functions`/`Invariants`
     section, unlike its `ADDRESS`/`CONTACT`/`PARTY_IDENTITY` siblings
     (each of which derives a `type()`/`purpose()` accessor plus an
     `Xxx_valid: xxx = name` invariant from the inherited `name`
     attribute). Transcribed literally as-is — no `type()`-style accessor
     invented for `CAPABILITY` even though the pattern would be
     mechanically obvious to add.
   - `GENERIC_ENTRY` embeds `LocatableData` directly rather than through a
     `ContentItemData` wrapper, since `CONTENT_ITEM` (its direct parent)
     adds zero attributes of its own beyond `LOCATABLE`'s — flagged with a
     forward-looking note that if the `ehr` package later introduces a
     `ContentItemData`/`ContentItemApi` pair for its own `ContentItem`
     enum's uniform-accessor needs, `GenericEntry` should switch to
     embedding that instead, to stay consistent with its `ContentItem`
     enum siblings (`SECTION`, `ENTRY`-and-descendants).
   - `X_VERSIONED_PARTY` exists as its own `.adoc` in the shared
     `uml_classes/` cache dir, but the demographic package's own
     `master02-demographic_package.adoc` `Class Definitions` include list
     does **not** `include::` it (only `versioned_party.adoc` is
     included) — so it was deliberately left untranscribed rather than
     added speculatively, on the theory that `X_`-prefixed classes are an
     ITS-XML serialization binding concern for a later phase, not this
     package's own class-definitions scope.

Depends on forward-references into `crate::common::{archetyped::locatable,
generic, versioned_object}` and `crate::data_structures::item_structure`
(sibling agents' concurrent work, not yet landed in this worktree at time of
writing — Phase A, nothing needs to resolve yet) and
`openehr_base::identification::{locatable_ref, party_ref}` (P1, already
landed). See `docs/ADRs/ADR-001-spec-transcription-shapes.md` §§1, 2, 3, 4
and `[[project-time-types-precedent]]` for the constants-class-inheritance
template this session reused a third time.

# Phase 01 — Foundation + Identification (BASE 1.2.0)

- Status: done
- Started: 2026-07-02   Completed: 2026-07-02   Owner: Ruben
- Consumes (spec/layer): BASE 1.2.0 specs / Layer 1 + Layer 2
- Compile required: no (Phase A)

## Objectives

Transcribe BASE Foundation Types and Base Types Identification literally into
`openehr-foundation` and `openehr-base`, and resolve — once, for the whole
project — the structural decisions every later RM class will inherit: how we
model multiple inheritance, covariant redefinition, and constrained generics
in Rust.

## Preconditions

- [x] Phase 00 exit criteria met: workspace builds, `openehr-foundation` and
      `openehr-base` crate skeletons exist

## Scope

In: Foundation Types (primitives, `Interval<T>`, containers, ISO 8601
temporals, functional types), Base Types Identification (`UID` hierarchy,
`OBJECT_ID` hierarchy, `OBJECT_REF`/`PARTY_REF`/`LOCATABLE_REF`), Resource
classes, the four structural-hazard decisions from Section 7.2.
Out: RM classes proper (Phase 03), terminology service interfaces (they live
in `rm.support`, ported in Phase 03 not here).

## Tasks

- [x] Transcribe Foundation primitives and `Interval<T: Ordered>` into `openehr-foundation` — 13 primitives + 5 interval classes; Multiplicity_interval follows the literal Inherit row (embeds ProperInterval<Integer>)
- [x] Transcribe Foundation containers (List, Set, Bag equivalents) and functional types — Container trait + List/Set/Array/OpenEhrHash; NO Bag (spec declares none — not invented); Tuple/Routine/Function/Procedure; TerminologyCode/Term
- [x] Transcribe ISO 8601 temporal foundation types (`Iso8601_type` and its multiple-inheritance siblings) — 8 files; Iso8601Type: Temporal + Iso8601TypeCore embed = the ADR-001 §2 worked example; TimeDefinitions as unit-struct constants class (a struct cannot be a supertrait)
- [x] Transcribe `UID` -> `ISO_OID` / `UUID` / `INTERNET_ID` into `openehr-base` — Data struct + closed enum + Api trait triple (ADR-001 refinement)
- [x] Transcribe `OBJECT_ID` -> `UID_BASED_ID` -> `HIER_OBJECT_ID` / `OBJECT_VERSION_ID`, plus `ARCHETYPE_ID` / `TEMPLATE_ID` / `TERMINOLOGY_ID` / `GENERIC_ID` — nested enums (ObjectId::UidBased(..)); _type via TYPE_NAME consts until serde lands at P4
- [x] Transcribe `OBJECT_REF` / `PARTY_REF` / `LOCATABLE_REF`, encoding the `LOCATABLE_REF.id` covariant redefinition (OBJECT_ID narrowed to UID_BASED_ID) directly on the concrete struct — done; PartyRef Type_validity as VALID_TYPES const; ACCESS_GROUP_REF excluded per settled hazard
- [x] Transcribe Base Types Resource classes (authored resource, resource description, translation details) — 5 classes incl. ResourceAnnotations (chapter include-gap flagged); parent_resource as Weak; plus definitions (4) and builtins (5) packages
- [x] Record the multiple-inheritance decision (composition + trait per parent) with a worked example from `Ordered_Numeric` or `Iso8601_type` — ADR-001 §2; worked example `primitive_types/ordered_numeric.rs` (supertrait + blanket impl, E0119 pitfall documented)
- [x] Confirm `Octet` (not `Byte`) naming and symbolic operators (`++`, `and then`) become named methods — `octet.rs` (newtype over u8); ADR-001 §1; Boolean semistrict connectives take `impl FnOnce()` for short-circuit
- [x] Add PORT STATUS trailers to every transcribed file — verified 69/69; rustfmt --check clean; 154 TODO(port), 123 PORT NOTE; attribution scan clean

## Exit criteria

- [x] `openehr-foundation` and `openehr-base` contain every class listed in Section 7.1's BASE (~25) entry — 69 classes transcribed (full chapter surface incl. definitions/builtins beyond the headline list)
- [x] The MI / covariance / generic-bound decisions are written down (ADR or `docs/ROSETTA.md` entries), not just implied by code — ADR-001 (+ P1 refinements section) and ~50 ROSETTA rows
- [x] Every file carries a PORT STATUS trailer — verified by sweep

## Decisions made this phase

- ADR-001 (spec-transcription shapes) is the binding record: traits for
  attribute-less abstract classes, supertrait composition for MI, closed
  enums for closed subtype sets, trait-bounded generics, narrowed-field
  covariance, std-backed primitive newtypes, file-per-class layout,
  unwired until P17.
- Refinement: abstract-with-attributes used polymorphically (UID, OBJECT_ID,
  UID_BASED_ID) gets Data struct + closed enum + Api trait; narrower enums
  nest inside wider ones so covariance stays a plain field type.
- Blanket-impl a capability composite (OrderedNumeric) but NOT a semantic
  category (Temporal — explicit empty impls); constants-only parent classes
  (Time_Definitions, BASIC_DEFINITIONS) become zero-sized structs with
  associated consts/fns, called directly rather than inherited.
- serde derives deferred to P4 (dep not yet wired); concrete classes carry
  `pub const TYPE_NAME` meanwhile.
- Spec-text problems are flagged, never silently resolved: ROUTINE
  description contradicts its signature; AUTHORED_RESOURCE.revision_history
  missing from the published attribute table; RESOURCE_ANNOTATIONS missing
  from the chapter include list; Interval.has postcondition parenthesization
  ambiguous; Any.not_equal parameter typed Ordered in the published table.
- No Bag class exists in BASE 1.2.0 — the phase task wording was loose; the
  spec wins.

## Handoff for next session

P1 complete: 69 BASE 1.2.0 classes across openehr-foundation (39) and
openehr-base (30), all trailer-carrying, rustfmt-clean, unwired (workspace
still builds green). Spec sources pinned under
docs/research/spec-cache/BASE-1.2.0/. Recommended before P3 leans on these
precedents: a port-reviewer sampling pass over one file per cluster. Next is
P2 (docs/plans/phase-02-terminology.md): bundle the TERM 3.x XML assets and
terminology service API in openehr-terminology — a leaf crate that SHOULD
compile, so it also debuts Cargo dependency wiring; cache the TERM release
under spec-cache/ the same way, and preserve the id=532 dual-rubric quirk.
Research dossiers for docs/research/ are still pending delivery.

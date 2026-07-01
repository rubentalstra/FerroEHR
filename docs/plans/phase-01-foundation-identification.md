# Phase 01 — Foundation + Identification (BASE 1.2.0)

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): BASE 1.2.0 specs / Layer 1 + Layer 2
- Compile required: no (Phase A)

## Objectives

Transcribe BASE Foundation Types and Base Types Identification literally into
`openehr-foundation` and `openehr-base`, and resolve — once, for the whole
project — the structural decisions every later RM class will inherit: how we
model multiple inheritance, covariant redefinition, and constrained generics
in Rust.

## Preconditions

- [ ] Phase 00 exit criteria met: workspace builds, `openehr-foundation` and
      `openehr-base` crate skeletons exist

## Scope

In: Foundation Types (primitives, `Interval<T>`, containers, ISO 8601
temporals, functional types), Base Types Identification (`UID` hierarchy,
`OBJECT_ID` hierarchy, `OBJECT_REF`/`PARTY_REF`/`LOCATABLE_REF`), Resource
classes, the four structural-hazard decisions from Section 7.2.
Out: RM classes proper (Phase 03), terminology service interfaces (they live
in `rm.support`, ported in Phase 03 not here).

## Tasks

- [ ] Transcribe Foundation primitives and `Interval<T: Ordered>` into `openehr-foundation`
- [ ] Transcribe Foundation containers (List, Set, Bag equivalents) and functional types
- [ ] Transcribe ISO 8601 temporal foundation types (`Iso8601_type` and its multiple-inheritance siblings)
- [ ] Transcribe `UID` -> `ISO_OID` / `UUID` / `INTERNET_ID` into `openehr-base`
- [ ] Transcribe `OBJECT_ID` -> `UID_BASED_ID` -> `HIER_OBJECT_ID` / `OBJECT_VERSION_ID`, plus `ARCHETYPE_ID` / `TEMPLATE_ID` / `TERMINOLOGY_ID` / `GENERIC_ID`
- [ ] Transcribe `OBJECT_REF` / `PARTY_REF` / `LOCATABLE_REF`, encoding the `LOCATABLE_REF.id` covariant redefinition (OBJECT_ID narrowed to UID_BASED_ID) directly on the concrete struct
- [ ] Transcribe Base Types Resource classes (authored resource, resource description, translation details)
- [ ] Record the multiple-inheritance decision (composition + trait per parent) with a worked example from `Ordered_Numeric` or `Iso8601_type`
- [ ] Confirm `Octet` (not `Byte`) naming and symbolic operators (`++`, `and then`) become named methods
- [ ] Add PORT STATUS trailers to every transcribed file

## Exit criteria

- [ ] `openehr-foundation` and `openehr-base` contain every class listed in Section 7.1's BASE (~25) entry
- [ ] The MI / covariance / generic-bound decisions are written down (ADR or `docs/ROSETTA.md` entries), not just implied by code
- [ ] Every file carries a PORT STATUS trailer

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This is the first spec-transcription phase and the one that
sets precedent for every later RM class, so read Section 7.2 in full before
writing code, and record each hazard decision in `docs/ROSETTA.md` as it is
made so Phase 03 can look it up instead of re-deciding it.

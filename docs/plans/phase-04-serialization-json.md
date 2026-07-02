# Phase 04 — Canonical JSON serialization (ITS-JSON)

- Status: done
- Started: 2026-07-02   Owner: Ruben
- Consumes (spec/layer): ITS-JSON (development, pinned commit) / Layer 5a
- Compile required: no (Phase A)

## Objectives

Implement canonical JSON serialization and deserialization for the RM classes
transcribed in Phase 03, matching the ITS-JSON schemas exactly (snake_case
attributes, `_type` discriminator, `_`-prefixed metadata, omitted nulls,
`{_type, value}` UIDs, inline base64 `DV_MULTIMEDIA.data`), and pin golden
vectors with `insta`.

## Preconditions

- [x] Phase 03 done: RM classes exist in `openehr-rm`

## Scope

In: serde derive/manual impls for every RM class in `openehr-serde`,
`_type` dispatch for closed enums, insta golden-vector tests per class family.
Out: canonical XML (Phase 05), FLAT/STRUCTURED/Web Template JSON (Phase 16 —
those are vendor formats, not ITS-JSON canonical JSON).

## Tasks

- [x] Pin the exact ITS-JSON git commit hash in `docs/VERSIONS.md` and vendor `components/openehr_rm_1.1.0_all.json` as a reference schema for validation — commit `5acae056`, vendored at `crates/openehr-serde/schemas/`
- [x] Implement `_type` discriminator dispatch (uppercase RM class name) for every closed enum (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>`) — via ADR-002 self-tagging payloads + untagged enums (tag-driven dispatch pinned by tests)
- [x] Implement snake_case field renaming and null-omission for all RM structs via serde attributes or a custom serializer — skip_serializing_if on every Option across base/rm/foundation
- [x] Implement `{_type, value}` UID serialization for the `OBJECT_ID` hierarchy — TypeTag on all 8 concretes, untagged ObjectId/UidBasedId/Uid enums
- [x] Implement inline base64 encoding/decoding for `DV_MULTIMEDIA.data` — golden vector pins "AQIDBA==" for [1,2,3,4]
- [x] Mark `_type` as required whenever the declared field type is abstract, matching the ITS-JSON rule — TypeTag rejects wrong _type; abstract slots dispatch only via _type (stricter: emitted everywhere, matching stock EHRbase)
- [x] Write insta golden-vector tests for rm.data_types (round-trip serialize -> deserialize -> equal) — 25 classes in openehr-serde/tests/
- [x] Write insta golden-vector tests for rm.data_structures, rm.common, rm.ehr, rm.demographic — full coverage harness now requires every schema class to have a fixture and fails by name for any gap
- [x] Validate serialized output against the vendored `openehr_rm_1.1.0_all.json` using `jsonschema` — every fixture is validated; no per-fixture validation exclusions
- [x] Add PORT STATUS trailers; update `docs/ROSETTA.md` with serde-specific quirks per class — trailers updated per file; ROSETTA rows added for the ADR-002 patterns

## Exit criteria

- [x] Every RM class round-trips through canonical JSON with `insta`-pinned output — `full_rm_coverage` fails by class name on any gap
- [x] `_type` dispatch works for all five closed enums — tag-driven untagged dispatch pinned by unit tests (incl. structure-identical DV_DATE/DV_TIME)
- [x] At least one golden vector per RM package validates against the ITS-JSON schema — 93 classes validate, all packages covered

## Decisions made this phase

- [ADR-002](../ADRs/ADR-002-canonical-json-self-tagging.md): `_type` via
  self-tagging `TypeTag<Self>` fields on every concrete class + untagged
  closed enums (payload-tag-driven dispatch), matching stock EHRbase's
  emit-`_type`-everywhere behaviour. Supersedes and unifies the first
  wave's three divergent mechanisms (tagged enums / no-op struct renames /
  manual impls).
- Serde derives live on the types in `openehr-rm`/`openehr-base` (orphan
  rule makes the phase file's "in openehr-serde" wording unimplementable);
  `openehr-serde` owns the acceptance instrument instead
  (`tests/full_rm_canonical_json.rs`: schema-definition coverage check +
  jsonschema validation + insta golden vectors).
- VERSIONED_X binding newtypes stay `#[serde(transparent)]` and never emit
  their own `_type` — the pinned ITS-JSON schema defines only
  `VERSIONED_OBJECT`, no per-binding definitions.

## Handoff for next session

Phase complete (2026-07-02). Canonical JSON is done end-to-end: every
concrete RM/BASE class self-tags via `openehr_foundation::serde_support::
{TypeName, TypeTag}` (ADR-002), closed enums are `#[serde(untagged)]` with
tag-driven dispatch, and `openehr-serde` owns the acceptance instrument —
`tests/full_rm_canonical_json.rs` covers all 134 pinned-schema classes with
no exclusions (round-trip + `_type`-first + jsonschema validation + insta
golden), and `cargo test` is green. P5 (canonical XML) should start from
the same fixture set in
`crates/openehr-serde/tests/fixtures/` — the constructors there are the
deterministic instances to serialize against the RM 1.1.0 XSDs — and reuse
the ADR-002 decision record for how `xsi:type` maps onto the same
class-name discriminator. Known debt for P17: untagged-enum error messages
are weak ("did not match any variant"); revisit if REST error parity needs
better diagnostics.

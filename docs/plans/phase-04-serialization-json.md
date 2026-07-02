# Phase 04 — Canonical JSON serialization (ITS-JSON)

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): ITS-JSON (development, pinned commit) / Layer 5a
- Compile required: no (Phase A)

## Objectives

Implement canonical JSON serialization and deserialization for the RM classes
transcribed in Phase 03, matching the ITS-JSON schemas exactly (snake_case
attributes, `_type` discriminator, `_`-prefixed metadata, omitted nulls,
`{_type, value}` UIDs, inline base64 `DV_MULTIMEDIA.data`), and pin golden
vectors with `insta`.

## Preconditions

- [ ] Phase 03 done: RM classes exist in `openehr-rm`

## Scope

In: serde derive/manual impls for every RM class in `openehr-serde`,
`_type` dispatch for closed enums, insta golden-vector tests per class family.
Out: canonical XML (Phase 05), FLAT/STRUCTURED/Web Template JSON (Phase 16 —
those are vendor formats, not ITS-JSON canonical JSON).

## Tasks

- [ ] Pin the exact ITS-JSON git commit hash in `docs/VERSIONS.md` and vendor `components/openehr_rm_1.1.0_all.json` as a reference schema for validation
- [ ] Implement `_type` discriminator dispatch (uppercase RM class name) for every closed enum (`DATA_VALUE`, `ITEM`, `CONTENT_ITEM`, `PARTY_PROXY`, `VERSION<T>`)
- [ ] Implement snake_case field renaming and null-omission for all RM structs via serde attributes or a custom serializer
- [ ] Implement `{_type, value}` UID serialization for the `OBJECT_ID` hierarchy
- [ ] Implement inline base64 encoding/decoding for `DV_MULTIMEDIA.data`
- [ ] Mark `_type` as required whenever the declared field type is abstract, matching the ITS-JSON rule
- [ ] Write insta golden-vector tests for rm.data_types (round-trip serialize -> deserialize -> equal)
- [ ] Write insta golden-vector tests for rm.data_structures, rm.common, rm.ehr, rm.demographic
- [ ] Validate a sample of serialized output against the vendored `openehr_rm_1.1.0_all.json` using `jsonschema`
- [ ] Add PORT STATUS trailers; update `docs/ROSETTA.md` with serde-specific quirks per class

## Exit criteria

- [ ] Every RM class round-trips through canonical JSON with `insta`-pinned output
- [ ] `_type` dispatch works for all five closed enums
- [ ] At least one golden vector per RM package validates against the ITS-JSON schema

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. Depends on Phase 03 being far enough along to serialize
meaningfully — can start on data_types/data_structures serde as soon as those
subtrees land, without waiting for all of rm.demographic.

# A1 Spec Audit — Verify + Fix — chapter `its-json`

- **Chapter:** ITS-JSON canonical-JSON schemas (RM Release-1.1.0 pin,
  commit 5acae05)
- **Date:** 2026-07-12
- **Scope:** all 42 requirements `its-json-R1 … R42`
- **Result (defer-nothing pass):** zero code defects — the wire contract is
  realized by the generated `#[derive(OpenEhrType)]` layer + the fail-closed
  typed validation (`validate.rs::run::<T>`) + the corpus/ITS-JSON fidelity
  gates; the one systemic deviation (unknown wire keys tolerated) is a
  documented, corpus-adjudicated superset (PORT NOTE F-04-02 in the derive).
  Two stray ADR citations scrubbed from files touched during verification.

## Verdict table (condensed)

| ids | classification | evidence |
|---|---|---|
| R1 | verified | a root without a versioned-root `_type` cannot commit (walker type conformance + the codec's `NotAStructureRoot` guard); every read/write fixture carries it (corpus gate) |
| R2 | verified | `_type` first in serialization (derive emit order) |
| R3 | verified-policy | unknown keys tolerated at deserialize — the documented corpus-adjudicated superset (PORT NOTE F-04-02: RM-version skew + SDK fixtures with stray keys); the archetype-constrained subset is closed-world-checked by the walker (ECC-VAL-119) |
| R4 | verified | present-but-wrong `_type` is a deserialize error (derive contract; fidelity gates) |
| R5–R9 | verified | monomorphic slots typed concrete in the generated model (EHR_STATUS.subject→PartySelf, category/language/territory→concrete) — foreign `_type` errors (ch1/ch3/ch20 audit work) |
| R10–R14 | verified | polymorphic slots (DATA_VALUE, PARTY_PROXY, uid, ITEM_STRUCTURE, CONTENT_ITEM) REQUIRE `_type`: the generated enum's missing-`_type` arm is a hard error naming the expected set |
| R15, R16 | verified | optional-`_type` two-member slots (name→DV_TEXT, hyperlink→DV_URI defaults) — the static-concrete fallback arm; corpus-proven |
| R17–R20 | verified | numeric JSON typing from the generated field types (i32/f64 per the BMM/BASE mapping); fidelity gates round-trip exact numbers |
| R21 | verified | `Array<Octet>` as base64 (derive + XML/JSON gates) |
| R22–R36, R38 | verified | required sets enforced as non-Option fields through the fail-closed typed validation (a missing mandatory attribute is a `does not conform to RM type` violation → 422); ECC-VAL + corpus |
| R37 | verified | present-empty lists rejected (`check_nonempty_lists`, ch3/ch8) — matching `minItems: 1` |
| R39 | verified | `None`/empty omitted, never `null` (derive emit) |
| R40 | verified | recursive thumbnail boxed + fully typed |
| R41, R42 | verified-structural | no pattern constraints in the 1.1.0 schemas (our RM-level temporal validity is a stricter-by-spec layer above the schema floor); flattened concrete definitions = the generated model's shape |

## Fixes applied

- ADR-citation scrubs in `openehr-rm/src/validate.rs` (ADR-008 → the
  spec-silence flag) and `openehr-flat/src/validation/mod.rs` (ADR-012 →
  the AOM2 §Rm_type_name citation + ECC-VAL-119).

## Deferred

None.

## Uncertain / runtime probes

None remaining.

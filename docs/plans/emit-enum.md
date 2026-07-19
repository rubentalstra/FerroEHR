# EMIT-ENUM — typed enumeration emission (implementation blueprint)

The main `emit` target renders BMM enumeration classes as transparent
primitive newtypes, dropping their literal sets. Fix: emit real typed Rust
enums, wire byte-identical. Blueprint researched 2026-07-19 against
`feat/adl2`; implement ON that branch (develop lacks the enumeration
loader + openehr-adl entirely). Deleted when the row closes.

## 1. Enumeration inventory (the loaded BMMs; 13 classes)

| Crate | Class | Backing | Items |
|---|---|---|---|
| openehr-base | VALIDITY_KIND | STRING | mandatory, optional, prohibited, disallowed |
| openehr-base | VERSION_STATUS | STRING | alpha, beta, release_candidate, released, build |
| openehr-rm | PROPORTION_KIND | INTEGER | pk_ratio..pk_integer_fraction = 0..4 |
| openehr-term | TERMINOLOGY_STATUS | STRING | trial, active, retired |
| openehr-am am14 | OPERATOR_KIND | STRING | op_eq…op_exponent (19) |
| openehr-am am24 | CONSTRAINT_STATUS | INTEGER | required..example = 0..3 |
| openehr-am am24 | VISIBILITY_TYPE | STRING | hide, show |
| openehr-lang | OPERATOR_KIND | STRING | eq..exponent (20 — different set from am14; separate crates, no conflict) |
| openehr-lang bmm3 | BMM_SCHEMA_METADATA_KEY / BMM_SCHEMA_STATE / BMM_ENTITY_METATYPE / BMM_OPERATOR_POSITION / BMM_PARAMETER_DIRECTION | STRING | (as declared) |

`naming::type_name` PascalCases all item names cleanly; no keyword
collisions.

## 2. Current defective emission — two shapes

- 12 classes → `#[serde(transparent)] struct X(String|i32)` via
  `Emission::Newtype` (`emit.rs:809-817`, `emit_newtype` L1780). Every
  transparent newtype in the generated crates is an enumeration (no
  genuine primitive aliases exist).
- **PROPORTION_KIND is extra-broken**: RM BMM lists it in
  `DV_PROPORTION.ancestors` (invariant-holder), so `decide` routes it to
  `Emission::PolyEnum` — it emits as a nonsense poly enum +
  `ProportionKindData`, is never used as a value, and `DV_PROPORTION.type`
  stays raw `i32`. Reclassify it too.

## 3. Wire tolerance — decision + evidence

Transparent newtypes accept ANY string/i32. **Out-of-range tolerance is
load-bearing**: `openehr-adl/src/source.rs:599` builds
`VersionStatus(token)` from arbitrary HRID version-status tokens; tests
assert `VersionStatus("rc")` and `VersionStatus("")` — not VERSION_STATUS
constants. Therefore every emitted enum carries an `Other(String)` /
`Other(i32)` catch-all with hand-written Serialize/Deserialize provably
byte-identical to the newtype (known variant ↔ name/value; Other ↔
verbatim), applied uniformly. Strict seam alongside: `TryFrom<&str>` /
`TryFrom<i64>` with per-enum typed error, never yielding Other. `Other`
documented with the flag: no openEHR spec governs an out-of-set value —
our own tolerance-preserving design (LANG BMM_ENUMERATION defines only
the listed constants).

## 4. Generated shape per enum

- STRING: `#[derive(Debug, Clone, PartialEq, Eq, Hash)]` + `Other(String)`;
  `as_str`, `from_wire(&str) -> Self` (total), `TryFrom<&str>`; manual
  Serialize = `serialize_str(self.as_str())`, Deserialize =
  `String::deserialize().map(from_wire)`.
- INTEGER: + `Copy`; `Other(i32)`; `value(self) -> i32` (by value — Copy),
  `from_value(i32) -> Self`, `TryFrom<i64>`; serialize_i32 / i32
  deserialize.
- Per-enum error `struct UnknownX(String|i64)` with hand-written Display +
  `std::error::Error` (no thiserror — keeps generated-crate deps
  unchanged), Debug derived (CI-deny lint).

## 5. Emitter change points (`crates/openehr-codegen/src/`)

1. `emit.rs:20` import `BmmEnumValue, BmmEnumeration`.
2. `emit.rs:764` `Emission` — add `EnumLiterals(&BmmEnumeration)`.
3. `emit.rs:780` `decide` — top guard (after is_mapped): class has
   `enumeration` → `EnumLiterals` (preempts Newtype AND the
   PROPORTION_KIND PolyEnum path; Newtype fallback stays).
4. `emit.rs:895-905` — `emit_enum_literals` arm + fn (mirror emit_newtype;
   reuse the value-resolution rule from `emit_rm_model.rs:211`).
5. `emit.rs:1989` `XmlType` — add `EnumLiterals { spec, rust,
   string_backed }`.
6. `emit.rs:2072` `concrete_carries_type` — enumeration classes return
   false (never dispatch targets; provably no output change).
7. `emit.rs:2104` `xml_types` — the EnumLiterals arm.
8. `emit_xml.rs:548/:780` — write/read arms (string: text element of
   as_str / from_wire; int: to_string / from_value). Only
   VALIDITY_KIND/VERSION_STATUS/PROPORTION_KIND are in the XML schema set;
   the current `.0` tuple access at `emit_xml.rs:553,785` breaks
   otherwise.

`emit_rm_model.rs` needs NO change (independent ENUMERATIONS table).

## 6. Consumer migration (hand-written sites only)

- `openehr-adl/src/cadl.rs:2513` strength_status → variants directly
  (+ test :2890 `ConstraintStatus(2)` → `::Preferred`).
- `openehr-adl/src/printer.rs:1040` strength_keyword takes
  `ConstraintStatus` BY VALUE (Copy; avoids trivially_copy_pass_by_ref at
  CI-deny) + call site.
- `openehr-adl/src/validate/conformance.rs:446` →
  `status.map_or(0, |s| s.value())`.
- `openehr-adl/src/assemble.rs:494` → `VisibilityType::from_wire(&s)`.
- `openehr-adl/src/source.rs:599` → `VersionStatus::from_wire(token)`;
  tests :697,706,711 → `Other("")`, `Other("rc")`, `::Alpha`.
- `openehr-adl/src/rules.rs:213`, `openehr-lang/src/bel/parser.rs:433`,
  `bel/mod.rs:354` → `OperatorKind::from_wire(..)` / `::ForAll`.
- `openehr-adl/tests/validation_phase1_cases.rs:257` →
  `VisibilityType::Hide`.
- NO change to `dv_proportion_impl.rs` (doc-comment ref only; the i32
  field + PK_* constants are a separate concern).
- Generated field-typed structs regenerate automatically.
- NOTE: A6's phase-2/conformance code may add further ConstraintStatus
  sites — re-inventory after A6 lands.

## 7. Gates

`emit` + `emit-xml` + `emit-rm-model` double-run byte-identical; zero
drift outside enum files; openehr-its fidelity gates + canonical-JSON
corpus pass UNCHANGED (a changed snapshot = changed wire = wrong);
workspace build/clippy/nextest green; fmt; ECC zero-drift at close.

# A1 Spec Audit — Verify + Fix — chapter `base-base-types`

- **Chapter:** BASE 1.3.0 base_types (identification, definitions, builtins)
- **Date:** 2026-07-11
- **Scope:** all 37 requirements `base-base-types-R1 … R37`
- **Result (defer-nothing pass):** 1 defect fixed (AQL archetype-id
  comparison was case-sensitive — the last unfolded composite-identifier
  seam). The identifier machinery was verified/fixed in chapter 7
  (rm-support) and chapter 1 (version ids); cross-references below.

## Verdict table (condensed)

| ids | classification | evidence / fix |
|---|---|---|
| R1–R5 | verified | `Uid` pattern dispatch (`uuid`/`iso_oid`/`internet_id` impls, mutually exclusive; empty fails all) — ch7 R1–R4 |
| R6 | verified-model | OBJECT_ID direct instances have no wire occurrence; the generated model carries the subtype set |
| R7–R9 | verified | `uid_based_id_impl.rs` root/extension/has_extension (derived XOR) |
| R10/R11 | verified | strict `ObjectVersionId::from_str` — 3-part + UID-typed parts (chapter-1 `version_id.rs` everywhere) |
| R12–R18 | verified | `version_tree_id_impl.rs` lexical form (1-or-3-part, numeric ≥ 1); branch semantics first-class since chapter 1 |
| R19/R20 | verified | ch7 fix: `is_valid_archetype_id` (numeric version; `.v1draft` rejected) |
| R21 | verified | ch7 fix: `is_valid_terminology_id` (+ the CNF space adjudication) |
| R22 | verified | `GenericId.scheme: String` non-optional |
| R23–R25 | verified | `object_ref_impl.rs` namespace regex; typed mandatory fields |
| R26 | verified | `party_ref_impl.rs` Type_validity |
| R27–R29 | verified | `LocatableRef.id: UidBasedId` narrowed; `as_uri` impl |
| R30 | verified | `TerminologyCode` mandatory fields (SM seams) |
| R31 | fixed-in-this-pass | the LAST case-sensitive composite-id seam: AQL archetype predicates now fold case (`lower() = lower()`, served by the new `idx_node_archetype_lower` functional index in the baseline); csid folded in ch7; OVID object ids normalize via `uuid` parse; If-Match compares parsed integers |
| R32 | verified | storage is byte-preserving everywhere; only comparisons fold |
| R33 | verified | all lexical productions ASCII-restricted |
| R34/R35 | verified | constants realized where consumed (`"local"` namespace/terminology id literals; UTF-8 default) |
| R36/R37 | verified | generated `ValidityKind`/`VersionStatus` newtypes carry the exact value sets (transparent enumerations) |

## Fixes applied

- **R31** — `app/ehrbase/src/aql/sql.rs::archetype_cond` folds case (BASE
  master05 §Composite Identifiers and Case) + `idx_node_archetype_lower`
  in the baseline (storage stays case-preserving).

## Deferred

None.

## Uncertain / runtime probes

None remaining.

# The generation subsystem, rewritten — design document

Owner directive (2026-07-19): a ground-up redesign + rewrite of
`openehr-codegen` + `openehr-derive` (the CODEGEN worklist row), designed
FIRST as this document. Evidence base: two read-only audits run 2026-07-19
on `feat/adl2` (the serde/canonical-JSON pipeline audit and the complete
`*_impl.rs` emittability audit), plus the emitter capabilities the ADL2 row
forced (cross-schema re-emission, back-reference cycle breaking,
constructibility proof, enumeration literals). Per the doc lifecycle this
file is the rewrite's working blueprint and is DELETED in the PR that
completes the row; durable outcomes land in `docs/architecture.md` +
`docs/PROGRESS.md`.

## 0. Why rewrite (the accretion diagnosis)

`emit.rs` alone now carries: BMM→Rust type emission, abstract-enum `_type`
dispatch, XML-type classification, cross-schema re-emission (least-fixpoint
closure), back-reference overrides + constructibility checking, newtype/
enum-literal decisions, and naming. `main.rs` carries schema assembly +
hand-written-module declaration (one real bug found + fixed there this
session). Every new spec need = another special case in the same two
files. The rewrite gives each concern a home and makes the invariants
tested properties instead of review-enforced hopes.

## 1. Target architecture — a four-stage pipeline

```
inputs (vendored BMM/XSD/OAS)          [stage 1: LOAD]
  → SchemaSet: per-component BMMs + include graph, verbatim
model analysis                          [stage 2: ANALYZE]
  → AnalyzedModel:
     - merged include-closures per emitted crate (the AM view ⊇ LANG view)
     - polymorphic seams (descendant sets per closure; cross-schema
       extension points → downstream re-emission sets)
     - ownership graph: cycles + the back-reference edges that break them
       (declarative override map, spec citation per entry)
     - constructibility proof (least-fixpoint; non-constructible = ERROR)
     - enumerations (names, values, backing types, defaults per the
       BMM_ENUMERATION rules)
     - constants, invariant expressions (see §4), function signatures
emission planning                       [stage 3: PLAN]
  → EmissionPlan per crate: every type with its decided shape
     (struct / closed enum / enum-literals / re-emitted twin / codec
     impls / xml impls), source-package-mirrored paths, import edges
rendering                               [stage 4: RENDER]
  → deterministic file writing; byte-stable ordering; the ONLY stage
     that produces text
```

Rules of the pipeline:
- Stage 2 outputs are plain data — unit-testable without rendering.
- All decision maps (back-references, overrides, allowlists) are
  **declarative data files with a spec citation per entry**, not code.
- **Tested emitter invariants** (each a test over stage-2/4 outputs):
  completeness (every loaded class is planned — nothing silently
  dropped), constructibility, byte-determinism (double-run identity),
  source-package mirroring, downstream-closure correctness (every
  cross-schema subtype reachable), zero-drift on the committed tree.

## 2. Placement + crate structure

- `openehr-codegen` moves to **`tools/openehr-codegen`** (dev tooling;
  nothing ships it), split into modules per pipeline stage
  (`load/`, `analyze/`, `plan/`, `render/`, plus `cli.rs`).
- **`openehr-derive` is RETIRED**: the audit confirms it is 100%
  mechanical serde plumbing with zero RM knowledge — the emitter renders
  the same impls directly (stage 4), and the proc-macro crate is deleted.
  The conformance-load-bearing behaviours it carries move into emitted
  code VERBATIM: `_type`-first ordering, None/empty omission, tolerant
  unknown keys (deliberate superset — RM-version skew), present-but-wrong
  `_type` = error / absent-`_type` rules per slot kind, interval
  `*_included`/`*_unbounded` defaults. Gate: the openehr-its fidelity
  suite unchanged.

## 3. The canonical-JSON codec question (serde audit, 2026-07-19)

**Finding that reframes the goal:** the measured JSON hot path
(comp-create/comp-read) is `serde_json::Value` passthrough end-to-end
(negotiate → storage codec → jsonb → reassemble → serialize); the typed
RM serde runs only at the XML edges and the typed validation tier
(`from_value::<T>(node.clone())`) plus the 104 polymorphic enums'
buffer-into-Value + second-parse dispatch. **A spec-type codec alone does
not speed up the hot path.** No committed profile attributes cost to
serde; the perf case as originally imagined is unproven.

Decision for the rewrite:
1. **Emit native `ToJson`/`FromJson` codecs anyway — for architecture and
   wire ownership, not claimed throughput.** Mirror the proven XML shape
   (482-LOC hand-written runtime under ~17k generated LOC): a
   `json/runtime.rs` (writer + borrowed-slice reader + primitive impls +
   `_type` dispatch convention) under emitted per-type impls. Wins that
   ARE real: the enum double-pass dies, the validation tier's
   `from_value + clone` dies, the `_type`/number/key-order contract
   becomes explicit emitted code instead of inherited serde behaviour,
   and serde/serde_json become removable from the spec crates entirely.
2. **Sequencing: Serialize side first, per-crate (base → rm → am),
   Deserialize second.**
3. **Prerequisite gate — byte-exact snapshots.** The current fidelity
   corpus gate is SEMANTIC equality (5 == 5.0 passes); byte identity is
   not proven anywhere today. Before any codec work: add byte-for-byte
   canonical-JSON snapshot gates over the corpus. A codec change that
   alters a byte fails.
4. **The two wire hazards, handled deliberately:**
   - Number lexemes: the Value passthrough preserves integer-vs-real
     distinctions BY ACCIDENT today; the codec must lock a deliberate
     number-formatting contract (Ryū-parity for f64; i64 paths never
     printing `.0`; DV_QUANTITY f64 vs DV_COUNT i64 verified per type)
     — the XML runtime's hand-patched `120.0` quirk generalises here.
   - Key order: emitted fixed order (`_type` first) — locked and
     snapshot-gated.
5. **Storage-codec conversion is a SEPARATE row** (not this rewrite):
   converting decompose/reassemble from Value trees to typed/streaming
   codecs is where any real throughput lives (comp-create-large,
   aql-ward), and it must be profile-driven (criterion micro-benches:
   `from_value::<Composition>` vs native FromJson; validation
   throughput). Register at pickup; do not fold in.

## 4. `*_impl.rs` rationalisation (audit, 2026-07-19)

Inventory: 65 files / 8,700 LOC, all in base+rm; 54% is tests; ~2,480
code LOC is the real target. Verdicts:

- **EMIT-NOW (cleanest wins):**
  - Terminology-identifier constants (2 files): the BMM carries the
    constant VALUES verbatim — emit them; the audit found the `valid_*`/
    `ALL_*` helper fns are DEAD (zero consumers) — drop them.
  - The serde plumbing (see §2 — derive retirement).
- **EMIT-WITH-NEW-INPUT (the structural win, ~700 LOC of thin
  delegators):** the BMM carries **155 invariant expressions** (loader
  already parses them; the emitter ignores them today). Bucketed: 84
  simple (`not X.is_empty`, `A xor B`, `X >= 0`) + ~45 medium
  (`valid_iso8601_date(value)`, count equalities) are mechanically
  emittable via a **small assertion-dialect parser** (the Eiffel/UML
  assertion surface — NOT base_expressions.g4; different dialect) that
  emits calls into the KEPT hand-written validator runtime
  (`validate.rs` push_* helpers + ISO-8601 validators). The 26 complex
  invariants (terminology service, quantifiers, repository access) stay
  hand-written — and are mostly unimplemented today, so emitting them is
  future capability, not deletion.
- **KEEP-HAND-WRITTEN (irreducible, ~1,530 LOC):** the real algorithms —
  `dv_ordered_impl` (civil-date math, ISO-8601 magnitudes, ordering
  authority), `interval_impl` (boundary algebra), item_table/history/
  proportion accessors, the shared validator runtime. BMM gives
  signatures, never bodies.
- **Lexical id parsers (3 files):** grammar-emittable in principle
  (base_lexer.g4 has the token rules) but not worth a bespoke
  grammar→parser emitter; keep hand-written unless the ADL2 pipeline
  later makes it free.
- Test hygiene (4,710 test LOC): a shared fixture/test-support module
  collapses repetition — worthwhile, but not emission.

## 5. Capabilities carried over (already built, redesigned in place)

These landed during the ADL2 row and become first-class stage-2/3
concepts with their own tests: cross-schema subtype re-emission
(downstream closure, source-package mirroring), back-reference cycle
breaking (declarative, spec-cited, constructibility-proved),
enumeration-literal emission (the EMIT-ENUM row's shape: typed enums +
tolerance-preserving `Other`, wire byte-identical), hand-written-module
declaration (files AND directories), the emit-rm-model surface
(attributes, multiplicity, descendants, generics, cardinality,
enumerations).

## 6. What the rewrite must NOT change

Wire bytes (all fidelity gates + new byte-exact snapshots + ITS-JSON
schema + XML c14n + a zero-drift ECC run), generated-output byte identity
at cutover (the codegen-drift gate proves the refactor half), the three
codegen hard rules (root CLAUDE.md), the versioning split, and the
dependency arrows (downstream re-emission only; upstream crates never
gain downstream knowledge).

## 7. Phasing (compiling, tested increments)

1. **R0 — byte-exact snapshot gates** (prerequisite; also closes the
   semantic-gate gap found by the audit).
2. **R1 — pipeline skeleton + move to tools/**: stages as modules,
   current behaviour preserved, output byte-identical (drift gate).
3. **R2 — decision maps to declarative data** + the tested emitter
   invariants (completeness, constructibility, determinism, mirroring).
4. **R3 — JSON codec, Serialize side** (per-crate) + derive retirement
   begins; fidelity + snapshots unchanged.
5. **R4 — JSON codec, Deserialize side** (native `_type` dispatch; the
   enum double-pass and validation `from_value` die); serde removed from
   spec crates; `openehr-derive` deleted.
6. **R5 — impl emission**: constants (+ dead-helper deletion), then the
   assertion-dialect invariant emitter over the validator runtime.
7. **R6 — close**: docs/architecture.md updated, this file deleted,
   ECC zero-drift, benchmarks re-run (honesty: expect ~no macro change;
   publish whatever is measured).

Follow-up rows registered at pickup, not folded in: storage-codec
typed/streaming conversion (the actual throughput candidate);
grammar-driven id-parser emission (only if ever justified).

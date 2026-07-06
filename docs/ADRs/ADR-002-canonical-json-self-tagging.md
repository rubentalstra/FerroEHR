# ADR-002: Canonical-JSON `_type` via self-tagging TypeTag fields and untagged closed enums

- **Status:** **superseded by ADR-004** (mechanism) and **ADR-008** (acceptance
  framing). The requirement — `_type` self-tagging + untagged closed enums on the
  wire — stands; the `TypeTag` field mechanism does not.
- **Date:** 2026-07-02

> ## ⚠️ AMENDMENT (2026-07-04/05, ADR-004 + ADR-008): TypeTag → `#[derive(OpenEhrType)]`; parity retired
>
> The spec crates are now **generated** from BMM (ADR-004), so the hand-authored
> `TypeTag<Self>` first-field ZST described below **no longer exists** (`grep -r
> TypeTag crates/` is empty). Canonical `_type` (de)serialization is supplied by
> `#[derive(OpenEhrType)]` (`openehr-derive`) on the generated types — a manual
> `Serialize` that emits `_type` first and a tolerant `Deserialize` that validates
> it, exactly the behaviour this ADR specified. Untagged closed enums dispatched
> on the payload's own `_type` (Decision §2) remain correct and are what the
> emitter produces. Also: `openehr-foundation` was folded into `openehr-base`
> (so `openehr_foundation::serde_support` is now in `openehr-base`), the schema
> path is `crates/openehr-its/schemas/json/openehr_rm_1.1.0_all.json`, and the
> "behavioural parity with EHRbase (P18)" acceptance bar was retired by ADR-008
> for openEHR CNF conformance + the fidelity round-trip gates.

## Context

ITS-JSON canonical JSON identifies every object's RM class with a `"_type"`
key (uppercase class name). The vendored schema
(`crates/openehr-its/schemas/openehr_rm_1.1.0_all.json`, pinned commit
`5acae056248e917a4b4c56f7e712f4fcfeb616a6`) defines `_type` as a `const`
property on **all 134 concrete class definitions** and defines **no entry at
all for abstract classes** (`DATA_VALUE`, `LOCATABLE`, `OBJECT_ID`, … are
flattened into the concretes). Stock EHRbase emits `_type` on essentially
every object it serializes, and behavioural parity with EHRbase at the REST
surface is this project's Stage-1 acceptance bar (P18).

The first P4 serde pass (multi-agent, 2026-07-02) left three mutually
inconsistent mechanisms in the tree, none complete:

1. `#[serde(tag = "_type")]` internally-tagged closed enums (~32 files) —
   correct output, but only covers *abstract-declared* slots; a field
   statically typed as a concrete class (e.g. `COMPOSITION.category:
   DV_CODED_TEXT`, any top-level `Composition`) emitted no `_type` at all.
2. Struct-level `#[serde(rename = "CLASS_NAME")]` (dozens of files) —
   experimentally verified to be a **no-op** on the wire
   (`.claude/agent-memory/rm-transcriber/feedback_serde_type_tag_pitfall.md`);
   it emits nothing.
3. Hand-written manual `Serialize`/`Deserialize` impls (parts of
   `openehr-base::identification`).

The struct-level mechanism decision was explicitly deferred in the agent
memos ("raise an ADR before rolling out") but the rollout happened anyway.
This ADR is that decision, made after the fact and applied uniformly.

Rust constraint: the orphan rule forbids `openehr-its` from implementing
`Serialize` for `openehr-rm`/`openehr-base` types, so whatever the mechanism
is, it must live on (or below) the crates that define the types. The phase
plan's wording "serde impls in `openehr-its`" was never implementable as
written; `openehr-its` instead owns the acceptance instrument (golden
vectors, schema validation, round-trip tests).

## Decision

1. **Every concrete RM/BASE class self-tags.** It implements
   `openehr_foundation::serde_support::TypeName` (`const NAME` = canonical
   class string, single-sourced from the file's existing `TYPE_NAME` const)
   and declares as its **first** field:
   `#[serde(rename = "_type", default = "TypeTag::new")] pub type_tag:
   TypeTag<Self>`. `TypeTag` always serializes `T::NAME`, tolerates a
   missing `_type` on input, and errors on a mismatched one. The
   function-path `default = "TypeTag::new"` form is mandatory (bare
   `default` makes serde's derive add a spurious `T: Default` bound on
   generic containers such as `ORIGINAL_VERSION<T>`).
2. **Closed subtype-set enums are `#[serde(untagged)]`.** The former
   `#[serde(tag = "_type")]` + per-variant renames are removed; dispatch is
   driven by each variant payload's own `TypeTag`, which fails
   deserialization on a wrong `_type`, making untagged probing tag-driven
   (verified even for structure-identical classes, `DV_DATE` vs `DV_TIME`).
   Variant order still lists structurally richer payloads first
   (`DvCodedText` before bare `DvText`) so tag-less input in
   concrete-declared slots resolves correctly.
3. **Abstract classes and embedded `*Data` structs carry no tag**, matching
   the schema's absence of abstract definitions. Exception: a `*Data`
   struct that doubles as the *bare instance of a concrete parent class*
   (`DvTextData` for a plain `DV_TEXT`) implements `TypeName` so the enum's
   bare variant can carry `TypeTag<DvTextData>` next to the
   `#[serde(flatten)]`ed data.
4. **`TypeTag` lives in `openehr-foundation::serde_support`** — the
   dependency root, reachable by `openehr-base` and `openehr-rm`; flagged
   `PORT NOTE` as infrastructure, not a spec class.
5. **`openehr-its` owns the acceptance instrument**: the full-RM coverage
   test enumerating all 134 schema definitions. Each definition must have a
   fixture with an insta golden vector, jsonschema validation, and
   serialize/deserialize round-trip coverage.

## Consequences

- `_type` appears on every serialized object, first in key order, matching
  stock EHRbase output byte-for-byte in key placement — the P18 parity diff
  never has to normalize `_type` away.
- One convention replaces three; the no-op struct-level renames are deleted,
  the 32 tagged enums are converted, and the manual impls in
  `identification` are replaced by tags.
- Constructors gain one `type_tag: TypeTag::new()` line per concrete class
  (mechanical churn, done in the same pass).
- Untagged enums produce weak error messages ("data did not match any
  variant") on malformed input. Accepted for P4; if REST-surface error
  parity demands better diagnostics, hand-written deserializers can be
  layered in `openehr-its` at P17/P18 without changing the wire format.
- Input with a *missing* `_type` in an **abstract** slot falls back to
  structural (declaration-order) matching instead of erroring. ITS-JSON
  declares such input invalid, so this leniency is harmless for valid data;
  strictness can be added later without breaking valid round-trips.
- A zero-sized field participates in every struct literal; `TypeTag`'s
  `PartialEq`/`Ord`/`Hash` are hand-written as inert so derived semantics
  on containing classes are unchanged.

## Alternatives considered

- **Keep the internally-tagged enums and hand-write `Serialize` for every
  standalone concrete class.** Rejected: two coexisting mechanisms, ~40+
  hand-written impls of pure boilerplate, and a duplicate-`_type`-key
  hazard anywhere a self-tagged payload sits inside a tagged enum.
- **Spec-minimal `_type` (emit only in abstract-declared slots).** Rejected:
  valid against the schema but diverges from what stock EHRbase actually
  returns, turning every P18 parity diff into noise the harness would have
  to normalize — betting the acceptance instrument on a normalization we'd
  have to write and trust.
- **A proc-macro derive (`#[derive(RmSerde)]`) generating manual impls.**
  Rejected for P4: a new proc-macro crate is heavier infrastructure than a
  ZST field, harder to audit for spec fidelity, and not needed to reach
  correct output; can be revisited at Stage 3 if the field is judged noisy.
- **Serialize via wrapper newtypes in `openehr-its`.** Rejected: the
  orphan rule workaround would require wrapping the entire nested object
  graph, effectively duplicating the RM type tree.

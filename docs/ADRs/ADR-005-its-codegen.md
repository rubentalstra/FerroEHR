# ADR-005: Spec-driven code generation of the ITS surfaces (XML + REST)

- **Status:** accepted
- **Date:** 2026-07-04
- **Extends:** ADR-004 (spec-driven codegen of the RM/BASE/AM crates from BMM).
- **Supersedes (in part):** the "ITS-XML is hand-written, the schema is a
  *validation target, not a codegen source*" rule in
  `.claude/rules/serialization.md`, and the "REST server is code-first via
  `utoipa`" framing of phase P11 — as *methods*. The behavioural targets they
  set (wire parity with stock EHRbase; the ITS-REST 1.0.3 contract) stand.

## Context

ADR-004 established that the openEHR **spec** layer is *generated* from the
vendored machine-readable meta-model (BMM), not hand-transcribed, because the
spec is deterministic and hand-work is slow and error-prone. That decision
covered the RM/BASE/AM **type** layer (BMM → structs/enums).

The **ITS** (Implementation Technology Specifications) surfaces are the *other*
deterministic, machine-readable openEHR artifacts, each with its own vendored
schema:

- **ITS-XML** — the canonical-XML XSDs (`specifications-ITS-XML`).
- **ITS-REST** — the OpenAPI (OAS) contract (`specifications-ITS-REST`), which
  even ships a dedicated `-codegen` variant per API group.
- **ITS-JSON** — the canonical-JSON Schemas (validation oracle; the JSON *model*
  is already the BMM types + `#[derive(OpenEhrType)]`).
- **ITS-BMM** — the meta-model itself (already ADR-004's codegen input).

The forcing question: are the XML and REST surfaces hand-written (per the
original P05/P11 framing), or generated from their vendored schemas, the same way
ADR-004 generates types from BMM? The prior rules said hand-written for XML and
code-first for REST. That predates the working ADR-004 generator and the
completed ITS vendoring, and it is the slow/error-prone path ADR-004 set out to
escape. openEHR publishing a `-codegen` OAS variant is a direct signal the
authors *intend* these to drive a generator.

## Decision

**Generate both ITS wire surfaces from their vendored schemas, extending the
existing `openehr-codegen` tool.** Concretely:

1. **One generator, two new emit targets.** `openehr-codegen` gains
   `emit-xml` and `emit-rest` subcommands beside `emit`, reusing its BMM model
   loader, naming, and file-writing scaffolding. The generated code is written
   into `openehr-its` under `src/**/generated/`, marked `// @generated … DO NOT
   EDIT`, and never hand-edited (change the emitter and regenerate). A
   non-wiping writer manages only the `generated/` subtree so the hand-written
   runtime survives regeneration.

2. **ITS-XML — generate `ToXml`/`FromXml` on the existing RM/BASE types**, one
   model with a second wire format (not a parallel XSD-shaped type hierarchy).
   - Rust facts (field idents, `Option`/`Vec`, enum variants, generics) come
     from the BMM model (`Model::xml_types`); the *wire* shape BMM does not
     encode — element order, attribute-vs-element split, `xsi:type` slots —
     comes from a small XSD reader (`openehr-codegen::xsd`, `roxmltree`).
   - Serialization is explicit generated impls over a hand-written runtime
     (`openehr-its::xml::runtime`, `quick-xml`), **not** a serde derive:
     `xsi:type` attribute dispatch on abstract/polymorphic slots, element
     ordering, and the attribute/element split exceed what serde + quick-xml
     express. `xsi:type` is emitted iff a value's concrete type differs from its
     declared slot type; enums dispatch inbound `xsi:type` through a full
     descendant→direct-variant map so a deep type (`DV_CODED_TEXT` in a
     `DATA_VALUE` slot) routes correctly.
   - `Hash<String, String>` uses the openEHR `StringDictionaryItem` shape
     (`<field id="key">value</field>`). `DV_MULTIMEDIA.data` (`Array<Octet>`)
     is inline base64 text.

3. **XML wire lineage — one impl set serves both namespaces.** The RM-instance
   wire shape is identical across ITS-XML v1 (`…/v1`) and v2 (`…/v2`); they
   differ only by the root `xmlns`, selected at serialize time (`Namespace`).
   A second impl set would be a duplicate-`impl` conflict. v1 — what stock
   EHRbase emits via `archie`, the Stage-1 parity target — is generated from the
   v1 XSD; the v2 XSDs stay vendored for a future v2-specific trait should the
   shape ever diverge.

4. **ITS-REST — spec-first: the vendored OAS is the source of truth.** The
   generator reads the `-codegen` OAS bundles and emits the Rust *contract* into
   `openehr-its`: request/response DTOs, param structs, a server trait per API
   group, and an axum router builder. RM payload types are **not** re-emitted —
   an OAS schema that names an RM class resolves to `openehr_rm::prelude` (the
   same cross-crate resolution ADR-004 uses). `ehrbase-rest` *implements the
   generated trait*; the handler bodies are the ported EHRbase Java logic (the
   only non-deterministic part — a generator cannot produce behaviour). An
   optional `utoipa` pass in `ehrbase-rest` may emit *our* OAS and diff it
   against the vendored upstream as a CI drift-check — a parity signal, never
   the source of truth. This runs spec→code, the opposite of the demoted
   code-first framing.

5. **ITS-JSON stays a validation oracle**, not a code source: the JSON model is
   already the BMM types + `OpenEhrType`. `openehr-its::json` validates output
   against the vendored ITS-JSON schema.

6. **Generated vs hand-written split (as ADR-004).** Generated impls carry the
   `// @generated` header and no PORT STATUS trailer; the hand-written runtime
   (`xml/runtime.rs`, the REST extractors/error-mapping, entry points) carries
   the trailer and annotation vocabulary.

## Consequences

- **Easier:** a spec-version bump is a re-run, not a rewrite; the XML/REST
  surfaces cannot silently drift from the vendored schemas (a CI
  regenerate + `git diff --exit-code` step enforces this); the team writes only
  the non-computable parts — spec behaviour (ADR-003) and the ported EHRbase
  handler logic.
- **Verified:** the XML surface round-trips the 48-composition openEHR corpus
  (RM → XML → RM → XML stable) with correct `xsi:type` dispatch, `archetype_
  node_id` attributes, and element order.
- **Harder / follow-on, honestly:**
  - The strongest XML parity gate — parse stock **EHRbase's own** XML → RM →
    re-serialize → **C14N**-compare (`xmllint --c14n`) — is not yet wired; the
    current gate proves internal `ToXml`/`FromXml` consistency, not byte-parity
    with EHRbase.
  - Whole-number `f64` formatting (`120.0` vs Rust's `120`) is handled but
    warrants an exact-number-parity pass against the corpus.
  - `Hash<String, ComplexType>` (archetype-resource `translations`/annotations)
    and `serde_json::Value` slots (ADR-004 monomorphization artifacts) have no
    RM canonical-XML shape and are documented scope boundaries, not serialized.
  - The XSD reader and the emitter carry small hardcoded decisions (the RM
    instance file set, the attribute rule) slated to join ADR-004's
    `codegen.toml` override layer.
  - The REST generator must tolerate the RM-version divergence (the OAS
    references RM 1.1.0-era shapes; our RM is 1.2.0) — a Stage-1 parity
    consideration shared with the XML gate.

## Alternatives considered

- **Keep ITS-XML hand-written** (the prior rule). Rejected: ~130 classes ×
  two directions of dense (de)serialization is exactly the slow, error-prone
  hand-work ADR-004 escaped, and the XSD makes the wire shape deterministic.
- **Generate a parallel XSD-shaped struct hierarchy** and map it to/from the RM
  types. Rejected: it duplicates the entire model and reintroduces the
  two-model maintenance burden ADR-004 removed — the RM types are already the
  one model; XML is just a second serialization of them.
- **A serde-based XML derive** (`quick-xml` serialize feature). Rejected: it
  cannot express `xsi:type` attribute dispatch on abstract slots, the
  element-vs-attribute split, or strict child ordering — the defining features
  of openEHR canonical XML.
- **Two `ToXml` trait families (v1 + v2)** to serve both namespaces. Rejected
  for now: the RM-instance wire shape is identical bar the root `xmlns`, so one
  impl set + a namespace parameter suffices; a second trait is dead weight until
  a real v2 structural divergence appears.
- **REST code-first via `utoipa`** (the P11 framing): hand-write DTOs + handlers,
  emit *our* OAS. Rejected as the primary path: it hand-writes every DTO and
  lets our contract drift from openEHR's authoritative OAS. Kept only as an
  optional drift-check running the other direction.
- **A third-party OAS generator** (`openapi-generator`'s rust-axum). Rejected:
  it is a heavyweight external (Java) toolchain producing non-idiomatic output
  we would not control, against the in-house, deterministic, single-generator
  ethos of ADR-004.

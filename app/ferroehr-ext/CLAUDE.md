# `ferroehr-ext` — the optional-integration crate (feature-gated)

The carve-out target for the platform's OPTIONAL integrations (#1890):
`fhir` (mapping engine, outbound publisher, feeder-audit probe, the typed
FHIR surface — the ATNA `AuditEvent` renderer and the
terminology-response decoder), `events` (subscriptions + publisher
transports), `multimedia` (blob store/offload). One ADDITIVE cargo feature
per integration; the shipped binary builds all-on, slim deployments compile
integrations out.

- **Dependency arrows:** `ferroehr → ferroehr-ext` (optional,
  feature-forwarded) → `crates/openehr-*` as needed — never a cycle, never a
  dependency on `ferroehr`/`ferroehr-rest`. What a module needs from the
  service layer arrives as PARAMETERS/callbacks at the seam, decided per
  extraction child (#1892–#1894); service-coupled orchestration glue stays in
  `ferroehr` behind `cfg(feature)`.
- **No openEHR spec governs these surfaces** — flag every behaviour decision
  as our own design; vendor implementations are prior art only. FHIR wire
  facts still cite official HL7/docs.rs sources.
- **Two FHIR release identities, deliberately** (`src/fhir/mod.rs` §Release
  identity): the CONNECTOR speaks **R4** (its wire is `/fhir/r4`; every
  resource it builds is outside R4B's changed set, so the documents are valid
  under either release) and cites `hl7.org/fhir/R4/…`; the TERMINOLOGY
  decoder (`fhir::terminology`) speaks **R4B** because the release belongs to
  the external server it reads, and cites `hl7.org/fhir/R4B/…`. Keep a new
  citation on the side its subsystem is on.
- Heavy external model dependencies (a generated FHIR model, brokers,
  codecs) land HERE, never in `ferroehr`. **`fhir-model` (fhir-sdk's `r4b`
  model generation) is named ONLY in this crate's `fhir` module** — no other
  crate may name a `fhir_model` type; the platform seams take the neutral
  descriptor structs (`fhir::audit::AuditRecord`) and get plain JSON / plain
  views back.
- Zero re-exports. The serde CONFIG sections stay in the `ferroehr` config
  tree (they carry `Secret`/`SecretUrl` and the tree's redaction semantics);
  this crate takes plain runtime parameter structs at construction
  (`BlobStoreParams`, the events/AMQP url, `MappedSubject`) — the platform's
  gated glue maps config → params.
- Features are additive — no `compile_error!` pairs; the `--all-features`
  workspace lanes stay valid.
- Gates: `cargo clippy -p ferroehr-ext --all-targets --all-features` +
  `cargo nextest run -p ferroehr-ext --all-features`, plus a
  `--no-default-features` check lane (the slim build).

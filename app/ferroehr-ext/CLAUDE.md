# `ferroehr-ext` — the optional-integration crate (feature-gated)

The carve-out target for the platform's OPTIONAL integrations (#1890):
`fhir` (mapping engine, outbound publisher, feeder-audit probe, the
terminology external-FHIR client candidates), `events` (subscriptions +
publisher transports), `multimedia` (blob store/offload). One ADDITIVE cargo
feature per integration; the shipped binary builds all-on, slim deployments
compile integrations out.

- **Dependency arrows:** `ferroehr → ferroehr-ext` (optional,
  feature-forwarded) → `crates/openehr-*` as needed — never a cycle, never a
  dependency on `ferroehr`/`ferroehr-rest`. What a module needs from the
  service layer arrives as PARAMETERS/callbacks at the seam, decided per
  extraction child (#1892–#1894); service-coupled orchestration glue stays in
  `ferroehr` behind `cfg(feature)`.
- **No openEHR spec governs these surfaces** — flag every behaviour decision
  as our own design; vendor implementations are prior art only. FHIR wire
  facts still cite official HL7/docs.rs sources.
- Heavy external model dependencies (a generated FHIR model, brokers,
  codecs) land HERE, never in `ferroehr` (#1885 adopts `fhir-sdk` in this
  crate when it lands).
- Zero re-exports; config types move with their integration and the
  `ferroehr` config tree composes them.
- Features are additive — no `compile_error!` pairs; the `--all-features`
  workspace lanes stay valid.
- Gates: `cargo clippy -p ferroehr-ext --all-targets --all-features` +
  `cargo nextest run -p ferroehr-ext --all-features`, plus a
  `--no-default-features` check lane (the slim build).

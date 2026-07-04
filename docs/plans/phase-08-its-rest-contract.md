# Phase 08 — ITS-REST contract (generated)

- Status: **done** — GENERATED from the vendored OAS (ADR-005)
- Build order: complete (spec/ITS foundation; last of the generated layer)
- Decisions: ADR-005

## Outcome

`openehr-codegen`'s `emit-rest` target generates the openEHR **ITS-REST 1.0.3
contract** into `openehr-its/src/rest/generated/`, spec-first from the vendored
`-codegen` OpenAPI bundles (`crates/openehr-its/vendor/rest-oas/`): per API group
(admin/definition/demographic/ehr/query) it emits the transport **DTOs** (the
non-RM component schemas), per-operation **param structs**, an
`#[async_trait]` **server trait**, and a **`ROUTES`** table — **96 operations**.
RM payload schemas resolve to `openehr_rm`/`openehr_base` (not re-emitted); field
names are idiomatic snake_case with `#[serde(rename)]`. The hand-written
`rest/runtime.rs` provides `ApiError: IntoResponse`.

This is the **contract** only. The server that *implements* these traits is the
`ehrbase-rest` application phase (**P11 — REST server foundation + auth**), where
`ehrbase-rest` provides `impl {Group}Api for AppState` with the ported EHRbase
behaviour + auth.

## Verification

`openehr-its` compiles + clippy-clean; `tests/rest_contract.rs` (DTO serde +
route table + trait nameable) green. Regenerate with
`cargo run -p openehr-codegen -- emit-rest`; the `codegen-drift` CI job guards it.

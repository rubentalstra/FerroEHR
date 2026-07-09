# ADR-011: App-crate redesign — compile-time-complete services, no stub backend, pure-SM native API

- **Status:** accepted
- **Date:** 2026-07-09
- **Amends:** ADR-010 (the SM trait layer stays; its *packaging* changes).

## Context

The SM spec itself mandates the target shape
(`docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc`
§General Assumptions): "a formal, abstract definition of the platform
interfaces … so as to be able to state the formal interface call semantics
**independent of any particular implementation technology**", with REST as
one *protocol adapter* among many (SOAP, protobuf, Kafka, …) reaching "the
nominal 'native API'". A native API whose signatures carry ITS-REST types
violates that sentence.

After SM-1…SM-4 the service seam works but the owner rejected three
structural smells, all real:

1. **Wire types leak into the native API.** `ehrbase-sm` trait methods take
   generated ITS-REST `*Params` structs and return
   `openehr_its::rest::runtime::ApiError` — so "endpoint logic" is smeared
   across three crates (`-sm` signatures, `-rest` dispatch, `ehrbase`
   impls + an `api/*` forwarding layer), and reading one operation means
   hopping all three.
2. **Default-`NotImplemented` trait methods + `StubBackend`.** Every trait
   method has a `Err(ApiError::NotImplemented)` default body. A forgotten
   implementation is a silent runtime 501 instead of a compile error; the
   codebase "looks unimplemented" while being fully implemented; and the
   `Arc<dyn Backend>` + stub injection pattern is bootstrap-era (P11)
   machinery whose reason (no service existed yet) is gone.
3. **Boilerplate.** Re-export shims (`ehrbase-rest::{backend,response}`),
   nine per-test-file `MockBackend`s each needing empty impls for every new
   trait, and pure-delegation `service/api/*` modules.

## Decision

1. **`ehrbase-sm` becomes the SM interface catalog, transcribed literally**
   (owner ruling 2026-07-09: the specs are the shape; internal behaviour
   preservation is NOT a constraint — greenfield). Every trait carries its
   SM interface's **exact call set**: spec call names (`create_ehr_with_id`,
   `get_composition_latest`, `commit_contribution`,
   `get_party_relationship_at_time`, …), spec parameter names and types
   (`UUID`→`uuid::Uuid`, `Iso8601_date_time`, `PARTY_REF`,
   `UPDATE_VERSION<T>` as the commit envelope, the
   `item_offset`/`items_to_fetch` cursor = `Page`), spec returns
   (`EHR_SUMMARY`, RM objects / canonical values, `RESULT_SET`), spec
   pre/post-conditions in doc-comments, and a new `SmError` over
   `CallStatusType` realizing the `I_STATUS` protocol. Zero
   `openehr_its::rest` imports. The SM's `I_EHR` accessor is realized as a
   generic handle (`i_ehr(ehr_id) -> IEhr<'_, S>` exposing
   `ehr_status()/directory()/compositions()/contributions()` sub-handles) —
   the literal shape, and good Rust. Adapter-support calls that the SM does
   not define (`*_latest_meta` for 412 decoration, tag CRUD) move to a
   clearly-separated `adapter` extension trait, PORT-NOTEd (ITS-REST
   extensions, not SM calls).
   **The one preserved behaviour is the wire**: the ITS-REST adapter must
   still speak ITS-REST 1.0.3 exactly (that is what a protocol adapter *is*
   per master02) — the ECC zero-drift gate (211/318) remains the invariant
   while every internal signature breaks freely.
2. **Compile-time completeness.** All default method bodies are removed:
   implementing a service trait requires implementing every method.
   `StubBackend` is deleted. A missing method is a build error, never a
   silent 501.
3. **Generic adapter state, no trait objects.** `Backend` (renamed
   **`Platform`** — the SM platform is the set of services) remains the
   supertrait union, but the adapter becomes generic:
   `AppState<S: Platform>` / `fn build<S: Platform>(…)`, monomorphized in
   the binary over the concrete `EhrbaseService`. No `Arc<dyn Backend>`,
   no injection indirection.
4. **Boilerplate deleted.** The `ehrbase-rest::{backend,response}` re-export
   shims go (consumers import `ehrbase_sm::…` directly). The nine test
   mocks collapse into one shared `test-support` mock behind a feature/dev
   crate. The `ehrbase::service::api/*` pure-delegation modules go — trait
   impls live beside the service logic they expose.
5. **Dependency direction unchanged** (`ehrbase-rest → ehrbase-sm ←
   ehrbase`; the binary in `ehrbase` instantiates the generic router).
   No new crates.

## Consequences

- Better: one place per operation (adapter maps, service implements);
  compile-time completeness; the SM layer is finally protocol-free (what
  ADR-010 promised); dramatically less mock/shim noise.
- Harder: one large mechanical refactor (dispatch generics, param-mapping
  moves into `-rest`, mock consolidation); executed as SM-4's closing wave
  behind the usual gates (full suite + ECC zero-drift 211/318).
- Rejected alternatives: `-rest` depending on the concrete service
  (dependency cycle; kills adapter reuse); keeping dyn + defaults
  (the owner's rejected status quo); per-interface handle structs
  (more plumbing than the supertrait bound buys).

## Execution

SM-4 wave 2 (`docs/plans/sm-phase-04-terminology-admin.md`): (a) structural
— defaults/stub/shims/dyn removed, `Platform` generics, mock consolidation;
(b) purity — `SmError` + native params replace wire types in `ehrbase-sm`.
Gates: workspace green + ECC zero-drift; wire behaviour byte-identical.

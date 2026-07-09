# ADR-011: App-crate redesign — compile-time-complete services, no stub backend, pure-SM native API

- **Status:** accepted
- **Date:** 2026-07-09
- **Amends:** ADR-010 (the SM trait layer stays; its *packaging* changes).

## Context

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

1. **`ehrbase-sm` becomes pure SM.** Trait signatures exchange only
   SM-native types: the existing `CallStatusType`-based error (a new
   `SmError { status: CallStatusType, message }` replaces `ApiError` in
   every signature) and plain/native parameter types (no generated
   `*Params`). The ITS-REST adapter (`ehrbase-rest`) owns the whole
   wire↔native mapping: params decoding, `ApiError` construction (via the
   one SM→HTTP table), headers. `ehrbase-sm` keeps zero `openehr-its::rest`
   imports.
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

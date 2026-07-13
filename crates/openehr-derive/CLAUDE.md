# `openehr-derive` — `#[derive(OpenEhrType)]` (proc-macro, hand-written)

The canonical-JSON `_type` discipline for every generated spec type:
a manual `Serialize` that emits `_type` first and omits `None`/empty, and
a tolerant `Deserialize` that validates `_type` when present (ADR-004).

- **This macro defines wire behaviour for the entire spec layer.** Any
  change to what it emits or accepts is a canonical-JSON conformance
  change: verify against the ITS-JSON schema oracle and the corpus
  round-trip fidelity gates in `openehr-its` before committing, and cite
  the ITS-JSON/RM spec section for the behaviour.
- Keep it a thin, dependency-light proc-macro: `_type` tagging and
  serde plumbing only — no RM knowledge, no validation logic (invariants
  live in `*_impl.rs` files; schema validation lives in `openehr-its`).
- Tolerance rules are deliberate: unknown `_type` on a concrete slot is an
  error; absent `_type` is accepted where the slot type is unambiguous —
  do not tighten or loosen without an ECC/fidelity-gate run.
- Gates: `cargo clippy -p openehr-derive --all-targets` +
  `cargo nextest run -p openehr-derive`, plus the `openehr-its` fidelity
  gates for any behaviour change.

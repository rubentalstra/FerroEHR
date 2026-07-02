---
name: serde-not-yet-wired
description: openehr-foundation and openehr-base have no serde dependency in Cargo.toml as of P1; use a TYPE_NAME const, not #[serde(rename)], for the _type discriminator until P4 wires it in.
metadata:
  type: project
---

As of 2026-07-02 (P1, Foundation + Identification phase), neither
`crates/openehr-foundation/Cargo.toml` nor `crates/openehr-base/Cargo.toml`
lists `serde` in `[dependencies]`, even though `serde` is pinned in the root
workspace `[workspace.dependencies]`. The existing precedent files in
`openehr-foundation/src/primitive_types/` (`any.rs`, `ordered.rs`,
`string.rs`, etc.) confirm this deliberately — none of them derive
`Serialize`/`Deserialize` or reference serde at all.

**Why:** Canonical JSON serialization (ITS-JSON, `_type` discriminator
wiring) is its own phase, P4, scheduled after P1-P3
(`PORT_MASTER_PLAN.md` §10). Adding a serde derive in P1 would require
editing `Cargo.toml`, which per-file transcription tasks are typically
instructed not to touch, and would reference a crate that isn't a declared
dependency yet — a different and worse kind of incompleteness than a
`todo!()` body.

**How to apply:** When a transcription task's instructions say "derive
Serialize/Deserialize with `#[serde(rename = "...")]` to the canonical
`_type` name" but also say not to touch `Cargo.toml`, check
`crates/openehr-base/Cargo.toml` and `crates/openehr-foundation/Cargo.toml`
first. If `serde` still isn't a `[dependencies]` entry, do not add the
derive. Instead, add a `pub const TYPE_NAME: &str = "THE_SPEC_NAME";` to the
concrete class's file, with a doc comment/TODO(port) explaining the
discriminator will become a real `#[serde(rename = ...)]` once P4 wires the
dependency in. This was the resolution used for the whole
`base_types.identification` package
(`crates/openehr-base/src/identification/`) — check whether this const has
since been superseded by real serde derives before assuming it's still the
pattern; if `Cargo.toml` now lists `serde` for the crate, the derive should
be used directly instead of the const workaround.

See also [[base-identification-package-shapes]] for the specific file this
pattern was established in.

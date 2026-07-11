---
name: crate-scaffold
description: >
  Creates a new crates/<name>/ directory with a Cargo.toml wired to the
  workspace (package fields, [lints] workspace = true, path deps per the
  Section 9 dependency arrows) and a doc-comment-only lib.rs. Use when a
  phase's task list calls for standing up a crate that does not exist yet.
allowed-tools: [Read, Write, Bash]
argument-hint: "<crate-name, e.g. ehrbase-rest>"
---

> **⚠️ ADR-004 / naming:** crate names split `openehr-*` (spec) / `ehrbase-*`
> (application); the binary is `ehrbase`. See `docs/architecture.md` (the crate
> map) for the current names (e.g. `openehr-term`, `openehr-lang`, `openehr-am`,
> `openehr-query`, `ehrbase-rest`, `ehrbase`). Do not scaffold
> `openehr-foundation` (folded into `openehr-base`) or the retired old names.
> Generated spec crates get their `lib.rs`/`src` from `openehr-codegen`, not
> this skill.

# /crate-scaffold

Stands up an empty, workspace-wired crate skeleton. Used mainly in Phase 0
for the ten openEHR spec crates and three server crates, but any later phase
that needs a new crate uses this too.

## Steps

1. **Look up the crate in `docs/architecture.md`** (the workspace layout +
   crate map) to get its correct dependency arrows (which other workspace
   crates it depends on) and its one-line purpose comment.
2. **Create `crates/<name>/Cargo.toml`**:
   - `[package]` with `name = "<name>"`, and `version`, `edition`,
     `rust-version`, `license`, `authors`, `repository` all as
     `.workspace = true` (inherit from `[workspace.package]` in the root
     `Cargo.toml` — never hardcode these per-crate).
   - `[lints] workspace = true`.
   - `[dependencies]`: path deps on whichever other workspace crates Section
     9 lists (`path = "../<other-crate>"`, `version.workspace = true` where
     the root manifest also publishes a version), plus any
     `[workspace.dependencies]` entries the crate is known to need
     immediately (e.g. `serde` for a spec crate that will serialize). Do not
     pre-add dependencies "just in case" — add them when a task actually
     needs them.
   - If the crate is `ehrbase` (the application binary), also add
     `[[bin]] name = "ehrbase" path = "src/main.rs"`.
3. **Create `crates/<name>/src/lib.rs`** (or `main.rs` for the `ehrbase` binary)
   containing only a top-of-file doc comment: the crate's one-line purpose
   (from Section 9), which spec component or Maven module it corresponds to,
   and a note that it starts empty pending its phase. No `mod` declarations,
   no placeholder types — an empty crate should compile as an empty crate.
4. **Add the crate to the root `Cargo.toml`'s `[workspace.members]`** if it
   is not already covered by a glob.
5. **Verify** with `cargo check -p <name>` that the new crate builds on its
   own before reporting done.

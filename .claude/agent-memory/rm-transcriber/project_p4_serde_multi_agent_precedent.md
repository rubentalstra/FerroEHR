---
name: project-p4-serde-multi-agent-precedent
description: Precedent from the first P4 canonical-JSON serde-annotation pass over crates/openehr-rm/src/{ehr,demographic,integration,support}/ (34 files) — multi-agent worktree coordination detection, the untagged-vs-tagged enum decision for Party/Actor, the #[serde(transparent)] VERSIONED_X shape, and confirming that "crate must stay compiling" can be trivially true (nothing wired yet) rather than a real constraint.
metadata:
  type: project
---

Ran the first real (not `TYPE_NAME`-const-only) serde-derive pass in
`openehr-rm`, scoped to `ehr/` (20 files), `demographic/` (13),
`integration/` (1), `support/` (0 structs — traits/doc-only, correctly a
no-op). This is the reusable account of the judgment calls made, not a
restatement of what serde attribute goes where (that's in the code itself
and in `docs/ROSETTA.md`).

**Why this matters for future transcribers:** four genuinely reusable
patterns came out of this pass — three about serde shape decisions the
spec itself doesn't dictate, one about how to behave inside a
concurrently-running multi-agent wave. Read this before doing a serde
pass anywhere else in the workspace.

**How to apply:** any future "add serde derives to package X" task should
(1) check for sibling worktrees before assuming you're the only one
touching the dependency graph, (2) reuse the untagged/tagged enum decision
tree below rather than re-deriving it, (3) reuse the `#[serde(transparent)]`
treatment for `VERSIONED_X` newtype bindings, and (4) not panic about
"the crate must compile" if `lib.rs` genuinely has zero `mod` declarations
— check that first, it changes what "must compile" even means.

1. **Multi-agent worktree coordination is real and detectable —
   check `git worktree list` before assuming you are the only agent
   touching a dependency graph.** This session ran inside
   `.claude/worktrees/agent-a9ce9af3cf7756418`, one of four active,
   `locked` sibling worktrees all branched from the same commit
   (`cdd68592d`). Two were genuinely mid-flight on adjacent territory:
   one on `openehr-rm::data_types::{basic,text}` (5 of many files done),
   one on `openehr-base::identification` (complete, all 14 files, already
   using the exact `#[serde(flatten)]` + `#[serde(tag = "_type")]` +
   per-variant-rename convention this session independently arrived at —
   confirming, not just guessing, that convention). **How to check:**
   `git worktree list` from any worktree path, then for each sibling,
   `git -C <sibling-path> status --short` and
   `git -C <sibling-path> diff <common-base> --stat` to see exactly which
   files each has touched and how far along they are. **How to apply the
   finding:** do not re-touch files a sibling has already modified (even
   if your own task nominally needs them serde-annotated too) — reference
   them as "not yet landed in my worktree, sibling P4 wave" via a
   `TODO(port):`, exactly like any other forward-reference. Worktrees do
   not share uncommitted state, so you cannot "just use" a sibling's work
   mid-session even if you can see it via `Read`; wait for it to merge.

2. **When a closed subtype set nests inside another closed subtype set
   (two-level `Inherit` chain, e.g. `Party{Actor(Actor), Role}` where
   `Actor` is itself `{Person, Organisation, Group, Agent}`), only the
   INNER enum gets `#[serde(tag = "_type")]`; the OUTER enum gets
   `#[serde(untagged)]`.** serde cannot put a `tag` on an enum whose
   variant payload is itself another enum unless that inner enum
   independently carries its own distinguishing tag — which `Actor` does
   (`#[serde(tag = "_type")]` with `PERSON`/`ORGANISATION`/`GROUP`/`AGENT`
   renames). Given that, `Party` uses `#[serde(untagged)]`: serde tries
   `Actor`'s payload shape first, then `Role`'s, in declaration order,
   until one deserializes successfully — which works correctly because
   `Actor`'s own internal tag makes every `Actor` payload
   self-distinguishing from a bare `Role` struct. **The caveat, flagged
   not resolved:** under this scheme, `Role` itself contributes no visible
   `_type` key at the `Party` level the way `Actor`'s four leaves do
   (their `_type` passes through from `Actor`'s own tag) — whether the
   real ITS-JSON schema actually wants `Role` untagged at this level, or
   wants a different scheme entirely, was left as an explicit
   `TODO(port):` on `Party`'s doc comment rather than guessed at, since
   the vendored `openehr_rm_1.1.0_all.json` schema hasn't been checked
   against a real golden vector yet. **Apply this whenever you hit a
   two-level closed-enum nesting**: tag the inner enum, untag the outer,
   and flag (don't silently resolve) whether the outer's non-enum variants
   need their own `_type` reintroduced some other way.

3. **`VERSIONED_X` binding newtypes (`struct VersionedComposition(pub
   VersionedObject<Composition>)`) get `#[serde(transparent)]`, and their
   `_type` discriminator does NOT actually get emitted by that derive —
   document this loudly rather than pretending the `TYPE_NAME` const
   works.** `#[serde(transparent)]` makes the newtype serialize/
   deserialize byte-identically to its single inner field; there is no
   wrapper JSON object of the newtype's own for a `#[serde(rename)]` to
   attach a `_type` key to. Every `VERSIONED_X` file's `TYPE_NAME` const
   and struct doc comment was updated to say this explicitly and defer
   the actual tagging mechanism (manual `Serialize` impl? decide
   `VERSIONED_OBJECT<T>` bindings never carry `_type` at all, since they're
   normally referenced via `OBJECT_REF` which does?) to a named P17
   decision. **Apply this to every future `VERSIONED_X` binding class**:
   derive `#[serde(transparent)]`, do not invent a workaround to force the
   tag through, and write the same kind of explicit doc-comment flag this
   batch used.

4. **"The crate must stay compiling" can be trivially, vacuously true —
   check `lib.rs`'s actual `mod` declarations before treating that
   constraint as load-bearing.** This crate's `lib.rs` had zero `pub mod`
   statements throughout this entire session (ADR-001 §9: "files stay
   unwired... until P17"), and several directories' `mod.rs` files
   (`ehr/mod.rs`, `demographic/mod.rs`, `integration/mod.rs`,
   `support/mod.rs`) did not exist on disk at all. `cargo check -p
   openehr-rm` therefore passed cleanly before, during, and after this
   entire 34-file serde pass, regardless of what shape the serde
   attributes took — none of the 34 files were actually part of the
   compiled crate tree. **This does not mean the serde shapes don't
   matter or weren't verified** — a full diagnostic (temporarily wiring
   every `mod` in a scratch copy outside the real worktree, run at the end
   of the session, then discarded) confirmed zero serde-specific compile
   errors (`grep -i "serialize\|deserialize\|flatten"` over the full error
   output came back empty); the ~87 compile errors that DID surface when
   fully wired were 100% pre-existing structural gaps unrelated to serde
   (`DvOrderedApi` not implemented for `DvDate`, an `item_structure`
   file-vs-directory module-path collision, unresolved sibling imports) —
   none attributable to this pass's own edits. **Apply this whenever a
   task says "the crate must compile" for an unwired Phase-A crate**:
   check `lib.rs` first; if it's genuinely unwired, `cargo check` passing
   is a necessary but not sufficient signal, and a temporary
   scratch-directory wiring experiment (never committed, never touching
   the real worktree) is the way to get real signal on your own edits
   without taking on the (out-of-scope) burden of actually wiring the
   crate.

5. **When a task's literal instructions require touching files/state
   outside the named scope to have any chance of being genuinely
   achievable (not just "would be nice"), the file-count math from the
   invoking prompt is not reliable evidence of the true blast radius —
   compute the real dependency closure before committing to a plan.**
   Before discovering point 4 above, this session traced the actual
   `#[serde(flatten)]` dependency closure required for the four named
   directories to reach genuine `cargo check` cleanliness (not just
   vacuous cleanliness from an unwired crate) and found it reached ~63
   additional files across `openehr-rm::common`/`data_structures`,
   `openehr-base::identification`, and `openehr-foundation` — i.e., most
   of two other crates. That closure computation was the right thing to
   do (do not skip it and just start editing), but the conclusion it
   should lead to is "verify whether the constraint is real first" (point
   4), not "expand scope to cover the whole closure" or "silently produce
   code you know won't compile." Both of the latter would have been
   wrong; checking `lib.rs` resolved the apparent conflict instead of
   requiring either.

Depends on/interacts with `[[worktree-isolation]]` (the mechanical
worktree-path-drift discovery from the same session) and
`[[serde-not-yet-wired]]` (updated in the same session to record that
`openehr-rm`'s `Cargo.toml` now has `serde`, while `openehr-base`/
`openehr-foundation` still don't as of this pass). See
`docs/ROSETTA.md` for the concrete per-class serde-shape rows this
session would add (not yet appended as of this memory being written —
check whether a `rosetta-mapping` delegation happened after this).

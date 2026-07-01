---
name: port-file
description: >
  Ports one already-moved .java file into a faithful .rs file beside it in
  the same crate directory. Use when the user names a specific Java file
  (or path) to port, or asks to "port <file>" / "translate <file> to Rust".
  Normally this should delegate to the porter subagent rather than doing the
  work inline.
allowed-tools: [Read, Grep, Glob, Agent]
argument-hint: "<path-to-java-file-already-under-crates/>"
---

# /port-file

Ports exactly one Java file that has already been relocated by the Phase 0
`git mv` into a `crates/openehr-*` directory. Produces a `.rs` file beside it
following the Bun-style methodology in `PORT_MASTER_PLAN.md` Section 4 and
the rules in `docs/PORTING.md`.

## Steps

1. **Validate the target.** Confirm `$1` is a `.java` path that exists under
   `crates/`. If it does not exist, or is still under the original Maven
   layout (not yet moved), stop and say so — Phase 0's `git mv` must run
   first.
2. **Check for an existing counterpart.** Compute the snake_case sibling
   (`AqlSqlLayer.java` → `aql_sql_layer.rs`) and check whether it already
   exists with a `PORT STATUS` trailer. If so, report that it is already
   ported and stop; do not silently overwrite a completed port.
3. **Delegate to the `porter` subagent** (`.claude/agents/porter.md`) with
   the single target path. The porter agent reads the Java file, consults
   `docs/LIFETIMES.tsv` for field ownership, writes the `.rs` beside it with
   the annotation vocabulary and PORT STATUS trailer, and updates
   `docs/ROSETTA.md` via the `rosetta-mapping` skill if it introduces a new
   mapping worth reusing.
4. **Relay the porter's summary** back to the user: file produced,
   confidence level, `TODO(port)` count, and anything the make-it-compile
   pass (P17) needs to know.
5. **Do not** tick a phase-file checkbox or commit yourself unless the user
   explicitly asks — porting one file is usually one task among several in
   a phase's task list (`docs/plans/phase-NN-*.md`).

## When not to delegate

If the caller explicitly asks for the port to happen inline (e.g. to review
it live, or because a subagent invocation would be wasteful for a two-line
file), you may do the port yourself following the same steps the porter
agent follows — read the Java, look up `docs/LIFETIMES.tsv`, write the `.rs`
with annotations and PORT STATUS trailer, and offer the ROSETTA update. This
is the exception, not the default.

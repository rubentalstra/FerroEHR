---
name: phase-a-forward-references
description: How to write use-statements and stub types when a builtins/interface class references a sibling foundation type (Terminology_code, Container<T>, Iso8601_date, Double, Real) that has not been transcribed yet anywhere in the workspace.
metadata:
  type: feedback
---

When transcribing a class whose spec signature names a sibling type that has
not been transcribed yet (observed for BASE 1.2.0 `base_types.builtins`:
`Env`/`Locale`/`Quantity_converter` need `Terminology_code`,
`Iso8601_date`/`time`/`date_time`/`timezone`; `Statistical_evaluator` needs
`Container<T>`; several need `Double`/`Real`), do not invent a local stub
struct and do not silently substitute a std type without flagging it.
Instead:

1. Write the `use` statement pointing at the type's **correct eventual
   module path**, inferred from the crate/package layout in
   `PORT_MASTER_PLAN.md` Section 9 (e.g.
   `openehr_foundation::primitive_types::terminology_code::TerminologyCode`,
   `openehr_foundation::time::iso8601_date::Iso8601Date`,
   `openehr_foundation::structures::container::Container`) — even though
   the file does not exist yet. This is a deliberate forward reference, not
   a mistake; Phase A explicitly does not require the crate to compile.
2. Add a `// TODO(port):` comment at the `use` block explaining the type is
   not transcribed yet and naming the expected owning module, so a later
   session or the P17 make-it-compile pass has an immediate breadcrumb
   instead of a bare unresolved-import error.
3. For primitive numeric types specifically (`Double`, `Real`) where the
   crate boundary is ambiguous or the type is a one-hop dependency likely to
   land very soon, it is acceptable to narrow directly to the underlying
   Rust primitive (`f64`) per the ADR-001 §7 std-mapping table, with a
   `TODO(port):` noting the narrowing and citing ADR-001 §7 — this mirrors
   how `Statistical_evaluator`'s spec-declared `Double` returns were
   transcribed. Do not do this for structural/composite types
   (`Terminology_code`, `Container<T>`, `Iso8601_*`) — those get a real
   forward-reference `use`, not a primitive substitution, since there is no
   faithful primitive equivalent.
4. Confirm via `find`/`grep` across the **worktree** copy (see
   [[worktree-isolation]]) that the type genuinely does not exist anywhere
   in the crate before treating it as a forward reference — do not assume
   absence from memory of a prior session.

**Why:** the alternative (blocking the file entirely until the dependency
lands, or inventing a placeholder type inline) either stalls unrelated
transcription work waiting on an artificial ordering, or creates a fake type
that a later phase must remember to delete and that could get mistaken for
the real transcription. A correctly-pathed but currently-unresolved `use`
plus a `TODO(port):` is self-documenting and costs nothing at Phase A.

**How to apply:** any `openehr-foundation`/`openehr-base` transcription task
where the per-class spec table names a type from a sibling package that has
not been assigned a session yet. Also applies going forward to `openehr-rm`
once it starts depending on not-yet-existing `openehr-terminology` types.

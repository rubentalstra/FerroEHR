---
name: project-structure-terminology-functional-precedent
description: Precedent decisions set while transcribing BASE 1.2.0 structure_types (Container/List/Set/Array/Hash), terminology (Terminology_code/Terminology_term), and functional (Tuple/Tuple1/Tuple2/Routine/Function/Procedure) into openehr-foundation.
metadata:
  type: project
---

Transcribed three BASE 1.2.0 Foundation Types packages into
`crates/openehr-foundation/src/`: `structure_types/` (container, list, set,
array, hash — 5 files), `terminology_types/` (terminology_code,
terminology_term — 2 files), `functional/` (tuple, tuple1, tuple2, routine,
function, procedure — 6 files). 13 files total, all unwired (no `mod` in
`lib.rs`, per Phase A convention), `rustfmt --edition 2024` clean.

**Why:** these are the remaining Foundation Types clusters phase-01 tasked
(alongside `primitive_types`, already done — see
[[project-primitive-types-precedent]]).

**How to apply:** check these decisions before re-deriving them for any
later class that touches containers, terminology codes, or function/agent
types (e.g. `Interval<T>` will want the same `Ordered`-bound-generic
treatment as `Hash`'s `K: Ordered`; any RM class with a `List<T>`/`Set<T>`
attribute should reference these types directly rather than reaching for
raw `Vec`/`HashSet`).

Key decisions:

1. **`Container<T>` is a Rust trait**, not a struct — no attributes, and
   `has`/`count`/`is_empty` are abstract while `there_exists`/`for_all`/
   `matching`/`select` have default bodies. **All four default-bodied
   methods are `todo!()` stubs**, because the spec never gives
   `Container<T>` an iteration primitive (no `iterate`/`for_each`) to build
   them from — only `has`/`count`/`is_empty` exist to combine. Each
   concrete container (`List`/`Set`/`Array`/`Hash`) should probably
   override these using its own std-container iterator once the crate
   compiles; flagged with `TODO(port)`, not silently implemented via a
   guessed iteration strategy.

2. **`List<T>`/`Array<T>` both wrap `Vec<T>`** (distinct newtypes, not a
   type alias to each other) — the spec's own type-cross-reference table
   lists both against `Array<T>`/`sequence` with no storage distinction
   beyond their separate function sets (`List` gets `first`/`last`; `Array`
   gets `item`). `Set<T>` wraps `HashSet<T>` (spec says "no order," so
   `HashSet` over `BTreeSet`). Both `List::first`/`last` and the
   `Container::select` widening below use `Option<T>`/`Option<&T>` for
   "or Void if no match" — this pattern will recur for any other RM
   function using that same Eiffel convention.

3. **`Hash<K,V>` is renamed `OpenEhrHash<K,V>` in Rust**, not `Hash` —
   same collision-avoidance reasoning as `String`→`OpenEhrString`
   ([[project-primitive-types-precedent]] decision 3), this time against
   `std::hash::Hash` (a trait, not a type, so no actual compile collision,
   but real reading confusion given `#[derive(Hash)]` is used throughout
   this same directory on `List`/`Set`/`Array`). `K` carries **both** the
   spec's `Ordered` bound and `HashMap`'s own structural `Eq + Hash`
   requirement — do not drop the spec's `Ordered` bound just because
   `HashMap` doesn't need it structurally.

4. **`TUPLE`/`TUPLE1<A>`/`TUPLE2<A,B>` map onto Rust's native tuples**,
   not wrapper structs — `TUPLE1<A>` = type alias `(A,)`, `TUPLE2<A,B>` =
   type alias `(A, B)`. `TUPLE` itself (the parent, no generic params) is a
   marker trait implemented directly on the underlying tuple types (`impl
   Tuple for (A,)`, etc.), since a type alias has no distinct `impl`
   target — the alias's declared "inherits TUPLE" relationship has to be
   implemented on the tuple type itself, documented as a PORT NOTE.
   `TUPLE`'s own per-class table has **no `Inherit` row at all** (unusual —
   every other class transcribed so far explicitly states `Any` or a named
   parent); `Any` was added as a supertrait anyway for consistency with the
   rest of the crate, flagged explicitly as not spec-stated rather than
   silently assumed.

5. **`ROUTINE<ARGS>`/`FUNCTION<ARGS,RESULT>`/`PROCEDURE<ARGS>` map onto
   `dyn Fn` type aliases**, not structs or marker traits for the latter
   two — `Function<Args, Result> = dyn Fn(Args) -> Result`,
   `Procedure<Args> = dyn Fn(Args)`. `Routine<Args: Tuple>` (the shared
   parent both inherit) stays a marker trait since it has no functions of
   its own and isn't itself meant to be an invocable value the way its two
   children are. **Genuine spec ambiguity found and NOT silently
   resolved**: `ROUTINE`'s own per-class table Description text says "a
   function with a return type" (verbatim identical to `FUNCTION`'s
   description) even though `ROUTINE<ARGS>`'s *signature* has no `RESULT`
   parameter and `PROCEDURE<ARGS>` (also inheriting `ROUTINE`) explicitly
   has no result. Transcribed from the signature (no result on `ROUTINE`
   itself), the discrepancy documented at length in `routine.rs` and
   flagged `confidence: low` in that file's trailer — this is the one file
   in this batch where a spec-errata check would meaningfully raise
   confidence. Do not silently give `ROUTINE` a `RESULT` parameter to match
   the description text; do not silently ignore the description
   mismatch either.

See also [[project-primitive-types-precedent]] and
[[project-openehr-foundation-crate-state]] (now stale on file inventory —
update if consulted before this memory's own contents are re-verified).

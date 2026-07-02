---
name: project-interval-cluster-precedent
description: Precedent decisions set while transcribing BASE 1.2.0 foundation_types.interval (Interval, Point_interval, Proper_interval, Multiplicity_interval, Cardinality) into openehr-foundation/src/interval/.
metadata:
  type: project
---

Transcribed the BASE 1.2.0 Foundation Types `interval` cluster into
`crates/openehr-foundation/src/interval/` (5 files: interval, point_interval,
proper_interval, multiplicity_interval, cardinality). This is the second
cluster in `openehr-foundation`, after `primitive_types` (see
[[project-primitive-types-precedent]]), and is the first worked example of
ADR-001 §3 ("abstract class with attributes → embedded struct + marker
trait") for a class whose descendants are meant to be genuinely
substitutable for the parent at runtime.

**Why:** `Interval<T>` is named explicitly in ADR-001 §5 and
PORT_MASTER_PLAN.md §7.2 as the canonical constrained-generic example
(`Interval<T: Ordered>`); this cluster is the first place that generic
actually gets built, and it sets the composition shape every later
constrained-generic-with-attributes class (`DV_INTERVAL<T>`,
`REFERENCE_RANGE<T>`, `HISTORY<T>`) should check against.

**How to apply:** any later transcription of a generic abstract-with-
attributes class that has multiple concrete substitutable descendants
(not just one) should look at how `PointInterval`/`ProperInterval` embed
`Interval<T>` by value here before inventing a new shape.

Key decisions:

1. **`Interval<T: Ordered>` is a concrete embeddable struct, not a trait**,
   even though the spec marks it `(abstract)` — because it carries six
   *attributes* (`lower`, `upper`, `lower_unbounded`, `upper_unbounded`,
   `lower_included`, `upper_included`), which per ADR-001 §3 means struct +
   embedding, not a pure-behaviour trait like `Any`/`Ordered`/`Numeric` in
   `primitive_types`. `PointInterval<T>` and `ProperInterval<T>` each hold a
   `pub interval: Interval<T>` field (composition) rather than flattening
   the six fields onto themselves directly. Nothing in the module
   constructs a bare `Interval<T>` — it exists only embedded.

2. **`has`/`intersects`/`contains` are stubbed `todo!()` on `Interval<T>`
   itself**, not pushed down as separate trait methods each concrete type
   must implement — because the spec's `(abstract)` marker at this level
   gives no per-descendant override in either `Point_interval` or
   `Proper_interval`'s own tables (neither table restates these three
   functions), so there is nothing to override yet. If a later reviewer
   expects `PointInterval`/`ProperInterval` to have their own `has`/
   `intersects`/`contains` inherent methods, check the spec tables again —
   as cached, they genuinely don't restate them.

3. **Invariant enforcement split by feasibility, not uniformly deferred.**
   `Point_interval`'s `Inv_point: lower = upper` **is** enforced today,
   structurally, by making `PointInterval::new(value: T)` the only
   constructor (it sets both `lower` and `upper` to the same value — the
   invariant cannot be violated through the public API). `Proper_interval`'s
   `Inv_not_point: lower /= upper` is **not** enforced (left as
   `TODO(port)`) because it needs a fallible `Validate`-style constructor
   (both limits are `Option<T>` and may be simultaneously absent/unbounded,
   so a plain `!=` check on two `Option`s is not obviously the right
   comparison without more of the `Ordered` surface wired through). Applying
   this split lesson: **prefer enforcing an invariant structurally at the
   type/constructor level when a single spec-faithful constructor can make
   violation impossible; only fall back to a documented `TODO(port)` +
   future `Validate` impl when the check genuinely needs a fallible path.**

4. **`Multiplicity_interval` embeds `ProperInterval<Integer>`, following
   the spec table's literal `Inherit: Proper_interval` line**, not
   `Interval<Integer>` directly — even though the class description's own
   prose ("An Interval of Integer") and the task that requested this
   transcription both framed it informally as `Interval<Integer>`. Recorded
   explicitly in the file's PORT NOTE that both framings describe the same
   embedded shape (since `Proper_interval<T>` adds no attributes beyond
   `Interval<T>`), but the literal `Inherit` row wins for which type gets
   embedded. **If a later class's `Inherit` row and its own prose disagree
   on which ancestor to embed, follow the `Inherit` row and document the
   prose reading as an equivalent-but-not-literal alternative**, rather than
   silently picking whichever is more convenient.

5. **`Cardinality` has no `Inherit` row and no `is_equal` row in its
   per-class table at all** (the only class in this cluster missing an
   explicit ancestor). Transcribed as implementing `Any` anyway, appealing
   to the crate-wide standing convention documented on `Any` itself
   (`primitive_types/any.rs`: every foundation type inherits `Any`, the
   diagram omits it "only for convenience"). Both the missing-`Inherit` and
   missing-`is_equal` gaps are flagged with a `PORT NOTE`/`TODO(port)`
   respectively rather than silently assumed — **a class table with no
   `Inherit` row is not "inherits nothing"; it defaults to the crate's
   documented `Any`-for-everything convention, but that inference should
   always be flagged, not applied silently.**

6. **Constants use Rust `const`, not a struct field** —
   `MultiplicityInterval::MULTIPLICITY_RANGE_MARKER: &'static str` and
   `::MULTIPLICITY_UNBOUNDED_MARKER: char` are inherent `const`s on the
   type, matching the spec table's own "Constants" section (as distinct from
   "Attributes"). This is the first foundation-types class with a
   `Constants` table section; the natural Rust mapping is `pub const` on the
   impl block, SCREAMING_SNAKE_CASE per Rust convention, doc-commented with
   the spec's literal signature line.

7. **PORT STATUS `todos:` count is a literal grep-count of the string
   `TODO(port)` in the file, including in the trailer's own `note:` line.**
   Caught myself writing a trailer `note:` that used the phrase "TODO(port)"
   twice in prose, which self-inflated the count from 2 to 3 when grepped.
   Fixed by rephrasing the note in plain prose (matching how
   `numeric.rs`/`integer.rs` describe their gaps without repeating the
   literal token). **When writing a trailer `note:` line, describe unfinished
   work in prose; do not use the literal string `TODO(port)` inside the
   note text itself, since the trailer's own count field is a grep count of
   that exact string across the whole file.**

See also [[project-openehr-foundation-crate-state]], which will need a
refresh once a later session checks crate file inventory again (it predates
this `interval/` addition).

---
name: project-time-types-precedent
description: Precedent decisions set while transcribing BASE 1.2.0 Foundation Types time_types cluster (Temporal, Time_Definitions, Iso8601_type, Iso8601_date, Iso8601_time, Iso8601_date_time, Iso8601_duration, Iso8601_timezone) into openehr-foundation/src/time/ — the ADR-001 §2 multiple-inheritance worked example.
metadata:
  type: project
---

Transcribed the BASE 1.2.0 Foundation Types `time_types` cluster into
`crates/openehr-foundation/src/time/` (8 files: temporal, time_definitions,
iso8601_type, iso8601_date, iso8601_time, iso8601_date_time,
iso8601_duration, iso8601_timezone). ADR-001 §2 names `Iso8601_type`
explicitly, alongside `Ordered_Numeric`, as the multiple-inheritance worked
example — but unlike `Ordered_Numeric` (both parents are symmetric
pure-behaviour traits, blanket-implemented), `Iso8601_type`'s two parents are
asymmetric, so this cluster is a *second*, distinct MI pattern, not a repeat
of the `OrderedNumeric` precedent.

**Why:** ADR-001 §2/§9 and the invoking task both flagged this as the named
worked example for "parent carries attributes" MI plus "one parent has no
trait shape at all" — a genuine judgment call future transcribers hitting a
similar case (a spec class inheriting a constants-only/namespace-only class)
should reuse.

**How to apply:** any later transcription (RM or BASE) with a class that
"inherits" a spec class made up entirely of constants and free functions
(no instance-level behaviour) should follow the `Time_Definitions` pattern
below, not try to force it into a trait bound.

Key decisions:

1. **`Temporal` (abstract, inherits `Ordered`, declares nothing) is a pure
   marker trait**, `pub trait Temporal: Ordered {}`, structurally identical
   in shape to `Ordered_Numeric`'s trait declaration — but **not**
   blanket-implemented. `OrderedNumeric` is blanket-impl'd because its two
   parents (`Ordered`, `Numeric`) are mechanically combinable — anything
   satisfying both automatically "is" `OrderedNumeric`. `Temporal` is
   different: it names a distinct semantic category ("time-related"), not a
   mechanical combination, so satisfying `Ordered` does not imply
   `Temporal`. Concrete types must `impl Temporal for X {}` explicitly (an
   empty impl, since the trait has no methods, but still a required,
   deliberate declaration). Apply this distinction going forward: **blanket
   -impl only when the combined trait is a mechanical AND of its parents;
   write explicit (even if empty) per-type impls when the trait names an
   additional semantic claim the parent traits don't already establish.**

2. **`Time_Definitions` — a spec class of only constants and stateless
   validity functions, no instance attributes anywhere in its table — is
   NOT modelled as a trait at all.** It is a zero-sized unit struct
   (`pub struct TimeDefinitions;`) carrying `pub const` items and
   associated `pub fn`s (e.g. `TimeDefinitions::valid_year(y)`,
   `TimeDefinitions::MAX_TIMEZONE_HOUR`). When `Iso8601_type`'s spec table
   states `Inherit: Temporal, Time_Definitions`, only the `Temporal` half
   becomes a Rust supertrait bound (`trait Iso8601Type: Temporal`); the
   `Time_Definitions` half is transcribed as **direct calls to
   `TimeDefinitions::*`** from every concrete descendant's invariant
   methods, not as a second supertrait (a struct cannot be a Rust
   supertrait in the first place, so this wasn't optional). This is the
   reusable template for "inherits a constants/namespace-only class"
   elsewhere in RM/BASE: don't invent a trait for it, use a unit struct
   with associated consts/fns, and transcribe the "inheritance" as
   qualified calls from whatever inherits it.

3. **Abstract class with one attribute (`Iso8601_type.value: String`) →
   embedded struct, per ADR-001 §3, even for a single field.** Created
   `Iso8601TypeCore { pub value: String }` that every concrete
   `Iso8601_*` struct holds as a `pub core: Iso8601TypeCore` field (not
   inlined directly), specifically so a later `#[serde(flatten)]` (P4/P5)
   has a natural target. Do not skip the embedded-struct step just because
   the parent has only one field — the serde-flatten future need is the
   deciding factor, not field count.

4. **These are string-value types with partial ISO 8601 precision, not
   resolved instants.** Every concrete type (`Iso8601Date`, `Iso8601Time`,
   `Iso8601DateTime`, `Iso8601Duration`, `Iso8601Timezone`) is a thin
   wrapper around `Iso8601TypeCore.value: String`; every accessor that
   needs to inspect or reformat that string (`year()`, `month()`,
   `is_extended()`, `add()`, `diff()`, etc.) is stubbed `todo!()`. This was
   an explicit instruction from the invoking task, not an inferred
   shortcut: **do not add a `jiff` dependency to `openehr-foundation`
   during this transcription pass** — the internal parsing/arithmetic
   engine is deferred to bridge to `jiff` at P17 (make-it-compile). If a
   later phase wires up real parsing, expect every `todo!()` in this
   cluster (78 total `TODO(port)` markers across the 8 files) to become the
   actual `jiff`-backed implementation — they were written with that future
   bridge specifically in mind (doc comments name which `jiff` capability
   would satisfy each one, e.g. calendar-arithmetic for `add_nominal`).

5. **Invariants in `openehr-foundation` are plain boolean-returning
   methods (`invariant_year_valid(&self) -> bool`), NOT a `Validate` impl.**
   The RM transcription rule's `Validate` trait (context + path + error
   accumulator) is scoped to `openehr-rm` invariants over composed/nested
   RM structures; a foundation-types value's own class invariants (e.g.
   `Iso8601_date`'s `Year_valid: valid_year(year)`) are simple, non-nested,
   single-value predicates, transcribed the same way
   `primitive_types::boolean.rs` documents (but doesn't `Validate`-encode)
   its own three Boolean laws. If `openehr-rm` later needs these
   foundation-type invariants surfaced through its own `Validate`
   machinery, wrap these boolean methods rather than re-deriving new
   predicate logic.

6. **Found and flagged several published-spec-table inconsistencies
   without silently "fixing" them** — this is the single most
   reviewer-relevant thing in this cluster, since some are genuine
   ambiguities a reviewer needs to weigh in on:
   - `Iso8601_date_time.add_nominal`/`subtract_nominal` are typed in the
     table as returning `Iso8601_date` (not `Iso8601_date_time`), even
     though they're declared on `Iso8601_date_time` and their own
     description delegates to `Iso8601_date.add_nominal`. Transcribed
     returning `Iso8601DateTime` (matching the receiver, matching every
     other same-type add/subtract pair in the module) with a loud PORT
     NOTE, since a literal `Iso8601_date` return would silently discard
     the time-of-day on every call — treated as a probable copy-paste
     artifact, not followed literally.
   - `Iso8601_date_time.minute_unknown`/`second_unknown` descriptions are
     both worded "indicates whether minute in hour **is known**" (positive
     phrasing, and both textually identical) — inconsistent with the
     function names and every sibling class's correctly-negative wording.
     Transcribed with name-implied ("is unknown") semantics.
   - `Iso8601_timezone.minute()`'s description says "Extract the **hour**
     part of timezone" — a copy-paste from `hour()` immediately above it.
     Transcribed with name-implied (minute) semantics.
   - `Iso8601_date_time`'s `Month_valid`/`Day_valid` invariants are stated
     unconditionally, unlike `Iso8601_date`'s conditional
     (`not month_unknown implies ...`) form for the equivalent invariants —
     transcribed literally as-is (not "fixed" to match `Iso8601_date`),
     flagged as an apparent stronger-than-intended constraint.
   - `Iso8601_date_time`'s `Partial_validity_hour` invariant depends on an
     `hour_unknown()` accessor that is **never declared** as a `Functions`
     row anywhere in that class's own per-class table (only implied by
     other functions' `Pre` clauses) — a genuine spec gap, left as
     `todo!()` rather than inventing the missing accessor's exact contract.
   - `Time_Definitions.valid_fractional_second`'s parameter is typed
     `Double` in the table while every call site elsewhere in this package
     uses `Real`-shaped attributes — both are `f64` in this crate already
     (see `[[project-primitive-types-precedent]]` item 5), so no actual
     behavioural difference, but flagged for visibility.
   - `Time_Definitions.Max_days_in_year` has no literal value in the
     table (every sibling constant does) — transcribed as an alias of
     `Days_in_leap_year`, the only value consistent with its own
     description.
   **Pattern for future transcribers:** when the spec table is internally
   inconsistent, transcribe the *name-implied* or *structurally-consistent*
   reading and document the discrepancy loudly (doc comment + PORT NOTE +
   trailer mention) rather than either (a) following the literal broken
   text silently, or (b) silently "fixing" it without a trace. Both silent
   options were explicitly rejected per the hard rule about flagging
   genuine spec ambiguity.

Depends on `crate::primitive_types::{any::Any, ordered::Ordered}` from the
sibling `primitive_types` cluster (see
`[[project-primitive-types-precedent]]` and
`[[project-openehr-foundation-crate-state]]` if those memory files exist in
whichever checkout is being read from — at the time this cluster was
written, this worktree's own copy of `primitive_types/` had not yet landed
from the parallel transcription work, so these imports point at the
*eventual* merged location, not a currently-resolvable one in this
worktree specifically. Phase A files are not required to compile.

See `docs/ADRs/ADR-001-spec-transcription-shapes.md` §2 for the ADR text
this cluster is the worked example for, and the spec cache at
`docs/research/spec-cache/BASE-1.2.0/uml_classes/{temporal,iso8601_type,
iso8601_date,iso8601_time,iso8601_date_time,iso8601_duration,
iso8601_timezone,time_definitions}.adoc` plus
`docs/research/spec-cache/BASE-1.2.0/foundation_types/master06-time_types.adoc`
for the ground truth transcribed from (Release-1.2.0 @ commit 9064413).

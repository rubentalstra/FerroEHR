---
name: project-date-time-encapsulated-precedent
description: Transcribing RM 1.1.0 data_types date_time + time_specification + encapsulated packages (DV_TEMPORAL, DV_DATE, DV_TIME, DV_DATE_TIME, DV_DURATION, DV_TIME_SPECIFICATION family, DV_ENCAPSULATED, DV_MULTIMEDIA, DV_PARSABLE) into openehr-rm/src/data_types/{date_time,time_specification,encapsulated}/ — a third distinct MI pattern beyond OrderedNumeric and Iso8601_type, plus the DV_MULTIMEDIA.thumbnail recursion worked example.
metadata:
  type: project
---

Transcribed RM 1.1.0 `data_types.date_time` (5 classes), `.time_specification`
(3 classes), and `.encapsulated` (3 classes) into
`crates/openehr-rm/src/data_types/{date_time,time_specification,encapsulated}/`
(11 files total). This is the first `openehr-rm` transcription work landed by
this session — the crate had only `Cargo.toml`/`lib.rs` before, and remains
so after (no `mod` wiring per ADR-001 §9's Phase A convention).

**Why this matters for future transcribers:** the invoking task explicitly
named `DV_DURATION` as the Section 7.2 multiple-inheritance hazard, but the
actual work surfaced **three distinct dual-parent shapes** in this one small
cluster, and conflating any two of them would be wrong. This is the reusable
generalization: "RM class inherits an RM abstract class plus a BASE
foundation-types class" is not one pattern, it is a spectrum, and which end
of the spectrum a given case sits on determines whether the second parent
gets a field or is fully subsumed.

**How to apply:** before transcribing any RM class with two `Inherit` entries
where one is an RM class and the other is a `base.foundation_types.*` class,
check which of these three shapes actually applies:

1. **Value-mixin-only dual inheritance** (`DV_DATE`/`Iso8601_date`,
   `DV_TIME`/`Iso8601_time`, `DV_DATE_TIME`/`Iso8601_date_time`): the
   foundation-types parent (`Iso8601_type` descendant) contributes *only* the
   `value: String` contract, and the RM class's own table separately
   redeclares `value: String (redefined)` with its own invariant calling a
   `valid_iso8601_*` predicate. In this shape, **do not embed the
   foundation-types struct at all** — declare `value: String` directly on
   the RM struct (not `Iso8601TypeCore`, not the concrete `Iso8601Date`
   struct), and leave a `// PORT NOTE:` explaining why (see `dv_date.rs`,
   `dv_time.rs`, `dv_date_time.rs`). The RM abstract parent (`DV_TEMPORAL`
   here) *is* embedded as a struct+trait per the ordinary ADR-001 §3 rule.
2. **Genuinely disjoint dual-parent MI** (`DV_DURATION`/`DV_AMOUNT` +
   `Iso8601_duration`): both parents contribute real, non-overlapping state
   and behaviour that the concrete class's own effected functions actually
   call into (here, `DV_DURATION.magnitude()`'s spec text literally says
   "Computed using the method `to_seconds()` inherited from
   `Iso8601_duration`"). In this shape, embed the **full concrete
   foundation-types struct** (`Iso8601Duration`, not just its
   `Iso8601TypeCore`), because the RM class's own functions delegate to that
   parent's richer API surface, not just its raw string. This is the
   genuine Section 7.2 hazard, and only this shape deserves the load-bearing
   "multiple inheritance" framing in the module doc.
3. Not seen in this cluster but worth flagging for the next transcriber: if
   a class's own table gives its `value` attribute a plain `DV_PARSABLE` or
   other RM type (not `String`) while also nominally "inheriting" a
   `base.foundation_types` class, check whether the foundation parent is
   even referenced by the concrete class's own functions before assuming it
   needs any Rust representation at all.

**Concrete decision rule going forward:** grep the concrete class's own
per-class function descriptions for the foundation-types parent's own method
names (e.g. does `DV_DURATION`'s table literally say "using the method
`to_seconds()`"?). If yes → shape 2 (embed the full struct). If the
foundation parent is only ever mentioned as "value is an ISO 8601 string,
see `Iso8601_X`" with no method delegation → shape 1 (declare `value:
String` directly, no embed).

**Other decisions worth reusing:**

- **`DV_TEMPORAL`'s covariant `accuracy` redefinition is nested one level
  above the concrete classes**, not on them directly — `DV_ABSOLUTE_QUANTITY`
  declares `accuracy: DV_AMOUNT (redefined)`, and `DV_TEMPORAL` narrows it
  again to `accuracy: DV_DURATION (redefined)`. This is a covariant
  redefinition (ADR-001 §6) happening on an *abstract* class, not a leaf
  concrete class as the other named ADR-001 §6 examples
  (`LOCATABLE_REF.id`, `DV_COUNT.magnitude`) are — the narrowed field still
  goes directly on the abstract class's embeddable `*Data` struct
  (`DvTemporalData.accuracy: Option<DvDuration>`), not deferred to each
  concrete descendant.
- **`DV_MULTIMEDIA.thumbnail: Option<Box<DvMultimedia>>` is the
  `DV_MULTIMEDIA.thumbnail` recursion case PORT_MASTER_PLAN.md §7.2 names
  explicitly by attribute path** (not just by class) — confirms the general
  rule ("recursive containment → Box") applies with `Option` wrapping the
  `Box` whenever the spec cardinality is `0..1`, not a bare `Box`.
- **`List<Byte>` RM attributes that are genuinely raw binary payloads
  (`DV_MULTIMEDIA.data`, `.integrity_check`) transcribe to `Vec<u8>`
  directly**, not
  `openehr_foundation::structure_types::list::List<octet::Octet>`. This is a
  deliberate divergence from the otherwise-settled `List<T>` -> `List<T>`
  foundation-types mapping: when the RM attribute is conceptually a flat
  byte buffer (not a foundation-types `List` value being manipulated via
  `Container`/`List` operations), use the plain `Vec<u8>` byte-buffer
  convention instead, matching `docs/PORTING.md` §14.2's `byte[]` ->
  `Vec<u8>` row. Flag this choice in a doc comment at the field (done in
  `dv_multimedia.rs`) since it is a genuine judgment call between two
  plausible readings, not a mechanical lookup.
- **Terminology-service-backed invariants get a `todo!()` trait-default
  method, not an omitted invariant.** `DV_ENCAPSULATED.invariant_language_valid`/
  `invariant_charset_valid` and three of `DV_MULTIMEDIA`'s five invariants
  all require a live `code_set(...).has_code(...)` lookup against
  `openehr_terminology::TerminologyService` — none of that plumbing is
  threaded through any signature yet (no `Validate`-context parameter
  exists at this point in the transcription). The pattern used: keep the
  method, `todo!()` the body, and note in the doc comment exactly which
  `OPENEHR_CODE_SET_IDENTIFIERS` constant it should eventually bind to.
  Purely structural invariants with no terminology dependency
  (`DV_MULTIMEDIA`'s `Not_empty`, `Integrity_check_validity`, `Size_valid`;
  `DV_PARSABLE`'s both invariants) are fully implemented in the same pass —
  do not blanket-`todo!()` every invariant in a class just because some of
  its siblings need the service.
- **Found and flagged a repeating `Post_result` postcondition defect**
  across `DV_DATE`/`DV_TIME`/`DV_DATE_TIME`'s `less_than`: the published
  text reads `Result = magnitude > other.magnitude`, which is backwards for
  a function named `less_than` (and inconsistent with `DV_DURATION`'s own
  `less_than`, whose text correctly reads `magnitude < other.magnitude`).
  Transcribed all three with the name-implied, `DV_DURATION`-consistent
  semantics (`self.magnitude() < other.magnitude()`), with a loud PORT NOTE
  each time rather than silently propagating three copies of a likely
  copy-paste error. This is the same "transcribe name-implied, flag loudly,
  never silently follow or silently fix" pattern from
  `[[project-time-types-precedent]]` item 6 — same discipline, new
  instance.
- **Two HL7v3-derived syntaxes (`PIVL`/`EIVL` phase/event-linked, and the
  more general recursive GTS grammar) have no parser designed anywhere in
  the crate yet**, and are a *distinct* deferred-engine category from the
  ISO-8601-specific jiff-bridging plan named in
  `[[project-time-types-precedent]]`. `DV_TIME_SPECIFICATION`'s three
  abstract functions and both concrete descendants' effected overrides are
  all `todo!()` for this reason — flag this as its own gap category
  (`"requires an HL7v3 ... syntax parser, not yet designed"`) rather than
  folding it into the jiff-bridging TODO wording, since a reviewer needs to
  know these are unrelated blocking dependencies.
- **`DV_PERIODIC_TIME_SPECIFICATION.institution_specified` and
  `DV_GENERAL_TIME_SPECIFICATION.institution_specified` are both flagged as
  genuinely underspecified** in the published table — their descriptions
  say only "Extracted from value" with no further detail on the derivation
  rule, unlike `calendar_alignment`/`event_alignment` which at least name
  the specific grammar term extracted. Flagged, not guessed at.

Depends on forward-references to three not-yet-landed concurrent
transcription passes at the time this file was written: `DATA_VALUE` and the
`quantity` package cluster (`DV_ABSOLUTE_QUANTITY`, `DV_AMOUNT`,
`DV_QUANTIFIED`) in `crate::data_types::quantity::*`; `CODE_PHRASE` in
`crate::data_types::text::code_phrase`; `DV_URI` in
`crate::data_types::uri::dv_uri`. All are plain `use` forward-references (not
stubbed locally), per the invoking task's explicit instruction not to touch
files outside the three target directories. Reconcile at whichever phase
wires `mod` declarations (P17 per ADR-001 §9).

See `docs/ADRs/ADR-001-spec-transcription-shapes.md` §3/§6/§8 for the rules
this cluster applies, and the spec cache at
`docs/research/spec-cache/RM-1.1.0/data_types/{master07-date_time_package,
master08-time_specification_package,master09-encapsulated_package}.adoc` plus
`docs/research/spec-cache/RM-1.1.0/uml_classes/{dv_temporal,dv_date,dv_time,
dv_date_time,dv_duration,dv_time_specification,
dv_periodic_time_specification,dv_general_time_specification,dv_encapsulated,
dv_multimedia,dv_parsable}.adoc` for the ground truth (Release-1.1.0 @
3cbd85b). Note these uml_classes files live at the *top level* of
`RM-1.1.0/uml_classes/`, not nested under a `data_types/uml_classes/`
subdirectory — the RM-1.1.0 spec cache is flatter than the BASE-1.2.0 cache's
per-package `uml_classes/` layout that `[[project-time-types-precedent]]`
was written against; check the actual directory listing rather than assuming
the BASE-1.2.0 nesting convention carries over.

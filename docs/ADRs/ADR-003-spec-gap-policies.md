# ADR-003 — Policies for spec-underdetermined behaviour (temporal arithmetic, modulo, URI, iteration)

- Status: accepted (still governs the hand-written `*_impl.rs` behaviour layer —
  ADR-004 confirms this). Only the crate name below is stale.
- Date: 2026-07-02
- Phase: P5 (docs/plans/phase-05-serialization-xml.md, spec-completion pass over P1–P3 crates)

> ## ⚠️ AMENDMENT (2026-07-03, ADR-004): `openehr-foundation` folded into `openehr-base`
>
> These behaviour policies still stand and govern the hand-written `*_impl.rs`
> sibling files (ADR-004 kept ADR-003 in force). One naming update: the
> `openehr-foundation` crate referenced below (as a crate the policies span and
> as gaining the `url` dependency) was folded into `openehr-base` by ADR-004 —
> read `openehr-foundation` as `openehr-base`.

## Context

The P1–P3 transcription pass deliberately left every function whose behaviour
the published specification underdetermines as `todo!()` with a
`// TODO(port):` marker. Closing those gaps requires policies that the spec
does not state. This ADR fixes each policy once, so the implementations (and
their tests) are consistent across `openehr-foundation`, `openehr-base`, and
`openehr-rm`. Where the spec **does** state semantics (definite vs nominal
arithmetic), this ADR records the reading, not a choice.

## Decisions

1. **Definite temporal arithmetic (spec-stated, BASE foundation_types
   master06-time_types §Computational Functions).** `add()`, `subtract()`,
   `diff()` treat an `Iso8601_duration` as an exact quantity: the years and
   months components convert to days via
   `Time_definitions::AVERAGE_DAYS_IN_YEAR` (365.24) and
   `AVERAGE_DAYS_IN_MONTH` (30.42), all remaining components to seconds, and
   the total is applied as an exact `jiff::SignedDuration`. `diff()` returns
   a normalized `Iso8601_duration` expressed in definite units (days and
   below — never years/months, which are nominal units).

2. **Nominal temporal arithmetic (spec-stated).** `add_nominal()` /
   `subtract_nominal()` apply years/months/weeks/days as calendar units via
   `jiff::Span` on civil (naive) values — "same date next month", with
   jiff's end-of-month clamping — and the sub-day components as exact time.

3. **Partial-precision anchoring (our policy — spec silent).** Arithmetic on
   a partial ISO 8601 value anchors it by filling each unknown component
   with its minimum (month → 01, day → 01, hour/minute/second → 0,
   fraction → 0), computes on the anchored `jiff` civil value, then
   truncates the result back to the **original precision** of the receiver.
   Timezone, when present, is preserved verbatim; arithmetic itself is civil
   (no DST shifts — ISO 8601 partial values carry no zone rules to apply).
   Rationale: deterministic, order-preserving, matches how partial dates are
   interpreted for DV_DATE magnitude semantics (days since origin with
   floored missing parts).

4. **`Integer.modulo` / `Integer64.modulo` sign convention (spec silent).**
   Truncated division (result takes the dividend's sign) — Rust's native
   `%`, identical to Java's `%`, i.e. the behaviour a faithful EHRbase port
   will exercise. Documented at each site with a `// PORT NOTE:`.

5. **URI syntax validation (spec references RFC 3986).** Foundation `Uri`
   and RM `DV_URI`/`DV_EHR_URI` validate with a conservative RFC 3986
   check built on the pinned `url` crate where the value is an absolute
   URI; values the `url` crate cannot represent losslessly (relative
   references) are checked against RFC 3986's generic-syntax grammar
   componentwise. Accessors continue to return the un-normalized stored
   text (RM canonical-form round-trips must not rewrite user URIs).

6. **Container iteration primitive (Rust necessity).** The spec's
   `Container<T>` declares `there_exists`/`for_all`/`select`/`matching`
   over agents but no iteration primitive. The Rust trait adds a required
   `fn items(&self) -> &[T]`-shaped accessor (`// PORT NOTE:` at the
   declaration); the four spec functions become default methods taking
   `impl Fn(&T) -> bool`. `Set<T>` (unordered backing store) implements the
   spec functions directly over its `HashSet`.

7. **`Any.instance_of(a_type_name)` stays unimplemented.** Reflection-by-
   name has no Rust analogue short of a global type registry with zero
   consumers in EHRbase. The method remains a documented deviation
   (`// PORT NOTE:`), not a `todo!()`.

8. **Class invariants become working `is_valid()`-family methods now; the
   walker/accumulator validation framework stays at P11.** Invariants that
   need terminology take `&TerminologyService`. Constructors that can
   enforce an invariant cheaply (`fn new(...) -> Result<Self, E>`) do so;
   the deep, archetype-driven validation framework remains the P11
   deliverable per the master plan.

## Consequences

- Every `todo!()` in the P1–P3 crates is now implementable without
  per-file policy improvisation; remaining `TODO(port)` markers must cite
  either a published-spec defect or a later phase (P11/P17) by name.
- Temporal tests can assert exact spec-derived values (e.g. definite
  `add(P1M)` moves a date by 30.42 days ≈ 2 629 728 s, while nominal
  `add_nominal(P1M)` lands on the same day next month).
- The `url` crate becomes a dependency of `openehr-foundation`.

# ADR-012: Closed-archetype validation semantics for OPT 1.4 commits

- **Status:** accepted
- **Date:** 2026-07-09
- **Resolves:** spec-audit finding F-07-05 (`docs/spec-audit/findings/07-validation.md`)
- **Scope:** the composition validator (`openehr-flat::validation`), B2.

## Context

AOM 1.4 defines only the *positive* recursive conformance function
`valid_value` (`AM/docs/AOM1.4/master04-constraint_model_package.adoc:60-62`)
and is **silent** on whether a present-but-unmatched instance node is an
error. Closed-archetype / RM-conformance closure semantics are formalized only
in AOM 2 (`AM/docs/AOM2/master08-validation.adoc`, `c_conforms_to()`,
VSONCT/VSONCO/VSONI). Our walker navigates template→instance, so instance
nodes matching no template constraint were never visited and never flagged —
an *open-world* walk: a composition could carry an extra archetyped ENTRY or
an unknown `archetype_node_id` and pass.

## Decision

**Adopt closed-archetype semantics for archetyped content, tolerating
RM-permitted unconstrained metadata.** Concretely, under a constrained,
non-`any_allowed` attribute of a matched node:

1. An instance child bearing an `archetype_node_id` (or archetype id) that
   matches **no** sibling constraint at that attribute is a violation
   (`unexpected node`).
2. Attributes the template does not constrain at all remain open (RM-governed:
   `name`, `uid`, `links`, `feeder_audit`, `language`, `encoding`, context
   attributes, …) — never flagged. The closure applies to *archetype-slot
   competition*, not to plain RM attributes.
3. The walk still never double-flags: a node rejected as unexpected is not
   descended into.
4. *(Scope amendment, B2 close)* Where an attribute carries **no**
   `ARCHETYPE_SLOT` constraint, an unmatched archetype-rooted child
   (`openEHR-…`) is tolerated — the flat OPT does not enumerate the full
   slot-fill universe, and the CNF corpus itself commits ENTRY archetypes the
   template does not list (archie accepts). Where slots are declared,
   archetype-rooted fillers remain subject to slot admission (include/exclude,
   F-07-10); at-coded children are closed everywhere. Verified by the
   zero-drift gate.

Rationale: matches de-facto openEHR CDR behaviour (EHRbase/archie treat
compositions as closed against the OPT), matches the AOM2 direction the spec
family is converging on, and is the semantics the CNF content chapters assume.
Since AOM 1.4 text does not compel it, the implementation carries a
`// PORT NOTE:` citing this ADR at the rejection site.

**Gate:** the change lands only behind a full ECC run with **zero drift** vs
the standing 293/319 baseline — the CNF fixtures are the empirical check that
the metadata-tolerance list (rule 2) is right; any fixture regression means
rule 2 is too narrow, and the fix is widening the tolerance, never weakening
a case.

## Consequences

- Better: extra/foreign archetyped content is rejected as every other
  production CDR does; the F-07-10 slot work (WebTemplate nodes for open
  `ARCHETYPE_SLOT`s) composes with this — an open slot admits its includes,
  closure rejects the rest.
- Risk: over-rejection of RM-permitted metadata — mitigated by rule 2 + the
  zero-drift gate + the owned-fixture register discipline (defective fixtures
  are corrected and registered, never silently tolerated).

## Alternatives considered

- **Stay open-world** (status quo): spec-defensible under AOM 1.4 alone, but
  diverges from every production validator and silently admits foreign
  clinical content — rejected.
- **Full AOM2 closure (flag unknown plain attributes too):** over-reads AOM2
  into 1.4 territory and would reject RM-legal instances; rejected.

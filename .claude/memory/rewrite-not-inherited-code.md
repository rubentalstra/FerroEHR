---
name: rewrite-not-inherited-code
description: "This is a REWRITE — existing hand-written code is never assumed correct; read the ancestor spec and the existing code before implementing, and prefer a breaking change over preserving bad code"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 7a390181-e5e7-4508-9316-33a300a4aedb
  modified: 2026-08-11T08:53:10.210Z
---

Existing code in this repo is prior work, not a baseline. Owner directive
(2026-08-11, after repeated defects): "old code is bad code and need breaking
changes so we implement everything proper". Never treat a file as settled
because it is already merged and green.

**Why:** two defects shipped and were only caught later by accident —
`DV_COUNT.add` dropped the accuracy `DV_AMOUNT` normatively specifies under a
doc comment claiming the spec was silent (the redefined ANCESTOR class had
never been read), and `DV_COUNT.multiply` read a `Real` factor's binary
approximation rather than the decimal its author wrote. No gate found either.
The owner's standing complaint is that these were spec-reading and
code-reading failures, not bad luck.

**How to apply:**
- Before implementing a spec function, read the class's own table AND every
  ancestor whose function it redefines/effects. A subclass table routinely
  omits pre-conditions and rules the parent states.
- Before writing a method, grep for it — it may already exist, or be produced
  by a macro applied elsewhere (`ordered_limit!` gives every DV_ORDERED
  descendant `less_than`/`is_strictly_comparable_to`).
- Read the region of a file before editing it, and verify the edit landed.
  Blind string substitution passed its anchor check and still orphaned a
  `#[test]` attribute.
- Never add `#[expect]`/`#[allow]` as the first move — the lints are strict
  deliberately. Find the shape that does not need one (a `Decimal` field
  instead of an `as f64` cast removed one outright).
- A wrong spec citation in a doc comment is a DEFECT, not a wording issue.
- Distrust measurement instruments too: the `unrealized_bmm_functions` ratchet
  silently skipped any class with no `*_impl.rs` sibling, hiding 239 declared
  functions (#2247) while reporting 75.

Related: [[spec-chapter-audit-programs]], [[comments-less-is-more]],
[[owner-work-style]], [[en-route-findings-always-filed]].

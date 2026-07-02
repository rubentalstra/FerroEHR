<!--
ADR numbering and linking convention:

- ADRs are numbered sequentially, zero-padded to three digits, starting at
  001 (this file, 000, is the template and is never itself a decision).
- Filename: `ADR-NNN-short-kebab-case-title.md`.
- Every ADR is linked from the phase file (`docs/plans/phase-NN-*.md`) whose
  "Decisions made this phase" section produced it. An ADR without a linking
  phase file is orphaned and should be traced back or removed.
- Never renumber an existing ADR, even if superseded — record supersession in
  the Status field instead (see below).
-->

# ADR-NNN: <Title>

- **Status:** proposed | accepted | superseded by ADR-NNN | rejected
- **Date:** YYYY-MM-DD

## Context

What is the problem or forcing function? What constraints (spec compliance,
performance, existing EHRbase behavior, Rust language limitations) bound the
decision? State the question being answered, not the answer.

## Decision

What was decided, stated as a direct, active-voice sentence. If the decision
has multiple parts, enumerate them.

## Consequences

What becomes easier or harder as a result. Include negative consequences and
follow-on work honestly, not just the benefits.

## Alternatives considered

For each alternative: the option, and the specific reason it was not chosen.
An alternative with no stated rejection reason is not a complete ADR.

---
name: write-adr
description: >
  Copies docs/ADRs/ADR-000-template.md to the next ADR number, fills in
  Context/Decision/Consequences from a decision summary, and links it from
  the current phase file's "Decisions made this phase" section. Use when the
  user asks to record an architectural decision or write an ADR.
allowed-tools: [Read, Write, Edit, Glob]
argument-hint: "<decision title>"
---

# /write-adr

Records a structural decision so it survives beyond the session that made
it. The blueprint (`docs/blueprint/00-THE-BLUEPRINT.md`) flags the deliberately
open decision points; write an ADR whenever a structural choice sets precedent
(storage/schema, service seams, plugin/authz posture, spec-silence resolutions).

## Steps

Note: an ADR that decides spec-facing behaviour must cite the governing
vendored spec sections (`docs/specs/openehr/...`, CNF test-case ids) in its
Context — including where the spec is silent (that silence is usually *why*
an ADR is needed).

1. **Find the next ADR number.** Glob `docs/ADRs/ADR-*.md`, take the highest
   numeric prefix, and use the next one, zero-padded to three digits
   (`ADR-000-template.md` is not a real decision — the first real one is
   `ADR-001-...`).
2. **Copy the template** (`docs/ADRs/ADR-000-template.md`) to
   `docs/ADRs/ADR-<NNN>-<slug>.md`, where `<slug>` is the title in
   kebab-case.
3. **Fill in the template sections** from the decision summary given as
   `$ARGUMENTS` and whatever context is available in the conversation:
   - **Context** — what problem forced the decision, and what constraint
     from the blueprint or a prior ADR bears on it (cite it).
   - **Decision** — the actual choice, stated as a single clear sentence
     first, then supporting detail.
   - **Consequences** — what this makes easier, what it makes harder, and
     any follow-up task it creates (e.g. "restoration is Stage 2, see
     Section 11").
4. **Do not invent a decision that was not actually made.** If the
   conversation has not settled on a choice yet, say so and ask, rather than
   writing an ADR that only records one option.
5. **Link it from the current phase file.** Read
   `docs/plans/current-phase.md` to find the active phase file, then add a
   bullet under that phase's `## Decisions made this phase` pointing at the
   new ADR path.
6. **Report the new ADR path** back to the caller; do not commit it
   yourself unless asked.

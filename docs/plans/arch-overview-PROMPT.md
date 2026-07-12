# Kickoff prompt — Architecture Overview study + checklist (WORKLIST W-3a)

Paste the block below into a fresh session, verbatim.

---

Read `docs/plans/WORKLIST.md` first — you are executing **W-3a**, which gates
the ADL2 work (W-4). Work on branch `claude/arch-overview-checklist` cut from
an up-to-date `develop`.

**The task:** fully read and understand the openEHR **Architecture Overview**,
vendored at `docs/specs/openehr/BASE/docs/architecture_overview/` (the
in-repo copy of
https://specifications.openehr.org/releases/BASE/development/architecture_overview.html
— read `master.adoc` for the include order, then every `masterNN-*.adoc`
chapter file in full, 00 through the last). This document is the map of the
WHOLE openEHR architecture — read it as the thing that must be internalized
before any further subsystem work, especially ADL2.

**The deliverable:** `docs/spec-audit/architecture-overview/CHECKLIST.md` —
a checklist of the ENTIRE document at chapter → section → subsection
granularity (numbered exactly like the document: 1, 1.1, 1.1.1, …). For every
numbered heading, list each load-bearing statement under it (requirements,
architectural rules, definitions that constrain implementations, package/
component relationships, invariant principles), one checklist row each:

- `[ ]` the statement, in one precise sentence, with the exact citation
  (`masterNN-file.adoc` §heading);
- a **verdict against ehrbase-rs as it stands**: `verified` (where in the
  code/design it is realized — file paths), `gap` (what is missing — these
  become WORKLIST rows, never open-ended notes), or `informative` (context
  with nothing to implement);
- tick the box only when the verdict is written and evidence-backed.

Purely narrative/historical passages get one `informative` row per
subsection, not per sentence — but do NOT compress normative content: if a
subsection carries five distinct rules, it gets five rows.

**Rules (all standing, all hard):**
- The vendored spec text is the only oracle; cite file + section on every row.
  Never resolve a question from memory.
- Spec-only citations — never ADRs.
- Work in committed increments (one commit per chapter batch is a good
  cadence), branch `claude/*`, no AI attribution anywhere.
- Findings that are gaps go into `docs/plans/WORKLIST.md` as new rows in the
  same commit that records them.
- When the checklist is complete: update the W-3a row to done, PR to develop,
  merge, then STOP and report — W-4 (ADL2) starts only after this is merged
  and reviewed.

---

## Handoff state (written 2026-07-12, end of the H1 session)

- **W-1 (H1 ADR sweep)**: branch `claude/h1-adr-citation-sweep` pushed; the
  full workspace suite was still running at session close — verify it is
  green (`cargo nextest run --workspace`), then PR + merge before or alongside
  this task. Zero `ADR-` mentions remain under `app/ crates/ tools/`.
- **W-2 (ECC skip elimination)**: inventoried in the worklist, not started.
- CI gates (audit/deny/machete): green as of PR #71.

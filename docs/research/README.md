# Research Dossiers

`PORT_MASTER_PLAN.md` (Section 2) synthesizes two completed research passes.
The full dossiers behind that synthesis belong in this directory as:

- `01-port-methodology-and-platform.md` — Pass 1: the Bun Zig→Rust methodology
  as the template; EHRbase's full architecture and port-difficulty map; the
  PostgreSQL 16-vs-18 decision; current Rust tooling; the Claude Code
  scaffolding pattern.
- `02-openehr-spec-surface.md` — Pass 2: the per-component openEHR release
  matrix, the class-by-class RM inventory, the AOM/ADL/AQL grammar sources,
  the ITS XML/JSON schema sources, and the transcription sequencing.

These are **inputs, not living documents**. They record the reasoning that
produced `PORT_MASTER_PLAN.md` at a point in time and are committed verbatim
so that reasoning travels with the repository. When project understanding
moves on, update `PORT_MASTER_PLAN.md` and the `docs/plans/` phase files —
do not edit these dossiers to match; if a dossier turns out to be wrong,
record the correction at its point of use instead of revising the source
document.

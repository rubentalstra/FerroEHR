# Upstream reports — the outbound openEHR report ledger

The outbound spec-defect reports raised from the CNF ambiguity register
(`tools/cnf-runner/artifacts/registers/ambiguities.yaml`). The register never
silently absorbs a spec divergence or silence — it documents it and points here
at the report that pushes the fix back to openEHR, so the spec is corrected
rather than worked around forever.

## Grounding rules (non-negotiable)

- **Docs text only.** Every report cites the openEHR **docs text**
  (`docs/specs/openehr/…`) — the normative prose. The vendored **OAS is STALLED
  and is NOT an oracle** (owner ruling 2026-07-24): it is never a citation here.
  A "defect" that exists ONLY because the OAS is stale/incomplete is an
  OAS-regeneration item, NOT an openEHR spec report, and does not belong in this
  ledger.
- **Proven before listed.** A report appears here ONLY after its register entry
  is CONFIRMED first-hand against the docs text (the `cnf-triage` adjudication).
  An entry the docs text actually resolves is REFUTED — removed from the
  register, its case made gating — and is never reported upstream.
- **Lifecycle.** `draft` (a `UPR-<n>` id) → filed on the openEHR channel
  (Jira / spec repo) → the `UPR-<n>` id is replaced, in both this ledger and the
  register `upstream_ref`, with the returned key (`SPECPR-<n>` /
  `SPECQUERY-<n>` / the merged editorial PR).
- **Channels.** `SPECPR` (RM/BASE/SM semantic gap) · `SPECQUERY` (AQL/QUERY) ·
  `editorial` (schedule/spec text defect, no semantic change) · `ITS-REST`
  (ITS-REST API-definition gap) · `ITS-XML` (serialization/XSD gap) · `TERM`
  (terminology verification) · `SEC` (Specifications Editorial Committee
  decision).

## Status: REGENERATING — do not file

The previous draft of this ledger was **discarded**. It was generated
mechanically from the register's `handling`/`source` fields, was **unproven**
(no first-hand adjudication), and cited the **stalled OAS** as authority for
many entries — so its citations and "problems" were not trustworthy.

The ledger is being **re-derived from the first-hand docs-text adjudication**
(the `cnf-triage` pass, OAS excluded) now in progress. Each surviving report
will be listed below with its **docs-text** citation and its CONFIRMED verdict;
REFUTED entries are dropped (and their cases made gating). Until an entry lands
here CONFIRMED, there is nothing to file.

<!-- CONFIRMED reports land below, one `### UPR-NN` per entry, once the
     docs-text adjudication confirms them. -->

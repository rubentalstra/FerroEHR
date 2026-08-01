---
name: upstream-reports-are-issues
description: "Owner 2026-08-01 — upstream spec-defect reports are GitHub issues (upstream-report label), the md ledger is deleted, and the issue-body convention changed"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-01T19:39:08.144Z
---

Owner rulings 2026-08-01 (PR #1612):

- `docs/conformance/upstream-reports.md` is DELETED and must never come back.
  An outbound openEHR spec-defect report = a GitHub issue labeled
  `upstream-report` (dark red #8B0000). Body shape: plain opening summary →
  `## What the released spec says` → `## What this implementation does` →
  `## Resolution sought upstream`. NEVER ticket-draft framing
  (Channel/Status/Ask/draft), never a retired ledger id (UPR-*).
- New reports are UNVERIFIED (owner found many old ones misread the docs):
  they enter the verification milestone (v3.20.0) with a re-verification
  checklist. Confirmed → `upstream-confirmed` label (amber #B45309) + leave
  the milestone; refuted → close + remove/re-ground the ambiguities.yaml
  entry + make the case gating. `blocked-upstream` is NOT for reports — it
  keeps its narrow meaning (resolved in upstream Jira, text not yet
  published).
- `ambiguities.yaml` is the machine layer only; entries point at the issue
  via `upstream_issue: <number>` (schema-enforced for report_only/editorial).
- Issue bodies repo-wide: `## Contract`/`## Exit criteria`/`## Phases` are
  RETIRED — plain opening summary + `## Acceptance criteria` (+ optional
  `## Tasks`). Owner: no big GitHub project says "Contract".

**Why:** the owner is sick of information floating across reference files
("hiding issues"); GitHub is the single home for narratives, machine files
keep only what tools read.

**How to apply:** never create a standing markdown ledger/register for
narrative content; file issues with the labels above; keep register entries
pointing at issues. Related: [[tracker-is-github-issues]],
[[issue-relationships]].

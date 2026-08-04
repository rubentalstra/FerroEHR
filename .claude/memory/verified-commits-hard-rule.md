---
name: verified-commits-hard-rule
description: "Owner hard rule 2026-08-01 — every commit must be verified/signed, including CI-created ones"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: af8ec1a8-3953-4ae1-a5d1-355a712f597b
  modified: 2026-08-01T19:47:37.938Z
---

Owner hard rule 2026-08-01: only verified/signed commits, everywhere —
including commits CI creates.

**Why:** unverified commits on any branch (the CI badge commit was the
offender) violate the repo's provenance bar.

**How to apply:**
- Local commits: already covered (`commit.gpgsign=true`, openpgp key) — never
  disable it, never commit with a stripped identity.
- CI-created commits: NEVER `git commit`/`commit-tree` + push in a workflow.
  Create commits through the Git Data REST API (`gh api`: blob → tree →
  commit → ref update) with the workflow token — GitHub signs them as
  `github-actions[bot]` (Verified). Precedent: the coverage-badge step
  (PR #1613, `.github/workflows/ci.yml`).
- Any NEW workflow that writes a commit must use the same API pattern.

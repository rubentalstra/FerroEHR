---
name: tick-pr-licensing-box
description: "Standing instruction to tick the contribution-licensing checkbox in every PR body I open, rather than leaving it for the owner"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 41ab4b7d-4974-4bc0-9a89-c1d7691c6eb6
  modified: 2026-09-06T18:42:19.565Z
---

Tick the PR template's `- [ ] I accept the terms in [CONTRIBUTING.md § Licensing of contributions]` box on every PR I open (owner ruling 2026-09-06). Do not leave it unticked "for the owner to sign".

**Why:** the `contribution-licence-guard` CI job fails on an unticked or missing line, so leaving it turns every PR red on a formality. The owner is the sole copyright holder AND the Licensor named in `LICENSE`, so the attestation is true by construction; a body edit unticks it if they ever disagree.

**How to apply:** include the full `## Licensing of contributions` section with `- [x]` in the PR body at creation time. The guard reads the PR event payload, so a body edit re-runs it. See [[merge-on-local-gates]] and [[autonomous-phase-flow]].

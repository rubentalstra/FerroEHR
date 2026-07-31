---
name: product-rename-ferroehr
description: "Owner decided 2026-07-31 the product rebrands from EHRbase-rs to FerroEHR (issue #1353); rename program not yet executed"
metadata: 
  node_type: memory
  type: project
  originSessionId: dcbecbcf-230c-4415-b3e6-d95dc66e6703
  modified: 2026-07-31T11:23:20.910Z
---

Owner decision 2026-07-31: the product's new name is **FerroEHR** (ferro/ferrum → Rust, "EHR" kept for category recognition). Tracked on issue #1353 — decision + collision-check evidence in its comments.

**Why:** "EHRbase" is vitagroup GmbH's product; this greenfield project needs a distinct identity. "openEHR" must NOT appear in the brand (Foundation trademark needs a Product Use License); prose says "an openEHR® CDR" with attribution.

**How to apply:** Until the rename program (repo, `ehrbase-*` crates, binary, `EHRBASE_*` env prefix, OCI/Helm, website) lands via #1353 sub-issues, keep using current names in code. The generated `openehr-*` spec crates keep their names regardless. Claimed 2026-07-31: the GitHub org `FerroEHR` (parked — repo stays on the owner's personal Pro account for now; free-org transfer would lose features) and the domain `ferroehr.eu` (the primary domain). Still open: formal EUIPO/USPTO trademark search; ferroehr.com/.org unregistered (squat risk before public announcement).

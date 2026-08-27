---
name: veredictum-other-session
description: "Owner directive 2026-08-27: Veredictum work happens in the owner's OTHER session — this session never touches that checkout, tracker, or dispatches there"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 32d068af-12e7-4654-9ece-124240b2367f
  modified: 2026-08-27T08:52:24.001Z
---

Owner directive (2026-08-27): "please ignore the Veredictum i am now doing
everything in an different session regarding Veredictum."

**Why:** two sessions editing one checkout collide (branch checkouts are
exclusive), and the other session was observed mid-build there
(`veredictum-console:scaffold-check`).

**How to apply:** from this session, never edit
`/Users/rubentalstra/RustroverProjects/Veredictum`, never file/close issues
or merge PRs on rubentalstra/Veredictum, never dispatch its workflows. When
FerroEHR work needs Veredictum prior art (workflow shapes, the pinned CLI's
behaviour), read it from the REMOTE (`gh api repos/rubentalstra/Veredictum/...`)
— pass the same instruction to workers. FerroEHR-side integration stays fair
game: the pin in `scripts/lib/veredictum.sh`, `scripts/conformance.sh`, the
committed conformance artifacts. Re-check with the owner if a task seems to
require crossing this line.

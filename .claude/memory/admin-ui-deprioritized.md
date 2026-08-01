---
name: admin-ui-deprioritized
description: "Owner 2026-08-01: the admin UI is currently broken and deprioritized — server side first; never compose the admin-ui service into SUT/stress stacks (name services explicitly: `up -d --wait ferroehr`)"
metadata: 
  node_type: memory
  type: project
  originSessionId: 39213bee-2a89-4d7c-8dca-5e502effeeab
  modified: 2026-08-01T15:55:05.419Z
---

Owner statement (2026-08-01): the admin UI is broken right now and that is
accepted — the focus is the server side first. Its build lane failing
(e.g. `openehr-its` erroring under the wasm/hydrate lane) is not a
server-tree defect; the native `cargo check -p openehr-its` is the truth
for the CDR.

**Why:** chasing UI-lane breakage mid-server-program wastes the session;
composing the UI into a SUT stack starts Keycloak + admin-ui containers a
conformance/stress run never touches.

**How to apply:** compose SUT stacks with explicit service names exactly
like `scripts/conformance.sh` does (`up -d --wait ferroehr`) — never a bare
`docker compose up`. Before any release cut, re-check with the owner
whether the UI lanes are expected green again. Related:
[[concurrent-sessions-shared-tree]].

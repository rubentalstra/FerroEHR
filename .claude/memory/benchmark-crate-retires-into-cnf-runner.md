---
name: benchmark-crate-retires-into-cnf-runner
description: "Owner ruling 2026-07-22 — nothing consumes cnf-runner as a lib; benchmark is condemned, its features MIGRATE INTO the runner and the crate deletes"
metadata: 
  node_type: memory
  type: project
  originSessionId: 6ac52978-20dc-48c9-8e32-4ecb66b5f383
  modified: 2026-07-22T14:46:38.728Z
---

Owner ruling (2026-07-22, during #233, sharpened mid-session): **nothing
consumes `cnf-runner` as a library dependency — ever** (it is the terminal
instrument: lib consumed only by its own bin/tests). `tools/benchmark` is
condemned: no interim coupling in either direction; its unique features
(knee ladder → `cnf-runner knee`, hospital-day workload model,
resource/storage sampling, cross-SUT compare) MIGRATE INTO cnf-runner as
subcommands (#237) and the crate is deleted in the PR that lands the last
migration. Temporary payload duplication inside benchmark is acceptable
precisely because the crate is condemned.

**Why:** one perf pipeline owned by one instrument; a dep arrow (either
direction) creates a half-coupled interim state the owner explicitly
rejected.

**How to apply:** never add new capability to tools/benchmark; never add a
dep on cnf-runner anywhere; build perf capability natively in cnf-runner.
Related: [[ecc-own-conformance-framework]].

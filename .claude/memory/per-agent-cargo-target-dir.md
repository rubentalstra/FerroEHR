---
name: per-agent-cargo-target-dir
description: "Concurrent cargo runs use one of four FIXED lanes target/agent-t1..t4 (owner rules 2026-07-12) — no root-target lock contention, no per-task dirs, no /tmp copies; target/ide reserved for RustRover"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: b2519eaf-9973-4146-a05b-36060f53a550
---

Owner rules (2026-07-12): when multiple agents/background tasks build the
workspace concurrently, each uses one of exactly **four fixed lanes**
`CARGO_TARGET_DIR=/Users/rubentalstra/RustroverProjects/ehrbase-rs/target/agent-t1`
… `agent-t4` on every cargo command (build, clippy, nextest). NEVER a fresh
per-task name, NEVER a /tmp or scratchpad target dir — fixed repo-local lanes
under the gitignored `target/` are reused across sessions (warm builds) and
cleaned in one place. `target/ide` is reserved for RustRover; `target-cli`
is retired (deleted 2026-07-12), do not recreate it.

**Why:** the root `./target` cargo lock serializes concurrent builds and
stalls agents; unbounded per-task/-session target dirs pile up cold copies
(1–35 GB each) and filled the disk to ~90 GB on 2026-07-12.

**How to apply:** bake the CARGO_TARGET_DIR lane into every implementer/
verifier agent prompt that runs cargo. The orchestrator's in-session cargo
runs keep the root target when nothing else is building. Hygiene: cargo never
GCs `target/debug/deps` — when `du -sh target` > ~30 GB, run `cargo clean`
(only after `pgrep -fl 'cargo|rustc'` shows no other builder — parallel
sessions share this tree) and delete stale lanes. Canonical write-up:
CLAUDE.md §"Target-dir & warm-build discipline". See also
[[concurrent-sessions-shared-tree]].

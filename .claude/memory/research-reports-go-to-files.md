---
name: research-reports-go-to-files
description: Every research deliverable (PostgreSQL docs sweeps, spec/law matrices, inventories) is written to a file the moment it exists; never left only in a subagent result or chat context
metadata:
  type: feedback
---

A research agent's report is written to a file immediately: the agent writes it itself (scratchpad `redesign/` or a repo path the orchestrator names) AND the orchestrator copies anything it needs into the repo (a `docs/plans/<topic>/` appendix committed with the plan) before moving on.

**Why:** the owner said on 2026-09-13 "this postgres research and other research needs to fill a file so it does not get lost, because this is already the second time that we fetch all docs from postgres". The first PostgreSQL 18 and prior-art reports for the storage redesign (#3337) came back inline, the transcript was not on disk after compaction, and both had to be re-run.

**How to apply:** every research-agent prompt ends with an instruction to write the full report to a named file; after the notification arrives, verify the file exists before using the content. Session transcripts under `~/.claude/projects/` are NOT a durable store. See [[session-workflow-gotchas]].

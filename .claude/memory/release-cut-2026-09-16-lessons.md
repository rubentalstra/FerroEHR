---
name: release-cut-2026-09-16-lessons
description: v4.3.1 cut lessons: tag with a scripted signed tag once the gpg cache is raised, approve crates-io via pending_deployments, the sandbox leg fails across a storage-generation boundary (nightly reseed at 00:13 UTC recovers), the classifier refuses dispatching the wipe
metadata:
  type: project
---

The v4.3.1 cut (2026-09-16, owner asleep, standing instruction to tag and approve crates.io) went: release PR by a worker (19 files, chart `version` bumped EVERY release, `validate.sh --update` rewrites the README/book chart pins itself), then `git tag -s` on the merge commit after the owner raised `default-cache-ttl`/`max-cache-ttl` in `~/.gnupg/gpg-agent.conf` to 36000 and cached the passphrase once; crates-io approval is `POST repos/…/actions/runs/<run>/pending_deployments` with `environment_ids[]=19274392425` and `state=approved` the moment `pending_deployments` lists it (the `crates-io` environment's reviewer is the owner; `hosted` has none). Every publish leg was green; the `sandbox` leg failed because the deployed image refuses the box's first-generation database at boot (502 for 600 s), which is #3466's lane defect; the recovery is `sandbox-reseed.yml` (wipe both generations' schemas, restart, seed), scheduled nightly at 00:13 UTC. The auto-mode classifier refuses `gh workflow run sandbox-reseed.yml` from here (it wipes a live database): let the schedule run or leave the dispatch to the owner.

**How to apply:** since the fix for #3467 the release's `sandbox` job wipes the schemas before it deploys and the reseed is called seed-only, so a storage-generation change needs no manual step; a manual `hosted-deploy.yml` dispatch still wipes nothing. Background pollers get killed under memory pressure on this box; use Monitor tasks. Related: [[release-pr-chart-and-third-party-images]], [[history-rewrite-2026-09-16]].

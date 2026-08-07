---
name: k8s-testing-uses-compose-postgres
description: "When testing the Helm chart on the local cluster, run PostgreSQL via docker compose and point the chart at the host — never deploy postgres into the cluster"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: dc90e7e8-afc4-4863-a4a2-d90e4bb606e0
  modified: 2026-08-07T17:22:12.901Z
---

For live Helm/Kubernetes testing, the database runs in **docker compose on the
host**, and the chart is pointed at it (`host.docker.internal:5432` from a
Docker Desktop cluster). Never deploy a postgres into the test cluster.

**Why:** the owner's call — it is easier and more stable. It is also what the
architecture already says: the chart provisions no database and takes an
external DSN (`database.existingSecret`), so a compose database is the faithful
shape rather than a convenience.

**How to apply:** bring up the compose database, create the DSN secret in the
test namespace pointing at the host, then `helm upgrade --install`. An
in-cluster `kubectl create deployment pg` has no PVC, so scaling it to zero and
back returns an EMPTY database — readiness then reports `migrations DOWN: core
schema tables missing` and pods never recover without a rollout restart. That
cost a diagnosis cycle on 2026-08-07 during the #2194 chart work.

Related: [[owner-work-style]], [[admin-ui-deprioritized]].

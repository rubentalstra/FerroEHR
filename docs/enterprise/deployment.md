# Kubernetes deployment (Helm chart)

The `deploy/helm/ehrbase-rs` chart deploys ehrbase-rs — a pure-Rust,
openEHR-conformant CDR (ITS-REST 1.0.3 + AQL 1.1) — as a hardened, non-root,
default-deny-ingress workload that connects to an **external** PostgreSQL 18.
It encodes the ADR-013 database-role posture and the ADR-013 Appendix
operational guidance (pgaudit / TLS / PITR). This is a **deployment artifact
only** — no Rust code participates.

> Governing docs: `docs/ADRs/ADR-013-enterprise-schema-baseline.md` (roles +
> Appendix §3/§5/§6: security, migration hygiene, backup/PITR),
> `docs/ADRs/ADR-008` (storage).

## Chart layout

```
deploy/helm/
├── ehrbase-rs/
│   ├── Chart.yaml              # apiVersion v2, kubeVersion >=1.25
│   ├── values.yaml             # every config surface, integrations OFF by default
│   └── templates/
│       ├── _helpers.tpl
│       ├── configmap.yaml      # non-secret EHRBASE_* env
│       ├── secret.yaml         # env secrets + mounted config files (gated)
│       ├── deployment.yaml     # probes, security context, env wiring
│       ├── service.yaml
│       ├── serviceaccount.yaml
│       ├── ingress.yaml        # optional
│       ├── networkpolicy.yaml  # default-deny ingress
│       ├── hpa.yaml            # optional (autoscaling/v2)
│       ├── poddisruptionbudget.yaml  # policy/v1
│       └── NOTES.txt
├── ci/                          # value overlays for validation
├── golden/                      # committed render snapshots
└── validate.sh                  # helm lint + template + security gate + golden
```

Quick install (production shape — external DB DSN from an existing Secret):

```bash
kubectl -n ehrbase create secret generic ehrbase-db \
  --from-literal=EHRBASE_DB_URL='postgres://ehrbase_app:***@pg-host:5432/ehrbase?sslmode=verify-full'

helm install ehrbase-rs deploy/helm/ehrbase-rs -n ehrbase \
  --set database.existingSecret=ehrbase-db \
  --set image.tag=0.1.0
```

## Database role architecture — who runs migrations (ADR-013 §3)

There is **no in-chart PostgreSQL**. A CDR stores PHI; its database must be an
externally-managed, backed-up, PITR-capable PostgreSQL 18 (a managed service or
an operator-run cluster), never a chart-side sidecar. The chart only carries the
**connection string** (preferably from an existing Secret).

ADR-013 §3 and Appendix §3.1 mandate a **four-role** model — never a
superuser at runtime:

| Role | Purpose | Used by |
|---|---|---|
| owner | owns the database | provisioning only |
| `ehrbase_migrator` | runs DDL (the append-only migrations); owns the `ext` functions | migration step |
| `ehrbase_app` | DML on `ehr.*` (INSERT/UPDATE/SELECT); `EXECUTE` on `ext` functions | **the running pod** |
| `ehrbase_reader` | SELECT only | read replicas / reporting |

The migrations create the roles idempotently, apply per-schema GRANTs +
`ALTER DEFAULT PRIVILEGES` (so future tables stay reachable), and
`REVOKE CREATE ON SCHEMA public FROM PUBLIC` (ADR-013 Appendix §3.2/§3.6).

**The runtime pod connects as `ehrbase_app`** — the DSN in `database.existingSecret`
should authenticate as that role, not the migrator or the owner.

### Applying migrations

The binary calls `run_migrations` on boot. You therefore choose one of:

- **(a) Grant the runtime DSN the migrator role** (simplest; single-tenant /
  small deployments). The app can then apply migrations itself at startup. Least
  isolation.
- **(b) Run migrations out-of-band** with a *migrator* DSN and start the app with
  its lower-privileged `ehrbase_app` DSN (recommended for least-privilege prod).
  The chart does not ship a migration Job; run migrations as a CI/CD step or a
  one-shot `Job` with the migrator credential before rolling the Deployment:

  ```bash
  # one-shot migrator pod (image carries the binary + embedded migrations)
  kubectl -n ehrbase run ehrbase-migrate --rm -it --restart=Never \
    --image=ghcr.io/rubentalstra/ehrbase-rs:0.1.0 \
    --env=EHRBASE_DB_URL="postgres://ehrbase_migrator:***@pg-host:5432/ehrbase?sslmode=verify-full" \
    --command -- /usr/local/bin/ehrbase   # boots, migrates, then serve — or use your migrate entrypoint
  ```

  In flow (b), scale the Deployment to 0 or gate its rollout until the migrator
  step succeeds so two versions never race the schema.

`migrations.runByMigratorRole` in values is an informational marker surfaced in
`helm install` NOTES; the chart never runs migrations itself.

## PostgreSQL security posture (ADR-013 Appendix §3, §6 — deployment-layer)

These are **database-side** and belong to whoever provisions PostgreSQL; the
chart references them but cannot enforce them:

- **TLS in transit (§3.6):** require `hostssl` on the PG server and set
  `?sslmode=verify-full` in the DSN. `ext` functions are plain (not SECURITY
  DEFINER); any definer function must pin `search_path`.
- **pgaudit (§3.5):** run pgaudit as the DB-layer complement to the app-level
  openEHR audit + the ATNA `system_log`. Recommended: `pgaudit.log = 'ddl, role,
  connection'` globally plus object-level audit on the PHI tables only; ship the
  audit log to an immutable store with ≈6-year retention.
- **At-rest encryption (§3.4):** encrypt at the volume/disk layer (PG has no
  native TDE). Do **not** pgcrypto-encrypt `node.data` — it would kill AQL
  jsonpath + the zero-translation storage design.
- **Backup / PITR (§6):** enable WAL archiving + PITR from day one (pgBackRest or
  a managed PITR). `UNLOGGED` is forbidden on all clinical/audit tables. The
  explicit btree `UNIQUE (vo_id, sys_version)` is the replica identity (§6.3).
- **RLS (§3.3):** skipped in single-tenant Stage 1; `ehr_id` is placed
  RLS-ready. Multi-tenancy (`tenancy.enabled`, ADR-015) adopts `FORCE ROW LEVEL
  SECURITY` — see the integrations matrix below.

## Pod security posture (chart-enforced, ADR-013)

The chart pins, and `validate.sh` asserts, the following on every render:

| Field | Value |
|---|---|
| `runAsNonRoot` | `true` (uid/gid 65532 — the distroless `nonroot` user) |
| `readOnlyRootFilesystem` | `true` (a writable `emptyDir` is mounted at `/tmp`) |
| `allowPrivilegeEscalation` | `false` |
| `capabilities.drop` | `[ALL]` |
| `seccompProfile.type` | `RuntimeDefault` (pod + container) |
| ServiceAccount token | not mounted (the workload never calls the K8s API) |
| NetworkPolicy | default-deny ingress; only the API (and management) port admitted |

The image is `gcr.io/distroless/cc-debian12:nonroot` — shell-less, no package
manager. Egress restriction is opt-in (`networkPolicy.egress.enabled`) because
egress targets (DB, broker, terminology server) are deployment-specific; when
enabled the chart always admits DNS and you add the DB/broker rules.

## Probe and metric endpoints

Probes use the management surface's public, unauthenticated, PHI-free health
routes (`ehrbase-rest/src/management/health.rs`):

| Probe | Route | HTTP status contract |
|---|---|---|
| liveness | `{management.basePath}/health/liveness` | 200 iff the process is up |
| readiness | `{management.basePath}/health/readiness` | 200 (UP/DEGRADED) / 503 (DOWN): DB ping + migrations-applied + audit-sender + events, each 1s-bounded |
| startup | liveness route | gates slow first boot |

Because the bare binary ships the management surface **off**, the chart defaults
`management.enabled=true` + `management.probesEnabled=true` (a deliberate
deployment deviation — the probe routes carry no PHI). Set
`management.port` to serve the surface on its **own** internal listener so
`/management` is never exposed on the clinical API port. To use the binary's
`ehrbase healthcheck` subcommand instead of HTTP probes (e.g. management off),
set `probes.exec.enabled=true`.

Metrics: set `metrics.enabled=true` to expose `{management.basePath}/prometheus`
(access level `public`) and add the `prometheus.io/{scrape,port,path}` pod
annotations. The port annotation follows the management listener (main port or
the separate `management.port`). The JSON `/management/metrics`, `/info`,
`/env`, and `/loggers` endpoints stay `admin_only`/off unless opted in.

## Upgrade strategy

- **Append-only migrations (ADR-013 §1, Appendix §5.1):** migrations are
  never edited once applied; a new schema change is a new `000N` file. A rolling
  Deployment upgrade must be compatible with the *previous* schema for the
  window where both versions run — additive DDL first, destructive changes in a
  later release after all pods are on the new version.
- **Lock-safe DDL (ADR-013 Appendix §5.2/§5.3):** the migration runner wraps DDL in
  a `lock_timeout` (≈5s) + bounded `statement_timeout` so a migration cannot
  block live traffic indefinitely on a lock; on a busy table use `CREATE INDEX
  CONCURRENTLY` and `NOT VALID` + later `VALIDATE`. Set these on the migrator
  connection, e.g. append `?options=-c%20lock_timeout%3D5000` to the migrator
  DSN, or `SET lock_timeout` at the session start of the out-of-band migrate step.
- **Image pinning:** always deploy an immutable tag or, better, a `@sha256`
  digest (`image.tag`); never `latest`. `imagePullPolicy: IfNotPresent` with a
  pinned tag gives reproducible rollouts. Roll back by re-pinning the prior
  tag/digest — the schema's backward compatibility (above) makes this safe.
- **PDB + surge:** `podDisruptionBudget` (default `minAvailable: 1`) keeps a
  replica serving during node drains; keep `replicaCount >= 2` (or autoscaling)
  so upgrades and disruptions never fully interrupt the API.
- **Graceful drain:** `terminationGracePeriodSeconds` (default 30) covers the
  binary's 5s audit/outbox drain on shutdown.

## Optional integrations matrix

Every integration is **OFF by default**, matching the binary. Each is a separate
switch; enabling one is an explicit, auditable deployment decision. Security
notes call out the PHI-bearing ones.

| Integration | Values key | Env prefix | Dependency | Security note |
|---|---|---|---|---|
| ADMIN API | `rest.adminEnabled` | `EHRBASE_REST_ADMIN__` | — | Physical/irreversible delete. Gate behind admin RBAC; off unless needed. |
| Terminology ext API | `rest.terminologyEnabled` | `EHRBASE_REST_TERMINOLOGY__` | — | Extension surface (404 when off). |
| Event-subscription API | `rest.eventSubscriptionEnabled` | `EHRBASE_REST_EVENT_SUBSCRIPTION__` | — | Admin CRUD over event filters. |
| Multi-tenancy | `tenancy.enabled` | `EHRBASE_REST_TENANCY__` | RLS (Stage 2) | Tenant resolved from a JWT claim; leave `tenancy.header` unset in prod (a client header must not select a tenant). Pairs with PG `FORCE ROW LEVEL SECURITY`. |
| OAuth2/OIDC auth | `auth.oidc.*` | `EHRBASE_REST_AUTH__OIDC__` | IdP (Keycloak) | Prefer JWKS/discovery over an HS256 secret. HS256 secret lands in the chart Secret. |
| ABAC | `authz.abac.enabled` | `EHRBASE_AUTHZ_ABAC__` | Cedar/remote PDP | Policies via a mounted `config.files` TOML. |
| **Eventing → AMQP** | `events.enabled` | `EHRBASE_EVENTS_` | RabbitMQ | Envelopes are **PHI-free by design** (ADR-014 §2). Broker URL carries creds → chart Secret; use `amqps://`/`tls=true`. |
| **FHIR inbound/façade** | `rest.fhirEnabled` | `EHRBASE_REST_FHIR__` | — | Read façade + inbound mapping (ADR-016). |
| **FHIR outbound → AMQP** | `fhirOutbound.enabled` | `EHRBASE_FHIR_OUTBOUND_` | RabbitMQ | ⚠ **Carries PHI** — the payload is the mapped FHIR *resource*. Publishes to a **separate exchange** (`ehrbase.fhir`) from the PHI-free event stream so broker ACLs can isolate it. Enable only with a TLS, access-controlled broker; an explicit, audited decision. |
| **S3 multimedia** | `multimedia.enabled` | `EHRBASE_MULTIMEDIA_` | S3/MinIO/SeaweedFS | ⚠ Offloaded blobs are **PHI**. Bucket must be private + encrypted + HTTPS. `allowHttp` is dev-only (SeaweedFS). Prefer IRSA/Workload-Identity over static keys. |
| External terminology | `externalTerminology.enabled` | `EHRBASE_VALIDATION_EXTERNAL_TERMINOLOGY_` | FHIR TS | Provider map via `config.files` TOML + `EHRBASE_VALIDATION_CONFIG` in `extraEnv`. |
| ATNA system log | `atna.enabled` | `EHRBASE_ATNA_` | ARR (syslog) | Use `transport: tls` for PHI-adjacent audit; TLS PEMs via `config.files`. `failMode: closed` rejects auditable ops when auditing is undeliverable. |
| Version signing | `signing.*` | `EHRBASE_SIGNING_` | — | On by default (`digest`). `pgp` mode fails **closed** at boot without a usable key; passphrase → chart Secret, key → `config.files`. |
| OTLP telemetry | `telemetry.otel.*` | `EHRBASE_OTEL_` | collector | Unset endpoint ⇒ OTel layer not installed (zero overhead). |

### The PHI-exchange warning (fhirOutbound)

`events` (ADR-014) publishes **PHI-free** envelopes — safe to fan out broadly.
`fhirOutbound` (ADR-016 §4a) publishes the **mapped clinical FHIR resource** —
PHI. They are deliberately independent switches on **different exchanges**
(`ehrbase.events` vs `ehrbase.fhir`) precisely so broker-level access control can
restrict the PHI-bearing stream without touching the envelope stream. When
enabling `fhirOutbound`: use a dedicated, TLS-only broker connection, lock down
the `ehrbase.fhir` exchange bindings, and treat every consumer as a PHI
processor.

## Configuration surface reference

The chart drives the binary entirely through `EHRBASE_*` environment variables
(from the ConfigMap for non-secret keys, valueFrom-Secret for secret keys), plus
optional mounted TOML/PEM files (`config.files` → `/etc/ehrbase/*`) for the
values env can't carry cleanly (the Basic-auth user store, full OIDC/ABAC/
terminology blocks, ATNA TLS PEMs, the PGP key). Every `values.yaml` key
documents its exact env var. Secrets never enter the ConfigMap; the DB DSN
should come from `database.existingSecret`.

## Validation

`deploy/helm/validate.sh` runs `helm lint` + `helm template` for the default and
all-features value sets, asserts the rendered manifests are valid multi-document
YAML, pins the security fields above, diffs against `deploy/helm/golden/`, and
runs `kubeconform` when it is on PATH (skipped with a note otherwise). Regenerate
the golden renders with `deploy/helm/validate.sh --update` after an intended
template change and review `git diff deploy/helm/golden`.

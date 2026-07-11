# E5 — Kubernetes deployment artifacts

- Status: in-progress
- Started: 2026-07-11   Owner: Ruben
- Governing design: docs/enterprise/product-roadmap.md §3 E5 — a Helm chart
  encoding the ADR-013 security posture + review doc 02's ops guidance.
  Deployment artifacts only; zero Rust changes (ECC gate trivially holds).

## Tasks

- [ ] 1. Helm chart (deploy/helm/ehrbase-rs): Deployment (distroless image,
      resources, probes wired to /management health), Service, ConfigMap/
      Secret-driven EHRBASE_* env (DB DSN, auth, tenancy/events/fhir/
      multimedia flags all off by default), optional dependencies toggles
      (postgres via external DSN only — never an in-chart PG for prod),
      RabbitMQ/SeaweedFS endpoints as values, NetworkPolicy, PodSecurity
      (runAsNonRoot, readOnlyRootFilesystem), HPA optional.
- [ ] 2. Ops documentation (docs/enterprise/deployment.md): the DB role
      architecture (migrator vs app_writer per ADR-013 — who runs
      migrations), pgaudit/TLS/PITR posture pointers (review doc 02),
      probe/metric endpoints, upgrade strategy (append-only migrations,
      lock_timeout wrapper guidance).
- [ ] 3. Chart validation: helm lint + helm template golden render in CI
      (a script or test asserting the rendered manifests parse and pin the
      security fields); kubeconform if available offline.

## Exit criteria

- [ ] helm lint clean + golden render committed; deployment doc complete;
      scorecard flipped. (No Rust changes — suites/ECC unaffected.)

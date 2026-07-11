# E5 — Kubernetes deployment artifacts

- Status: done (2026-07-11)
- Started: 2026-07-11   Owner: Ruben
- Governing design: docs/enterprise/product-roadmap.md §3 E5 — a Helm chart
  encoding the ADR-013 security posture + review doc 02's ops guidance.
  Deployment artifacts only; zero Rust changes (ECC gate trivially holds).

## Tasks

- [x] 1. Helm chart (deploy/helm/ehrbase-rs): Deployment (distroless image,
      resources, probes wired to /management health), Service, ConfigMap/
      Secret-driven EHRBASE_* env (DB DSN, auth, tenancy/events/fhir/
      multimedia flags all off by default), optional dependencies toggles
      (postgres via external DSN only — never an in-chart PG for prod),
      RabbitMQ/SeaweedFS endpoints as values, NetworkPolicy, PodSecurity
      (runAsNonRoot, readOnlyRootFilesystem), HPA optional.
      — Done: apiVersion v2, every EHRBASE_* surface surfaced (all integrations
      OFF matching the binary), external-DSN-only DB (existing-Secret ref),
      default-deny NetworkPolicy, seccomp RuntimeDefault, HPA/PDB/Ingress
      optional. lint clean; 9 resource kinds render.
- [x] 2. Ops documentation (docs/enterprise/deployment.md): the DB role
      architecture (migrator vs app_writer per ADR-013 — who runs
      migrations), pgaudit/TLS/PITR posture pointers (review doc 02),
      probe/metric endpoints, upgrade strategy (append-only migrations,
      lock_timeout wrapper guidance).
      — Done: four-role model + both migration flows, review-doc-02 §3/§6
      pointers, probe/metric table, upgrade strategy, integrations matrix with
      the PHI-exchange warning (fhirOutbound).
- [x] 3. Chart validation: helm lint + helm template golden render in CI
      (a script or test asserting the rendered manifests parse and pin the
      security fields); kubeconform if available offline.
      — Done: deploy/helm/validate.sh (lint + template + PyYAML validity +
      security-field gate + golden diff + optional kubeconform); golden renders
      committed under deploy/helm/golden/ (+ regen README). Passes on helm
      v4.1.3 (schema v2 forward-compatible with latest v4.2.3).

## Exit criteria

- [x] helm lint clean + golden render committed; deployment doc complete;
      scorecard flipped. (No Rust changes — suites/ECC unaffected.)
      — helm lint clean (both value sets), golden committed, deployment doc
      complete. Scorecard flip + commit left to the orchestrator.

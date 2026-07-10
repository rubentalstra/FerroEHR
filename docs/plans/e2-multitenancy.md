# E2 — Fully-integrated multi-tenancy (Stage-2 flagship)

- Status: in-progress
- Started: 2026-07-11   Owner: Ruben
- Governing design: docs/enterprise/product-roadmap.md §2.3 (owner-confirmed)
  → ADR-015; spec basis BASE architecture-overview §The EHR System (one
  deployment "may house multiple logical EHR systems in a multi-tenant
  fashion"; boundary = system_id); tenancy model itself is spec-silent.
- Gates: workspace suites green; full ECC zero drift (341/315/0) in the
  default single-tenant mode; tenant-isolation integration tests.

## Tasks

- [x] 1. ADR-015 — tenancy design record: tenant ↔ logical openEHR system
      (per-tenant system_id), session tenant context (JWT claim →
      SET LOCAL), RLS with FORCE on tenant-scoped tables, single-tenant
      default (tenancy off = today's behaviour, zero overhead).
- [ ] 2. Schema: migration — `tenant` table + `tenant_id` on the scoping
      roots (ehr, definition stores, stored_query, sp_*, event outbox/subs)
      with defaults preserving single-tenant behaviour; RLS policies
      (FORCE) keyed on `current_setting('ehrbase.tenant_id')`, enabled only
      when tenancy is on; baseline discipline throughout.
- [ ] 3. Service/wire: TenancyConfig (off by default); tenant resolution
      middleware (JWT claim / header per config) → per-request SET LOCAL in
      the pool acquisition path; per-tenant system_id in audits/version
      ids; admin tenant CRUD (config-gated extension routes, adapter-trait
      precedent).
- [ ] 4. Tests: cross-tenant isolation (tenant A cannot read/write/list B's
      EHRs, templates, queries, subscriptions — engine-enforced via RLS);
      single-tenant mode unchanged (full suites); per-tenant ECC smoke
      (two tenants on one SUT, filtered runs both green).

## Exit criteria

- [ ] ADR-015 accepted; isolation proven engine-level; default-mode ECC
      341/315/0 zero drift; roadmap scorecard flipped.

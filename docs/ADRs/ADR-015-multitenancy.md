# ADR-015: Fully-integrated multi-tenancy (tenant = logical openEHR system)

- **Status:** accepted (design owner-confirmed 2026-07-10 via product
  roadmap §2.3)
- **Date:** 2026-07-11
- **Spec basis:** BASE architecture-overview §The EHR System — a "system" is
  the legally-responsible logical repository identified by `system_id`, and
  one deployment "may house multiple logical EHR systems in a multi-tenant
  fashion"; the spec boundary is `system_id` (EHR, AUDIT_DETAILS, every
  OBJECT_VERSION_ID's creating_system_id). The tenancy model itself is
  **spec-silent** — this ADR fills it (B8 spec-grounding pass, capability 4).

## Decision

1. **Tenant = one logical openEHR system.** A `tenant` row carries id, name,
   and its **own `system_id`** — so per-tenant version identity, audits and
   EHR.system_id stay spec-correct per tenant, and a tenant's data is a
   self-contained logical repository (exportable/movable per master06).
2. **Engine-enforced isolation: PostgreSQL RLS with FORCE.** `tenant_id`
   (uuid, FK) on the scoping roots: `ehr`, `template_store`,
   `archetype_store`, `adl2_artefact`, `stored_query`, `sp_*`,
   `event_subscription`, `event_outbox`, plus `contribution`/`vo_version`/
   `node`/`item_tag`/`audit` (denormalized for RLS on the hot tables —
   child rows carry their root's tenant). Policies compare against
   `current_setting('ehrbase.tenant_id')`; `FORCE ROW LEVEL SECURITY` so
   even the table owner is filtered.
3. **Single-tenant default, zero overhead.** Tenancy is OFF by default: a
   reserved default tenant (fixed uuid) owns all rows; RLS policies are
   created but the default session setting matches, and the middleware is a
   no-op — today's behaviour and performance are unchanged, and every
   existing test/ECC run passes unmodified.
4. **Tenant resolution at the edge.** `TenancyConfig` selects the source
   (JWT claim `tenant` by default; optional header for dev). The resolved
   tenant becomes a per-request `SET LOCAL ehrbase.tenant_id` on the
   acquired connection (a pool wrapper at the service boundary), so every
   statement in the request's transactions is RLS-scoped. Cross-tenant =
   engine-level empty set, not a 403 (no existence leakage).
5. **Admin surface.** Config-gated `/admin/tenant` CRUD (extension-route +
   adapter-trait precedents); tenant deletion only when empty (physical
   purge of a tenant is the existing admin delete machinery per EHR).
6. **Conformance.** Default-mode full ECC must stay zero-drift; multi-tenant
   mode is evidenced by isolation integration tests + a two-tenant ECC
   smoke (filtered runs against one SUT with two tenant credentials).

## Consequences

- Append-only migrations (tenant table, tenant_id columns with the default
  backfill, policies); service pool acquisition gains the SET LOCAL hook;
  auth middleware maps claim→tenant; audits/version ids read the tenant's
  system_id instead of the global config when tenancy is on.
- RLS predicate cost is paid only in multi-tenant mode (policy true-check
  in default mode); measured at P20.

## Alternatives considered

App-level scoping only (HIP's asterisked tier — weaker guarantee, rejected
by the owner's "fully integrated" choice); schema-per-tenant (operationally
heavy: migrations × tenants, connection-pool fragmentation); database-per-
tenant (maximal isolation but kills shared-infrastructure economics; the
per-EHR dump/load already covers extraction).

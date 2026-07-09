# Current phase

**The roadmap is `docs/blueprint/00-THE-BLUEPRINT.md`** — read it first. It is
the single source of truth for the trajectory toward "first fully
spec-compliant openEHR CDR". This file is the live pointer under it; the
consolidated gap surface is `docs/GAP_REGISTER.md`.

## Active work — the ADR-011 rebuild convergence

The current push is the **ADR-011 app-crate redesign** landing as the closing
waves of **SM-4** (`docs/plans/sm-phase-04-terminology-admin.md`):
compile-time-complete services, no stub backend, a protocol-free `ehrbase-sm`
native API (literal SM interface catalog), `Platform`-generic adapter state,
and the dissolution of the former `ehrbase-audit`/`ehrbase-signing` leaf crates
into modules of `ehrbase` (`system_log`, `signing`) with the op-id
classification + audit middleware in `ehrbase-rest`. The workspace is **red by
design mid-rewrite**; the gate to close on is *workspace green + ECC
zero-drift* (211/318 baseline — ECC is suspended during the rebuild and
re-converges at P19).

Current app layout (3 crates + tools): `app/{ehrbase, ehrbase-rest,
ehrbase-sm}`, `tools/{conformance, benchmark}`, `crates/openehr-*`.

## Priority order (from GAP_REGISTER §3)

1. **Finish the ADR-011 rebuild** — green workspace, ECC re-converged.
2. **ArchetypeValidation depth** — the single biggest gap (81 failing ECC
   cases, ~76% of all failures); needs its own validation-depth phase with the
   ECC data sets as the oracle, before/at P19.
3. **SM-4 close** (Admin dump/load) → SM-5 (Message / EHR Extract / TDD) →
   SM-6 (Subject Proxy) — designed in `docs/design/sm-platform/`.
4. **P19** ECC re-convergence + the remaining small wire-edge tails
   (`phase-19-conformance-parity.md`); then P20 optimization, P99 cutover.

## Remaining Stage-1 P-phases

`phase-17` (EhrScape + admin compat), `phase-18` (workspace integration),
`phase-19` (openEHR conformance), `phase-20` (optimization), `phase-99`
(cutover) — sequenced under the blueprint; SM phases interleave.

**Read first:** `docs/blueprint/00-THE-BLUEPRINT.md`, then
`docs/ADRs/ADR-011-app-crate-redesign.md` (current app-crate reality) and
`docs/ADRs/ADR-008-greenfield-pg18-storage.md` (own PG18 internals; the node
table + temporal `vo_version`, `ALL_VERSIONS` supported).

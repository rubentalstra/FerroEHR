# ECC coverage review — the one-time checklist against the legacy CNF corpus

> **Blueprint anchor:** the live CNF/conformance authority is blueprint chapter
> [`07-cnf.md`](../blueprint/07-cnf.md) (framework state + runner audit). This file
> is a **completed one-time design review** (2026-07-08) proving the ECC
> catalogue covers every behaviour the legacy CNF corpus tested — kept as
> verification evidence, not a live tracker. Its `▶ build-out` items are now
> ordered work in the blueprint build order (B2–B6). Revisit only if the
> vendored reference corpus is re-vendored.

Design v4 §3 requires a **human design review** (not machinery): every
behaviour the legacy openEHR CNF corpus tested must be covered by the ECC
catalogue *by design* — as existing cases or as a named build-out item in
`docs/plans/s2-phase-05-cnf-engine-rewrite.md`. This is that review, built
from the exhaustive 2026-07-08 corpus inventories (schedule: 324 headings /
~1,371 truth rows; robot: 464 cases; fixtures; profiles/guide/certificate
docs). It is a checklist for authors; it carries no ids into the framework.

Legend: **✔ covered** = ECC cases exist today · **▶ build-out** = named plan
task · **✖ out of scope** = deliberate, with reason.

## 1. Functional service behaviours

| Legacy corpus tested (behaviour clusters) | ECC | Where |
|---|---|---|
| EHR create (default + provided EHR_STATUS incl. the 16-combination valid matrix; invalid EHR_STATUS classes; duplicate EHR; two EHRs same patient; get by id/subject; has_ehr) | ✔ + ▶ | `EHR` area (12 cases); the 16-combination valid matrix + invalid classes become generated variants (▶ VAL/EHR generator task) |
| EHR_STATUS get / set+clear queryable / set+clear modifiable / bad-EHR negatives / versioned status reads (revision history, at-time, at-version) | ✔ | `STA` area (10 cases); versioned-object read matrix also ▶ REST task |
| COMPOSITION create (event/persistent, invalid, bad OPT, bad EHR, same-OPT-twice), update (incl. wrong template, preceding-version), delete (logical delete → new VERSION, lifecycle `openehr::523|deleted|`), get latest/at-time/at-version/versioned, has_composition, version number = 1 on create, time-zone variants | ✔ | `COM` area (31 cases) |
| CONTRIBUTION: transactional commit (any invalid VERSION rejects the whole set), change_type × lifecycle × validity matrix, multi-version commits, EHR_STATUS commits (subject external_ref variants; creation/deleted rejected), FOLDER commits, list/has/get + negatives | ✔ + ▶ | `CTB` area (31 cases); the commit matrices become generated variants (▶ VAL generator task) |
| DIRECTORY: full FOLDER CRUD, at-time (incl. empty-time variants, multi-version), at-version, versioned-directory, has_path against a reference tree (path data sets incl. random negatives + special characters) | ✔ | `DIR` area (37 cases) |
| OPT provisioning: upload/validate/get/get-all/delete × valid/invalid OPT classes (empty file, empty template_id, removed mandatory elements, alien tags, duplicate template_id conflict/no-conflict, versioned templates latest/specific) | ✔ | `TPL` area (16 cases); invalid-OPT payloads reused from `testdata` |
| Stored queries: store valid/invalid/bad formalism, list empty/non-empty/select | ✔ | `SQR` area (7 cases) |
| AQL execution: ad-hoc + stored, empty vs loaded DB, the A–D query corpus (119 valid queries) with golden RESULTSETs, invalid queries rejected, smoke tests | ✔ + ▶ | `QRY` area (13 cases today); full corpus + goldens + AQL 1.1 construct checklist is the ▶ QRY build-out task |
| Demographic party/party-relationship CRUD + versioning (legacy chapter was an empty stub; robots absent) | ✔ | `DEM` area (24 cases — ours exceed the legacy corpus, which shipped nothing) |
| Admin: EHR/composition/contribution/directory/template/cache admin ops (legacy chapter stub; 29 robots) | ✔ + ▶ | `ADM` area (6 cases); template-admin depth (20 robot cases' behaviours) folded into ▶ REST matrix + ADM growth |
| Messaging (EHR Extract / TDS import) | ✖ | Not implemented in the server (OPTIONS-only in the legacy profiles too); `MSG` area reserved |
| ADL2/OPT2 provisioning | ✖ | Server returns explicit 501; OPTIONS-only. Covered as REST-matrix negative evidence (▶ REST task) |

## 2. Content / data-validation behaviours (legacy master15–17 → `VAL`)

The legacy truth tables enumerate accept/reject rows per constraint. The ECC
covers the same constraint *semantics* and generates the rows (▶ VAL
generator task = cardinality grids {0..*, 1..*, 3..*, 0..1, 1..1, 3..5} ×
presence/absence × border values — a superset of the legacy 1,371 rows):

- ✔ today (80 DV cases + 26 entry-structure + 12 composition-structure):
  COMPOSITION content cardinality × context presence; OBSERVATION
  state/protocol existence; HISTORY events cardinality + summary existence;
  EVENT/ITEM_STRUCTURE type restriction; DV_BOOLEAN true/false lists;
  DV_IDENTIFIER pattern/list (issuer/assigner/id/type); DV_TEXT/DV_CODED_TEXT
  pattern/list/local codes/external terminology; DV_ORDINAL/DV_SCALE lists;
  DV_COUNT open/range/list; DV_QUANTITY property/units/magnitude;
  DV_PROPORTION kinds; DV_INTERVAL over COUNT/QUANTITY/DATE_TIME/DATE/TIME/
  DURATION/ORDINAL/SCALE/PROPORTION; DV_DURATION/DATE/TIME/DATE_TIME
  fields/ranges/patterns; DV_PARSABLE value/formalism; DV_MULTIMEDIA media
  type; DV_URI/DV_EHR_URI pattern/list (RFC 3986; `ehr:` scheme).
- ▶ build-out: the full generated row grids per case (per-variant
  `ECC-VAL-nnn.vv` outcomes); DV_STATE + DV_PARAGRAPH + time-specification
  types (legacy left them TBD/unused — we decide from RM 1.2.0 text whether
  to test or record as unsupported-by-design).

## 3. Non-functional behaviours

| Legacy corpus | ECC |
|---|---|
| SECURITY_TESTS (OAuth2/Keycloak flows, 8 robots) | ▶ `SEC` sweeps task (RBAC 401/403 under Basic + Bearer; compose-tier Keycloak) |
| Signing (legacy: zero test material despite being a STANDARD capability) | ✔ `SIG` area (5 cases — ours are the capability's only evidence anywhere) |
| Anonymous EHRs | ✔ within `EHR` (create without subject) |
| Profiles / certificate-shaped claim | ✔ machine profile verdict (`model/profile.rs`) + generated statement |
| RM-version declaration in the claim | ✔ `SpecVersions` in model/config/statement |

## 4. Verdict

No behaviour tested by the legacy corpus is absent from the ECC design:
everything is either covered by existing cases or carried by a named
build-out task in the phase plan. The reverse is decidedly not true — the
ECC exceeds the legacy corpus on signing, security, demographic, admin,
`ALL_VERSIONS`, exact status-code coverage (REST matrix), and generated
validation grids. Review complete 2026-07-08; revisit only if the vendored
reference corpus is ever re-vendored with new content (upstream is dormant).

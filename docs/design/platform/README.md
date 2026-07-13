# Platform-crate design registers (W-3f)

Spec-first registers for the redesign of `app/ehrbase` (the binary +
`Platform` implementation). Method (owner ruling 2026-07-12): **map the spec
onto the code** — each register's spine is the governing spec chapter
enumerated section-by-section; the code is mapped onto each item
(conformant / divergent / missing); code with no spec home is flagged
spec-silent / quarantined to `extensions/` / deleted. The target design in
each register derives from the spec's own decomposition, never from the
legacy file layout.

## Registers — area → oracle

| # | Register | Oracle (primary) | Target |
|---|---|---|---|
| 01 | [Versioning + integrity](01-versioning-integrity.md) | RM common master06 (change_control incl. §Digital Signature) + master04; BASE base_types master05; arch-overview master07/08/09 | `src/versioning/` (signing/ dissolves into `versioning/signature/`) |
| 02 | [Storage / node codec / db](02-storage.md) | spec-silent internals (flagged) + base_types master05 identifier law + canonical-JSON fidelity | `src/storage/`, `src/db/` |
| 03 | [Service: EHR](03-service-ehr.md) | arch-overview master06 (design of the EHR) × SM I_EHR_* (42 ops) | `src/service/ehr/` |
| 04 | [Service: demographic + EHR index](04-service-demographic-ehr-index.md) | RM demographic + SM I_DEMOGRAPHIC/I_PARTY/I_PARTY_RELATIONSHIP/I_EHR_INDEX | `src/service/demographic/`, `src/service/ehr_index/` |
| 05 | [Service: definition + query](05-service-definition-query.md) | SM I_DEFINITION_ADL14/ADL2/QUERY + I_QUERY_SERVICE; QUERY spec service semantics | `src/service/definition/`, `src/service/query/` |
| 06 | [Service: message + admin](06-service-message-admin.md) | RM ehr_extract + RM common (IMPORTED_VERSION) + SM I_EHR_EXTRACT/I_TDD/I_ADMIN_* | `src/service/message/`, `src/service/admin/` |
| 07 | [Service: subject proxy + terminology + validity](07-service-subject-proxy-terminology-validity.md) | SM I_SUBJECT_PROXY/I_DATA_BINDING/I_TERMINOLOGY/I_VALIDITY_CHECKER; arch-overview master12 | `src/service/subject_proxy/`, `src/service/terminology/`, `src/service/validity.rs` |
| 08 | [AQL engine](08-aql.md) | QUERY master03 (38 constructs) + arch-overview master11 (paths); lowering internals spec-silent (flagged) | `src/aql/` (sql.rs → `sql/` split) |
| 09 | [Validation](09-validation.md) | AOM 1.4/2.4 constraint taxonomy (20 rows); foundation_types interval/multiplicity | `src/validation/` |
| 10 | [Templates](10-templates.md) | BASE resource (AUTHORED_RESOURCE) + AM OPT + base_types identity law | `src/templates/` |
| 11 | [System log](11-system-log.md) | SM master02 ATNA line (+ external DICOM PS3.15 / RFC 3881 / RFC 5424/5425) | `src/system_log/` (re-ground) |
| 12 | [Extensions quarantine](12-extensions.md) | spec-homeless by verified inverse mapping (master13/14 checked) | `src/extensions/` |

## Cross-register rulings (orchestrator, 2026-07-12)

Where two registers claimed or disclaimed the same code, the resolution is:

- **`ehr_access_cache.rs` → `service/ehr/access.rs`** (register 03's
  design). EHR_ACCESS is spec-governed (arch-overview master06 §EHR_ACCESS);
  the cache mechanics ride along as a flagged spec-silent internal. Register
  12's reassignment out of extensions/ stands; its landing spot is the EHR
  chapter, not versioning.
- **`codes.rs` → `versioning/`** (register 12's reassignment). The
  change_type / lifecycle code groups are RM common master06/master04 law;
  non-versioning consumers import from `versioning` (or receive a
  `TODO(w3f-integrate)` at the fix pass).
- **Storage owns a `version_repo` beside `node_repo`.** Register 01 §5
  assigns ALL `vo_version`/`audit`/`contribution`/`vo_attestation` SQL to
  storage; register 02's layout named only the node side. The storage target
  gains `version_repo.rs` (row I/O for version/audit/contribution/
  attestation, signatures driven by versioning's builders). Versioning codes
  against it; mismatches reconcile at the ONE fix pass.
- **Event outbox**: the row write stays inside the commit transaction
  (storage executes it as part of the version commit); the payload building
  and draining live in `extensions/events/`. Wired at the fix pass.
- **`src/terminology/` (FHIR provider) merges under `service/terminology/`**
  (register 07: SM defines one I_TERMINOLOGY_SERVICE; the interface/provider
  split is logical, not structural).
- **`subject_proxy/` is kept as-is** (W-3c pattern donor), re-grounded
  headers only.

## Stage-2 ownership matrix (disjoint file ownership)

Workers create files ONLY under their target paths. `lib.rs`, `main.rs`,
`service/mod.rs`, all legacy deletions, and the single fix pass are
orchestrator-owned. Cross-area needs are `// TODO(w3f-integrate):` markers.

| Worker | Creates under | Carries logic from (legacy, deleted later) |
|---|---|---|
| versioning | `src/versioning/**` | vobject.rs (semantics), contribution.rs, version_id.rs, versioned.rs, signing/, codes.rs |
| storage | `src/storage/**`, `src/db/**` | storage/*, db/*, vobject.rs (SQL/plumbing), dump_load.rs (reassemble) |
| ehr | `src/service/ehr/**` | ehr.rs, composition.rs, directory.rs, item_tag.rs, ehr_uri.rs, ehr_access_cache.rs, api/ehr.rs |
| demographic | `src/service/demographic/**`, `src/service/ehr_index/**` | demographic.rs, relationship.rs, ehr_index.rs, api/demographic.rs |
| definition-query | `src/service/definition/**`, `src/service/query/**` | definition.rs, stored_query.rs, aql_query.rs, api/definition.rs |
| message-admin | `src/service/message/**`, `src/service/admin/**` | message.rs, dump_load.rs (service half), tdd.rs, admin.rs, api/admin.rs |
| terminology-validity | `src/service/terminology/**`, `src/service/validity.rs`, `src/service/subject_proxy/**` (re-ground) | service/terminology.rs, src/terminology/, api validity impl |
| aql | `src/aql/**` | aql/* (sql.rs → sql/) |
| validation | `src/validation/**` | opt_validation.rs(+/), adl2_validation.rs(+/) |
| templates | `src/templates/**` | template.rs (+ store/cache seams) |
| system_log | `src/system_log/**` (re-ground) | system_log/* |
| extensions | `src/extensions/**` | events/, service/fhir/, fhir_outbound/, multimedia/, tenant.rs, event_subscription.rs |

## W-3f executed (2026-07-13)

The redesign landed. `app/ehrbase/src` is now organised by the spec's own
decomposition rather than the legacy flat-file layout, and the standalone
`signing/` module was dissolved into versioning. Final module map:

- **`versioning/`** — change-control semantics + builders + the dissolved
  `signature/` submodule + the five-state `lifecycle.rs` machine.
- **`storage/`** + **`db/`** — node codec, `node_repo`/`version_repo`/
  `ehr_repo`/`tag_repo`, the completed `iden` catalog (the D1 semantics/SQL
  seam).
- **`service/`** — the ten SM chapters (`ehr/`, `demographic/`, `ehr_index/`,
  `definition/`, `query/`, `message/`, `admin/`, `subject_proxy/`,
  `terminology/`) plus the `validity.rs` peer file.
- **`aql/`** — `sql.rs` split into `aql/sql/` (`from`/`select`/`predicate`/
  `expr`/`value`).
- **`validation/`** — the artefact validators (`opt/`, `adl2/`, `structure.rs`)
  moved out of `service/` and split along AM boundaries.
- **`templates/`**, **`system_log/`** (re-grounded on the external ATNA
  standards), **`extensions/`** (the quarantined spec-silent modules: `events/`,
  `fhir/`, `multimedia/`, `tenancy.rs`).

Per-register closure tables were appended to each of `01`–`12` (a
`## W-3f closure (2026-07-13)` section mapping every G-row to its disposition —
FIXED in code / carried PORT NOTE / reassigned — with `file:line` or PORT-NOTE
evidence). Every register closed with **Open residue: none**.

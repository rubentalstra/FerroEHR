# Versioning + Integrity — spec-first redesign (W-3f)

Owner ruling (2026-07-12): map the **spec onto the code**, never the code onto
the spec. This register's spine is the openEHR change-control / identification /
integrity model enumerated section-by-section from the vendored oracle; the
existing `ehrbase` code is then mapped onto each item with a `file:line`
verdict. The target is a fresh `app/ehrbase/src/versioning/` module whose layout
**derives from the spec's own decomposition** (§4), into which the standalone
`app/ehrbase/src/signing/` module **dissolves** (§3, §4) — the spec places the
digital signature *inside* the version / change-control model, not beside it.

**Spec oracles** (precedence order — read before any change):

1. `docs/specs/openehr/RM/docs/common/master06-change_control_package.adoc`
   — the change-control law (VERSIONED_OBJECT, VERSION, ORIGINAL/IMPORTED,
   CONTRIBUTION, committal & audits, **Digital Signature**, Attestation,
   version lifecycle, logical deletion, version identification,
   copying/merging/moving).
2. `docs/specs/openehr/RM/docs/common/master04-generic_package.adoc`
   — AUDIT_DETAILS, ATTESTATION, REVISION_HISTORY(_ITEM), PARTY_PROXY family.
3. `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
   — OBJECT_VERSION_ID / VERSION_TREE_ID lexical forms, UID/HIER_OBJECT_ID,
   OBJECT_REF/PARTY_REF, composite-identifier case rules. **The versioning
   core's identification law.**
4. `docs/specs/openehr/BASE/docs/architecture_overview/master08-versioning.adoc`
   — the CM paradigm, change-set-as-transaction, virtual version tree.
5. `docs/specs/openehr/BASE/docs/architecture_overview/master07-security.adoc`
   §Integrity — versioning + digital signature as the integrity mechanism.
6. `docs/specs/openehr/BASE/docs/architecture_overview/master09-identification.adoc`
   — the three identification levels; content-`uid` = OBJECT_VERSION_ID copy.

**Prior verdicts cross-referenced, not re-derived:** the PR #33 formal
change-control audit (blueprint §2.1 — the seven findings fixed: five-state
lifecycle, per-version `creating_system_id`, audit copy rule, jsonb round-trip,
`System_id_valid` CHECK, merge/indelibility PORT NOTEs); blueprint ch 01 (RM
change control DONE, formally audited 1:1); `docs/spec-audit/rm-common-change-control/`.

**Code inventory audited** (verified 2026-07-12):

| File | Lines | Role today |
|---|---|---|
| `app/ehrbase/src/service/vobject.rs` | 2139 | versioning **semantics** (Change/apply_change, next_version tree placement, sign_version, lifecycle resolve, attestation, import) **mixed with** storage plumbing (insert_vo_version/audit/contribution/nodes, version_read, read_current/version/at, close_ordinal) |
| `app/ehrbase/src/service/contribution.rs` | 1355 | CONTRIBUTION classify + commit orchestration, commit-audit validation, get/list, aggregate change type, attestation completion |
| `app/ehrbase/src/service/version_id.rs` | 344 | OBJECT_VERSION_ID / VERSION_TREE_ID decoding (`TreeId`, parsers) |
| `app/ehrbase/src/service/versioned.rs` | 301 | ORIGINAL_VERSION / VERSIONED_OBJECT / REVISION_HISTORY builders + verify-on-read |
| `app/ehrbase/src/signing/{mod,signer,key,verify,config}.rs` | 764 | VERSION.signature digest/PGP signer + verifier + config |

**Fixed contracts (not up for change):** the SM native traits in
`app/ehrbase-sm` (`EhrCompositionService`, `EhrContributionService`,
`EhrDirectoryService`, `EhrStatusService`) are the service seam. The DB schema
(`vo_version`, `audit`, `contribution`, `vo_attestation`,
`0001_baseline.sql`) is settled — a G-row may propose a schema change **only**
with a spec citation demanding it.

---

## 1. Spec enumeration → code map (the register spine)

Each row is a spec item (the spine) with its citation, mapped to the code that
realizes it and a verdict: **conformant** / **divergent** / **missing**.
`S-nn` ids are stable. "Storage seam" marks code that is physically in the
versioning files today but is storage plumbing owned by register **02-storage**
(§5).

### A. Version identification (BASE master05; RM master06 §Version Identification)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-01 | `OBJECT_VERSION_ID` = 3 `::`-parts `object_id '::' creating_system_id '::' version_tree_id`, strict | master05 §Identifying Versions / §Syntaxes | `version_id.rs:179` `parse_object_version_id`, `:166` `parse_version_uid` (over `openehr_base ObjectVersionId::from_str`) | conformant |
| S-02 | `VERSION_TREE_ID` = `trunk` \| `trunk.branch_number.branch_version`, all ≥ 1 | master05 §Syntaxes; master06 §Local Versioning | `version_id.rs:35` `TreeId`, `:84` `from_version_tree`, `:158` `parse_tree_id` | conformant |
| S-03 | `object_id` of every VERSION = the container `uid` (HIER_OBJECT_ID / UUID) | master06 §Version Identification | `version_id.rs:184` (`object_id` → `vo_id` UUID key); `NotAUuid` reject | conformant (this CDR keys by UUID — documented) |
| S-04 | `HIER_OBJECT_ID` bare form accepted where a container id is expected | master05 §UID-based Identifiers | `version_id.rs:210` `parse_uid_based_id` | conformant |
| S-05 | Composite identifiers **case-insensitive** equality, case-preserving | master05 §Composite Identifiers and Case | `creating_system_id` compared in `next_version` (`vobject.rs:1149`), `uq_vo_version_tree` (schema) — stored verbatim | **divergent** → G-09 |
| S-06 | `If-Match` precondition parse (quoted/bare OVID or bare trunk int) | master09 §levels; ITS-REST If-Match | `version_id.rs:224` `expected_from_if_match` | conformant |
| S-07 | Content `uid` SHOULD copy the containing VERSION's OBJECT_VERSION_ID | master09 §Levels of Identification | node codec / reassemble (storage seam) — set from `build_original_version:242` uid | conformant (storage seam) |

### B. VERSIONED_OBJECT (RM master06 §Versioned Objects; versioned_object.adoc)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-08 | `uid: HIER_OBJECT_ID` (container GUID), `owner_id: OBJECT_REF` | master06 §Versioned Objects | `versioned.rs:109` `versioned_object` (uid + owner_id OBJECT_REF→EHR) | conformant |
| S-09 | `time_created` = commit time of first held version | master06; versioned_object.adoc | `versioned.rs:114` (earliest `sys_version` audit time) | conformant (import PORT NOTE `versioned.rs:104`) |
| S-10 | Functional interface: `all_versions`, `version_with_id`, `version_at_time`, `latest_version`, `latest_trunk_version`, `commit_original_version`, `commit_attestation` | versioned_object.adoc | free fns + SQL: `read_current:2009`, `read_version:2055`, `version_at:2083`, `revision_history:17`; `attest:955` | conformant (realized as functions, not a class — spec-silent shape) |
| S-11 | `ALL_VERSIONS` supported (no filter over `vo_version`) | ADR-008 goal vs master06 | `vo_version` temporal table, no current/history split | conformant |

### C. VERSION / ORIGINAL_VERSION / IMPORTED_VERSION (RM master06 §Version and its Subtypes)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-12 | `VERSION.uid: OBJECT_VERSION_ID`, `.contribution`, `.commit_audit (1..1)`, `.preceding_version_uid` (Void iff first) | master06 §Version; version.adoc | `versioned.rs:225` `build_original_version` (uid/contribution/commit_audit/preceding) | conformant |
| S-13 | `ORIGINAL_VERSION.data` (Void ⇒ deleted), `.lifecycle_state`, `.attestations`, `.other_input_version_uids`, `.is_merged` (derived) | master06 §Version subtypes; original_version.adoc | `versioned.rs:225`–`:299` (data omitted when Null; lifecycle DV_CODED_TEXT; other_input_version_uids; attestations appended `:174`) | conformant |
| S-14 | `IMPORTED_VERSION` wraps an ORIGINAL in `.item`; own contribution + commit_audit; uid/preceding are **functions** of the wrapped original | master06 §Version subtypes, §Committal | import stores the wrapped original verbatim (`vobject.rs:1778` `commit_import`); PORT NOTE `vobject.rs:1760` — **served as ORIGINAL_VERSION, the IMPORTED_VERSION wrapper is not reconstructed on read** | **divergent** → G-08 |
| S-15 | Version signed form: `signature (0..1)` stored on the version | master06 §Digital Signature | `vobject.rs:602` `sign_version`; `versioned.rs:296` (signature field) | conformant → integrity §3 |

### D. CONTRIBUTION + committal/audit (RM master06 §Contributions, §Committal and Audits; master08)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-16 | CONTRIBUTION = uid + `versions` list + `audit`; every change is a CONTRIBUTION | master06 §Contributions; master08 §Managing Changes | `contribution.rs:327` `commit_version_set`, `:303` `create_ehr_contribution`; direct writes wrap one (`vobject.rs:1327/1371/1419`) | conformant |
| S-17 | Contribution is a **nested transaction** — all versions/attestations commit or none | master06 §Committal ("similar to nested transactions") | one `sqlx::Transaction` per commit act (`commit_contribution:1469`) | conformant |
| S-18 | Change-type mapping: 249 creation (new VO, no preceding), 523 deleted (data Void), 250 amendment / 251 modification / 252/253/816/817 (content, existing), 666 attestation (existing, no data) | master06 §Contributions | `contribution.rs:76` `classify` — full code set + spec-invalid-combination rejects | conformant |
| S-19 | CONTRIBUTION.audit `change_type` = aggregate of member change types | master06 §Contributions | `contribution.rs:936` `aggregate_change_type` | conformant |
| S-20 | AUDIT_DETAILS = `system_id`, `committer: PARTY_PROXY`, `time_committed`, `change_type`, `description?` | master04 §Audit Details; audit_details.adoc | `vobject.rs:129` `AuditInput`; `contribution.rs:900` `audit_details` builder | conformant |
| S-21 | `system_id`/`committer`/`time_committed` **copied** from CONTRIBUTION audit into each version's `commit_audit` when the version omits them | master06 §Committal (m4) | `contribution.rs:685` `parse_version_audit` (fallback from contribution audit) | conformant (PR #33 audit-copy fix) |
| S-22 | `time_committed` **computed on the server** | master06 §Committal | `audit.time_committed DEFAULT now()` (schema); `write_contribution` returns server time | conformant |
| S-23 | `AUDIT_DETAILS.System_id_valid` (non-empty) + committer PARTY invariants (Basic_validity, Name_valid, PARTY_RELATED.Relationship_valid) | master04; audit_details.adoc / party_identified.adoc / party_related.adoc | `contribution.rs:186` `validate_commit_audit`, `:206` `validate_committer`, `:248` party-related group check | conformant |

### E. Attestation (RM master06 §Attestation; master04 §Attestation)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-24 | ATTESTATION ⊂ AUDIT_DETAILS: `items?`, `reason`, `proof`, `is_pending` | master04 §Attestation; attestation.adoc | `contribution.rs:1064` `complete_attestation` (verbatim canonical ATTESTATION in `vo_attestation.data`) | conformant |
| S-25 | Attestation "added at any time after committal"; a 666 member adds **no new version** | master06 §Attestation, §Contributions | `vobject.rs:955` `attest` (no new `vo_version` row; `sys_period` untouched); `classify` Attest arm | conformant |
| S-26 | Signing-at-committal: `ORIGINAL_VERSION.commit_audit` may be an ATTESTATION (`is_pending`) + accompanying attestations | master06 §Attestation ("Signing content at committal") | `vobject.rs:928` `insert_accompanying_attestations` (UPDATE_VERSION.attestations) | conformant |
| S-27 | Attestations of an old version are **not** valid for a new version | master06 §Attestation | attestations keyed by `(vo_id, sys_version)` (schema FK); never copied forward | conformant |
| S-28 | Attestation NOT part of the version's signed canonical form (added after signing) | master06 §Attestation + §Digital Signature | `versioned.rs:174` (attestations appended **after** `verify_on_read`) | conformant |

### F. Version lifecycle + logical deletion (RM master06 §Version Lifecycle, §Logical Deletion)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-29 | Five states 532 complete / 553 incomplete / 523 deleted / 800 inactive / 801 abandoned | master06 §Version Lifecycle | `codes.rs:46` (lifecycle group); `resolve_lifecycle:563`; schema CHECK `('532','553','523','800','801')` | conformant |
| S-30 | **State-machine transitions** (e.g. `deactivate` complete→inactive, `abandon` incomplete→abandoned, `retrieve`, `reactivate`, `delete`); every transition ⇒ a new version | master06 §Version Lifecycle (state machine diagram + table) | **no transition validation** — any target state accepted regardless of the current state | **divergent** → G-01 |
| S-31 | Incomplete (553): relaxed validity (missing mandatory allowed, nothing "wrong") | master06 §Incomplete Content (NOTE) | `composition.rs:426` (relaxed pass when `incomplete`) | conformant |
| S-32 | Logical deletion: new version, `data` Void, `lifecycle_state = 523` | master06 §Logical Deletion | `vobject.rs:837` Delete arm (`Value::Null` data, `lifecycle::DELETED`) | conformant |
| S-33 | Any state transition (incl. no content change) generates a **new** VERSION | master06 §Version Lifecycle | new `vo_version` row per commit (`insert_vo_version`) | conformant (but see G-01: an unchanged-content transition is not forced when the client re-commits the same state) |

### G. Distributed versioning — copy / branch / merge / move (RM master06 §Semantics in Distributed Systems)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-34 | Local create sets `creating_system_id` = local system; trunk numbering from 1 | master06 §Distributed Versioning | `apply_change` Create → `TreeId::trunk(1)`, `ctx.system_id` (`vobject.rs:681`) | conformant |
| S-35 | Modifying a version copied from **another** system must **branch** (`t.(max+1).1`); later trunk versions from origin stay importable | master06 §Distributed Versioning, §Subsequent Local Modifications | `next_version:1149` auto-fork when preceding `creating_system_id` ≠ local | conformant |
| S-36 | Copy unit = ORIGINAL_VERSION; import wraps IMPORTED_VERSION with its own commit; clone EHR reuses `ehr_id`; new VERSIONED_OBJECT reuses source `object_id` | master06 §Copying; master09 §Identification of the EHR | `commit_import_scoped:1816` (per-lineage periods, first-receipt vs append, reused vo_id) | conformant (wrapper caveat G-08) |
| S-37 | Merge: new trunk version records source ids in `other_input_version_uids`; `is_merged` derived | master06 §Version Merging | `Change::Modify.other_input_version_uids` (`vobject.rs:534`) stored; `is_merged` derived (`versioned.rs:279`) — **no server-side merge operation** (accepts client-declared provenance only) | **divergent** → G-07 |
| S-38 | Disjoint merging (merge two containers for one real entity) | master06 §Disjoint Merging | absent | **missing** → G-05 |
| S-39 | Moving version containers (creating_system_id changes along trunk after a move) | master06 §Moving Version Containers | absent | **missing** → G-06 |

### H. Digital signature / integrity (RM master06 §Digital Signature; master07 §Integrity)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-40 | Signature over the **canonical serialized form** of the whole Version (signature attr Void during serialization), hashed → digest | master06 §Digital Signature; master07 §Integrity | `signing/signer.rs:119` `sign`; canonical = `openehr-rm ...version_impl::canonical_form_of_json` (signature-independent, `version_impl.rs:184`); signed over the reassembled served form (`vobject.rs:684`) | conformant |
| S-41 | Signature per openPGP RFC 4880; radix-64 ASCII; algorithms self-describing | master06 §Digital Signature; master07 | `signing/key.rs` (rPGP detached RFC 4880, armored) | conformant (pgp mode) |
| S-42 | Digest-only variant (no key infrastructure) = pure integrity check | master07 §Digital Signature ("encryption step might be omitted, resulting in a digest only") | `signing/signer.rs:135` `digest_signature` (`sha256:` + radix-64) — default mode | conformant (our `sha256:` prefix is our documented self-description — PORT NOTE G-10) |
| S-43 | Exact serialization is **openEHR TBD** (ODIN preferred; XML libraries differ) | master06 §Digital Signature `[.tbd]` | canonical openEHR **JSON** (RFC 8785 JCS), not ODIN/XML | **divergent-by-necessity** → G-10 (spec TBD; PORT NOTE) |
| S-44 | Signature is a stored fact carried with the data (for Extracts) | master06 §Digital Signature; master07 | `vo_version.signature text`; served verbatim (`versioned.rs:296`); optional read-time verify (`versioned.rs:187`) | conformant |
| S-45 | **Placement**: digital signature is defined *within* the change_control / version model (and master07 §Integrity §Versioning+Digital Signature) | master06 §Digital Signature (a section of the change-control package) | today a **standalone** `app/ehrbase/src/signing/` module | **divergent (structure)** → G-02 (dissolve into `versioning/signature/`) |

### I. Revision history + references (RM master04 §Revision History; BASE master05 §References)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-46 | REVISION_HISTORY = ordered REVISION_HISTORY_ITEMs; item = `version_id` + `audits` (commit audit + attestations for that revision) | master04 §Revision History; revision_history_item.adoc | `versioned.rs:17` `revision_history` (per-version audit + its attestations) | conformant |
| S-47 | OBJECT_REF/PARTY_REF used for cross-object references (owner_id, contribution ref) | master05 §References | OBJECT_REF built inline in `versioned.rs` builders | conformant |

### J. CM paradigm / repository guarantees (BASE master08; master07 §Integrity)

| # | Spec item | Citation | Code | Verdict |
|---|---|---|---|---|
| S-48 | Indelibility — no physical modification, only new versions | master07 §General (Indelibility); master08 §Change Management | append-only `vo_version` + `node` (temporal periods, never UPDATE content) | conformant |
| S-49 | Every write audit-trailed with user id, time, reason (mandatory commit audits) | master07 §Integrity §Versioning | `audit` row per contribution + per version (FK NOT NULL) | conformant |
| S-50 | Any previous state reconstructable (time-travel / rollback) | master08 §Change Management goals | `version_at:2083` (`sys_period @> instant`) | conformant |

---

## 2. Unmapped code — classification

Code that maps to **no** spec item, classified per the owner's method:
(a) spec-silent internal · (b) extension / quarantine · (c) delete.

| Code | What | Class | Justification / disposition |
|---|---|---|---|
| `vobject.rs:313`–`464` (`insert_audit`, `insert_contribution*`, `write_contribution`, `insert_nodes`, `insert_ehr_folder_rank`) | SQL row writes | (a) spec-silent — **no openEHR spec governs SQL** | Storage plumbing → **register 02-storage** repository seam (§5). Not versioning semantics. |
| `vobject.rs:1126`–`1327` (`NextVersion`, `insert_vo_version`, `next_version` SQL body, advisory lock) | tree-placement **decision** (semantic) + its SQL (plumbing) | mixed | The *decision* (trunk/branch/fork, close lineage) is versioning semantics (S-35) → `versioning/change.rs`; the SQL execution → 02-storage. Split at the seam. |
| `vobject.rs:2009`–`2139` (`read_current`/`read_version`/`version_at`/`read_nodes`, `version_read`) | version read SQL + reassembly | (a) spec-silent | Storage read seam → 02-storage; versioning consumes a `VersionRead` value. |
| `vobject.rs:1622`–`1778` (`insert_imported_vo_version`, `close_lineage_at`, `imported_container_state`) | import SQL | (a) spec-silent | Storage seam; the import *policy* stays in `versioning/import.rs`. |
| `vobject.rs:395` `write_outbox`, `:1065` `record_composition_commit`, `:1084` `sync_ehr_subject` | event outbox, Prometheus metric, EHR subject-column sync | (b) extension — "no openEHR spec governs this — our own design" (eventing, telemetry, one-EHR-per-subject index) | Keep, but **not** in versioning: outbox+metrics → cross-cutting (`TODO(w3f-integrate)`); `sync_ehr_subject` is EHR-status semantics → register **for EHR/status** area, invoked as a hook. |
| `vobject.rs:1016` `check_versioned_composition_invariants` | VERSIONED_COMPOSITION cross-version invariants | (a) spec — but **RM ehr**, not change_control | Belongs to the composition/validation register; versioning calls it as a pre-commit hook (`TODO(w3f-integrate)`). |
| `signing/config.rs` `EHRBASE_SIGNING_*` | signing configuration | (b) extension (config surface) | Keep; moves under `versioning/signature/config.rs`. |
| `contribution.rs:839`–`899` `list_contributions`/`count_contributions` | contribution listing | (a) spec-silent (query convenience) | Keep in `versioning/contribution.rs`. |

Nothing in the area is a **delete** candidate — all code either maps to a spec
item or is a justified spec-silent internal / extension.

---

## 3. Dissolution of `signing/` into `versioning/` (owner ruling)

The spec places the digital signature **inside** the versioning/integrity
model: master06 §Digital Signature is a section of the *change_control package*,
and master07 §Integrity groups "Versioning" and "Digital Signature" as the two
faces of one integrity mechanism (S-40..S-45). A standalone `signing/` sibling
module (G-02) contradicts that placement. **Disposition:** move the whole module
under `app/ehrbase/src/versioning/signature/` (mod, signer, key, verify, config)
and the read-time verifier (`versioned.rs:187` `verify_on_read`) into
`versioning/integrity.rs`. No behaviour change — the signer/verifier logic
(S-40..S-44) is conformant; only its **home** changes. The `Signer` stays
`pub` (constructed at boot from `SigningConfig`), re-exported from
`versioning::signature`.

---

## 4. Target design — `app/ehrbase/src/versioning/`

Layout **derived from the spec's decomposition** (each file ≤ ~700 lines; the
2139-line `vobject.rs` and 1355-line `contribution.rs` are split by spec area,
not by mechanics):

```
versioning/
  mod.rs              module doc (cites master06/master04/master05/master07);
                      re-exports; SigningCtx; the versioning error surface
  object_version_id.rs  S-01..S-06  (from service/version_id.rs, verbatim)
  audit.rs            S-20..S-23  AuditInput, audit_details builder,
                      validate_commit_audit / _committer / party_related
  contribution.rs     S-16..S-19  classify, commit_version_set orchestration,
                      aggregate_change_type, get/list contributions
  change.rs           S-12,S-13,S-32,S-34,S-35  Change enum, apply_change,
                      resolve_lifecycle, next_version tree-placement DECISION,
                      NextVersion (SQL execution delegated to storage seam)
  lifecycle.rs        S-29,S-30,S-33  the five-state machine + transition
                      validation (NEW — closes G-01)
  attestation.rs      S-24..S-28  PendingAttest, complete_attestation, attest,
                      insert_accompanying_attestations, attestations_of
  import.rs           S-14,S-36  ImportVersion, ImportContainer, commit_import,
                      import policy (per-lineage, first-receipt vs append)
  revision_history.rs S-08..S-10,S-46,S-47  VERSIONED_OBJECT / ORIGINAL_VERSION /
                      REVISION_HISTORY builders (from service/versioned.rs)
  integrity.rs        S-15,S-44  verify_on_read policy + metering
  signature/          S-40..S-45  (the dissolved signing/ module, §3)
    mod.rs signer.rs key.rs verify.rs config.rs
```

**Key design decisions:**

- **D1 — semantics/plumbing seam (§5).** Versioning owns the *decisions*
  (classify, tree placement, lifecycle transition, sign, attest, import policy)
  and the *builders* (ORIGINAL_VERSION/VERSIONED_OBJECT/REVISION_HISTORY value
  construction). All `sqlx` execution moves behind a storage-owned repository
  trait (register 02-storage); versioning consumes `VersionRead`/`Committed`
  value types and hands back `Change`/`ImportContainer`. This is what lets
  `vobject.rs` shrink under the line budget.
- **D2 — signature lives here (§3).** No separate crate/module; `versioning::signature`.
- **D3 — lifecycle state machine is real (G-01).** A new `lifecycle.rs` encodes
  the master06 transition table and rejects illegal transitions (422) reading
  the current state from the preceding version. Today any state is accepted.
- **D4 — identification is the core's law.** `object_version_id.rs` sits at the
  top of the module (S-01..S-06); every builder and the tree-placement logic
  depend on `TreeId`.
- **D5 — no schema change.** Every S-row maps onto existing columns; the only
  spec-cited schema question is G-09 (case-folding), solvable in SQL/decoder
  without DDL.

---

## 5. Ownership seam with register 02-storage

| Concern | Owner | Interface |
|---|---|---|
| VERSION / VERSIONED_OBJECT / CONTRIBUTION / AUDIT_DETAILS / ATTESTATION **semantics** (classify, lifecycle, tree placement, sign, attest, import policy, revision history) | **versioning** (this register) | produces `Change`, `Committed`, `ImportContainer`, the JSON builders |
| node codec (decompose/reassemble), `node`/`vo_version`/`audit`/`contribution`/`vo_attestation` SQL, table DDL, temporal periods, advisory locks | **02-storage** | a repository trait taking value types, returning `VersionRead` |
| decompose/reassemble called by versioning at commit/read | 02-storage provides; versioning calls | `TODO(w3f-integrate)` |
| VERSIONED_COMPOSITION cross-version invariants (`vobject.rs:1016`) | composition/validation register | pre-commit hook `TODO(w3f-integrate)` |
| `sync_ehr_subject`, `is_modifiable` write guard (`ehr.rs:612`, already present) | EHR/status register | pre-commit hook `TODO(w3f-integrate)` |
| event outbox + Prometheus metrics | cross-cutting | post-commit hook `TODO(w3f-integrate)` |

---

## 6. Gap register (G-rows)

| G | Spec citation / spec-silent flag | Severity | Disposition |
|---|---|---|---|
| G-01 | master06 §Version Lifecycle (state machine) — S-30/S-33: transitions unvalidated (any target state accepted) | MED | **fix-in-rewrite** (`lifecycle.rs`) |
| G-02 | master06 §Digital Signature placement (a change_control section); master07 §Integrity — S-45: signing is a standalone module | MED (structure) | **fix-in-rewrite** — dissolve into `versioning/signature/` (§3) |
| G-03 | master06 §Version subtypes / §Copying — S-14: `IMPORTED_VERSION` wrapper not reconstructed on read (served as ORIGINAL_VERSION) | MED | **PORT NOTE** keep + re-verify (existing `vobject.rs:1760`); reconstruct only if an ECC/MSG case demands the wire IMPORTED_VERSION shape |
| G-04 | "no openEHR spec governs SQL" — spec-silent: 2139-line `vobject.rs` mixes semantics + storage plumbing | HIGH (maintainability) | **fix-in-rewrite** — split at the D1 seam (§5) |
| G-05 | master06 §Disjoint Merging — S-38: missing | LOW | **PORT NOTE** (branching/merging trunk-only until distributed SM work; blueprint ch01) |
| G-06 | master06 §Moving Version Containers — S-39: missing | LOW | **PORT NOTE** (same trunk-only scope) |
| G-07 | master06 §Version Merging — S-37: no server-side merge op; client-declared `other_input_version_uids` accepted only | LOW | **PORT NOTE** (provenance stored & served; merge is a client/distributed concern, trunk-only) |
| G-08 | (merged into G-03) | — | — |
| G-09 | master05 §Composite Identifiers and Case — S-05: `creating_system_id` equality is case-sensitive; spec mandates case-insensitive+case-preserving | MED | **fix-in-rewrite** — canonicalise at the decoder boundary (`object_version_id.rs`); cross-check `docs/spec-audit/rm-common-change-control` (flagged there for rm-support) |
| G-10 | master06 §Digital Signature `[.tbd]` — S-43: canonical form is openEHR JSON (RFC 8785), not the (undefined) ODIN/XML; digest prefix `sha256:` is our self-description | LOW | **PORT NOTE** keep (spec is explicitly TBD; JSON is our canonical form, deterministic and signature-independent) — already noted `signing/key.rs:4`, `signer.rs:15` |
| G-11 | master06 §Committal — S-14 IMPORTED_VERSION own-contribution vs preserved original audit | — | **already-correct** (PORT NOTE `vobject.rs:1760/1771` — keep) |
| G-12 | master04 §Revision History — S-46: REVISION_HISTORY assembled ad-hoc via `json!` rather than typed `openehr-rm` builders | LOW | **PORT NOTE** (spec-silent serialization choice; wire-correct) — optionally typed builder in rewrite |
| G-13 | RM ehr (not change_control) — `check_versioned_composition_invariants` lives in `vobject.rs` | LOW | **fix-in-rewrite** — relocate to composition/validation register; call as a hook (§5) |

---

## 7. PORT-NOTE residue (existing notes in the area — keep / re-verify / drop)

| Location | Note | Verdict |
|---|---|---|
| `vobject.rs:1760` | IMPORTED_VERSION representation (master06 §Committal) — imported original stored, own contribution records local committal | **keep** (S-14 / G-03) |
| `vobject.rs:1771` | local temporal periods (master06 §Copying) — commit times are the local act | **keep** (S-36) |
| `versioned.rs:104` | EHR-Extract import — `time_created` = earliest **held** version, not `sys_version = 1` | **keep** (S-09) |
| `contribution.rs:286` | SM `commit_contribution` is a typed subset of the wire CONTRIBUTION; raw-body seam restores full fidelity | **keep** (S-16) |
| `contribution.rs:388` | a first version legitimately carries no preceding | **re-verify** against S-18 wording (fold into `classify` doc in rewrite) |
| `contribution.rs:434` | client-supplied CONTRIBUTION uid honoured (SHOULD, not MUST) | **keep** |
| `contribution.rs:971` | `UPDATE_AUDIT.change_type` is a `Terminology_code` | **keep** |
| `contribution.rs:1023` | JSON `null` treated as absent | **keep** |
| `signing/key.rs:4` | RFC 4880 detached sig over canonical_form = spec's "signature from the hash" | **keep** (G-10) |
| `signing/verify.rs:9` | client-supplied signatures exempt from recomputation (format classification) | **keep** — re-cite to master06 §Digital Signature in the moved file (design-§ citation replaced by spec citation per the hard rule) |

---

*Chapters upstream: blueprint ch 01 (RM change control), `docs/spec-audit/rm-common-change-control/`. This register is the W-3f spec-onto-code map for the versioning + integrity area; the sibling register **02-storage** owns the node codec + SQL seam referenced in §5.*

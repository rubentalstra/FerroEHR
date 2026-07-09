# SM design — Message & EHR Extract integration (the three-crate mapping)

Grounded in the full spec extraction (2026-07-09): SM
`master09-message_service.adoc` + `i_{message,ehr_extract,tdd}_service.adoc`;
RM **EHR Extract IM** (STABLE) `docs/specs/openehr/RM/docs/ehr_extract/`
master03–09 + every `org.openehr.rm.ehr_extract.*` class; the generated
types in `crates/openehr-rm/src/ehr_extract/`; ITS-REST + CNF coverage
checks.

## 1. What the specs give us

- **SM (TRIAL)**: `I_EHR_EXTRACT_SERVICE` — `export_ehrs(an_ehr_id) ->
  List<EXTRACT>`, `export_ehr_extracts(extract_spec) -> List<EXTRACT>`,
  `import_ehr(an_ehr_id?, an_extract)`, `import_ehr_extract(an_ehr_id,
  an_extract)`; `I_TDD_SERVICE` — `import_tdd(an_ehr_id, tdd)`,
  `import_tdds` (unsigned); `I_MESSAGE_SERVICE` — **empty stub**. No
  pre/post/errors anywhere in the three files.
- **RM EHR Extract IM (STABLE — the real substance)**: five packages.
  `common` (EXTRACT / EXTRACT_REQUEST / EXTRACT_ACTION_REQUEST /
  EXTRACT_SPEC / EXTRACT_MANIFEST / EXTRACT_ENTITY_MANIFEST /
  EXTRACT_VERSION_SPEC / EXTRACT_UPDATE_SPEC / chapters / folders /
  content-items / EXTRACT_PARTICIPATION / EXTRACT_ERROR);
  `openehr_extract` (`X_VERSIONED_OBJECT<T>` + the five `X_VERSIONED_*`
  wrappers holding `ORIGINAL_VERSION<T>` lists + revision history);
  `generic_extract` (GENERIC_CONTENT_ITEM for 13606/CDA);
  `sync_extract` (SYNC_EXTRACT[_REQUEST/_SPEC] + **X_CONTRIBUTION** —
  contribution-based synchronisation); `message` (MESSAGE /
  ADDRESSED_MESSAGE / MESSAGE_CONTENT with openPGP-style signature).
  Operational semantics: request/reply with the reply carrying its own
  `specification`; latest-only default vs `include_all_versions` /
  revision-history-only (`include_data=False`, invariant enforced);
  whole-Composition transmission (the 13606 diff model explicitly
  rejected, master09); delta via `EXTRACT_UPDATE_SPEC` + `is_changed`;
  the master09 creation algorithm (primary set → chapters → demographic
  resolution → link-following with `is_primary=False`).
- **Already generated, everything**: `openehr-rm::ehr_extract::{common,
  openehr_extract, generic_extract, sync_extract, message}` — all classes
  emitted with canonical JSON; `X_VERSIONED_*` are monomorphized structs
  over `OriginalVersion<T>`.
- **No wire contract**: ITS-REST vendors zero extract/message/TDD
  endpoints (the CNF-referenced "MESSAGE REST API" is not vendored).
- **CNF**: the messaging suite (master13) exists but is 100 % stubbed
  (every body TBD); Messaging is **OPTIONS-profile only** — not required
  for CORE/STANDARD conformance.

## 2. The three-crate mapping

### `ehrbase-sm` (SM catalog)

- `EhrExtractService` — the four SM calls transcribed literally
  (`SmError`; `an_ehr_id: Uuid`; `extract_spec`/`an_extract` as the
  generated `openehr_rm::ehr_extract` types — RM types in signatures are
  already the catalog convention). PORT NOTEs: the SM/CNF naming mismatch
  (`I_EHR_EXTRACT_SERVICE` vs `I_EHR_EXTRACT`; CNF's phantom singular
  `export_ehr()/export_ehr_extract()` pair), no spec pre/post/errors —
  preconditions filled by design (`has_ehr`; import target exists;
  `Sequence_nr_valid` etc. from the RM invariants).
- `TddService` — `import_tdd(an_ehr_id, tdd: String)`; `import_tdds`
  designed as batch with per-item `DUMP_LOAD_FAIL_REPORT`-style results
  (spec signature empty — PORT NOTE).
- `MessageService` — the SM stub filled by design as the umbrella:
  supertrait of the two above plus `receive_message(AddressedMessage)`
  / `send semantics deferred until a transport exists` — kept minimal,
  PORT NOTE citing the empty `i_message_service.adoc`.
- `SyncExtractService` (design-filled, optional but cheap):
  `sync_extract(spec: SyncExtractSpec) -> SyncExtract` — our contribution
  table makes X_CONTRIBUTION assembly nearly free; PORT NOTE that the SM
  defines no call for it (the RM model exists; capability = OPTIONS).

### `ehrbase` (the component)

Module `message` (submodules `extract`, `tdd`, `sync`):
- **Export** = the master09 creation algorithm over our store: resolve the
  entity manifest → per-entity `EXTRACT_CHAPTER`; primary set from
  `criteria` (AQL via `QueryService` with `$ehr`) or `item_list`
  (version-container uids — direct `vobject` reads); build
  `X_VERSIONED_*` from `vo_version`/`vo_attestation` (our
  `build_original_version` already produces exact `ORIGINAL_VERSION`s;
  revision history builder exists); honor `EXTRACT_VERSION_SPEC`
  (latest / all versions / revision-history-only with the
  `Includes_revision_history_valid` invariant); demographic chapter via
  the party store; `include_multimedia` + `link_depth` per the algorithm
  (`is_primary=False` for followed links). The reply's `specification`
  reflects actual content.
- **Import** = replay through `commit_contribution`: whole-EHR import
  creates the EHR (optionally with the fixed id — the SM's cross-system
  same-patient case), then commits each `X_VERSIONED_*`'s versions in
  order preserving audits/attestations (IMPORTED_VERSION semantics —
  PORT NOTE: we store as ORIGINAL_VERSION with provenance in the audit,
  since our RM scope holds originals; master06 sanctions non-distributed
  systems holding only ORIGINAL_VERSION).
- **TDD** = TDD XML → OPT-guided content model → COMPOSITION → the normal
  validated commit path.
- **Sync export** = `SYNC_EXTRACT` from the `contribution` table
  (`contribution_list`/`contributions_since`/`all_contributions`;
  `includes_versions` toggles X_CONTRIBUTION.versions).
- Deferred with PORT NOTEs: `EXTRACT_REQUEST` persistence +
  `EXTRACT_UPDATE_SPEC` periodic/trigger sends and
  `EXTRACT_ACTION_REQUEST` (need a scheduler/transport);
  `generic_extract` import (13606/CDA sources).

### `ehrbase-rest` (adapter)

Extension routes only (no ITS-REST contract exists — PORT NOTE; design 08
§7 namespace): `POST /message/extract/request` (EXTRACT_REQUEST →
List<EXTRACT>), `GET /message/extract/ehr/{ehr_id}` (whole-EHR export),
`POST /message/extract/import{?ehr_id}`, `POST /message/tdd{?ehr_id}`,
`POST /message/sync` — canonical JSON bodies of the generated RM types;
documented in our own OAS, excluded from the ITS-REST drift check;
migrate to `emit-rest` if openEHR publishes the MESSAGE REST API.
Messaging is OPTIONS-profile: no CORE/STANDARD conformance impact.

## 3. Spec gaps carried as design decisions (all PORT-NOTEd)

The extraction's gap list (naming mismatch; stub interfaces; the
`MESSAGE_CONTENT` hierarchy gap — non-sync `EXTRACT` has no modelled path
into a `MESSAGE` payload; `EXTRACT_UPDATE_SPEC`'s phantom
`send_changes_only`; the `X_CONTRIBUTION.versions` self-referential
generic; `EXTRACT_ACTION_REQUEST` action-code inconsistency; undocumented
`EXTRACT_ERROR`) is adopted verbatim into the implementation notes — each
resolved by explicit design with citations, never silently.

## 4. Sequencing

SM-5 (per roadmap 09), after SM-4 closes: (1) `ehrbase-sm` traits + types,
(2) export path (+ tests against the corpus store), (3) import path
(+ round-trip test: export → wipe → import → byte-compare canonical
content), (4) TDD, (5) sync extract, (6) extension routes.

# B7 schema review — 03: openEHR persistence requirements (2026-07-10)

DOCS-FIRST extraction (Opus fan-out, B7 task 1b); full per-requirement
citations in the B7 research transcript — the numbered checklist below keeps
the citation anchors. Feeds ADR-013.

## 1. Change control / versioning (RM common master06 + classes)
- 1.1.1 VERSIONED_OBJECT: bare-UID container id (no extension), owner_id,
  time_created. 1.1.2 container uid identical across systems (client/source
  assignable — never a per-system surrogate). 1.1.3 point-in-time lookup,
  latest-trunk vs latest-any, revision history reconstruction.
- 1.2.1 OBJECT_VERSION_ID = object_id::creating_system_id::version_tree_id —
  the 3-tuple is the version identity. 1.2.2 trunk from 1, branch 3-part.
  1.2.3 creating_system_id is per-version data (varies within a trunk after
  container moves). 1.2.4 local edits of foreign versions must branch.
- 1.3.1 VERSION: contribution ref + commit_audit + optional signature.
  1.3.2 `preceding_version_uid NULL iff first` (invariant). 1.3.3/1.3.4
  ORIGINAL_VERSION carries other_input_version_uids (merge provenance;
  `Other_input_version_uids_valid xor is_merged`) + attestations + optional
  data. 1.3.5 IMPORTED_VERSION = wrapped ORIGINAL verbatim + its OWN local
  contribution/commit_audit (two audits coexist).
- 1.4.1 Indelibility (ARCH-OV master07:138): version rows append-only;
  physical delete admin-only. 1.4.2 change_type from the audit-change-type
  group (249/250/251/523/666) — terminology-validated.
- 1.5 lifecycle_state ∈ {532,553,523,800,801}; every transition = new
  version; 553 relaxes content validity (DB must not NOT-NULL content).
- 1.6 logical delete = new version, data Void, state+change 523.
- 1.7 CONTRIBUTION: own uid + audit + version refs; strictly transactional
  (all-or-nothing); enumerable + time-rangeable per EHR; contribution-level
  change_type informational only.
- 1.8 AUDIT_DETAILS: system_id non-empty, time_committed SERVER-computed,
  change_type coded, committer PARTY_PROXY, description optional;
  contribution audit's system_id/committer/time copied into member version
  audits.
- 1.9 Attestations: append-only child list of a specific version (never
  cascade), committed via a contribution (666) without creating a version.
- 1.10 signature: opaque radix-64 text; canonical form excludes signature;
  **canonicalisation TBD in spec (S2)** — PORT NOTE territory.
- 1.11 revision history = derived view over version→(commit audit +
  attestation audits).

## 2. EHR structure (RM ehr + CNF master06/08)
- 2.1 EHR root: **immutable system_id, ehr_id, time_created** — system_id is
  a per-EHR recorded value, not just service config.
- 2.2 EHR references containers (typed); directory = folders[1].
- 2.3 exactly one EHR_STATUS + one EHR_ACCESS, both versioned LOCATABLEs.
- 2.4/2.5 subject/is_queryable/is_modifiable live IN the versioned status;
  is_modifiable=false blocks content writes but never status writes.
- 2.6 creation defaults true/true/anonymous PARTY_SELF. 2.7 ehr_id unique.
- 2.8 one EHR per subject (CNF-hard, RM-soft — flagged S5): unique
  (subject_id, namespace) where present + subject→ehr lookup.
- 2.9 EHR_STATUS never creation/deleted change types; no 553.
- 2.10 EHR_ACCESS settings implementation-dependent (spec-silent) — store as
  archetyped content, never a fixed access-model schema.
- 2.11 VERSIONED_COMPOSITION: archetype_node_id + is_persistent constant
  across versions of one container.
- 2.12 folders reference compositions (N:1 allowed, no containment FK).
- 2.13 committal time is the only server-managed temporal axis.

## 3. Demographic (RM demographic)
- 3.1/3.2 parties are versioned in their own containers on the same
  substrate (node/vo_version) — uid mandatory.
- 3.3 PARTY_RELATIONSHIP stored by-value in source party content;
  endpoints are container-level HIER_OBJECT_ID refs (continuants — never
  version-specific). 3.4 reverse relationships derived (index target refs).
- 3.6 physical party delete cascades relationships.

## 4. Audit & logging
- 4.1 change-audit fully satisfied by contribution/version audits.
- 4.2 FEEDER_AUDIT is content (inside canonical JSON), not a table.
- 4.3 SM System Log = "IHE ATNA-compliant" one-liner; 4.4/4.5 **spec-silent**
  on read-access logs / a persistent ATNA store (S3) — any DB-level access
  log (e.g. pgaudit) is enterprise choice, complementary to the app layer.

## 5. Definitions / queries / extract / tags / SPS
- 5.1.1 ADL1.4 OPT keyed by UUID (upsert-by-id, list, delete); 5.1.2 ADL2 by
  HRID + UUID; 5.1.3 template **versioning is NOT required** (replace-in-
  place sanctioned) — keeping history is a choice (S-flag).
- 5.2 stored queries: qualified name (namespace default "misc"), formalism
  case-insensitive + semver (default major "1"), registration_time, source;
  regex listing by name AND by referenced artefact ids. Query sets TBD.
- 5.3 ITEM_TAG: (owner, target uid [container OR version], path, key,
  value); loose coupling — tags mutable, outside the version chain, no
  contribution required, EHR-scoped, AQL-retrievable.
- 5.4.1 internal storage format explicitly unconstrained (S1 — validates
  ADR-008); X_VERSIONED_* only for interop. 5.4.2 dump/load per-entity
  reports + duplicate-ehr_id failure. 5.4.3 archive = move to archival tier.
- 5.5.1 Subject-proxy: persist CONFIG only (bindings + variable defs);
  results/data frames transient; support reset().

## 6. Protection / retention / deletion
- 6.1 indelible by policy; 6.2 physical delete admin-only (EHR purge, party
  purge + relationships) — CNF admin suite TBD (S4).
- 6.3 anonymity: subject linkage optional/separable.
- 6.4 STATUS/ACCESS changes versioned+audited like content.
- 6.5 **spec-silent**: retention windows, purge rules, access-control model,
  read-access logging (S3/S5) — deployment/ADR decisions.

## Spec-silence register (PORT NOTE/ADR points)
S1 internal storage shape · S2 signature canonicalisation · S3 ATNA store +
read-access logs · S4 CNF admin suite TBD · S5 one-EHR-per-subject
(CNF-hard/RM-soft) · template versioning · query sets · EHR_ACCESS model ·
retention/purge policy.

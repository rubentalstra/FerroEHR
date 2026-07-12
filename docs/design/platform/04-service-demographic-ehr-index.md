# Platform service — Demographic + EHR Index (W-3f spec-first redesign)

Read-only audit for **W-3f** — the spec-first redesign of the `ehrbase`
platform crate's demographic and EHR-index service surfaces. Per the owner
ruling (commit `43fa1326f`, "the register skeleton is the spec structure"),
this document is built **spec-first**: §1 enumerates the governing openEHR
model operation-by-operation from the vendored oracle and gives each item its
citation; §2 maps the existing `ehrbase`-crate code onto each item with a
`file:line` verdict; §3 flags code that maps onto **no** spec item; §4 is the
consolidated G-row register; §5 is the target module layout; §6 is the
PORT-NOTE residue. It absorbs the impl-side G-rows of the SM-side audits
`docs/design/sm-platform/06-demographic.md` and `…/07-ehr-index.md` and
supersedes them as the code-mapping record for the `ehrbase` crate.

This is the ONE area covered here: the platform-crate realization of the
DEMOGRAPHIC group (`I_DEMOGRAPHIC_SERVICE` / `I_PARTY` /
`I_PARTY_RELATIONSHIP`, the `UV_PARTY` / `UV_PARTY_RELATIONSHIP` envelopes and
the RM demographic model behind them) and the EHR_INDEX group
(`I_EHR_INDEX`). The SM trait surface in `app/ehrbase-sm/src/services/` is
**FIXED** — this redesign changes only the `ehrbase`-crate implementation
(`app/ehrbase/src/service/`).

## Spec oracle (read before any change)

- SM Demographic — `docs/specs/openehr/SM/docs/openehr_platform/master06-demographic_service.adoc`,
  including the class files it `include::`s from `SM/docs/UML/classes/`:
  `i_demographic_service.adoc`, `i_party.adoc`, `i_party_relationship.adoc`,
  `uv_party.adoc`, `uv_party_relationship.adoc`.
- SM EHR Index — `master07-ehr_index_service.adoc` + `i_ehr_index.adoc`,
  `resource_status.adoc`, `resource_instance_type.adoc`, `location_desc.adoc`.
- RM demographic — `docs/specs/openehr/RM/docs/demographic/master02-demographic_package.adoc`
  + `RM/docs/UML/classes/org.openehr.rm.demographic.{party,actor,role,person,
  organisation,group,agent,party_relationship,versioned_party,party_identity,
  contact,address,capability}.adoc`.
- BASE identification (`PARTY_REF` law) —
  `docs/specs/openehr/BASE/docs/base_types/master05-identification_package.adoc`
  + `BASE/docs/UML/classes/org.openehr.base.base_types.{object_ref,party_ref,
  hier_object_id}.adoc`.
- FIXED SM trait surface — `app/ehrbase-sm/src/services/demographic/{service,
  relationship}.rs`, `…/ehr_index/{service,types}.rs`.

## Verdict up front

The demographic + EHR-index domain logic is a **faithful, well-grounded
realization on the shared `vobject` versioned-object machinery** — every
wire-reachable SM operation is present with the correct versioning effect and
error mapping, the enforceable PARTY invariants are enforced, and the EHR-index
N:M semantics are complete and PG-tested. W-3f is therefore a **surgical
redesign, not a rebuild**. Its work is (a) decompose the three flat files
(`demographic.rs` at **733 lines exceeds the ≤~700 target**) into
spec-shaped `demographic/` + `ehr_index/` modules mirroring
`app/ehrbase-sm/src/services/`; (b) close the two substantive compliance gaps
— EHR-index error-name granularity (G-8/G-9) and the unenforced
`PARTY_REF.Type_validity` invariant (G-17); (c) scrub the dangling
`03-demographic-ehr-index-query.md` citations (G-5) onto this file; (d) convert
the remaining honest divergences into explicit PORT NOTEs.

---

## 1. Spec skeleton (spec-first enumeration)

### 1.1 RM demographic model (the domain types the service versions)

`RM/docs/demographic/master02-demographic_package.adoc` §Versioning Semantics:
"`PARTY` and its descendants `ACTOR` and `ROLE` are all potentially versioned…
Every Party is stored in its own Version container"; the party's identity is
its `uid`, copied from the enclosing `VERSION` (§Party Identification).

| RM type | Key attributes / invariants | Citation |
|---|---|---|
| `PARTY` (abstract) | `identities[1..1]`, `contacts[0..1]`, `details[0..1]`, `relationships[0..1]`, `type():DV_TEXT`, `reverse_relationships():List<LOCATABLE_REF>[0..1]`. Inv `Identities_valid` (`not identities.is_empty`), `Contacts_valid`, `Relationships_validity` (present⇒non-empty **and** each `r.source = self`), `Reverse_relationships_validity`, `Type_valid` (`type = name`), `Is_archetype_root`, `Uid_mandatory` (`uid /= Void`) | `…demographic.party.adoc` |
| `ACTOR` (abstract) | `languages[0..1]`, `roles[0..1]:List<PARTY_REF>`. Inv `Roles_valid` | `…demographic.actor.adoc` |
| `ROLE` | `time_validity[0..1]`, `performer[1..1]:PARTY_REF`, `capabilities[0..1]`. Inv `Capabilities_valid` | `…demographic.role.adoc` |
| `PERSON`, `ORGANISATION`, `GROUP`, `AGENT` | concrete `ACTOR` leaves | `…demographic.{person,organisation,group,agent}.adoc` |
| `PARTY_RELATIONSHIP` | `details[0..1]`, `source[1..1]:PARTY_REF`, `target[1..1]:PARTY_REF`, `time_validity[0..1]`, `type():DV_TEXT`. Inv `Source_valid` (`source.relationships.has(self)`), `Target_valid`, `Type_validity` (`type = name`). Stored under the `source` party; refs use `OBJECT_REF`+`HIER_OBJECT_ID` (the continuant), **never** `OBJECT_VERSION_ID` | `…demographic.party_relationship.adoc`; master02 §Party Relationships |
| `VERSIONED_PARTY` | `VERSIONED_OBJECT<PARTY>` | `…demographic.versioned_party.adoc` |
| `PARTY_IDENTITY`, `CONTACT`, `ADDRESS`, `CAPABILITY` | compositional archetyped structures | master02 §Names and Addresses |

### 1.2 BASE identifier law (`PARTY_REF`)

| Item | Rule | Citation |
|---|---|---|
| `OBJECT_REF` | `namespace[1..1]` ∈ {`"local"`, `"unknown"`, regex `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*`}, `type[1..1]` (RM class name or `ANY`), `id[1..1]:OBJECT_ID` | `…object_ref.adoc` |
| `PARTY_REF` | inherits `OBJECT_REF`; Inv **`Type_validity`**: `type ∈ {PERSON, ORGANISATION, GROUP, AGENT, ROLE, PARTY, ACTOR}` | `…party_ref.adoc` |
| `HIER_OBJECT_ID` | concrete `UID_BASED_ID` — the version-container id used by relationship refs | `…hier_object_id.adoc`; master05 §Object References |

### 1.3 SM `I_DEMOGRAPHIC_SERVICE` (`i_demographic_service.adoc`)

| # | Operation | Pre / errors | Citation |
|---|---|---|---|
| D1 | `create_party(a_version: UV_PARTY[1]): UUID` — new `VERSIONED_OBJECT`+`ORIGINAL_VERSION`+`CONTRIBUTION` | pre `definitions_valid`, `valid_content`; err `definition_unknown`, `content_invalid` | `i_demographic_service.adoc` |
| D2 | `create_party_relationship(a_version: UV_PARTY_RELATIONSHIP[1]): UUID` | pre `valid_content`; err `definition_unknown`, `content_invalid` | ″ |
| D3 | `i_party(a_versioned_party_id): I_PARTY` (accessor factory) | err `versioned_object_does_not_exist` | ″ |
| D4 | `i_party_relationship(a_versioned_party_rel_id): I_PARTY_RELATIONSHIP` | err `versioned_object_does_not_exist` | ″ |

### 1.4 SM `I_PARTY` (`i_party.adoc`)

| # | Operation | Pre / errors | Citation |
|---|---|---|---|
| P1 | `has_party(UUID): Boolean` | — | `i_party.adoc` |
| P2 | `has_party_version_id(UUID): Boolean` | — | ″ |
| P3 | `get_party(UUID): PARTY` | pre `has_party`; err `versioned_object_does_not_exist` | ″ |
| P4 | `get_party_at_time(UUID, Iso8601_date_time): PARTY` | err `versioned_object_does_not_exist` | ″ |
| P5 | `update_party(UUID, UV_PARTY): UUID` | pre `definitions_valid`, `has_party`; err `versioned_object_does_not_exist`, `object_version_does_not_exist`, `definition_unknown`, `content_invalid` | ″ |
| P6 | `delete_party(UUID)[0..1]` | pre `has_party`; post `not has_party`; err `versioned_object_does_not_exist` | ″ |
| P7 | `get_party_at_version(UUID): PARTY` | pre `has_party_version`; err `object_version_does_not_exist` | ″ |

### 1.5 SM `I_PARTY_RELATIONSHIP` (`i_party_relationship.adoc`)

| # | Operation | Pre / errors | Citation |
|---|---|---|---|
| R1 | `has_party_relationship(UUID): Boolean` | — | `i_party_relationship.adoc` |
| R2 | `get_party_relationship(UUID): PARTY_RELATIONSHIP` | err `versioned_object_does_not_exist` | ″ |
| R3 | `get_party_relationship_at_time(UUID, Iso8601): PARTY_RELATIONSHIP` | err `versioned_object_does_not_exist` | ″ |
| R4 | `update_party_relationship(UUID, UV_PARTY_RELATIONSHIP): UUID` | pre `definitions_valid`, `has_relationship`; err (4, as P5) | ″ |
| R5 | `delete_party_relationship(UUID)[0..1]` | pre `has_relationship`; post `not has_relationship`; err `versioned_object_does_not_exist` | ″ |
| R6 | `get_party_relationship_at_version(UUID): PARTY_RELATIONSHIP` | err `object_version_does_not_exist` | ″ |

`UV_PARTY` / `UV_PARTY_RELATIONSHIP` are "Form of `UPDATE_VERSION` specific to
`PARTY`/`PARTY_RELATIONSHIP`" (`uv_party.adoc`, `uv_party_relationship.adoc`),
carrying `lifecycle_state` / `preceding_version_uid` / `contribution` /
`commit_audit` (master03 §Version Update Semantics).

### 1.6 SM `I_EHR_INDEX` (`i_ehr_index.adoc`, master07)

master07 §Overview: the index records **N:M** subject↔EHR associations so an
EHR persisted with only an EHR id can recover its subject id (the MPI key); the
two multiple-association cases are error states the `RESOURCE_STATUS` metadata
exists "to detect and rectify".

| # | Operation | Errors | Citation |
|---|---|---|---|
| I1 | `add_ehr_subject(ehr_id: UUID, subject: OBJECT_REF, status: RESOURCE_STATUS[0..1], loc: LOCATION_DESC[0..1])[0..1]` | — | `i_ehr_index.adoc` |
| I2 | `update_ehr_subject_status(ehr_id, subject, status: RESOURCE_STATUS[1])[0..1]` | `subject_id_does_not_exist`, `ehr_id_does_not_exist` | ″ |
| I3 | `update_ehr_subject_loc_desc(ehr_id, subject, loc: LOCATION_DESC[0..1])[0..1]` | `subject_id_does_not_exist`, `ehr_id_does_not_exist` | ″ |
| I4 | `remove_ehr_subject(ehr_id, subject)[0..1]` | `subject_id_does_not_exist`, `ehr_id_does_not_exist` | ″ |
| I5 | `remove_subject(subject)[0..1]` | `subject_id_does_not_exist` | ″ |
| I6 | `RESOURCE_STATUS`: `instance_type[1]`, `start_valid_time[0..1]`(`@@`), `end_valid_time[0..1]`(`@@`), `notes[0..1]` | — | `resource_status.adoc` |
| I7 | `RESOURCE_INSTANCE_TYPE`: Primary / Duplicate / Supplementary | — | `resource_instance_type.adoc` |
| I8 | `LOCATION_DESC`: **empty stub** (description, no attributes) | — | `location_desc.adoc` |

---

## 2. Code mapped onto each spec item (`file:line`, verified 2026-07-12)

Realization layers: SM trait (FIXED) → `service/api/*.rs` adapter (parses SM
args, maps `ServiceError`→`SmError`) → `service/{demographic,relationship,
ehr_index}.rs` domain logic → shared `service/vobject.rs` + `ehr_index` table.

### 2.1 Demographic (RM invariants, D1–D4, P1–P7)

| Item | Realization | Verdict |
|---|---|---|
| RM PARTY invariants | `typed_check` (`demographic.rs:61`): `Identities_valid` (`:78`), present⇒non-empty `Contacts_valid`/`Relationships_validity`/`Roles_valid`/`Capabilities_valid` (`:92`), `Relationships_validity` 2nd arm — inline `source=self` (`:113`); `Uid_mandatory` by `uid` injection on read (`party_version_response:691`→`with_uid`); `Type_valid` trivial by construction | **conformant** for the wire-enforceable set |
| D1 `create_party` | `create_party` (`demographic.rs:163`)→`vobject::create(ehr_id=None)`; adapter `create_party` (`api/demographic.rs:86`) unwraps `UpdateVersion.data`, routes by `_type` | **conformant** (versioning + `valid_content`→422); **partial** — `definitions_valid`/`definition_unknown` absent (G-2) |
| D2 `create_party_relationship` | `create_relationship` (`relationship.rs:107`); adapter `api/relationship.rs` | **conformant**; same G-2 (no `definitions_valid`, correct per spec — R has none) |
| D3/D4 accessor factories | Realized implicitly — flat `I_PARTY`/`…RELATIONSHIP` calls take the id directly; `party_kind_at` (`demographic.rs:671`)→404 `versioned_object_does_not_exist` | **conformant** (accessor pattern, master02-overview §Interface Calls) |
| P1 `has_party` | `api/demographic.rs:94` via `party_kind_at`+`read_party` | **conformant** |
| P2 `has_party_version_id` | `api/demographic.rs:111` via `party_version` | **conformant** (was flagged folded-in; a distinct call exists) |
| P3 `get_party` | `read_party` (`demographic.rs:195`); deleted→`Null`; wrong-kind→404 (`load_party_version:630`) | **conformant** |
| P4 `get_party_at_time` | `read_party` w/ `at` (`demographic.rs:201`)→`vobject::version_at` | **conformant** |
| P5 `update_party` | `update_party` (`demographic.rs:211`): `ensure_party`(has)→validate→`vobject::update`; `If-Match`→412 | **conformant**; **partial** — `definitions_valid`/`definition_unknown` absent (G-2); bare-body envelope (G-1) |
| P6 `delete_party` | `delete_party` (`demographic.rs:249`): logical delete `523|deleted|`; post `not has_party` holds (deleted reads 404 via `ensure_party`) | **conformant** |
| P7 `get_party_at_version` | `party_version` (`demographic.rs:390`); adapter maps miss→`object_version_does_not_exist` (`api/demographic.rs:155`) | **conformant** |
| R1–R6 | `relationship.rs` (`create:107`, `read:138`, `update:155`, `delete:190`, `relationship_version:331`, `…_at_time:348`); refs checked to be `HIER_OBJECT_ID` not `OBJECT_VERSION_ID` (`relationship.rs:64`, master02) | **conformant** (positive: continuant-ref check) |
| `PARTY_REF.Type_validity` | **not enforced** — `typed_check` (`relationship.rs:45`) checks presence + id-type but never `type ∈ {PERSON…ACTOR}`; ACTOR `roles`/ROLE `performer` refs unchecked | **divergent** (G-17) |

### 2.2 EHR Index (I1–I8)

| Item | Realization | Verdict |
|---|---|---|
| I1 `add_ehr_subject` | `index_add_subject` (`ehr_index.rs:56`); default Primary status (`:64`); idempotent `ON CONFLICT DO UPDATE` (`:73`) | **conformant** (upsert silent — G-14) |
| I2 `update_ehr_subject_status` | `index_update_status` (`ehr_index.rs:94`) | **conformant** behaviour; **divergent** errors (G-8/G-9) |
| I3 `update_ehr_subject_loc_desc` | `index_update_loc_desc` (`ehr_index.rs:122`) | **conformant** behaviour; G-8/G-9 |
| I4 `remove_ehr_subject` | `index_remove_ehr_subject` (`ehr_index.rs:144`) | **conformant** behaviour; G-8/G-9 |
| I5 `remove_subject` | `index_remove_subject` (`ehr_index.rs:163`) | **conformant** behaviour; G-8 |
| I6 `RESOURCE_STATUS` | `ehrbase_sm` type; stored `ehr_index.rs:68`; reassembled `row_to_entry:231`; validity times ISO from `@@` (`parse_valid_time:21`) | **conformant** (G-16 spec-defect note) |
| I7 `RESOURCE_INSTANCE_TYPE` | `ehrbase_sm::ResourceInstanceType`; DB CHECK `ck_ehr_index_instance_type` | **conformant** |
| I8 `LOCATION_DESC` | designed `{system_id, uri?, description?}` (`location_json:32`, `row_to_entry:239`) over empty spec stub | **spec-silent** (G-12) |
| master07 duplicate detection | `instance_type` representable; **no detection query anywhere** | **missing** (G-10) |
| Errors `ehr_id_/subject_id_does_not_exist` | both → `ServiceError::NotFound` → `versioned_object_does_not_exist` (`service/mod.rs`); dedicated `SmError` variants exist, unused | **divergent** (G-8) |

---

## 3. Code mapping onto NO spec item (extension / silence flags)

These surfaces have **no** `I_DEMOGRAPHIC_SERVICE`/`I_EHR_INDEX` counterpart and
ITS-REST 1.0.3 vendors no demographic/EHR-index wire contract (CNF master10
demographic schedule is all-TBD; demographic is OPTIONS-profile only). All are
legitimate own-design extensions — **flag, keep, do not quarantine**:

- **`VERSIONED_PARTY` / `VERSIONED_OBJECT` read surface** (`demographic.rs:315`
  `versioned_party`, `:346` `party_revision_history`, `:390` `party_version`,
  `:405` `party_version_at_time`; `relationship.rs:252–369`) — "no ITS-REST
  demographic contract governs this — our own extension by analogy with the EHR
  group." Keep.
- **Demographic CONTRIBUTION** (`demographic.rs:432` create, `:445` get) —
  ehr-less contributions; own extension. Keep.
- **Demographic item tags** (`demographic.rs:509–624`, `party_tag_json:706`) —
  the RM `ITEM_TAG` extension applied to parties; own extension. Keep.
- **`VERSIONED_OBJECT.owner_id` self-reference** (`demographic.rs:326`,
  `relationship.rs:263`) — "no openEHR spec governs the owner of an EHR-less
  demographic versioned object — our own design" (a party has no owning EHR).
  Keep (G-6).
- **EHR-index design-filled reads** `index_ehr_subjects` / `index_subject_ehrs`
  (`ehr_index.rs:177`, `:193`) — "the SM defines no read op — our own design."
  Keep.
- **`RESOURCE_STATUS.start/end_valid_time` typed `@@`** — a spec defect
  resolved to ISO-8601; flag verbatim, not silently filled (G-16).

No **delete-candidate** or **quarantine** code was found — the extension
surfaces are all reachable and spec-flagged; the flat-file layout is a
decomposition target, not dead code.

---

## 4. Consolidated G-row register

`sev`: HIGH = compliance/behaviour, MED = conformance-surface/honesty, LOW =
doc/cosmetic. `disp`: fix-in-rewrite / PORT NOTE / already-correct / quarantine
/ delete.

| G | Domain | Gap | Citation / flag | sev | disp |
|---|---|---|---|---|---|
| G-1 | Demog | Wire seam accepts a **bare RM party**, not the `UV_PARTY` envelope; `lifecycle_state` dropped (always default). The native SM `create_party`/`update_party` **do** take `UpdateVersion` (`api/demographic.rs:86,162`); only the wire seam is bare | `uv_party.adoc`; `i_party.adoc §update_party` | MED | PORT NOTE (defensible ITS-REST-style adaptation; document) |
| G-2 | Demog | `definitions_valid` precondition + `definition_unknown` error unimplemented (no demographic archetype/OPT store); only `valid_content`→422 enforced | `i_demographic_service.adoc §create_party`; `party.adoc §Is_archetype_root` | MED | PORT NOTE (demographic OPTIONS-only per blueprint) |
| G-3 | Demog | `reverse_relationships` neither computed nor validated | `party.adoc §Reverse_relationships_validity` | LOW | PORT NOTE (derived 0..1; may derive on read in rewrite) |
| G-5 | Demog | **Dangling design-doc citation** `docs/design/sm-platform/03-demographic-ehr-index-query.md` (does not exist) in `relationship.rs:14,17`, `api/demographic.rs:239` | file existence | LOW | fix-in-rewrite (repoint onto this file) |
| G-6 | Demog | `VERSIONED_OBJECT.owner_id` self-references the demographic object (no owning EHR) | spec-silent flag | LOW | PORT NOTE (already in code) |
| G-7 | Demog | SM `create_party` returns `UUID`; wire returns `201`+representation (id in `ETag`/`Location`) | `i_demographic_service.adoc §create_party` | LOW | already-correct (deliberate REST adaptation) |
| G-17 | Rel | **`PARTY_REF.Type_validity` invariant not enforced** — relationship `source`/`target`, ACTOR `roles`, ROLE `performer` refs never checked for `type ∈ {PERSON,ORGANISATION,GROUP,AGENT,ROLE,PARTY,ACTOR}` | `party_ref.adoc §Type_validity` | HIGH | fix-in-rewrite |
| G-18 | Demog/Rel | `OBJECT_REF.namespace` regex/`"local"`/`"unknown"` legality not validated on inbound refs | `object_ref.adoc §namespace` | LOW | PORT NOTE (or fix-in-rewrite; cheap) |
| G-8 | Index | Two declared errors collapse to generic `versioned_object_does_not_exist`; `EhrIdDoesNotExist`/`SubjectIdDoesNotExist` exist unused (`ehrbase-sm/src/error.rs`) | `i_ehr_index.adoc §Errors` | HIGH | fix-in-rewrite |
| G-9 | Index | `update_*`/`remove_ehr_subject` cannot distinguish unknown-EHR from unknown-association to the caller | `i_ehr_index.adoc §Errors` | MED | fix-in-rewrite (folds into G-8) |
| G-10 | Index | Duplicate/error-state detection only **representable**, not detected (master07 "need to be detected and rectified") | `master07 §Overview` | MED | fix-in-rewrite (design-filled advisory read; **not** a hard reject) |
| G-11 | Index | No wire surface + zero ECC evidence (ITS-REST has no EHR-index binding) | `master07:11`; ITS-REST silence | MED | PORT NOTE (native-API-only; optional config-gated extension) |
| G-12 | Index | `LOCATION_DESC` designed `{system_id, uri?, description?}` over an attribute-less spec stub | `location_desc.adoc` (spec-silent flag) | LOW | PORT NOTE ("no openEHR spec governs this — our own design") |
| G-13 | Index | Subject reduced from `OBJECT_REF` to `{id, namespace, type}`; `OBJECT_ID` subtype not round-tripped | `i_ehr_index.adoc` (OBJECT_REF) | LOW | PORT NOTE |
| G-14 | Index | `add_ehr_subject` is a silent idempotent upsert (spec verb "Add") | `i_ehr_index.adoc §add_ehr_subject` | LOW | PORT NOTE (acceptable; `0..1` cardinality) |
| G-15 | Index | No auto-population from EHR creation; index and `ehr.subject_id` decoupled — index empty for EHRs created via the normal API | `master07 §Overview`; `service/ehr.rs` | MED | PORT NOTE (decide + document intentional decoupling) |
| G-16 | Index | `start/end_valid_time` typed `@@` in SM (spec defect); resolved to ISO-8601 `timestamptz` | `resource_status.adoc:20,24` | LOW | PORT NOTE (record defect verbatim) |
| G-19 | both | **Flat-file layout** — `demographic.rs` (733) **exceeds ≤~700**; `relationship.rs` (432), `ehr_index.rs` (259). Not spec, but the W-3f structural mandate | W-3f decomposition target | MED | fix-in-rewrite (§5) |

G-4 (from the SM-side audit — `has_party_version_id` folded in) is **resolved**:
a distinct adapter call exists (`api/demographic.rs:111`) → **already-correct**,
dropped from the register.

---

## 5. Target module layout (mirrors `app/ehrbase-sm/src/services/`)

Decompose the three flat domain files into two spec-shaped modules whose
internal split follows the SM interface boundaries (each file ≤~700 lines). The
`service/api/*.rs` trait-impl adapters move **into** each module as `api.rs`, so
each SM interface's realization (domain + adapter) is co-located — mirroring the
SM crate, where the trait definition and its doc-contract live per service.

```
app/ehrbase/src/service/
  demographic/                         # mirrors ehrbase-sm/services/demographic/
    mod.rs        # module root; shared PARTY helpers: kind_of/party_kind_of,
                  #   typed_check (PARTY invariants + G-17 PARTY_REF.Type_validity),
                  #   validate_party_body, load_party_version, ensure_party,
                  #   party_kind_at, ensure_any_party, party_version_response
    party.rs      # D1 + I_PARTY CRUD: create/read/update/delete/current_meta   (~230)
    relationship.rs  # I_PARTY_RELATIONSHIP (D2 + R1–R6) — mirrors SM relationship.rs (~330)
    versioned.rs  # EXTENSION: VERSIONED_PARTY / revision_history / version reads (~150)
    contribution.rs  # EXTENSION: demographic (ehr-less) CONTRIBUTION            (~90)
    tags.rs       # EXTENSION: demographic ITEM_TAG surface                      (~140)
    api.rs        # DemographicService + PartyRelationshipService trait impls
  ehr_index/                           # mirrors ehrbase-sm/services/ehr_index/
    mod.rs        # module root; helpers: ehr_exists, require_association,
                  #   parse_valid_time, location_json, row_to_entry, error mapping
    index.rs      # I1–I5 + design-filled reads (index_ehr_subjects/subject_ehrs) (~210)
    conflicts.rs  # G-10 design-filled duplicate-detection read (advisory)        (~60)
    api.rs        # EhrIndexService trait impl
```

Delete the flat `demographic.rs` / `relationship.rs` / `ehr_index.rs` and the
`api/demographic.rs` / `api/relationship.rs` / `api/ehr_index.rs` adapters once
their content lands in the modules above; update `service/mod.rs:32,37,44` decls
(`mod demographic;` → `mod demographic { … }` module dir).

### Seams — `TODO(w3f-integrate)` candidates

These are the cross-cutting dependencies the demographic/index modules consume;
W-3f factors them into dedicated modules and the new modules reference them by a
`TODO(w3f-integrate)` marker rather than re-reaching into today's flat neighbours:

- **`versioning/`** — the `super::vobject` machinery (`create`/`update`/`delete`/
  `read_*`/`version_at`/`object_kind`, `TreeId`, `VersionRead`, `Kind`,
  `object_version_id`, `audit`, `signing_ctx`). Versioned parties + relationships
  are pure consumers of it; the extension `versioned.rs` reads especially.
- **`storage/`** — the `sqlx` pool, `db::iden`, the `ehr_index` /
  `item_tag` / `contribution` / `vo_version` tables (migrations settled — do not
  re-author). The EHR-index domain is direct-SQL; the party domain is via
  `versioning/`.
- **error-mapping seam** — `ServiceError → SmError` in `service/mod.rs`. Closing
  G-8/G-9 requires typed EHR-index errors here (`EhrIdDoesNotExist` /
  `SubjectIdDoesNotExist`) instead of the flattened `NotFound`; this is the one
  shared seam the rewrite must touch outside the two modules.

---

## 6. Standing PORT-NOTE residue (keep / re-verify / drop)

- **KEEP** — `UV_PARTY`/`UV_PARTY_RELATIONSHIP` envelope realized server-side;
  the wire seam carries bare RM content (`lifecycle_state` defaulted) — G-1,
  `uv_party.adoc`.
- **KEEP** — `definitions_valid`/`definition_unknown` deliberately unimplemented
  (no demographic OPT store; OPTIONS-profile) — G-2, `i_demographic_service.adoc`.
- **KEEP** — `reverse_relationships` a derived 0..1 attribute the server may
  leave unpopulated — G-3, `party.adoc §Reverse_relationships_validity`.
- **KEEP** — `VERSIONED_OBJECT.owner_id` self-references the EHR-less demographic
  object (spec-silent — our design) — G-6.
- **KEEP** — no ITS-REST demographic/EHR-index contract; the whole
  `/demographic/*` + any `/ehr_index` surface is our extension (OPTIONS/out of
  CORE-STANDARD) — G-7/G-11.
- **KEEP** — `LOCATION_DESC` `{system_id, uri?, description?}` over the empty
  spec stub (spec-silent) — G-12, `location_desc.adoc`.
- **KEEP** — subject modelled `{id, namespace, type}`, not full `OBJECT_REF` —
  G-13, `i_ehr_index.adoc`.
- **KEEP** — index entries are not versioned objects (SM silence on versioning) —
  `ehr_index.rs:1-8`.
- **KEEP** — `RESOURCE_STATUS.start/end_valid_time` `@@` → ISO-8601 (spec
  defect, recorded verbatim) — G-16, `resource_status.adoc:20,24`.
- **RE-VERIFY on rewrite** — `add_ehr_subject` silent upsert (G-14) and the
  EHR-create/index decoupling (G-15): decide explicitly and PORT-NOTE the chosen
  behaviour rather than leaving it implicit.
- **DROP** — the four `03-demographic-ehr-index-query.md` citations (G-5): the
  doc does not exist; repoint onto this file. The G-4 "`has_party_version_id`
  folded in" note is stale — a distinct call exists — drop it.

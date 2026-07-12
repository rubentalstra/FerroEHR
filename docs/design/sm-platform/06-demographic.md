# Demographic Service (SM `I_DEMOGRAPHIC_SERVICE`) — spec-compliance audit

Read-only audit (2026-07-12) of the demographic service — `I_DEMOGRAPHIC_SERVICE`,
`I_PARTY`, `I_PARTY_RELATIONSHIP` and the `UV_PARTY` / `UV_PARTY_RELATIONSHIP`
update-version envelopes — against the vendored SM chapter. Structure mirrors
`docs/design/sm-platform/10-subject-proxy.md`: spec oracle → verified current
state (faithful realizations **and** gaps) → gap register with G-n rows → target
design → PORT-NOTE residue.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master06-demographic_service.adoc`
  — the chapter; `include::`s the five class files below.
- `docs/specs/openehr/SM/docs/UML/classes/i_demographic_service.adoc` — the
  service interface: `create_party`, `create_party_relationship`, the `i_party`
  / `i_party_relationship` accessor factories.
- `docs/specs/openehr/SM/docs/UML/classes/i_party.adoc` — `has_party`,
  `has_party_version_id`, `get_party`, `get_party_at_time`, `update_party`,
  `delete_party`, `get_party_at_version`.
- `docs/specs/openehr/SM/docs/UML/classes/i_party_relationship.adoc` — the six
  `PARTY_RELATIONSHIP` operations.
- `docs/specs/openehr/SM/docs/UML/classes/uv_party.adoc` /
  `uv_party_relationship.adoc` — "Form of `UPDATE_VERSION` specific to
  `PARTY`/`PARTY_RELATIONSHIP`"; both inherit `UPDATE_VERSION`.
- Adjacent RM: `RM/docs/UML/classes/org.openehr.rm.demographic.party.adoc`
  (PARTY invariants), `…demographic.actor.adoc` (`Roles_valid`),
  `…demographic.role.adoc` (`Capabilities_valid`),
  `RM/docs/common/` (Change Control — VERSIONED_OBJECT / ORIGINAL_VERSION /
  CONTRIBUTION / AUDIT_DETAILS versioning semantics the create/update/delete
  operations invoke).

**Verdict up front.** The demographic service is a **faithful, well-grounded
realization**: every wire-reachable SM operation is present with the right
versioning effect (server-side `VERSIONED_OBJECT` + `ORIGINAL_VERSION` +
`CONTRIBUTION` on create; new `ORIGINAL_VERSION` + `CONTRIBUTION` on update;
logical-delete satisfying `delete_party`'s `not has_party` post-condition), and
the PARTY invariants that are enforceable at the wire are enforced. The gaps are
narrow: the `definitions_valid` precondition / `definition_unknown` error is not
implemented (no archetype/template validation for parties); the wire accepts a
**bare RM party** rather than the SM `UV_PARTY` envelope (a deliberate ITS-REST-
style adaptation, but undocumented as such); `reverse_relationships` is neither
computed nor validated; and every code doc-comment cites a design file
(`03-demographic-ehr-index-query.md`) that **does not exist** — this document is
its replacement.

---

## 1. Verified current state

**Realization map** (verified file:line 2026-07-12). The SM interfaces are split
across two catalog traits, wired to the `EhrbaseService` domain logic through
thin `service/api/` adapters, and served by one `ehrbase-rest` dispatcher.

- Catalog traits: `app/ehrbase-sm/src/services/demographic.rs`
  (`DemographicService`, PARTY slice) + `…/relationship.rs`
  (`PartyRelationshipService`), re-exported at `…/services/mod.rs:47,58`.
- Domain logic: `app/ehrbase/src/service/demographic.rs` (PARTY, 709 lines) +
  `app/ehrbase/src/service/relationship.rs` (PARTY_RELATIONSHIP, 433 lines), on
  the shared `service/vobject.rs` versioned-object machinery.
- Trait adapters: `app/ehrbase/src/service/api/demographic.rs`,
  `…/api/relationship.rs`.
- Wire: `app/ehrbase-rest/src/dispatch/demographic.rs` (one dispatcher serving
  both the generated per-kind demographic `ROUTES` and the hand-declared
  `RELATIONSHIP_ROUTES`, `demographic.rs:311`).
- Tests: `app/ehrbase/tests/service_demographic.rs`,
  `app/ehrbase-rest/tests/demographic_http.rs`; ECC `Area::Demographic`
  (`tools/conformance/src/suites/demographic.rs`).

### 1.1 `I_DEMOGRAPHIC_SERVICE` (`i_demographic_service.adoc`)

| SM operation (signature / pre / errors) | Realization | Verdict |
|---|---|---|
| `create_party(a_version: UV_PARTY[1]): UUID` — pre `definitions_valid`, `valid_content`; errors `definition_unknown`, `content_invalid` | `create_party` (`service/demographic.rs:150`) → `vobject::create` (new VERSIONED_OBJECT + ORIGINAL_VERSION + CONTRIBUTION); trait `party_create` (`api/demographic.rs:34`); wire `POST /demographic/{kind}` → `201` + `ETag`/`Location` (`dispatch/demographic.rs:106`) | **Present, partial** — versioning + `valid_content`→`422` faithful; **`definitions_valid`/`definition_unknown` NOT implemented** (G-2); input is a **bare party**, not `UV_PARTY` (G-1) |
| `create_party_relationship(a_version: UV_PARTY_RELATIONSHIP[1]): UUID` — pre `valid_content`; errors `definition_unknown`, `content_invalid` | `create_relationship` (`service/relationship.rs:107`); trait (`api/relationship.rs:27`); wire `POST /demographic/party_relationship` (`dispatch/demographic.rs:370`) | **Present** — no `definitions_valid` in spec, correctly omitted; `valid_content`→`422`; same G-1 (bare body) |
| `i_party(a_versioned_party_id): I_PARTY` | Object-oriented accessor factory — realized implicitly (all `I_PARTY` calls take the id directly); no wire op | **N/A (faithful)** — accessor pattern, not a REST resource |
| `i_party_relationship(a_versioned_party_rel_id): I_PARTY_RELATIONSHIP` | As above | **N/A (faithful)** |

### 1.2 `I_PARTY` (`i_party.adoc`)

| SM operation | Realization | Verdict |
|---|---|---|
| `has_party(UUID): Boolean` | `ensure_party` (`service/demographic.rs:643`) via `load_party_version:617`; used as a precondition, not exposed as a wire op | **Present (as precondition)** |
| `has_party_version_id(UUID): Boolean` | No distinct query; the get path parses the version out of `uid_based_id` and 404s if absent (`api/demographic.rs:44`, `version_id::parse_uid_based_id:210`) | **Folded in** (G-4, minor) |
| `get_party(UUID): PARTY` — pre `has_party`; error `versioned_object_does_not_exist` | `read_party` (`service/demographic.rs:182`), `uid` injected on read for `Uid_mandatory` (`party_version_response:667` → `with_uid`); unknown/wrong-kind → `404` (`load_party_version:626`) | **Faithful** |
| `get_party_at_time(UUID, Iso8601_date_time): PARTY` | `read_party` with `at` (`service/demographic.rs:188`), `vobject::version_at`; `version_at_time` parsed at `api/demographic.rs:45` | **Faithful** |
| `update_party(UUID, UV_PARTY): UUID` — pre `definitions_valid`, `has_party`; errors `versioned_object_does_not_exist`, `object_version_does_not_exist`, `definition_unknown`, `content_invalid` | `update_party` (`service/demographic.rs:198`): `ensure_party` (has_party) → validate → `vobject::update` (new ORIGINAL_VERSION + CONTRIBUTION); `If-Match` optimistic concurrency → `412` (`dispatch/demographic.rs:150`) | **Present, partial** — `has_party`, versioning, `content_invalid`→`422` faithful; **`definitions_valid`/`definition_unknown` missing** (G-2); bare body (G-1) |
| `delete_party(UUID)` [0..1] — pre `has_party`, post `not has_party` | `delete_party` (`service/demographic.rs:236`): logical delete (`523\|deleted\|` version via `vobject::delete`); already-deleted → `400`; `204` + `ETag`/`Location` | **Faithful** — post-condition holds (a deleted party reads `404` via `ensure_party`); the optional `If-Match`/path-OVID guard is a stricter extension |
| `get_party_at_version(a_party_version_id: UUID): PARTY` — pre `has_party_version`; error `object_version_does_not_exist` | `read_party` with a parsed `TreeId` (`api/demographic.rs:44` when `uid_based_id` is an OVID), and `party_version` (`service/demographic.rs:377`) for the `versioned_party/…/version/{version_uid}` route | **Faithful** |

### 1.3 `I_PARTY_RELATIONSHIP` (`i_party_relationship.adoc`)

All six operations — `has_party_relationship`, `get_party_relationship`,
`get_party_relationship_at_time`, `update_party_relationship`,
`delete_party_relationship`, `get_party_relationship_at_version` — are realized
in `service/relationship.rs` (`create_relationship:107`, `read_relationship:138`,
`update_relationship:155`, `delete_relationship:190`, `relationship_version:331`,
`relationship_version_at_time:348`) with the same versioning effect and error
mapping as the PARTY family. Two SM asymmetries are normalized to the PARTY
pattern and documented in-module (`relationship.rs:16-26`): the spec lists a
`versioned_object_does_not_exist` error on `get_party_relationship` without a
`has_party_relationship` precondition (treated as `404`), and `update` keeps the
SM's `definitions_valid` wording, which reduces to the same structural
`typed_check` here. **Faithful.** The relationship refs are additionally checked
to be `HIER_OBJECT_ID` version-containers, not `OBJECT_VERSION_ID`s (RM
demographic master02), at `relationship.rs:60-74` — a correct positive.

### 1.4 Extensions beyond the SM interface (correctly flagged as such)

The `versioned_party` read surface (VERSIONED_PARTY / revision_history / VERSION
at-time / VERSION by-id, `service/demographic.rs:298-411`), demographic
CONTRIBUTION create/get (`:419-494`), and demographic item tags (`:499-611`) have
no `I_DEMOGRAPHIC_SERVICE` counterpart; they mirror the ITS-REST EHR group by
analogy. ITS-REST 1.0.3 vendors **no** demographic wire contract and the CNF
`master10-func_tc_demographic.adoc` schedule is all-TBD placeholders (confirmed
in `tools/conformance/src/suites/demographic.rs:4-8`); demographic is an
OPTIONS-profile capability. These are legitimately our own extension surface.

### 1.5 PARTY invariant enforcement (RM `…demographic.party.adoc`)

`typed_check` (`service/demographic.rs:48`) enforces, on create/update:
`Identities_valid` (`not identities.is_empty`, `:65`); the "present ⇒ non-empty"
list invariants `Contacts_valid`, `Relationships_validity` (first arm),
`Roles_valid` (ACTOR), `Capabilities_valid` (ROLE) (`:79-95`); and
`Relationships_validity` second arm — every inline relationship's `source` must
reference this party (`:100-114`). `Uid_mandatory` is met by injecting `uid` on
read (`party_version_response:667`), matching the COMPOSITION service and the RM
NOTE that `uid` is copied from the enclosing VERSION. `Type_valid` (`type = name`)
is trivially satisfied by construction (the RM `type()` function returns `name`),
so its absence is not a gap. **Faithful** for the enforceable set.

---

## 2. Gap register

Every gap cites governing spec text. G-1/G-2 are the substantive compliance
gaps; the rest are minor or documentation.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **The wire accepts a bare RM party/relationship, not the SM `UV_PARTY` / `UV_PARTY_RELATIONSHIP` envelope.** `create_party(a_version: UV_PARTY)` and `update_party(…, UV_PARTY)` take a `UPDATE_VERSION` form (`uv_party.adoc`: "Form of `UPDATE_VERSION`") carrying `lifecycle_state`, `preceding_version_uid`, `contribution`, `commit_audit`. The impl deserializes a bare `Person`/`Agent`/…/`PartyRelationship` (`service/demographic.rs:122-133`, `validate_party_body`; `relationship.rs:81`). The envelope fields are instead supplied out-of-band (server-generated audit/contribution; `If-Match` for the preceding version). | `i_demographic_service.adoc` §create_party; `i_party.adoc` §update_party; `uv_party.adoc`, `uv_party_relationship.adoc` | Undocumented as a divergence — the doc-comments say "commit a new party version" without noting the `UV_PARTY` shape is not honoured on the wire. This is the same content-on-the-wire adaptation ITS-REST makes for COMPOSITION, so it is defensible, but must be an explicit PORT NOTE. `lifecycle_state` in particular is dropped (parties are always committed as the default state). |
| G-2 | **`definitions_valid` precondition and `definition_unknown` error unimplemented.** `create_party`/`update_party` carry pre `definitions_valid(a_version)` and error `definition_unknown`; PARTY objects are archetype roots (`party.adoc` invariant `Is_archetype_root`). The impl performs only structural + RM-invariant checks (`typed_check`), never validating the party against a known archetype/template, so `definition_unknown` can never be emitted. | `i_demographic_service.adoc` §create_party (`Pre_party_definitions_valid`, error `definition_unknown`); `i_party.adoc` §update_party | No definition/archetype validation exists for demographic objects; every structurally-valid body is accepted. `content_invalid` (the second error) IS realized as `422` (`ServiceError::Unprocessable`). |
| G-3 | **`reverse_relationships` neither computed nor validated.** PARTY has `reverse_relationships(): List<LOCATABLE_REF>` [0..1] with `Reverse_relationships_validity` requiring each target relationship exist in the demographics repository with `target = self`. | `party.adoc` §reverse_relationships, §Reverse_relationships_validity | Not surfaced on any read and not validated. Relationships are stored under their `source` (per the enforced first arm), so the reverse view is derivable but absent. |
| G-4 | **`has_party_version_id(UUID): Boolean` not exposed as a distinct operation.** SM lists it `1..1`. | `i_party.adoc` §has_party_version_id | Folded into the get path (an unknown version → `404` at `service/demographic.rs:633,638`); no standalone existence query. Minor — the effect is reachable, only the discrete boolean call is absent. |
| G-5 | **Dangling design-doc citation across four source files.** Code cites `docs/design/sm-platform/03-demographic-ehr-index-query.md`, which **does not exist** (only `10-subject-proxy.md` + `README.md` are present under `docs/design/sm-platform/`). | file existence vs `app/ehrbase-sm/src/services/demographic.rs:76`, `app/ehrbase/src/service/relationship.rs:14`, `app/ehrbase/src/service/api/demographic.rs:68`, `app/ehrbase-rest/src/dispatch/demographic.rs:199` | Same failure mode the subject-proxy audit flagged. This document replaces the missing `03-…`; the four citations must be repointed here. |
| G-6 | **`VERSIONED_OBJECT.owner_id` self-reference for demographic objects.** `VERSIONED_OBJECT.owner_id` is `1..1` but a demographic party has no owning EHR; the impl references the party's own versioned-object id as owner (`service/demographic.rs:313-324`, `relationship.rs:263-275`). | `party.adoc` / RM common VERSIONED_OBJECT; ITS-REST silent on demographic wire | Already a `// PORT NOTE`; recorded here as standing residue, not a defect. |
| G-7 | **`create_party` returns `UUID` in the SM; the wire returns `201` + representation.** | `i_demographic_service.adoc` §create_party (return `UUID`) | Deliberate REST adaptation (the new versioned-object id travels in `ETag`/`Location`); consistent with the EHR group. Note only. |

---

## 3. Target design (to close the register)

The service is close to compliant; the changes are surgical, not a rebuild.

1. **G-1 — honour `UV_PARTY` OR document the bare-body adaptation.** Preferred:
   keep the ITS-REST-style bare-content wire (it matches COMPOSITION and the
   OPTIONS-profile extension nature), and record an explicit PORT NOTE on
   `validate_party_body` / `create_relationship` stating that the SM `UV_PARTY` /
   `UV_PARTY_RELATIONSHIP` `UPDATE_VERSION` envelope is realized by the server
   (audit + contribution generated server-side; `preceding_version_uid` taken
   from `If-Match`; `lifecycle_state` defaulted to `complete`), citing
   `uv_party.adoc` / `i_party.adoc`. If a caller-supplied `lifecycle_state` is
   ever required, extend the wire to accept the envelope on `Content-Type` of the
   VERSION shape — but the spec does not mandate it at the REST surface.

2. **G-2 — implement `definitions_valid` or PORT-NOTE it as out of scope.** Two
   honest options: (a) wire demographic-archetype/template validation into
   `validate_party_body` (a `definition_unknown` → `422`/`400` when the party's
   `archetype_node_id`/`archetype_details` names an unknown definition), reusing
   the AM validation machinery; or (b) if demographic archetype ingestion is not
   in scope, record a PORT NOTE that `definitions_valid`/`definition_unknown` are
   deliberately unimplemented (no demographic OPT store exists) and that only
   `valid_content` is enforced. The blueprint treats demographic as OPTIONS-only,
   so (b) is acceptable — but it must be stated, not silent.

3. **G-3 — derive `reverse_relationships` on read (or PORT-NOTE).** Since inline
   relationships are stored under `source`, a query "relationships whose
   `target` references this party" yields the reverse view. Either expose it when
   materializing a PARTY read, or PORT-NOTE that `reverse_relationships` is a
   derived 0..1 attribute the server does not populate (the client may query the
   PARTY_RELATIONSHIP surface). Cite `party.adoc §Reverse_relationships_validity`.

4. **G-4 — optional.** Add nothing unless a client needs the discrete boolean;
   the existence semantics are already reachable. If added, an `EXISTS` query on
   `vo_version` by version keyed id, exposed only on the native trait.

5. **G-5 — scrub the four dangling `03-…` citations**, repointing them at this
   file (`docs/design/sm-platform/06-demographic.md`). Mechanical, same-PR.

6. **G-6/G-7 — no change**; recorded as standing residue below.

Verification: the existing `service_demographic` / `demographic_http` suites plus
the ECC `Area::Demographic` cases already cover the lifecycle; add negative cases
for whichever of G-2 lands (unknown-definition → error) and, if G-3 is
implemented, a reverse-relationship read. Gates: workspace suites green, clippy
clean, ECC zero-drift.

---

## 4. Standing PORT NOTEs (the honest residue after closure)

- **`UV_PARTY` envelope** is realized server-side; the wire carries bare RM
  party/relationship content (ITS-REST-style adaptation) — `uv_party.adoc`,
  `i_party.adoc §update_party`. (G-1, once documented in code.)
- **`definitions_valid` / `definition_unknown`** deliberately unimplemented if
  demographic archetype ingestion stays out of scope; only `valid_content` is
  enforced — `i_demographic_service.adoc §create_party`. (G-2 option b.)
- **`reverse_relationships`** is a derived 0..1 attribute the server may leave
  unpopulated — `party.adoc §Reverse_relationships_validity`. (G-3 if not
  implemented.)
- **`VERSIONED_OBJECT.owner_id`** self-references the demographic object (no
  owning EHR) — already in code. (G-6.)
- **No ITS-REST demographic contract**: the whole `/demographic/*` surface,
  including the `PARTY_RELATIONSHIP` routes, is our own extension (OPTIONS
  profile); CNF `master10` is all-TBD. (G-7 / §1.4.)

# ITS-REST Demographic API — wire spec-compliance audit

Read-only audit (2026-07-12) of our **demographic HTTP wire** against the
newly-vendored ITS-REST **Demographic API** (development edition, `DEVELOPMENT`
maturity). This is the *first* time an openEHR wire contract exists for
demographics: our wire (`app/ehrbase-rest/src/dispatch/demographic.rs`) was
built as an **extension by analogy with the EHR group** on the premise that
ITS-REST defined no demographic contract. That premise is now false. This
document audits the analogy wire against the real spec; every divergence is a
`G-n` row. Structure mirrors `docs/design/sm-platform/10-subject-proxy.md`
(spec oracle → verified current state with file:line → gap register with
citations → target design → PORT-NOTE residue).

Companion doc: `docs/design/sm-platform/06-demographic.md` audits the *SM
service* (`I_DEMOGRAPHIC_SERVICE` / `I_PARTY` / `I_PARTY_RELATIONSHIP`); this
doc audits only the *ITS-REST wire* realizing it.

**Spec oracle** (read before any change):

- `docs/specs/openehr/ITS-REST/specifications/demographic.openapi.yaml` — the
  Demographic API OAS: the full path set, tags (`AGENT`/`GROUP`/`ORGANISATION`/
  `PERSON`/`ROLE`/`VERSIONED_PARTY`/`CONTRIBUTION`/`ITEM_TAG`), `x-status:
  DEVELOPMENT`, server base `…/v1`.
- `docs/specs/openehr/ITS-REST/specifications/docs/demographic/Description.md`
  — purpose + `DEVELOPMENT` status.
- `docs/specs/openehr/ITS-REST/specifications/operations/` — the per-operation
  contracts: `person_create.yaml`, `person_get.yaml`, `person_update.yaml`,
  `person_delete.yaml` (+ identical `agent_*`/`group_*`/`organisation_*`/
  `role_*`), `versioned_party_get.yaml`,
  `versioned_party_revision_history.yaml`,
  `versioned_party_version_get_at_time.yaml`,
  `versioned_party_version_get_by_id.yaml`,
  `demographic_contribution_create.yaml`, `demographic_contribution_get.yaml`,
  `demographic_tags_get.yaml`, `person_tags_get.yaml`,
  `person_tags_update.yaml`, `person_tags_delete.yaml`.
- Responses/headers: `responses/201_PERSON.yaml`,
  `responses/409_PERSON_with_uid_based_id.yaml`,
  `responses/400_already_deleted.yaml`,
  `responses/201_demographic_CONTRIBUTION.yaml`, `headers/Location_PERSON.yaml`,
  `headers/openehr-item-tag.yaml`, `headers/openehr-version-item-tag.yaml`;
  params `parameters/path/uid_based_id.yaml`,
  `parameters/path/versioned_object_uid_PARTY.yaml`,
  `parameters/header/Accept_LOCATABLE.yaml` vs
  `parameters/header/Accept_canonical.yaml`.
- Schemas: `schemas/demographic/Person.yaml` (`allOf` ACTOR, `_type: PERSON`),
  `NewContribution.yaml`, `UpdateVersion.yaml`.

**Current implementation** (verified 2026-07-12):

- Wire dispatch: `app/ehrbase-rest/src/dispatch/demographic.rs` (639 lines).
- Generated contract (route table + param structs, emitted from the vendored
  OAS): `crates/openehr-its/src/rest/generated/demographic.rs` (`ROUTES` at
  line 1048; e.g. `AgentDeleteParams` at 177, `PersonDeleteParams` at 360).
  Vendored OAS: `crates/openehr-its/vendor/rest-oas/demographic-codegen.openapi.yaml`.
- Mounting: `app/ehrbase-rest/src/dispatch/mod.rs:85` mounts
  `g::demographic::ROUTES`; `:89` additionally mounts the non-spec
  `demographic::RELATIONSHIP_ROUTES`.
- Native seam (SM-true trait + wire methods):
  `app/ehrbase-sm/src/services/demographic/service.rs` (258 lines),
  `.../relationship.rs` (164), `.../mod.rs` (23).

**Verdict up front.** The analogy wire is, on **paths and the read/update
happy-paths, close to the real spec** — because the generated `ROUTES`
(`demographic.rs:1048`) are now emitted from the vendored OAS, so every
spec path (`/demographic/{kind}`, `/demographic/{kind}/{uid_based_id}`,
`/demographic/versioned_party/…`, `/demographic/contribution`,
`/demographic/tags`, the `…/tags` sub-resources) is routed and dispatched
(`run_party` `demographic.rs:94`, `run_shared` `:220`). `get` (current /
at-time / by-version, deleted→`204`), `update` (`If-Match` → `412`), the
versioned-party reads, `contribution` create/get, and the tag sub-resources
all match the operation contracts. The divergences are concentrated in
**delete semantics, item-tag headers, the `Prefer`-driven tag response, and
the stale "no wire contract" design premise** — plus one whole subtree
(`PARTY_RELATIONSHIP`) the spec does not define at all.

---

## 1. Gap register (what is not spec-true today)

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **The design premise is stale — a demographic wire spec now exists.** The module + dispatch headers assert "ITS-REST 1.0.3 defines **no** demographic wire contract … demographic is OPTIONS-profile only … CNF demographic schedule (master10) is all TBD" and cast the *entire* wire as "our own extension by analogy with the EHR group". A development-edition Demographic API is now vendored and generated. The wire is no longer analogy-only; it is a spec-defined (development-maturity) contract. Doc comments and PORT NOTEs must be re-scoped: only the genuinely spec-absent pieces (PARTY_RELATIONSHIP — G-5) remain extensions. | `demographic.openapi.yaml` (`info.x-status: DEVELOPMENT`, full `paths`); `docs/demographic/Description.md` | `dispatch/demographic.rs:4-16` and `services/demographic/mod.rs:10-16` both state no contract exists / OPTIONS-only; every trait doc comment tags the wire methods "extension — module PORT NOTE" (`service.rs:130`, `:18`; `relationship.rs:89`). |
| G-2 | **DELETE preceding-version + `409`/`400_already_deleted` semantics diverge.** Spec `person_delete`: `uid_based_id` **MUST** be an `OBJECT_VERSION_ID` = the `preceding_version_uid` to delete (`uid_based_id_as_version_uid`); there is **no `If-Match`**; responses are `204_version_deleted`, **`400_already_deleted`**, `404`, and **`409_PERSON_with_uid_based_id`** ("supplied `uid_based_id` doesn't match the latest version" + latest `version_uid` in `ETag`). Our delete instead takes the preceding version from the **`If-Match` header**, treats the path as a versioned-object uid, builds a `get`-shaped param struct (spuriously reading `version_at_time`), and produces neither the `409` mismatch nor a distinct `400_already_deleted`. | `operations/person_delete.yaml`; `responses/409_PERSON_with_uid_based_id.yaml`; `responses/400_already_deleted.yaml`; generated `PersonDeleteParams`/`AgentDeleteParams` carry **only** `uid_based_id` (`demographic.rs:360`/`:177`) | `run_party_delete` `dispatch/demographic.rs:200-216`: `params::build::<AgentGetParams>` (`:208`) + `party_delete(kind, uid, if_match_of(h))` (`:211`); `if_match_of` `:603`; trait `party_delete(…, if_match: Option<String>)` `service.rs:165`. No `409`/`400_already_deleted` arm. |
| G-3 | **ITEM_TAG response headers not emitted on create/get.** `person_create` declares `openehr-item-tag` + `openehr-version-item-tag` as **request and response** headers; `201_PERSON` lists both response headers; `person_get` returns them when tags exist. Our create/get set only `ETag` + `Location`; the incoming item-tag request headers are parsed into the generated params but discarded, and nothing is echoed back. | `operations/person_create.yaml` (params + description); `responses/201_PERSON.yaml` (`headers: openehr-item-tag`, `openehr-version-item-tag`); `operations/person_get.yaml` | `set_headers` sets only `ETag`/`Location` (`dispatch/demographic.rs:612-625`); create discards the built params (`let _p = …` `:108`); no item-tag header emission on any party path. |
| G-4 | **Tag update ignores `Prefer`/`204`.** `person_tags_update` responses are `200` (`200_…ItemTagList_updated`) **and `204`** (`204_updated`, for `Prefer: return=minimal`) + `400`/`404`; request body is an array of `UpdateItemTag`. Our handler always returns `200` and never honours `Prefer=minimal`→`204`. | `operations/person_tags_update.yaml` (`Prefer` param; `200`/`204` responses) | `"tags_update"` arm always `negotiate::respond(h, StatusCode::OK, …)` (`dispatch/demographic.rs:173-181`) — no `Prefer` branch. |
| G-5 | **`PARTY_RELATIONSHIP` wire has no spec basis.** The vendored Demographic API defines **no** `party_relationship` / `versioned_party_relationship` paths anywhere. Our 8 relationship routes are entirely invented. Keeping them is defensible (they realize SM `I_PARTY_RELATIONSHIP`), but with a real spec in hand they must be labelled explicitly non-spec and excluded from any conformance-profile claim — the prior "by analogy" framing no longer covers them. | `demographic.openapi.yaml` `paths` (no relationship entry); SM `i_party_relationship.adoc` is the *service* basis, not a wire basis | `RELATIONSHIP_ROUTES` `dispatch/demographic.rs:311-352`; `run_relationship` `:358`; mounted `dispatch/mod.rs:89`. |
| G-6 | **CONTRIBUTION create input contract (`NewContribution` + `UPDATE_AUDIT`) not enforced at the wire.** Spec: body is `NewContribution`; `audit` and each `versions[i].commit_audit` are `UPDATE_AUDIT` (an `AUDIT_DETAILS` minus server-assigned `time_committed`/`system_id`); servers **SHOULD** accept `_type: UPDATE_AUDIT`, `AUDIT_DETAILS`, or omitted; `audit.change_type`/`lifecycle_state` are `DV_CODED_TEXT`. Our dispatch passes the raw JSON straight to the service (`demographic_contribution_create(body)`); tolerant `_type` acceptance and the relaxed-schema validation are unverified at this layer. | `operations/demographic_contribution_create.yaml`; `schemas/demographic/NewContribution.yaml`, `UpdateVersion.yaml` | `"contribution_create"` arm: `negotiate::json_value(h, &parts.body)` then pass-through (`dispatch/demographic.rs:268-284`); no `NewContribution`/`UPDATE_AUDIT` typing at the wire. |
| G-7 | **Canonical-XML output on versioned-party / contribution / tags unverified.** Spec splits content negotiation: party CRUD uses **LOCATABLE** JSON+XML (`Accept_LOCATABLE`/`ContentType_LOCATABLE`), while `versioned_party_*`, `contribution_*`, and `tags` use **canonical** JSON+XML (`Accept_canonical`/`ContentType_canonical`). Party bodies go through `negotiate::rm_value`/`respond_rm` (JSON+XML — matches LOCATABLE), but the versioned-party / contribution / tag reads use `negotiate::respond` (stored-body passthrough), so their XML representation is not produced — the same version-family XML gap tracked for COMPOSITION (blueprint F-05-06). | `operations/versioned_party_get.yaml` etc. (`Accept_canonical`/`ContentType_canonical`); `parameters/header/Accept_canonical.yaml` | `respond_party`/`respond_rm` for parties (`dispatch/demographic.rs:534-548`) vs `negotiate::respond` for versioned-party/contribution/tags (`:237`, `:245`, `:266`, `:291`, `:299`, `:171`). |
| G-8 | **`create` `404` response modelled by the spec, absent here.** `person_create` lists `404` among its responses; our create arm has no `404` branch (only propagates whatever the service returns). Minor, but a documented response of the contract. | `operations/person_create.yaml` (`responses: '404'`) | `"create"` arm (`dispatch/demographic.rs:106-120`) — `201` + error passthrough only. |
| G-9 | **DEVELOPMENT-maturity conformance evidencing not re-based on the real contract.** The prior DemographicApi ECC cases were built against the analogy wire while the spec was assumed absent. With the OAS vendored + generated, the ECC demographic cases should be re-adjudicated against the actual operation contracts (status codes, headers, the `409` delete path) rather than the invented behaviour. The spec is `DEVELOPMENT` maturity, so this is not CORE/STANDARD-gated — but it is now testable against a real contract. | `demographic.openapi.yaml` `x-status: DEVELOPMENT`; blueprint §2.3 row 5 (DemographicApi) | ECC DemographicApi edges closed at B6 predate the vendored contract; no re-adjudication recorded. |

**Faithful realizations (no gap — recorded so the audit is honest):**

- Path set matches the spec 1:1 (generated `ROUTES` `demographic.rs:1048`
  from the vendored OAS): `/demographic/{agent,group,organisation,person,role}`
  POST + `/{uid_based_id}` GET/PUT/DELETE; `/demographic/versioned_party/…`
  four GETs; `/demographic/contribution` POST + `/{contribution_uid}` GET;
  `/demographic/tags` GET; `/{kind}/{uid_based_id}/tags` GET/PUT +
  `/tags/{key}` DELETE.
- `get` (`operations/person_get.yaml`): `uid_based_id` accepts both
  `OBJECT_VERSION_ID` and `HIER_OBJECT_ID`, `version_at_time` query, latest by
  default, deleted current version → `204` — realized at
  `dispatch/demographic.rs:121-132` (`party_get` + `is_empty()`→`204`).
- `update` (`operations/person_update.yaml`): path is the versioned-object
  uid, `If-Match` carries `preceding_version_uid`, `412` on mismatch (with
  latest `ETag`/`Location`), `200`/`204` per `Prefer`, `422` on invalid
  content — realized at `dispatch/demographic.rs:133-166` + `is_precondition`
  (`:89`) + `ContentInvalid → 422` (`overview/error.rs:78`).
- `versioned_party` reads, `contribution` create (`201` + `ETag`/`Location`
  per `Prefer`, `oneOf[Contribution, Identifier]`) and get, `demographic_tags`
  filter query — routed with the right status/`Location` shape
  (`dispatch/demographic.rs:230-300`; `Location` `:617`,
  `headers/Location_PERSON.yaml`).

---

## 2. Target design

### 2.1 Re-scope the design premise (G-1, G-5, G-9)

Rewrite the module + dispatch doc comments (`services/demographic/mod.rs:10-16`,
`dispatch/demographic.rs:4-16`) to state the truth: **the demographic wire is a
vendored ITS-REST contract (development edition), generated into
`openehr-its::rest::generated::demographic`; the dispatch implements it.** Cite
`demographic.openapi.yaml` + the per-operation YAMLs, not "analogy with the EHR
group". The only surviving extension is `PARTY_RELATIONSHIP` (G-5) — keep its
routes but re-label them explicitly: *no openEHR ITS-REST contract defines
these — our own extension realizing SM `I_PARTY_RELATIONSHIP`
(`i_party_relationship.adoc`); excluded from conformance-profile claims.* Where
the EHR-group envelope (`ETag`/`Location`/`Prefer`/`If-Match`) genuinely
coincides with the spec, keep it — but justify it from the demographic
operation YAML, not by analogy.

### 2.2 Delete semantics (G-2)

Bring `run_party_delete` (`dispatch/demographic.rs:200-216`) to the spec:

- Build `PersonDeleteParams`/`AgentDeleteParams` (the generated struct carrying
  only `uid_based_id`, `demographic.rs:360`), not `AgentGetParams`.
- Treat `uid_based_id` as the `OBJECT_VERSION_ID` = `preceding_version_uid`;
  drop the `If-Match` read for delete (`if_match_of`, `:603`) — the spec places
  the preceding version in the **path**, not the header. (The trait
  `party_delete` signature `service.rs:165` should take the preceding
  `version_uid` positionally, not `if_match: Option<String>`.)
- Map "supplied `uid_based_id` doesn't match the latest version" →
  **`409`** with the latest `version_uid` in `ETag`
  (`responses/409_PERSON_with_uid_based_id.yaml`); "already deleted" →
  **`400`** (`responses/400_already_deleted.yaml`); unknown → `404`.
- `204_version_deleted` on success with `ETag`/`Location` of the deleted
  version (as today).

### 2.3 ITEM_TAG headers + tag `Prefer` (G-3, G-4)

- On `create`/`update`/`get`, parse the incoming `openehr-item-tag` /
  `openehr-version-item-tag` request headers (already in the generated params —
  stop discarding `_p`, `demographic.rs:108`), persist them, and **echo the
  server-set tags** in the corresponding response headers
  (`headers/openehr-item-tag.yaml`, `headers/openehr-version-item-tag.yaml`).
  Extend `set_headers` (`:612`) or add a tag-header emitter.
- In `tags_update` (`:173`), branch on `Prefer`: `return=representation` →
  `200` + the tag list; `return=minimal`/absent → `204` (`204_updated`).

### 2.4 CONTRIBUTION input typing + canonical XML (G-6, G-7)

- Type the `contribution_create` body as `NewContribution` with the
  `UPDATE_AUDIT` relaxation and tolerant `_type` acceptance
  (`UPDATE_AUDIT`/`AUDIT_DETAILS`/omitted) at the wire, or delegate to a
  service-layer check that provably enforces
  `operations/demographic_contribution_create.yaml`; record which layer owns it.
- Route `versioned_party_*` / `contribution_*` / `tags` reads through the
  canonical XML encoder (as the party CRUD already does via `respond_rm`) so
  `Accept: application/xml` is honoured — shared with the F-05-06
  version-family XML work.

### 2.5 Verification

- ECC: re-adjudicate the DemographicApi area against the vendored contract
  (delete `409`/`400_already_deleted`, item-tag headers, tag `204`), keeping
  the DEVELOPMENT-maturity caveat; PARTY_RELATIONSHIP cases stay extension-only.
- Unit/integration: delete-with-mismatched-uid→`409`; already-deleted→`400`;
  item-tag round-trip headers on create/get; `tags_update` `Prefer` matrix;
  canonical-XML reads of versioned_party/contribution.
- Gates: workspace suites green, clippy clean, ECC zero-drift.

---

## 3. Standing PORT NOTEs after the fix (the honest residue)

- **`PARTY_RELATIONSHIP` wire** (routes + `run_relationship`,
  `dispatch/demographic.rs:311-477`): no openEHR ITS-REST contract defines
  these — our own extension realizing SM `I_PARTY_RELATIONSHIP`
  (`i_party_relationship.adoc`); out of conformance-profile scope.
- **Bare-RM party body vs `UV_PARTY` envelope**: the wire accepts a bare RM
  party and wraps it into `UV_PARTY` with a server-default audit
  (`service.rs:132-136`) — a deliberate ITS-REST-style adaptation (the same
  choice the COMPOSITION/EHR_STATUS wire makes); the SM `create_party`/
  `update_party` envelope is `UV_PARTY` (`uv_party.adoc`).
- **DEVELOPMENT maturity**: the Demographic API is `DEVELOPMENT`
  (`demographic.openapi.yaml` `x-status`); conformance claims are advisory, not
  CORE/STANDARD-gated, until the spec matures — recorded, not a defect.
- **`definitions_valid` / `definition_unknown`**: no archetype/template
  validation for parties (tracked in `06-demographic.md`, a service-layer gap,
  not a wire gap).

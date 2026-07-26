# Recipe `ehr_status` — EHR_STATUS synthesis from a create_ehr matrix row

Deterministic `row -> RM fragment` contract (the runner implementation must
match this spec; its digest pins the recipe version).

- Input: one `create_ehr-main` matrix row binding `ehr_status`,
  `is_queryable`, `is_modifiable`, `subject`, `other_details`, `ehr_id`.
- `ehr_status = absent` => emit no EHR_STATUS at all (the class-1.a rows).
- Otherwise emit a canonical-JSON `EHR_STATUS` (RM ehr, EHR_STATUS),
  always carrying `archetype_details` (`ARCHETYPED` with
  `archetype_id = openEHR-EHR-EHR_STATUS.generic.v1`, `rm_version`
  `1.2.0`) — an EHR_STATUS is unconditionally an archetype root
  (RM ehr `ehr_status.adoc` `Is_archetype_root`) and a root without
  ARCHETYPED violates RM common `locatable.adoc` `Archetyped_valid`:
  `is_queryable`/`is_modifiable` verbatim from the row;
  `subject = provided` => a `PARTY_SELF` carrying an external ref with a
  deterministic namespace `cnf` and id `subject-<case id>-<row index>` —
  case-scoped so single-EHR-per-subject SUTs never collide across cases;
  `other_details = provided` => an `ITEM_TREE` with one fixed
  `DV_TEXT` element (`"cnf other_details"`); `absent` => omitted.
- `ehr_id = provided` => a deterministic UUIDv5 over namespace
  `cnf.create_ehr` (itself UUIDv5 of that literal under the URL namespace)
  and the name `<case id>/<row index>` — case-scoped so distinct cases on a
  shared SUT never collide; `absent` => omit (server assigns).
- Seed: none required — the outputs are pure functions of (case, row).

# Catalogue audit — ADMIN chapter

Issue #231 · audited 2026-07-24 · 18 cases · verdicts: 17 ok / 1 DEFECT (binding-level, fixed in this audit) / 0 ambiguities

Chapter context: CNF `master12-func_tc_admin.adoc` is placeholder-only
(verified: every table under all nine SM-operation headings is "Test Case
aaaa/bbbb"), so the 18 cases are SM-derived pairs (positive + negative per
operation). AMB-33 verified precise against the vendored admin OAS: the
DEVELOPMENT-stage Admin API surfaces exactly `DELETE /admin/ehr/{ehr_id}` and
`DELETE /admin/ehr/all` — physical EHR deletion is the ONE wired operation;
the other eight (list_contributions, contribution_count,
versioned_composition_count, composition_version_count,
physical_party_delete, export_ehrs, archive_ehrs, archive_parties) are
unrealized, their 16 cases N/A-with-citation under AMB-33. All nine SM
operations verified to exist with matching names in `i_admin_service.adoc` /
`i_admin_dump_load.adoc` / `i_admin_archive.adoc`.

| case family | verdict | evidence | resolution |
|---|---|---|---|
| physical_ehr_delete-delete_existing | DEFECT (binding — fixed) | The case's ok_empty ground is sound (hard delete produces no VERSION; follow-up get → not_found), but the binding pinned `ok_empty → 204` while the vendored OAS enumerates BOTH 204 (synchronous) and 202 (async acceptance) for `admin_ehr_delete` — the same over-restriction shape as the #265 minimal-return defect: a conformant async server would have been misclassified red | binding now carries `alt_status: [202]` with the OAS citation |
| physical_ehr_delete-delete_non_existing | ok | 404 per `404_unknown_ehr_id`; empty-server ground | none |
| physical_party_delete (2) | ok | SM signature verified; no wire (AMB-33) — N/A with citation; drafts consistent with the outcome vocabulary | none |
| list_contributions / contribution_count / versioned_composition_count / composition_version_count (8, -all + -time_range each) | ok | SM signatures verified (time-range parameters per `i_admin_service.adoc`); commits-any grounds give the counters something to count; AMB-33 tagged | none |
| export_ehrs (2) | ok | SM `i_admin_dump_load.adoc` export_ehrs (location + format parameters); AMB-33 tagged | none |
| archive_ehrs / archive_parties (4) | ok | SM `i_admin_archive.adoc` signatures; selected/unknown pair per operation; AMB-33 tagged | none |

Checks common to the chapter:
- **Ground (dim 1):** master12 placeholder status verified; SM-derived ids follow the stub-chapter posture (AMB-33 carries the realization law).
- **Expectations (dim 2):** each expect maps to the outcome vocabulary consistently with the SM signatures; the one wire-facing family (physical EHR delete) recomputed against the OAS responses — producing the fixed defect above.
- **Fixtures (dim 4):** no corpus fixtures beyond minted EHRs; no shared-SUT collisions.
- **Ambiguity tags (dim 6):** AMB-33 on all 16 unrealized-op cases; the two realized physical_ehr_delete cases correctly carry none.

# Catalogue audit — DEMOGRAPHIC chapter

Issue #231 · audited 2026-07-24 · 24 cases · verdicts: 24 ok / 0 defects / 0 ambiguities
(23 files carry the systemic dangling `REQUIREMENTS.md` comment pointer — the largest single group; swept in the audit-wide pass)

Chapter context: CNF `master10-func_tc_demographic.adoc` is a stub chapter
(AMB-36, verified: every case table is a TBD aaaa/bbbb placeholder) whose
official headings ARE the aaaa/bbbb variants under each operation section —
the catalogue keeps those ids verbatim (`create_party-aaaa`, …), 12 operations
× 2. Every case carries the dual disclosure guard (DEVELOPMENT-stage
Demographic API per the ITS-REST lifecycle + the stub-chapter authoring note).
All 12 `sm_operation` values verified to resolve exactly onto the vendored SM
interfaces: `I_DEMOGRAPHIC_SERVICE` (create_party, create_party_relationship),
`I_PARTY` (get_party, get_party_at_time, get_party_at_version, update_party,
delete_party), `I_PARTY_RELATIONSHIP` (the five relationship counterparts).

| case family | verdict | evidence | resolution |
|---|---|---|---|
| create_party / create_party_relationship (aaaa success, bbbb invalid) | ok | SM `i_demographic_service.adoc` signatures; invalid fixtures (`cnf.demographic.person.invalid`, `.party_relationship.invalid`) exist; retrievability postconditions wired | none |
| get_party / get_party_relationship (aaaa success, bbbb unknown) | ok | SM Meaning + Errors paths; unknown → not_found; `instance_of PERSON`/`PARTY_RELATIONSHIP` on the positive reads | none |
| get_party_at_time / _at_version (+ relationship counterparts) | ok | The formerly-defective literal instants are gone: `version_at_time` derives from the CAPTURED commit window (`${time:after(t0)}`) — the reactive fix the #231 contract cites, verified in place across all four at_time/at_version families | none |
| update_party / update_party_relationship (aaaa success, bbbb stale precondition) | ok | Two-version fixtures; preceding_version_uid as If-Match (AMB-3 pattern); change_type MODIFY asserted per RM change_control | none |
| delete_party / delete_party_relationship (aaaa success, bbbb unknown) | ok | SM delete semantics; unknown version id → not_found | none |

Checks common to the chapter:
- **Ground (dim 1):** AMB-36 verified (stub chapter; SM-derived flows with disclosure guards); official aaaa/bbbb ids kept verbatim.
- **Expectations (dim 2):** recomputed from the SM operation Meanings + .Errors; outcome kinds consistent.
- **Fixtures (dim 4):** all six `cnf.demographic.*` keys verified in `corpus/MANIFEST.yaml`; `server: empty` per case — no shared-SUT collisions.
- **Captures (dim 5):** versioned_object_uid / version_uid / commit_time chains all bind before use; the at_time instants are capture-relative.
- **Ambiguity tags (dim 6):** AMB-36 governs the chapter (carried via guards; the register entry names the whole chapter rather than per-case divergences).

Id-policy observation (no action now): the two stub chapters diverge —
demographic keeps the official aaaa/bbbb placeholder ids verbatim while admin
(master12, equally aaaa/bbbb) authored SM-derived ids. Both were deliberate
wave decisions and both flows get replaced when upstream authors the chapters
(AMB-36/AMB-33 handling); harmonizing ids now would churn the committed
baseline for no verdict effect. Candidate for the next schedule-release
adoption instead.

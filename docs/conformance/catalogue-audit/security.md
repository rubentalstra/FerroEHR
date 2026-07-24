# Catalogue audit — SECURITY chapter

Issue #231 · audited 2026-07-24 · 9 cases · verdicts: 4 ok / 5 DEFECT (all five citation-precision, fixed in this audit) / 0 ambiguities

Chapter context: the SEC-BASIC family is an **authored proposal** (the 2017
schedule's Security BASIC rung; design record §8.15) — the official CNF
profiles book `docs/specs/openehr/CNF/docs/profiles/master03-profiles.adoc`
§Non-Functional names only **Signing** (STANDARD) and **Anonymous EHRs**
(CORE + STANDARD) under Security & Privacy (verified in the vendored table).
The capability matrix (`vocab/capability_matrix.yaml`) already carries the
correct proposal-flagged sources for the four proposal capabilities; the case
YAMLs had drifted from that form.

| case | verdict | evidence | resolution |
|---|---|---|---|
| SEC-ANONYMOUS_EHRS-anonymous_lifecycle | DEFECT (guard citation) | RM `master04-ehr_package.adoc`:44+219 verified verbatim ("the use of `PARTY_SELF` allows completely anonymous EHRs"); Anonymous EHRs IS in the official profiles table so the spec_ref stands; but the guard cited the profiles chapter for the SEC-BASIC tier, which that chapter does not define | guard re-cited to the proposal form — FIXED |
| SEC-AUDIT_ACCOUNTABILITY-server_set_commit_audit | DEFECT (spec_ref + guard citation) | Grounds verified: RM `master06-change_control_package.adoc`:88 (commit captures time/place/committer), :90 (time_committed "should … be computed on the server"); ITS-REST `Requests_and_responses.md`:81 — clients MAY supply `change_type`, `description`, `committer`, `system_id`; time_committed is NOT in the sanctioned list, so a 1990 client value cannot be the recorded committal time. But the fourth spec_ref cited the profiles chapter for "Audit accountability", a capability name absent from the official table | spec_ref + guard re-cited to the proposal form — FIXED |
| SEC-AUTHENTICATED_ACCESS-unauthenticated_sweep | DEFECT (spec_ref + guard citation) | 401 ground verified (ITS-REST `Requests_and_responses.md`:224 + :34); `selectors.yaml` universal_outcomes maps unauthenticated → 401 route-table-wide with the same source; the profiles citation named a capability absent from the official table | spec_ref + guard re-cited — FIXED |
| SEC-AUTHORIZATION_SEPARATION-readonly_write_denied | DEFECT (spec_ref + guard citation) | 403 ground verified (`Requests_and_responses.md`:225); universal forbidden → 403 in selectors; same profiles-citation defect | spec_ref + guard re-cited — FIXED |
| SEC-EHR_DEMOGRAPHIC_SEPARATION-status_subject_opaque | DEFECT (spec_ref + guard citation) | Grounds verified: RM `master04-ehr_package.adoc`:44 (subject is PARTY_SELF, anonymous or opaque ref); ITS-REST `ehr-codegen.openapi.yaml` POST /ehr — default EHR_STATUS is `subject: a PARTY_SELF object` (no identity attached → no external_ref), so the absent-external_ref assertion is derivable; same profiles-citation defect | spec_ref + guard re-cited — FIXED |
| SIG-VERSION-signature_present | ok | RM `master06-change_control_package.adoc` §Digital Signature (:96–:104) verified: committal-time signature over the canonical serialisation, radix-64, openPGP RFC 4880; Signing IS in the official profiles table (STANDARD) so both citations stand; N/A guard correct for a conditional capability | none |
| SIG-VERSION-verifiable | ok | Same §Digital Signature ground ("digital signature … using the user's private key", openPGP); the ixit-key-material guard correctly flags the non-wire input; ECC digest wire format correctly excluded as an engine extension | none |
| SIG-VERSION-across_version_kinds | ok | §Digital Signature + Version Lifecycle grounds; `openehr::523\|deleted\|` verified against the terminology asset (`crates/openehr-term/assets/en/openehr_terminology.xml`: concept id 523 rubric "deleted"); AMB-3 (If-Match) tag carried and covers the update step | none |
| SIG-VERSION-client_supplied_verbatim | ok | RM :104 verified verbatim ("the `IMPORTED_VERSION` instance will carry its own signature which signifies the act of importing") — the verbatim-storage ground; postcondition byte-equality against the fixture's own signature; `cnf.security.signed_version` exists in the corpus MANIFEST | none |

Checks common to the chapter:
- **Ground (dim 1):** no official CNF security test chapter exists; the SEC-BASIC family follows the §8.15 authored-proposal posture (2017 lineage), the SIG family sits under the official Signing capability. The five fixed citations now carry the proposal flag exactly like the performance chapter and the capability matrix.
- **Expectations (dim 2):** every expect/assert recomputed from RM change_control, RM ehr, ITS-REST overview status codes/audit-details, and the wire default-EHR_STATUS semantics; no observed-behaviour echoes.
- **Fixtures (dim 4):** `cnf.opt.minimal_event`, `cnf.composition.minimal_event.v1/.v2`, `cnf.security.signed_version` all present in `corpus/MANIFEST.yaml`; `server: any` cases mint their own EHRs — no shared-SUT collisions.
- **Captures (dim 5):** capture→parameter flows checked (ehr_id/version_uid/contribution_uid chains all bind before use; the update step correctly feeds `preceding_version_uid` per AMB-3's If-Match handling).
- **Ambiguity tags (dim 6):** AMB-3 read and covers the tagged divergence; no other tags carried.

Post-fix machine floor: `cnf-runner validate` — 393 cases, 88 bindings, 0 findings.

---
name: rm-common-master04-design-principles
description: Verified findings for RM common master04 §4.2 (Design Principles) — the ATTESTATION-as-commit_audit silent downgrade, the description DV_TEXT collapse, the PARTY_SELF scheme-3 coverage hole, and the internal-doc citations shipped in SQL COMMENTs
metadata:
  type: feedback
---

Verified 2026-08-02 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master04-generic_package.adoc` lines 11–88
(§4.2 Design Principles; §4.3 Class Descriptions starts line 89).

**CONFORMANT, do not re-check:** PARTY_SELF/PARTY_IDENTIFIED/PARTY_RELATED
roster + the EHR_STATUS.subject monomorphism guard
(`service/ehr/validation.rs:500-552`, PARTY_REF.type enforced by the generated
`PartyRef` required field); scheme 1 (anonymous) + scheme 2 (external_ref once
in EHR_STATUS) both work and both have CNF cases (SEC-ANONYMOUS_EHRS,
SEC-EHR_DEMOGRAPHIC_SEPARATION, create_ehr subject family);
PARTICIPATION.Function_valid/Mode_valid ARE enforced at the ITS dispatcher
(`openehr-its/src/rm_terminology.rs:284-296`); audit-per-write is in the commit
CTEs (`storage/version_repo/commit.rs`); `REVISION_HISTORY` is now typed through
the codec (`versioning/wire.rs:87-109`) and carries commit audit + attestations,
with attestations surfacing as `_type: ATTESTATION`.

**VERIFIED DEFECTS (§4.2):**
- **ATTESTATION as `VERSION.commit_audit` is unrepresentable and silently
  downgraded.** master04 §Attestation: "the most common scenario will be that a
  Composition Version will be committed with a `_commit_audit_` of type
  `ATTESTATION`" (corroborated master06 line 119). The `audit` table has 5 fixed
  columns (`migrations/ehr/0001_baseline.sql:120-145`); `parse_audit`
  (`versioning/contribution.rs:698-723`) and the EHR-Extract import
  (`service/message/import.rs:525-560`, comment falsely says "preserved
  verbatim") read only 4 fields; the read path always builds
  `AuditDetails::AuditDetails` (`versioning/audit.rs:audit_details_typed`).
- **`AUDIT_DETAILS.description` collapses to `value`** — DV_CODED_TEXT / any
  formatting/mappings/language is dropped (`audit.description text` +
  `/description/value` + `dv_text()` rebuild). Class table types it DV_TEXT 0..1.
- **PARTY_SELF scheme 3 has ZERO coverage** — verified: all 29
  `corpus/fixtures/composition/*.json` carry `/content[0]/subject` with NO
  external_ref; nothing in the tree exercises "external_ref set in every
  instance of PARTY_SELF".
- **`ATTESTATION.items` scope conflict is unregistered**: §4.2 "the list must
  contain a set of paths to items within the item to which the attestation is
  attached" vs the class table "may include fine-grained items which have been
  attested in some other system" (+ §4.2's own later "nothing stopping it").
  Class table governs; no register entry, no `// NOTE:`.
- **`review doc 02/03 req N.N` internal-doc citations**: 31 in
  `migrations/ehr/0001_baseline.sql`, 4 in `migrations/ext/…`, of which **13 are
  inside `COMMENT ON` strings** (shipped into the live PG catalog). Hard-rule
  violation.
- Dangling headings: `master06 §AUDIT_DETAILS`, `master06 §CONTRIBUTION`,
  `master06 §Change Control` (real headings: §Committal and Audits,
  §Contributions, §Attestation). `service/version_update.rs:30-33` misdescribes
  ITS-REST `UDvText` as "plain string or DV_TEXT" — it is
  `oneOf[DV_TEXT, DV_CODED_TEXT]`, no string branch.
- The two-object-pattern CNF case
  (`schedule/contribution/…attestation_pending_then_final.yaml`) models BOTH
  objects as `attestations[]` members; master04's FIRST object is the version's
  `commit_audit`. Honest about testing RETENTION, but the spec's shape is
  uncovered.

**Spec-internal staleness (upstream-report candidates):** §4.2 §Revision History
still says REVISION_HISTORY serves `AUTHORED_RESOURCE`, but BASE removed the
property (`BASE/docs/resource/master00-amendment_record.adoc:71`) and AM removed
it from ADL2/AOM2 (SPECAM-61). Our ADL-1.4 side keeps it as an ODIN section
(`openehr-adl/src/source.rs:124`) — the right posture.

**Already registered, do NOT re-find:** AMB-90 covers the ITS-REST `system_id`
"will be validated" silence + the master06 copy-down rule. Client-supplied
`system_id` IS explicitly permitted (ITS-REST overview
`Requests_and_responses.md:81,94`).

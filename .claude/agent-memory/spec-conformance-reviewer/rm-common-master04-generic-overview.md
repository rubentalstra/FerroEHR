---
name: rm-common-master04-generic-overview
description: Verified findings for RM common master04 §4.1 (generic package Overview) — roster is complete/conformant; the real defects are stale identity literals, un-vendored UML diagrams, archie-as-oracle citations, and dangling master04 section refs
metadata:
  type: feedback
---

Verified 2026-08-02 against RM 1.2.0
`docs/specs/openehr/RM/docs/common/master04-generic_package.adoc` lines 3–10
(§4.1 = ONE paragraph + a diagram; §4.2 Design Principles starts line 11).

**§4.1 yields almost no enforceable requirement** — its only structural claim
delegates to `image::{uml_diagrams_uri}/RM-common.generic.svg`. **FIXED in
#1633 (2026-08-02): the 33 UML class-diagram SVGs ARE now vendored** at
`docs/specs/openehr/RM/docs/UML/diagrams/` by `scripts/vendor/spec-docs.sh`.
STILL un-vendored: the `{diagrams_uri}` PNGs (172 refs repo-wide) and
`{images_uri}` (30) — the script copies `{uml_diagrams_uri}` SVGs only, and
each PROVENANCE.md discloses the exclusion honestly. Corroborate any
roster claim from the BMM package + the §4.3 include list anyway.

**Roster is CONFORMANT and complete (do not re-check):** all 9 classes emitted
at `crates/openehr-rm/src/common/generic/` matching the BMM package exactly;
prelude exports all 9 (`prelude.rs:29-39`); RM attribute model has all 9
(`crates/openehr-rm/src/model/data.rs`); JSON + XML codecs for all 9;
`PARTY_PROXY` slot decode requires `_type` and rejects both absent and
unknown (`json_codec/generated/impls.rs:6931-6956`).

**The committer PARTY_PROXY IS invariant-checked** (I wrongly hypothesized a
gap): `app/ferroehr/src/versioning/audit.rs:283 validate_commit_audit` →
`validate_committer` enforces Basic_validity, Name_valid, and PARTY_RELATED
Relationship_valid on the audit committer, which the COMPOSITION-only RM pass
never sees. Same file enforces `AUDIT_DETAILS.Change_type_valid` via the
terminology bundle. `ATTESTATION` Reason_valid + Items_valid ARE enforced at
`app/ferroehr/src/versioning/attestation.rs:169-200`.

**VERIFIED DEFECTS:**
- The system-identity committer is still literally `"EHRbase"`
  (`app/ferroehr/src/service/ehr/meta.rs:254`,
  `app/ferroehr/src/service/version_update.rs:178`) while the REST adapter's
  own fallback says `"ferroehr.local"`
  (`app/ferroehr-rest/src/api/ehr/mod.rs:154`) — two spellings, one stale.
  Reachable whenever no principal is authenticated (auth disabled / internal
  writes) via `CommitEnv::default_committer` (`service/commit_env.rs:31`).
- The invariant register row `ATTESTATION.Items_valid` = "Unrealized" with
  reason "the reference implementation marks this invariant `ignored`" is
  BOTH factually wrong (attestation.rs:192 realizes it) and archie-as-oracle.
  Source: `tools/openehr-codegen/src/plan/overrides.rs:1892-1899`.
- All four generic-package `*_impl.rs` module headers cite **archie** as the
  authority ("Mirrors archie `PartyIdentified`", "archie's own … are both
  `ignored`") — prior art quoted as oracle.
- Dangling citations into this chapter: `RM common master04 §Party Proxies`
  (audit.rs:280,295,337 — no such heading), `RM common
  master04-revision_history.adoc` (ferroehr-rest demographic/mod.rs:176,429 —
  no such FILE), `master04-generic_package.adoc §PARTY_PROXY`
  (its flat/build.rs:1165 — a §4.3 rendered class heading, weak form).
- ATTESTATION has ZERO CNF catalogue cases and NO instance fixture anywhere in
  the repo (only `app/ferroehr/tests/it/service_contribution.rs` covers it).
  REVISION_HISTORY has 6 CNF cases; PARTY_RELATED has corpus fixtures.
- `REVISION_HISTORY`/`AUDIT_DETAILS`/`ATTESTATION` wire bodies are hand-built
  `serde_json::json!` literals (`versioning/wire.rs:88`, `audit.rs:242`,
  `attestation.rs:213`), not openehr-rm types through the ToJson codec — the
  `// NOTE:` calls it "spec-silent" but the rule it deviates from is the repo
  never-re-serialize hard rule, not a spec question.

**Chapter heading list (for citation checking):** Overview · Design Principles ·
Referring to Demographic Entities · PARTY_SELF and Referring to the Patient
from the EHR · Participation · Audit Information · Audit Details · Revision
History · Attestation · Class Descriptions. Nothing else exists.

**Spec-internal slip for #987 (§4.3):** REVISION_HISTORY's class purpose says
"most-recent-first order" while its `items` attribute says "most-recent-last";
`versioning/wire.rs` emits oldest-first (= most-recent-last).

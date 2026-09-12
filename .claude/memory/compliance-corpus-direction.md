---
name: compliance-corpus-direction
description: Owner direction 2026-09-12 — FerroEHR aims to be the first source-available CDR whose regulatory posture (EU GDPR/EHDS, NL, DE, CH, more later) is checkable against the vendored regulation texts by a dedicated compliance-researcher agent, distinct from cnf-triage/spec-researcher
metadata:
  type: project
---

The owner's stated aim (2026-09-12): be the first source-available CDR that
supports every applicable regulation for the Netherlands, Germany,
Switzerland and the EU (GDPR, EHDS, NIS2, CRA, MDR, the EDPB pseudonymisation
guidelines; UAVG, Wabvpz, Wgbo, the logging decree; BDSG, SGB V ePA
provisions, GDNG, § 203 StGB; FADP/DSV, EPDG/EPDV), with every claim checkable
against the text itself. Programme #3290 (P1, v4.3.0) with one sub-issue per
jurisdiction plus the agent/skill/guard (#3291–#3295); `docs/law/<jur>/<act>/`
vendored by `scripts/vendor/law-<jur>.sh`, provenance-stamped, REUSE-declared.

**Why:** the compliance pages cited regulations by URL and the review read
them from memory; the openEHR specs are vendored and every spec review reads
the vendored text, and the owner wants regulations held to the same bar.

**How to apply:** the regulations agent (`compliance-researcher`) and the
`/compliance-audit` skill are SEPARATE from the CNF/openEHR instruments
(`cnf-triage`, `spec-researcher`, `/spec-audit`) — never fold one into the
other. Published wording stays "supports/designed to support", never
"compliant"/"certified" (`compliance/index.md` wording policy). Paywalled
standards (NEN 7510 family, ISO) are never vendored; their PROVENANCE says so.
See [[en-route-findings-always-filed]].

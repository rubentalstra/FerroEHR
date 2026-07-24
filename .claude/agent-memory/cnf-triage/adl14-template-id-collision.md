---
name: adl14-template-id-collision
description: Catalogue defect — master08 contribution provisioning reuses template_ids of the server:empty master04 ADL14 upload/validate cases
metadata:
  type: project
---

Confirmed 2026-07-24 (branch feat/cnf-catalogue-audit). The 2 red
I_DEFINITION_ADL14.{upload_opt,validate_opt}-valid_opt rows (expected
`created`, observed `already_exists`) are a CATALOGUE test-isolation defect,
NOT a SUT idempotency bug and NOT (primarily) a runner bug.

The branch-new master08 case
`I_EHR_CONTRIBUTION.commit_contribution-valid_composition`
(`requires.templates`) provisions `cnf.opt.time_series` (template_id
`time_series.en.v1`) and `cnf.opt.obs_admin` (`obs_admin.en.v1`) — the SAME
manifest aliases the master04 `server: empty` DEFINITION_ADL14 cases upload
as their created-path fixtures (upload_opt row 2 = cnf.opt.time_series;
validate_opt row 0 = cnf.opt.obs_admin). The runner shares ONE freshly
composed SUT per run with no per-case template wipe (`reset_per_row` only
re-provisions the case's own `requires`; `ServerState::Empty` gets no reset,
only `Exclusive` is run-first) — documented design, unchanged from baseline.
So the contribution case loads the ids first, and the DEFINITION cases'
`requires: server: empty` precondition is broken → `already_exists`.

SUT is spec-correct: CNF master04 §upload_opt-valid_opt_twice_conflict
(master04-func_tc_definition_adl.adoc L161): "uploading an OPT with the same
`template_id` twice will make the second upload fail (conflict)" (AMB-4).

Established catalogue convention this violates: MANIFEST.yaml L275
(`cnf.opt.versioned.v3`) — upload-flow cases mint a FRESH template_id "so the
upload-flow case owns a fresh pair". Fix: give the master08 contribution
provisioning distinct template_ids (new OPT + re-point the composition
fixtures' archetype_details.template_id), decoupling it from the master04
server:empty exemplars. Secondary hardening (register-worthy, NOT the
branch-introduced cause): the runner recognizes `ServerState::Empty` but
neither resets nor gates on it.

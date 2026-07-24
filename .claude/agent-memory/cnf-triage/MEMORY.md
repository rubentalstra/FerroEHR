# CNF triage memory index

- [Results/wire evidence locations](results-evidence-locations.md) — where the observation, catalogue, and spec live
- [SUT reproduction setup](sut-reproduction-setup.md) — composed SUT curl access + credentials
- [cnf.opt.vitals RESOLVED](cnf-opt-vitals-alias-mismatch.md) — was a catalogue defect; now a real body_temperature OBSERVATION (template_id cnf.vitals) — old attribution dead
- [Interval + coded-text spec overreach](interval-and-coded-text-spec-overreach.md) — 2 confirmed app defects: DV_INTERVAL bound-presence + DV_CODED_TEXT value==rubric; both invent invariants the closed spec sets lack
- [Runner driver gaps (confirmed)](runner-driver-gaps.md) — option-gating, header-mutation variants, ctx/ equivalence normalization
- [content_synth HISTORY events existence](content-synth-history-events-existence.md) — RUNNER defect: builders hardcode events existence 1..1; zero-event+summary rows (any/opt cardinality) wrongly fail; SUT spec-correct
- [ADL14 template_id collision](adl14-template-id-collision.md) — CATALOGUE defect: master08 contribution provisioning reuses master04 server:empty upload/validate template_ids (time_series/obs_admin); SUT already_exists is spec-correct
- [Upstream EHRbase Java non-conformance (#266)](upstream-ehrbase-java-nonconformance.md) — EHRbase 2.34.0 400s the spec-standard QUOTED If-Match (composition/ehr_status update classes) + 404s the STABLE item-tag surface; our driver/pack spec-valid, outcome (b), record stands

# Recipe `bp_series` — the generated blood-pressure query corpus

Deterministic generated-set contract for `cnf.set.bp-10`.

- 10 canonical-JSON COMPOSITIONs against the blood-pressure template
  (`cnf.opt.blood_pressure`, openEHR-EHR-OBSERVATION.blood_pressure.v2).
- Composition k (k = 0..9): systolic magnitude = 100 + 10k mmHg
  (100..190), diastolic = 60 + 5k mmHg; event time =
  2026-01-01T00:00:00Z + k hours; all other fields fixed.
- Committed in index order into the case's EHR.
- Views over the set are declarative projections evaluated on these values
  (e.g. `magnitude_ge_140_by_uid`: the committed uids of the compositions
  with systolic magnitude >= 140, ordered by uid ascending).

Every node whose `archetype_node_id` is an archetype id carries a matching
`archetype_details` (RM common LOCATABLE/ENTRY `Is_archetype_root`), and the
root carries `archetype_node_id` = its archetype id.

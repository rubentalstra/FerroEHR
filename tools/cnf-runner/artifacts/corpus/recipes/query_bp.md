# Recipe `query_bp` — the generated blood-pressure query corpus

Deterministic generated-set contract for `cnf.set.query-bp-10` (the QUERY
chapter's loaded-db corpus).

- 10 canonical-JSON COMPOSITIONs against the blood-pressure template
  (`cnf.opt.blood_pressure`, openEHR-EHR-OBSERVATION.blood_pressure.v2),
  category event (openehr::433).
- Composition k (k = 0..9): systolic magnitude = 100 + 10k mmHg (100..190),
  diastolic = 60 + 5k mmHg; event time = 2026-01-01T00:00:00Z + k hours; all
  other fields fixed; name/value = "Blood pressure".
- Committed in index order into the case's EHR.
- Views over the set are declarative projections evaluated on these values
  (runner-independent):
  - `all_uids_asc`: the committed uids of all 10 compositions, ordered by uid
    ascending.
  - `systolic_ge_140_uids_asc`: the committed uids of the compositions whose
    systolic magnitude >= 140 (k = 4..9), ordered by uid ascending.
  - `top3_systolic_desc_uids`: the committed uids of the 3 compositions with the
    highest systolic magnitude (190, 180, 170), ordered by systolic magnitude
    descending.

Every node whose `archetype_node_id` is an archetype id carries a matching
`archetype_details` (RM common LOCATABLE/ENTRY `Is_archetype_root`), and the
root carries `archetype_node_id` = its archetype id.

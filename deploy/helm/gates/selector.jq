# Selector immutability + disjointness, over the array of rendered documents.
#
# Emits one line per violation and nothing when clean, so the caller decides the
# exit status from the output. Ported from the Python this replaced; the
# assertions are unchanged.

["app.kubernetes.io/instance", "app.kubernetes.io/name"] as $expect
| . as $docs
| [ $docs[]
    | select(.kind == "Deployment")
    | { name:   .metadata.name,
        labels: (.spec.template.metadata.labels // {}),
        keys:   ((.spec.selector.matchLabels // {}) | keys) } ] as $deps
| (
    [ $deps[]
      | select(.keys != $expect)
      | "\(.name): selector.matchLabels is \(.keys), expected \($expect) — this field is IMMUTABLE; changing it breaks helm upgrade on every existing release" ]
  )
  +
  (
    # A Service/PDB selector is a SUBSET match, so a selector naming only the
    # labels two workloads share silently selects both. Key sets alone cannot
    # catch this: after the CDR/viewer fix both Deployments have the SAME
    # selector keys and differ only in the name's value.
    [ $docs[]
      | select(.kind == "Service" or .kind == "PodDisruptionBudget")
      | . as $d
      | ( ($d.spec.selector // {})
          | if (type == "object" and has("matchLabels")) then .matchLabels else . end ) as $sel
      | select(($sel | type) == "object" and ($sel | length) > 0)
      | ( [ $deps[]
            | . as $dep
            | select([ $sel | to_entries[] | ($dep.labels[.key] == .value) ] | all)
            | $dep.name ] ) as $hit
      | select(($hit | length) > 1)
      | "\($d.kind)/\($d.metadata.name): selector \($sel | tostring) matches \($hit) — a subset match selects BOTH workloads; give each its own app.kubernetes.io/name rather than a shared name plus a component" ]
  )
| .[]

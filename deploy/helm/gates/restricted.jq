# Pod Security `restricted` compliance, plus the availability and isolation
# properties that live in the same walk, over the array of rendered documents.
#
# Emits one line per violation and nothing when clean. Ported from the Python
# this replaced; every assertion is unchanged.

[ "configMap", "csi", "downwardAPI", "emptyDir", "ephemeral",
  "persistentVolumeClaim", "projected", "secret" ] as $allowed_volumes
| . as $docs
| [ $docs[]
    | select(.kind == "Deployment" or .kind == "StatefulSet"
             or .kind == "DaemonSet" or .kind == "Job")
    | select(.spec.template != null)
    | { kind: .kind, name: .metadata.name,
        replicas: .spec.replicas,
        pod: (.spec.template.spec // {}) } ] as $workloads
| ( [ $workloads[]
      | . as $w
      | ($w.pod.securityContext // {}) as $sc
      | (
          ( if ($sc.runAsNonRoot != true)
            then ["\($w.kind)/\($w.name): pod runAsNonRoot must be true"] else [] end )
        + ( if ((($sc.seccompProfile // {}).type) as $t
                | $t != "RuntimeDefault" and $t != "Localhost")
            then ["\($w.kind)/\($w.name): pod seccompProfile.type must be RuntimeDefault or Localhost"] else [] end )
        # The kubelet's per-Service link variables land in the reserved
        # FERROEHR_ namespace and the strict env sweep then refuses to boot, so
        # losing this makes every install crash-loop.
        + ( if ($w.pod.enableServiceLinks != false)
            then ["\($w.kind)/\($w.name): enableServiceLinks must be false"] else [] end )
        + ( [ ($w.pod.volumes // [])[]
              | . as $v
              | ($v | keys | map(select(. != "name")))[]
              | select(. as $t | ($allowed_volumes | index($t)) == null)
              | "\($w.kind)/\($w.name): volume \($v.name) type \(.) is outside the restricted set" ] )
        + ( [ (($w.pod.containers // []) + ($w.pod.initContainers // []))[]
              | . as $c
              | ($c.securityContext // {}) as $csc
              | (
                  ( if ($csc.allowPrivilegeEscalation != false)
                    then ["\($w.kind)/\($w.name)/\($c.name): allowPrivilegeEscalation must be false"] else [] end )
                + ( if ($csc.runAsNonRoot != true and $sc.runAsNonRoot != true)
                    then ["\($w.kind)/\($w.name)/\($c.name): runAsNonRoot must be true"] else [] end )
                + ( if ($csc.readOnlyRootFilesystem != true)
                    then ["\($w.kind)/\($w.name)/\($c.name): readOnlyRootFilesystem must be true"] else [] end )
                + ( if ((($csc.capabilities // {}).drop // []) | index("ALL")) == null
                    then ["\($w.kind)/\($w.name)/\($c.name): capabilities.drop must include ALL"] else [] end )
                + ( if ((($csc.capabilities // {}).add // []) | length) > 0
                    then ["\($w.kind)/\($w.name)/\($c.name): capabilities.add must be empty, got \((($csc.capabilities // {}).add))"] else [] end )
                )[] ] )
        )[] ] )
  +
  # Availability, not security: a Deployment asking for more than one replica
  # with nothing telling the scheduler to spread them can put every replica on
  # one node. An ABSENT `replicas` counts as multi-replica — the chart omits the
  # field when autoscaling is on, so reading it as the API default of 1 would
  # exempt precisely the elastic workload that most needs spreading.
  ( [ $workloads[]
      | select(.kind == "Deployment")
      | select(.replicas == null or .replicas > 1)
      | select((.pod.topologySpreadConstraints // []) == [] and (.pod.affinity // {}) == {})
      | "Deployment/\(.name): multi-replica with neither topologySpreadConstraints nor affinity — every replica may land on one node" ] )
  +
  ( if ($workloads | length) == 0
    then ["no pod-bearing workload found — the gate would pass vacuously"] else [] end )
  +
  ( if ([ $docs[] | select(.kind == "NetworkPolicy") ] | length) == 0
    then ["no NetworkPolicy in the render"] else [] end )
  +
  # Pod-level ISOLATION, recorded per workload rather than asserted per workload.
  # These are not restricted-profile controls, so the property that matters is
  # not "every pod sets X" but that every pod of one release AGREES. A release
  # whose console shares the host user namespace while its server does not has a
  # posture nobody can state.
  ( [ $workloads[]
      | { k: "\(.kind)/\(.name)",
          v: [ (if .pod.hostUsers == null then true else .pod.hostUsers end),
               ((.pod.securityContext // {}).supplementalGroupsPolicy) ] } ] as $iso
    | if (($iso | map(.v) | unique | length) > 1)
      then ["pod isolation differs across workloads of one release: "
            + (($iso | sort_by(.k) | map("\(.k) hostUsers=\(.v[0]) supplementalGroupsPolicy=\(.v[1])")) | join(", "))]
      else [] end )
| .[]

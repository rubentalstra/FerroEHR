#!/usr/bin/env bash
#
# Validate the ferroehr Helm chart:
#   1. helm lint      — default + all-features value sets (must be clean)
#   2. helm template  — render both; assert the output is valid multi-doc YAML
#   3. security gate   — assert the Kubernetes Pod Security Standards
#                        "Restricted" fields are pinned in the rendered
#                        Deployment (runAsNonRoot, seccompProfile
#                        RuntimeDefault, drop ALL caps,
#                        allowPrivilegeEscalation:false), plus our
#                        readOnlyRootFilesystem hardening
#                        https://kubernetes.io/docs/concepts/security/pod-security-standards/
#   4. golden render   — compare against deploy/helm/golden/ (or --update it),
#                        under the helm version pinned in deploy/helm/.tool-versions
#                        (`helm template` output is NOT byte-stable across helm
#                        releases, so the goldens are only reproducible on the pin)
#   5. kubeconform     — schema-validate the manifests IF kubeconform is on PATH
#                        (optional; skipped with a note when absent/offline)
#   6. values schema   — assert values.schema.json REFUSES what it claims to
#                        (typos, wrong types, bad enums, out-of-range ports) and
#                        still ACCEPTS what must stay open (the server's own
#                        config vocabulary, a parent chart's `global`)
#
# What it does NOT check is printed at the end of every run, and is not a
# footnote: every one of those properties has failed while this script was green
# (#2159 — all three overlays rendered, linted and matched their goldens while
# producing configurations the server refuses to start on). A green run here
# means the MANIFESTS are well-formed, hardened and unchanged. It says nothing
# about whether the deployment works.
#
# Usage:
#   deploy/helm/validate.sh            # validate (fails on lint/render/golden drift)
#   deploy/helm/validate.sh --update   # regenerate the golden renders, then validate
#
# No cluster and no network are required for steps 1–4.
# The CI gate is the `helm-golden` job in .github/workflows/ci.yml.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHART_DIR="${SCRIPT_DIR}/ferroehr"
CI_DIR="${SCRIPT_DIR}/ci"
GOLDEN_DIR="${SCRIPT_DIR}/golden"
RELEASE_NAME="ferroehr"
NAMESPACE="ferroehr"

UPDATE=0
[[ "${1:-}" == "--update" ]] && UPDATE=1

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

command -v helm >/dev/null 2>&1 || { red "helm not found on PATH"; exit 1; }
bold "helm: $(helm version --short)"

# The golden renders are byte-exact output of ONE helm version (see
# .tool-versions). A skewed helm produces a whitespace-only diff that reads as
# chart drift, so the skew is reported as itself instead.
PIN_FILE="${SCRIPT_DIR}/.tool-versions"
PINNED_HELM="$(sed -nE 's/^helm[[:space:]]+v?([0-9][^[:space:]]*)[[:space:]]*$/\1/p' "$PIN_FILE" | head -1)"
[[ -n "$PINNED_HELM" ]] || { red "no helm version declared in ${PIN_FILE}"; exit 1; }
ACTUAL_HELM="$(helm version --short | sed -E 's/^v?([0-9][^+]*).*$/\1/')"
GOLDEN_SKEW=0
if [[ "$ACTUAL_HELM" != "$PINNED_HELM" ]]; then
  GOLDEN_SKEW=1
  red "helm version skew: pinned ${PINNED_HELM} (${PIN_FILE}), running ${ACTUAL_HELM}"
fi

# The value sets to validate: <label>:<values-file>
declare -a CASES=(
  "default:${CI_DIR}/default-values.yaml"
  "all-features:${CI_DIR}/all-features-values.yaml"
  "basic-auth:${CI_DIR}/basic-auth-values.yaml"
)

# ── Rendered-manifest structure check (awk; there is no Python in this repo) ───
# WELL-FORMEDNESS is not checked here because it is already checked twice, by
# tools that must be present anyway — verified by feeding a deliberately broken
# template through both: `helm template` exits 1, and `helm lint` reports
# "unable to parse YAML: error converting YAML to JSON". Both run above.
#
# What those do NOT catch is a document that is valid YAML but not a Kubernetes
# object: a template rendering a bare map produces only a helm-lint WARNING and a
# successful `helm template` (verified the same way). So that is what this checks,
# and it is the coverage the previous PyYAML implementation uniquely added — kept
# rather than narrowed: every rendered document must carry a top-level
# `apiVersion` and `kind`, and at least one document must exist.
yaml_ok() {
  local file="$1"
  awk '
    function flush() {
      if (has_content) {
        docs++
        if (!has_api || !has_kind) {
          printf "  document %d (ending line %d) is missing %s%s\n", docs, NR,
                 (has_api ? "" : "apiVersion "), (has_kind ? "" : "kind") > "/dev/stderr"
          bad++
        }
      }
      has_content = 0; has_api = 0; has_kind = 0
    }
    /^---[[:space:]]*$/ { flush(); next }
    # A top-level key has no leading whitespace; comments and blanks are neither
    # content nor keys.
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*#/ { next }
    { has_content = 1 }
    /^apiVersion:/ { has_api = 1 }
    /^kind:/       { has_kind = 1 }
    END {
      flush()
      if (docs == 0) { print "  no YAML documents rendered" > "/dev/stderr"; exit 1 }
      if (bad > 0)   { printf "  %d document(s) are not Kubernetes objects\n", bad > "/dev/stderr"; exit 1 }
      printf "  %d rendered document(s), each with apiVersion + kind\n", docs
    }
  ' "$file"
}


# ── Selector immutability: the one field that breaks `helm upgrade` ──────────
# A Deployment's spec.selector.matchLabels is IMMUTABLE
# (https://kubernetes.io/docs/concepts/workloads/controllers/deployment/), so a
# change here is not a diff to review — it is an upgrade that fails on every
# existing release with an error most operators read as a chart bug. The obvious
# way to separate the console's pods from the server's is to add `component` to
# the shared selectorLabels helper, which is exactly the change this forbids.
#
# Pinned as an explicit expectation rather than left to the golden diff: a
# golden churns for a dozen innocent reasons and this field must never move
# quietly among them.
assert_selector_stable() {
  local file="$1"
  python3 - "$file" <<'PYSEL' || { red "  selector immutability gate FAILED for ${file}"; exit 1; }
import sys, yaml
EXPECTED = {"ferroehr": {"app.kubernetes.io/name", "app.kubernetes.io/instance"},
            "admin-ui": {"app.kubernetes.io/name", "app.kubernetes.io/instance",
                         "app.kubernetes.io/component"}}
bad = []
for d in yaml.safe_load_all(open(sys.argv[1])):
    if not d or d.get("kind") != "Deployment":
        continue
    name = d["metadata"]["name"]
    keys = set((d["spec"]["selector"]["matchLabels"] or {}).keys())
    which = "admin-ui" if name.endswith("-admin-ui") else "ferroehr"
    if keys != EXPECTED[which]:
        bad.append(f"  {name}: selector.matchLabels is {sorted(keys)}, expected {sorted(EXPECTED[which])}"
                   f" — this field is IMMUTABLE; changing it breaks helm upgrade on every existing release")
for b in bad:
    print(b)
sys.exit(1 if bad else 0)
PYSEL
  echo "  selector.matchLabels unchanged (the immutable upgrade-breaking field)"
}

# ── Restricted-profile gate: EVERY pod, control by control ───────────────────
# The Kubernetes Pod Security Standards "restricted" profile
# (https://kubernetes.io/docs/concepts/security/pod-security-standards/) is
# checked per CONTAINER, not by grepping the file. A substring search passes as
# soon as one container carries a field, so with two workloads in a render — the
# server and the optional admin console — a compliant server would vouch for a
# non-compliant console. That is compliance by luck, and this render is the only
# place it can be caught before a cluster refuses the pod.
assert_security() {
  local file="$1"
  python3 - "$file" <<'PYGATE' || { red "  restricted-profile gate FAILED for ${file}"; exit 1; }
import sys, yaml

ALLOWED_VOLUMES = {
    "configMap", "csi", "downwardAPI", "emptyDir", "ephemeral",
    "persistentVolumeClaim", "projected", "secret",
}
bad = []

def check_pod(kind, name, spec):
    pod = spec.get("securityContext") or {}
    if pod.get("runAsNonRoot") is not True:
        bad.append(f"{kind}/{name}: pod runAsNonRoot must be true")
    if (pod.get("seccompProfile") or {}).get("type") not in ("RuntimeDefault", "Localhost"):
        bad.append(f"{kind}/{name}: pod seccompProfile.type must be RuntimeDefault or Localhost")
    # The kubelet's per-Service link variables land in the reserved FERROEHR_
    # namespace and the strict env sweep then refuses to boot, so losing this
    # makes every install crash-loop. Not a profile control; checked here
    # because this is where pod specs are inspected.
    if spec.get("enableServiceLinks") is not False:
        bad.append(f"{kind}/{name}: enableServiceLinks must be false")
    for v in spec.get("volumes") or []:
        used = [k for k in v if k != "name"]
        for t in used:
            if t not in ALLOWED_VOLUMES:
                bad.append(f"{kind}/{name}: volume {v.get('name')} type {t} is outside the restricted set")
    for c in (spec.get("containers") or []) + (spec.get("initContainers") or []):
        sc = c.get("securityContext") or {}
        cn = c.get("name")
        if sc.get("allowPrivilegeEscalation") is not False:
            bad.append(f"{kind}/{name}/{cn}: allowPrivilegeEscalation must be false")
        if sc.get("runAsNonRoot") is not True and pod.get("runAsNonRoot") is not True:
            bad.append(f"{kind}/{name}/{cn}: runAsNonRoot must be true")
        if sc.get("readOnlyRootFilesystem") is not True:
            bad.append(f"{kind}/{name}/{cn}: readOnlyRootFilesystem must be true")
        caps = sc.get("capabilities") or {}
        if "ALL" not in (caps.get("drop") or []):
            bad.append(f"{kind}/{name}/{cn}: capabilities.drop must include ALL")
        if caps.get("add"):
            bad.append(f"{kind}/{name}/{cn}: capabilities.add must be empty, got {caps['add']}")

pods = 0
netpol = False
for doc in yaml.safe_load_all(open(sys.argv[1])):
    if not doc:
        continue
    kind = doc.get("kind")
    if kind == "NetworkPolicy":
        netpol = True
    tmpl = (doc.get("spec") or {}).get("template")
    if kind in ("Deployment", "StatefulSet", "DaemonSet", "Job") and tmpl:
        pods += 1
        check_pod(kind, (doc.get("metadata") or {}).get("name"), tmpl.get("spec") or {})

if pods == 0:
    bad.append("no pod-bearing workload found — the gate would pass vacuously")
if not netpol:
    bad.append("no NetworkPolicy in the render")
for b in bad:
    print(f"  {b}")
sys.exit(1 if bad else 0)
PYGATE
  echo "  restricted profile satisfied by every pod in the render (per container)"
}

mkdir -p "$GOLDEN_DIR"
FAIL=0

# ── Secret-leak gate: no credential may reach the ConfigMap ───────────────────
# Two halves, because there are two ways to get this wrong. (1) Every secret the
# chart CARRIES is set to a unique sentinel, which must appear in a Secret and in
# no ConfigMap — absence alone would also be satisfied by dropping the secret on
# the floor. (2) Every secret-bearing CONFIG key must be refused outright, so a
# future secret key cannot leak by default — the last probe is a key that does
# not exist in the server's config tree at all, which is what proves the guard
# classifies by name shape rather than from a list of today's keys.
secret_leak_gate() {
  bold "── secret-leak gate ─────────────────────────────────────"
  local leak_values="${CI_DIR}/secret-leak-values.yaml"
  local cm secrets misdelivered=0
  cm="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" \
        -f "$leak_values" -s templates/configmap.yaml)"
  secrets="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" \
        -f "$leak_values" -s templates/secret.yaml)"
  # No `mapfile`: it is bash 4+, and macOS ships bash 3.2 as /usr/bin/env bash.
  local -a sentinels=()
  while IFS= read -r sentinel; do
    sentinels+=("$sentinel")
  done < <(grep -oE 'SENTINEL_[A-Z0-9]+_[0-9a-f]{4}' "$leak_values" | sort -u)
  for sentinel in "${sentinels[@]}"; do
    if grep -qF -- "$sentinel" <<<"$cm"; then
      red "  LEAKED into the ConfigMap: ${sentinel}"
      misdelivered=1
    fi
    if ! grep -qF -- "$sentinel" <<<"$secrets"; then
      red "  NOT DELIVERED by any Secret: ${sentinel}"
      misdelivered=1
    fi
  done
  if [[ "$misdelivered" -eq 0 ]]; then
    echo "  all ${#sentinels[@]} carried secrets delivered by a Secret, none in the ConfigMap"
  else
    FAIL=1
  fi

  # ROUTED secrets: the chart carries each through a `secrets:` key, so a value
  # under `config:` is an operator mistake and must be refused BY NAME.
  local -a routed_paths=(
    "db.url"
    "events.url"
    "fhir.outbound.url"
    "audit.fhir_feed.url"
    "auth.oidc.hmac_secret"
    "signing.key_passphrase"
    "multimedia.secret_access_key"
    "terminology.external.oauth2_clients.tx.client_secret"
    "auth.basic.users[0].password_hash"
  )
  local refused=0
  for path in "${routed_paths[@]}"; do
    local out
    if out="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" \
              -f "${CI_DIR}/default-values.yaml" \
              --set-string "config.${path}=SENTINEL_PROBE" 2>&1)"; then
      red "  NOT REFUSED: config.${path} rendered instead of failing"
      refused=1
    elif ! grep -qF -- "config.${path}" <<<"$out"; then
      red "  config.${path} was refused, but the message does not name it:"
      echo "$out" | head -3
      refused=1
    fi
  done
  if [[ "$refused" -eq 0 ]]; then
    echo "  all ${#routed_paths[@]} routed secret paths refused, each naming its secrets: key"
  else
    FAIL=1
  fi

  # UNROUTED secrets: no `secrets:` key carries them, so the whole rendered TOML
  # must move into the Secret and NO ConfigMap may exist. The second path is not
  # a real server key at all — it is the deny-by-default probe, and it is what
  # proves the guard classifies by name shape rather than from a fixed list.
  # Nothing the server models is unroutable any more, so the only input that
  # reaches this branch is a secret-shaped key the config tree does not define —
  # which is exactly the deny-by-default case the branch exists for.
  local -a unrouted_paths=(
    "server.future_api_key"
  )
  local moved=0
  for path in "${unrouted_paths[@]}"; do
    local render
    if ! render="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" \
                   -f "${CI_DIR}/default-values.yaml" \
                   --set-string "config.${path}=SENTINEL_PROBE" 2>&1)"; then
      red "  config.${path} failed to render; it should move the config into the Secret:"
      echo "$render" | head -3
      moved=1
      continue
    fi
    if grep -qE '^kind: ConfigMap$' <<<"$render"; then
      red "  config.${path}: a ConfigMap was still rendered"
      moved=1
    fi
    if ! grep -qF "SENTINEL_PROBE" <<<"$render"; then
      red "  config.${path}: the value vanished from the render entirely"
      moved=1
    fi
  done
  if [[ "$moved" -eq 0 ]]; then
    echo "  all ${#unrouted_paths[@]} unrouted secret paths delivered by Secret, with no ConfigMap"
  else
    FAIL=1
  fi
}
secret_leak_gate

# ── The values schema must refuse and permit exactly what it claims to ────────
# A schema that is present but vacuous is worse than none: `helm template` stays
# green, Artifact Hub renders a Values-schema tab, and an operator reads both as
# evidence their values file was checked. So the file is exercised from both
# sides — every refusal it promises, and every place it must stay OPEN.
#
# It has to stay open in two directions. `config:` is a 1:1 passthrough of the
# SERVER's TOML tree, adjudicated at boot by the binary that owns that
# vocabulary; typing it here would fork the config tree into the chart and refuse
# keys the server accepts. And `global:` is what a parent chart injects when this
# chart is used as a dependency — refusing it would make the chart unusable as a
# subchart, with an error that names nothing the operator wrote.
schema_gate() {
  bold "── values schema ────────────────────────────────────────"
  local schema="${CHART_DIR}/values.schema.json"
  if [[ ! -f "$schema" ]]; then
    red "  missing: ${schema} — nothing validates an operator's values file, and"
    red "  Artifact Hub reports the chart as having no values schema."
    FAIL=1
    return
  fi
  # Not a preference: helm validates through a multi-draft library, but Artifact
  # Hub's renderer is typed against json-schema draft 4/6/7, so a newer dialect
  # renders wrong on the page that consumes the file.
  if ! grep -q 'json-schema.org/draft-07/schema' "$schema"; then
    red "  ${schema} does not declare the draft-07 dialect"
    FAIL=1
  fi

  # <helm --set argument>|<the refusal must name this>
  local -a refusals=(
    "autoscalng.enabled=true|additional properties 'autoscalng' not allowed"
    "probes.startup.periodSecond=5|/probes/startup"
    "replicaCount=two|/replicaCount"
    "image.pullPolicy=Sometimes|/image/pullPolicy"
    "image.digest=sha256:nothex|/image/digest"
    "service.port=70000|/service/port"
    "metrics.serviceMonitor.interval=30|/metrics/serviceMonitor/interval"
  )
  local refused=0 probe want out
  for case in "${refusals[@]}"; do
    probe="${case%%|*}"
    want="${case##*|}"
    if out="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" \
              -f "${CI_DIR}/default-values.yaml" --set "$probe" 2>&1)"; then
      red "  NOT REFUSED: --set ${probe} rendered instead of failing schema validation"
      refused=1
    elif ! grep -qF -- "$want" <<<"$out"; then
      red "  --set ${probe} was refused, but the message does not mention '${want}':"
      printf '%s\n' "$out" | head -4
      refused=1
    fi
  done
  if [[ "$refused" -eq 0 ]]; then
    echo "  all ${#refusals[@]} malformed-values probes refused, each naming the offending path"
  else
    FAIL=1
  fi

  local -a acceptances=(
    "config.server.future_api_key=probe"
    "config.telemetry.otlp_endpoint=http://otel-collector:4317"
    "global.imageRegistry=registry.example.com"
  )
  local accepted=0
  for probe in "${acceptances[@]}"; do
    if ! out="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" \
                -f "${CI_DIR}/default-values.yaml" --set "$probe" 2>&1)"; then
      red "  WRONGLY REFUSED: --set ${probe} must be accepted (see the note above this gate):"
      printf '%s\n' "$out" | head -4
      accepted=1
    fi
  done
  if [[ "$accepted" -eq 0 ]]; then
    echo "  all ${#acceptances[@]} must-stay-open probes accepted (server config vocabulary, parent-chart global)"
  else
    FAIL=1
  fi
}
schema_gate

# ── The live-test fixture must still track the chart ──────────────────────────
# WHY a `.claude/` path appears in a deploy gate: `.claude/skills/k8s-test/` holds
# the values overlay every cluster run installs with, so it is a consumer of this
# chart exactly like the overlays in ci/ — and the only thing that ever exercised
# it was a human running the skill, which happens when they are testing something
# ELSE. That is how it broke: the secret-routing change made `password_hash` under
# `config:` a refusal, the fixture still set it there, and nothing failed until the
# next live run would have. Same shape as the golden gate that ran in no workflow.
#
# This RENDERS, it does not deep-verify. The property is "the fixture still tracks
# the chart", not "the fixture is correct" — a live run checks the second.
fixture_gate() {
  bold "── live-test fixture ────────────────────────────────────"
  local repo_root fixture
  repo_root="$(cd "${SCRIPT_DIR}/../.." && pwd)"
  fixture="${repo_root}/.claude/skills/k8s-test/test-values.yaml"
  if [[ ! -f "$fixture" ]]; then
    red "  missing: .claude/skills/k8s-test/test-values.yaml — the k8s-test skill installs with it"
    FAIL=1
    return
  fi
  local rendered out
  rendered="$(mktemp)"
  if ! out="$(helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" -f "$fixture" 2>&1)"; then
    red "  the k8s-test fixture NO LONGER RENDERS against this chart."
    red "  This is the FIXTURE being stale, not your chart change being wrong:"
    red "  .claude/skills/k8s-test/test-values.yaml needs the same edit your values change implies."
    printf '%s
' "$out" | head -6
    FAIL=1
    rm -f "$rendered"
    return
  fi
  printf '%s' "$out" > "$rendered"
  # The objects a live run actually drives; a fixture that renders but produces no
  # Deployment would pass a bare render check and fail the run.
  local missing=0 kind
  for kind in Deployment Service ServiceAccount NetworkPolicy; do
    grep -qE "^kind: ${kind}$" "$rendered" || { red "  fixture renders no ${kind}"; missing=1; }
  done
  # The live run authenticates, so the fixture must still configure a usable
  # mechanism — and the hash must arrive by the mounted route, not in the ConfigMap.
  grep -q 'password_hash_file' "$rendered" || { red "  fixture configures no Basic user via the mounted route (password_hash_file absent)"; missing=1; }
  if grep -A200 '^kind: ConfigMap$' "$rendered" | grep -q 'argon2id'; then
    red "  fixture puts an Argon2id hash in the ConfigMap"
    missing=1
  fi
  if [[ "$missing" -eq 0 ]]; then
    echo "  the k8s-test fixture still renders a Deployment/Service/SA/NetworkPolicy with a mounted hash"
  else
    FAIL=1
  fi
  rm -f "$rendered"
}
fixture_gate

for case in "${CASES[@]}"; do
  label="${case%%:*}"
  values="${case##*:}"
  bold "── ${label} ───────────────────────────────────────────────"

  echo "helm lint:"
  helm lint "$CHART_DIR" -f "$values"

  rendered="$(mktemp)"
  helm template "$RELEASE_NAME" "$CHART_DIR" -n "$NAMESPACE" -f "$values" > "$rendered"

  echo "YAML validity:"
  yaml_ok "$rendered"

  echo "security gate:"
  assert_security "$rendered"
  assert_selector_stable "$rendered"

  if command -v kubeconform >/dev/null 2>&1; then
    echo "kubeconform:"
    kubeconform -strict -summary -ignore-missing-schemas "$rendered" || FAIL=1
  else
    echo "kubeconform: not installed — skipping schema validation (optional)"
  fi

  golden="${GOLDEN_DIR}/${label}.yaml"
  if [[ "$GOLDEN_SKEW" -eq 1 ]]; then
    red "golden ${label}: SKIPPED — running helm ${ACTUAL_HELM}, goldens are pinned to ${PINNED_HELM}."
    red "  Install helm ${PINNED_HELM}, or bump ${PIN_FILE} AND re-run with --update in the same change."
    FAIL=1
  elif [[ "$UPDATE" -eq 1 ]]; then
    cp "$rendered" "$golden"
    green "golden updated: ${golden}"
  elif [[ -f "$golden" ]]; then
    if diff -u "$golden" "$rendered" > /tmp/golden.diff 2>&1; then
      green "golden matches: ${golden}"
    else
      red "golden DRIFT for ${label} — re-run with --update if intended:"
      head -40 /tmp/golden.diff
      FAIL=1
    fi
  else
    red "golden missing: ${golden} — run: deploy/helm/validate.sh --update"
    FAIL=1
  fi
  rm -f "$rendered"
done

# ── the chart README is GENERATED, so it is drift-checked, not reviewed ───────
# helm-docs renders README.md from README.md.gotmpl + the `# --` comments in
# values.yaml. A hand-edited README, or a new value documented nowhere, is drift:
# the values table is the chart's published reference (it is the front page on a
# registry and on Artifact Hub), and a table that disagrees with values.yaml is
# worse than no table. Skipped with a note when helm-docs is absent, so the rest
# of this script still runs locally.
echo
if command -v helm-docs >/dev/null 2>&1; then
  README="${CHART_DIR}/README.md"
  if [[ -f "$README" ]]; then
    before="$(mktemp)"; cp "$README" "$before"
    helm-docs --chart-search-root "$CHART_DIR" --template-files README.md.gotmpl \
      >/dev/null 2>&1
    if diff -u "$before" "$README" > /tmp/readme.diff 2>&1; then
      green "chart README matches values.yaml"
    else
      red "chart README DRIFT — README.md is generated; regenerate it with:"
      red "  helm-docs --chart-search-root ${CHART_DIR} --template-files README.md.gotmpl"
      head -40 /tmp/readme.diff
      cp "$before" "$README"
      FAIL=1
    fi
    rm -f "$before"
  else
    red "chart README missing: ${README} — it is published metadata, and Artifact"
    red "Hub renders it as the listing body; generate it with helm-docs."
    FAIL=1
  fi
else
  printf '%s\n' "helm-docs: not installed — skipping the chart README drift check"
fi

# ── What a green run above does NOT mean ─────────────────────────────────────
# Printed unconditionally, on success and on failure. It is the direct answer to
# #2159: every committed overlay was rendering, linting and matching its golden
# while none of them produced a configuration the server accepts, and the reason
# nobody noticed is that a green run here reads like a working deployment.
bold "── not checked by this script ───────────────────────────"
cat <<'BOUNDARY'
  This script renders. It never runs the server, and never talks to a cluster.
  It therefore cannot tell you any of the following, and a green result above
  must not be read as evidence of them:

    * that the server ACCEPTS the rendered ferroehr.toml — the keys it carries,
      their types, and the semantic rules validated at boot (an auth mechanism,
      the RFC 8725 32-byte HMAC floor, a real Argon2id PHC string, the SMART
      requirements). Run: deploy/helm/ci/boot-check.sh   [CI: chart-boot]
    * that the image the chart selects UNDERSTANDS those keys. `appVersion` and
      the chart's config defaults move on different clocks, so a key added
      in-tree can be rejected by the released image.
      Run: FERROEHR_IMAGE=ghcr.io/rubentalstra/ferroehr:<tag> \
             deploy/helm/ci/boot-check.sh   [CI: publish-chart.yml, on a tag]
    * that a pod STARTS, migrates the database, passes its probes and serves a
      request — config validation opens no socket. Run: the k8s-test skill
      (.claude/skills/k8s-test/) against a real cluster.
    * that the values are semantically sensible for a real deployment: that the
      issuer exists, the broker is reachable, the S3 bucket resolves, the
      NetworkPolicy egress rules actually match the peers in use.
    * anything about the CHART's behaviour under `helm upgrade`, or about
      resource sizing, scheduling or autoscaling behaviour under load.
BOUNDARY

echo
if [[ "$FAIL" -eq 0 ]]; then
  green "ALL RENDER CHECKS PASSED (see 'not checked' above)"
else
  red "VALIDATION FAILED"
  exit 1
fi

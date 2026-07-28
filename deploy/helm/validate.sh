#!/usr/bin/env bash
#
# Validate the ehrbase-rs Helm chart:
#   1. helm lint      — default + all-features value sets (must be clean)
#   2. helm template  — render both; assert the output is valid multi-doc YAML
#   3. security gate   — assert the Kubernetes Pod Security Standards
#                        "Restricted" fields are pinned in the rendered
#                        Deployment (runAsNonRoot, seccompProfile
#                        RuntimeDefault, drop ALL caps,
#                        allowPrivilegeEscalation:false), plus our
#                        readOnlyRootFilesystem hardening
#                        https://kubernetes.io/docs/concepts/security/pod-security-standards/
#   4. golden render   — compare against deploy/helm/golden/ (or --update it)
#   5. kubeconform     — schema-validate the manifests IF kubeconform is on PATH
#                        (optional; skipped with a note when absent/offline)
#
# Usage:
#   deploy/helm/validate.sh            # validate (fails on lint/render/golden drift)
#   deploy/helm/validate.sh --update   # regenerate the golden renders, then validate
#
# No cluster and no network are required for steps 1–4.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHART_DIR="${SCRIPT_DIR}/ehrbase-rs"
CI_DIR="${SCRIPT_DIR}/ci"
GOLDEN_DIR="${SCRIPT_DIR}/golden"
RELEASE_NAME="ehrbase-rs"
NAMESPACE="ehrbase"

UPDATE=0
[[ "${1:-}" == "--update" ]] && UPDATE=1

red()   { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold()  { printf '\033[1m%s\033[0m\n' "$*"; }

command -v helm >/dev/null 2>&1 || { red "helm not found on PATH"; exit 1; }
bold "helm: $(helm version --short)"

# The value sets to validate: <label>:<values-file>
declare -a CASES=(
  "default:${CI_DIR}/default-values.yaml"
  "all-features:${CI_DIR}/all-features-values.yaml"
)

# ── YAML validity check (PyYAML if present, else a helm re-parse) ─────────────
yaml_ok() {
  local file="$1"
  if command -v python3 >/dev/null 2>&1 && python3 -c "import yaml" >/dev/null 2>&1; then
    python3 - "$file" <<'PY'
import sys, yaml
docs = list(yaml.safe_load_all(open(sys.argv[1])))
docs = [d for d in docs if d]
assert docs, "no YAML documents rendered"
for d in docs:
    assert "kind" in d and "apiVersion" in d, f"doc missing kind/apiVersion: {d.get('metadata', {})}"
print(f"  parsed {len(docs)} valid YAML document(s)")
PY
  else
    echo "  (PyYAML absent — YAML validity implied by successful helm render)"
  fi
}

# ── Security-field gate: every Restricted-profile field must be present ──────
assert_security() {
  local file="$1"
  local -a required=(
    "runAsNonRoot: true"
    "readOnlyRootFilesystem: true"
    "allowPrivilegeEscalation: false"
    "type: RuntimeDefault"
    "- ALL"
  )
  local missing=0
  for field in "${required[@]}"; do
    if ! grep -qF -- "$field" "$file"; then
      red "  MISSING required security field: '${field}'"
      missing=1
    fi
  done
  # The default-deny NetworkPolicy must be present (both cases enable it).
  if ! grep -q "kind: NetworkPolicy" "$file"; then
    red "  MISSING NetworkPolicy (default-deny ingress)"
    missing=1
  fi
  [[ "$missing" -eq 0 ]] || { red "  security gate FAILED for ${file}"; exit 1; }
  echo "  security fields pinned (runAsNonRoot, readOnlyRootFilesystem, seccomp RuntimeDefault, drop ALL, no priv-esc, NetworkPolicy)"
}

mkdir -p "$GOLDEN_DIR"
FAIL=0

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

  if command -v kubeconform >/dev/null 2>&1; then
    echo "kubeconform:"
    kubeconform -strict -summary -ignore-missing-schemas "$rendered" || FAIL=1
  else
    echo "kubeconform: not installed — skipping schema validation (optional)"
  fi

  golden="${GOLDEN_DIR}/${label}.yaml"
  if [[ "$UPDATE" -eq 1 ]]; then
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

echo
if [[ "$FAIL" -eq 0 ]]; then
  green "ALL CHECKS PASSED"
else
  red "VALIDATION FAILED"
  exit 1
fi

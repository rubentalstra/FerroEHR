#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
#
# Seed the hosted sandbox (#2710) with demo data through the PUBLIC API —
# the same surface visitors use, so the seed itself proves the deployment.
# Uploads a small slice of the vendored CKM operational templates and
# commits each one's committed example composition into a handful of fresh
# EHRs.
#
# Every request retries: right after the nightly wipe, serverless instances
# booted BEFORE the wipe keep serving until they idle out, and the platform
# load-balances across old and fresh instances — a request landing on a
# stale one answers 5xx until it recycles (observed live, #2710). Retrying
# for a few minutes rides that window out.
#
# Environment: SANDBOX_BASE (default https://sandbox.ferroehr.eu),
# SANDBOX_USER / SANDBOX_PASS (default the public demo credentials).
set -Eeuo pipefail

BASE="${SANDBOX_BASE:-https://sandbox.ferroehr.eu}/ferroehr/rest/openehr/v1"
AUTH="${SANDBOX_USER:-ferroehr}:${SANDBOX_PASS:-ferroehr}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TPL_DIR="$ROOT_DIR/tools/cnf-runner/artifacts/corpus/templates/ckm"

# A small, stable slice of the curated CKM pack: recognizable clinical
# content without inflating the free-tier storage. Each slug names an
# `<slug>.opt` + `<slug>.example.json` pair in the pack.
TEMPLATES=(
  vital-signs
  medicines-list
  problem-list
  covid19-infection-report
)
EHRS=3
COMPOSITIONS_PER_EXAMPLE=2
ATTEMPTS=12
RETRY_DELAY=15

# POST with retry; prints the final status code. `$1` = a label, `$2` = the
# URL, remaining args go to curl verbatim. Retries any 5xx/connection
# failure; any non-5xx answer is final.
post_retry() {
  local label="$1" url="$2"
  shift 2
  local attempt code
  for attempt in $(seq 1 "$ATTEMPTS"); do
    code=$(curl -sS -m 120 -u "$AUTH" -o /dev/null -w '%{http_code}' -X POST "$url" "$@" || echo 000)
    case "$code" in
      5?? | 000)
        echo "  $label answered $code (attempt $attempt/$ATTEMPTS); a stale instance may still be serving — retrying" >&2
        sleep "$RETRY_DELAY"
        ;;
      *)
        printf '%s' "$code"
        return 0
        ;;
    esac
  done
  printf '%s' "$code"
}

echo "==> seeding $BASE"

# The demo EHRs, shared across every template so each carries a mixed record.
ehrs=()
for e in $(seq 1 "$EHRS"); do
  ehr=""
  for attempt in $(seq 1 "$ATTEMPTS"); do
    ehr=$(curl -sS -m 60 -u "$AUTH" -X POST "$BASE/ehr" -H 'Prefer: return=minimal' \
      -D- -o /dev/null 2>/dev/null | tr -d '\r' | awk -F'/ehr/' 'tolower($0) ~ /^location/{print $2}' | tr -d ' ') || true
    [[ -n "$ehr" ]] && break
    echo "  EHR creation returned no Location (attempt $attempt/$ATTEMPTS); retrying" >&2
    sleep "$RETRY_DELAY"
  done
  if [[ -z "$ehr" ]]; then
    echo "::error::EHR creation kept failing after $ATTEMPTS attempts — the sandbox is not serving writes." >&2
    exit 1
  fi
  ehrs+=("$ehr")
done

seeded=0
for slug in "${TEMPLATES[@]}"; do
  opt="$TPL_DIR/$slug.opt"
  example="$TPL_DIR/$slug.example.json"
  [[ -f "$opt" ]] && [[ -f "$example" ]] || {
    echo "::error::missing template pair for $slug in $TPL_DIR" >&2
    exit 1
  }
  code=$(post_retry "template $slug" "$BASE/definition/template/adl1.4" \
    -H 'Content-Type: application/xml' --data-binary @"$opt")
  case "$code" in
    201 | 204 | 409) echo "template $slug -> $code" ;;
    *)
      echo "::error::template upload for $slug answered $code" >&2
      exit 1
      ;;
  esac

  for ehr in "${ehrs[@]}"; do
    for c in $(seq 1 "$COMPOSITIONS_PER_EXAMPLE"); do
      code=$(post_retry "composition $slug" "$BASE/ehr/$ehr/composition" \
        -H 'Content-Type: application/json' -H 'Prefer: return=minimal' \
        --data-binary @"$example")
      if [[ "$code" != "201" ]]; then
        echo "::error::composition commit answered $code ($slug into $ehr)" >&2
        exit 1
      fi
      seeded=$((seeded + 1))
    done
  done
done

echo "==> seeded $seeded compositions across ${#ehrs[@]} demo EHRs"

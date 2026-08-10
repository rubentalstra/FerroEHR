#!/usr/bin/env bash
# The deployment-conformance harness (#2178).
#
# `cnf-runner` exists because "we believe the REST surface is conformant" was
# not good enough — it had to be MEASURED, repeatably, by something that
# outlives the person who ran it. This is the same instrument for the other
# half: "we believe the deployment works".
#
# The gap it covers is real and was expensive. A 4447-test suite was green
# while ten defects shipped, every one of them living between "the code is
# correct" and "the thing we hand an operator works" — a gateway with no
# bucket, an OIDC realm that could not mint an acceptable token, a Prometheus
# that scraped nothing, host environment silently dropped on the floor. None of
# them was reachable by rendering YAML or by a unit test with a mock.
#
# Three rules it holds itself to, each one a lesson from those defects:
#
#   1. Observe at the FAR END. Not "the API said 201" but "the blob is in the
#      bucket"; not "the config looks right" but "the server reports it".
#   2. Follow the DOCUMENTATION, not the source. Several findings were the book
#      describing something that does not work, and a probe configured the way
#      the source says would never catch that class again.
#   3. Say what was NOT exercised. Silence read as coverage is how this
#      happened; the report ends with the gaps, in its own output.
#
# Usage:
#   scripts/deploy-probe.sh                  # the compose platform
#   scripts/deploy-probe.sh --keep-up        # leave the stack running to poke at
#
# Env:
#   FERROEHR_IMAGE   the server image under probe (default: the compose pin).
#                    The harness measures the SUT it is given — pointing it at
#                    an older image is a legitimate way to watch a regression
#                    probe fail.
#   PROBE_OUT        where the machine-readable record lands.
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

KEEP_UP=0
[ "${1:-}" = "--keep-up" ] && KEEP_UP=1

PROBE_TMP="$(mktemp -d)"
export PROBE_TMP
PROBE_OUT="${PROBE_OUT:-docs/conformance/deployment/compose.json}"

# shellcheck source=scripts/deploy-probes/lib.sh
. scripts/deploy-probes/lib.sh
# shellcheck source=scripts/deploy-probes/compose.sh
. scripts/deploy-probes/compose.sh
# shellcheck source=scripts/deploy-probes/oidc.sh
. scripts/deploy-probes/oidc.sh
# shellcheck source=scripts/deploy-probes/signing.sh
. scripts/deploy-probes/signing.sh
# shellcheck source=scripts/deploy-probes/observability.sh
. scripts/deploy-probes/observability.sh
# shellcheck source=scripts/deploy-probes/tenancy.sh
. scripts/deploy-probes/tenancy.sh
# shellcheck source=scripts/deploy-probes/events.sh
. scripts/deploy-probes/events.sh
# shellcheck source=scripts/deploy-probes/signing_pgp.sh
. scripts/deploy-probes/signing_pgp.sh

cleanup() {
  if [ "$KEEP_UP" -eq 0 ]; then
    compose_down
  else
    dim "── stack left running (--keep-up): $CDR"
  fi
  rm -rf "$PROBE_TMP"
}
trap cleanup EXIT

command -v docker >/dev/null || { red "docker not found on PATH"; exit 1; }

bold "── deployment probes: compose ──────────────────────────────"
echo "  image:  ${FERROEHR_IMAGE:-<the compose default pin>}"
echo "  cdr:    $CDR"
echo

# The documented recipe, used as documented: the multimedia keys are EXPORTED,
# not written into a file. #2169 was exactly this path silently doing nothing,
# so the harness must take it rather than a shortcut the book never mentions.
export FERROEHR__MULTIMEDIA__ENABLED=true
export FERROEHR__MULTIMEDIA__ENDPOINT=http://seaweedfs:8333
export FERROEHR__MULTIMEDIA__BUCKET=openehr-multimedia
export FERROEHR__MULTIMEDIA__ALLOW_HTTP=true

compose_down
bold "bringing the stack up (postgres + CDR + seaweedfs + init)"
compose_up ferroehr seaweedfs seaweedfs-init

probes_shipped_config_boots
probes_multimedia
probes_management
probes_management_separate_listener
probes_signing
probes_signing_pgp
probes_signing_rotation
probes_multimedia_restart
probes_multimedia_off
probes_multimedia_broken
probes_health_broken
probes_oidc
probes_oidc_roles
probes_observability
probes_tenancy
probes_events

# ── The honest half ───────────────────────────────────────────────────────────
# Everything #2178 asks for that this run does NOT do. Each entry is a probe
# somebody still has to write; none of them is silently absent.
uncovered "kubernetes platform" \
  "a separate harness covers it — scripts/deploy-probe-k8s.sh, recorded in kubernetes.json"
uncovered "admin console journeys (#2164)" \
  "scripts/ui-e2e.sh already drives these with a real browser; folding it in is the next step"
uncovered "FHIR, terminology" \
  "no probes yet — each needs its dependency composed (a receiver, a terminology server)"

probe_report "$PROBE_OUT" "compose"

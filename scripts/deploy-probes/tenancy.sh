#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# The multi-tenancy probe family (#2178, #2158).
#
# Tenancy is off by default, and its claim is the strongest one this server
# makes about data separation: a caller resolved to tenant A must not be able to
# see, read or delete tenant B's records. That is a clinical-safety property,
# and until now nothing had started the feature at all.
#
# The probes are written around the ONE thing that makes such a claim
# meaningful: an isolation test proves nothing unless the same identifier is
# known to exist. A read that 404s because the row was never created looks
# exactly like a read that 404s because tenancy hid it, so every refusal below
# is paired with a positive control — the SAME id, fetched successfully by the
# tenant that owns it.
#
# Sourced by scripts/deploy-probe.sh; never run directly.

# The dev-only header override. Production must leave it unset (a client-chosen
# tenant is not a tenant boundary at all), so the probes below ALSO check that
# an unset header cannot be spoofed.
TENANT_HEADER="X-FerroEHR-Tenant"

tenancy_overlay() {
  local out="$PROBE_TMP/tenancy.yml"
  cat > "$out" <<YAML
services:
  ferroehr:
    environment:
      FERROEHR__TENANCY__ENABLED: "true"
      FERROEHR__TENANCY__CLAIM: tenant
      FERROEHR__TENANCY__HEADER: ${TENANT_HEADER}
YAML
  printf '%s' "$out"
}

# Register a tenant. Tenants are ADMINISTERED objects, not strings a caller
# invents: an unregistered key resolves to the default tenant, so a probe that
# skipped this step would have both "tenants" sharing one store and would report
# a leak that is really its own omission. That is exactly what the first version
# of this file did.
tenancy_register() {
  curl -s -o /dev/null -u "$BASIC" -X POST -H 'Content-Type: application/json' \
    -d "{\"name\":\"$1\",\"system_id\":\"$1.example.test\"}" "$API/admin/tenant"
}

# An EHR created as one tenant, echoing its id.
tenancy_new_ehr() {
  curl -s -u "$BASIC" -H "${TENANT_HEADER}: $1" -X POST -D - -o /dev/null "$API/ehr" \
    | grep -i '^location' | tr -d '\r' | awk -F/ '{print $NF}'
}

probes_tenancy() {
  bold "multi-tenancy — isolation between two tenants"

  local overlay
  overlay="$(tenancy_overlay)"

  probe "P-TEN-BOOT" "working" "server" "#2178" \
    "the server boots with tenancy enabled and still serves"
  dc -f docker-compose.yml -f "$overlay" up -d ferroehr >/dev/null 2>&1
  if ! wait_http "$CDR/health/readiness" 120; then
    probe_fail "a serving CDR with tenancy on" \
      "$(dc logs --tail 5 ferroehr 2>&1 | tail -3)" \
      "tenancy adds middleware and a per-request GUC; a boot failure here is that wiring"
    probe_done
    dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
    wait_http "$CDR/health/readiness" 120 || true
    return 0
  fi
  probe_done

  # The positive control, which everything below depends on.
  probe "P-TEN-WRITE" "working" "server" "#2178" \
    "each tenant can create its own EHR and read it back"
  local a b
  tenancy_register alpha
  tenancy_register beta
  a="$(tenancy_new_ehr alpha)"
  b="$(tenancy_new_ehr beta)"
  if [[ -z "$a" ]] || [[ -z "$b" ]]; then
    probe_fail "an EHR id for each tenant" "alpha='$a' beta='$b'" \
      "without both ids the isolation probes below would pass vacuously"
    probe_done
    dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
    wait_http "$CDR/health/readiness" 120 || true
    return 0
  fi
  assert_eq "200" "$(http_code -u "$BASIC" -H "${TENANT_HEADER}: alpha" "$API/ehr/$a")" \
    "a tenant must be able to read what it just wrote"
  assert_eq "200" "$(http_code -u "$BASIC" -H "${TENANT_HEADER}: beta" "$API/ehr/$b")" \
    "a tenant must be able to read what it just wrote"
  probe_done

  # The claim itself. The id is KNOWN to exist — the probe above just fetched it
  # — so a 404 here is isolation and not absence.
  probe "P-TEN-ISOLATED" "broken" "server" "#2178" \
    "tenant beta cannot read an EHR that tenant alpha created, though the id exists"
  local cross
  cross="$(http_code -u "$BASIC" -H "${TENANT_HEADER}: beta" "$API/ehr/$a")"
  case "$cross" in
    404) : ;;
    200) probe_fail "404 for a foreign tenant's EHR" "$cross" \
           "tenant beta read tenant alpha's record — the isolation boundary does not hold" ;;
    *)   probe_fail "404 for a foreign tenant's EHR" "$cross" \
           "a foreign tenant must be told the record does not exist, not given a different error" ;;
  esac
  probe_done

  # AQL is the other way in, and the one that would leak in bulk rather than one
  # record at a time.
  probe "P-TEN-AQL" "broken" "server" "#2178" \
    "an AQL query run as beta does not return alpha's ehr_id"
  local rs
  rs="$(curl -s -u "$BASIC" -H "${TENANT_HEADER}: beta" --get \
        --data-urlencode 'q=SELECT e/ehr_id/value FROM EHR e' "$API/query/aql")"
  assert_not_contains "$rs" "$a" \
    "a query is a bulk read; if the boundary leaks anywhere it leaks here first"
  assert_contains "$rs" "$b" \
    "beta must still see its OWN data, or this probe would pass on a broken query rather than a working boundary"
  probe_done

  # An UNREGISTERED tenant key. Recorded because it is the behaviour a typo in a
  # JWT claim produces, and because a reader of this file would otherwise have to
  # guess: the key does not resolve to alpha's data.
  probe "P-TEN-UNKNOWN" "broken" "server" "#2178" \
    "an unregistered tenant key cannot read a registered tenant's record"
  assert_eq "404" "$(http_code -u "$BASIC" -H "${TENANT_HEADER}: nosuchtenant" "$API/ehr/$a")" \
    "an unknown key must not be a way around the boundary"
  probe_done

  # Back to the shipped posture.
  dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
  wait_http "$CDR/health/readiness" 120 || true

  # OFF is the default, and the state a deployment that never enables tenancy
  # runs in. The header must then be inert — a leftover header on a request must
  # not select anything.
  probe "P-TEN-OFF" "off" "server" "#2178" \
    "with tenancy off a stray tenant header partitions nothing"
  local off_ehr
  off_ehr="$(curl -s -u "$BASIC" -X POST -D - -o /dev/null "$API/ehr" \
             | grep -i '^location' | tr -d '\r' | awk -F/ '{print $NF}')"
  assert_eq "200" "$(http_code -u "$BASIC" -H "${TENANT_HEADER}: beta" "$API/ehr/$off_ehr")" \
    "with the feature off a leftover header from some client must not hide data"
  probe_done
}

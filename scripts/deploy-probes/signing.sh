#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
# The VERSION-signing probe family (#2163).
#
# `[signing]` is on by default in `digest` mode with read-time verification
# `strict`, which means the server recomputes the signature of a version it
# signed on every read and refuses to serve a provably corrupt record. That is a
# strong claim, and it had never been watched work.
#
# The probe that matters is the TAMPER one: a signing feature with no
# demonstrated detection is decoration. So this reaches past the API into the
# stored rows, changes a byte of clinical content, and asserts the next read
# FAILS — the only way to show the verification is real rather than a field
# nobody checks.
#
# Sourced by scripts/deploy-probe.sh; never run directly.

# Run SQL against the composed database as the quickstart's role.
probe_psql() {
  dc exec -T ferroehr-postgres psql -U "${PG_INIT_USER:-ferroehr}" \
    -d "${PG_INIT_DB:-ferroehr}" -tAc "$1" 2>/dev/null
}

probes_signing() {
  bold "VERSION signing (digest mode)"

  # A committed version must carry a signature the server generated, and the
  # read path must serve it. Digest mode is the shipped default, so this is the
  # posture nearly every deployment runs.
  probe "P-SIGN-DIGEST" "working" "server" "-" \
    "a committed VERSION carries a server signature and reads back cleanly"
  local ehr version
  ehr="$(curl -s -u "$BASIC" -X POST -D - -o /dev/null "$API/ehr" \
    | grep -i '^location' | tr -d '\r' | awk -F/ '{print $NF}')"
  if [ -z "$ehr" ]; then
    probe_fail "a committed EHR" "no id returned"
    probe_done
    return 0
  fi
  version="$(curl -s -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")"
  assert_contains "$version" '"signature"' \
    "with signing enabled a served ORIGINAL_VERSION must carry its signature"
  assert_eq "200" "$(http_code -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")" \
    "an untampered version verifies on read"
  probe_done

  # The one that makes the feature more than decoration. Reaching past the API
  # into the stored rows is the point: an attacker or a corrupt disk does not
  # go through the write path, so neither does this. The tamper target is
  # vo_version.body — the materialized projection every point read serves
  # (the node rows are the AQL index; the parity between the two copies is
  # pinned by the persistence suite, not by this probe).
  probe "P-SIGN-TAMPER" "broken" "server" "-" \
    "a tampered stored version is REFUSED on read, not served"
  local before after rows
  before="$(http_code -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")"
  rows="$(probe_psql "UPDATE ehr.vo_version
                         SET body = jsonb_set(body, '{name,value}', '\"tampered\"')
                       WHERE ehr_id = '$ehr'::uuid AND kind = 'EHR_STATUS';" \
          && probe_psql "SELECT count(*) FROM ehr.vo_version
                          WHERE ehr_id = '$ehr'::uuid
                            AND body #>> '{name,value}' = 'tampered';")"
  if [ "${rows:-0}" = "0" ]; then
    probe_fail "a tampered stored row" "the UPDATE matched nothing" \
      "the probe could not reach the stored content, so detection was never tested"
  else
    after="$(http_code -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")"
    case "$after" in
      5*) : ;;
      *)  probe_fail "a 5xx integrity refusal" "$after" \
            "verify_on_read is strict by default, so a corrupt record must not be served (was $before before tampering)" ;;
    esac
  fi
  probe_done
}

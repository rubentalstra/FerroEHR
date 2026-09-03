#!/usr/bin/env bash
# SPDX-FileCopyrightText: Ruben Talstra
# SPDX-License-Identifier: BUSL-1.1
# The Docker Compose probe family.
#
# Every probe here observes at the FAR END — a blob in the bucket, a claim in a
# token, the server's own effective configuration — rather than trusting a 2xx.
# Each one that carries `regression-of` reproduces a defect this project shipped:
# it must fail on the unfixed code and pass on the fixed code, which is what
# turns a one-off sweep into a permanent net.
#
# Sourced by scripts/deploy-probe.sh; never run directly.

# The compose stack this family drives. Ports are shifted off the defaults so a
# probe run can coexist with a developer's own stack.
COMPOSE_PROJECT="ferroehr-probe"
export FERROEHR_PORT="${PROBE_CDR_PORT:-18080}"
export FERROEHR_S3_PORT="${PROBE_S3_PORT:-18333}"
# Inert against the base file since #2879 (the quickstart publishes no DB
# port); kept for runs that add the db-publish overlay so a probe stack still
# coexists with a developer's own.
export FERROEHR_DB_PORT="${PROBE_DB_PORT:-15432}"
CDR="http://localhost:${FERROEHR_PORT}"
S3="http://localhost:${FERROEHR_S3_PORT}"
BASIC="ferroehr:ferroehr"
API="$CDR/ferroehr/rest/openehr/v1"

dc() { docker compose -p "$COMPOSE_PROJECT" "$@"; }

compose_down() { dc down -v --remove-orphans >/dev/null 2>&1 || true; }

# Bring the stack up with the multimedia keys taken from the ENVIRONMENT, which
# is the documented recipe — not by editing a file. That is deliberate: the
# recipe not working was itself a defect (#2169), so the harness must use the
# path the book tells an operator to use.
compose_up() {
  local -a services=("$@")
  # --wait is load-bearing: it gates on seaweedfs-init EXITING 0 (the compose
  # file's own contract — the initializer "exits 0 only in a state the probe
  # accepts, and `up --wait` gates on that exit"), which is also the recipe
  # exactly as the compose file documents it. Without it P-MM-BUCKET races
  # the one-shot initializer on a slow runner and measures the race, not the
  # deployment (the CI flake on the #2617 push run).
  dc -f docker-compose.yml --profile s3 up -d --wait "${services[@]}" >/dev/null 2>&1
}

# A composition-free way to commit a large DV_MULTIMEDIA: an EHR_STATUS carrying
# one. No template upload is needed, and it exercises the same versioning path a
# COMPOSITION does — which is exactly where #2197 lived.
#
# Built with `base64` and `printf` rather than a scripting language: #2178 pins
# the harness to bash and Rust.
probe_status_payload() {
  local bytes="${1:-409600}" out="$2"
  local b64
  b64="$(head -c "$bytes" /dev/urandom | base64 | tr -d '\n')"
  printf '%s' '{"_type":"EHR_STATUS",
    "name":{"_type":"DV_TEXT","value":"EHR status"},
    "archetype_node_id":"openEHR-EHR-EHR_STATUS.generic.v1",
    "archetype_details":{"_type":"ARCHETYPED",
      "archetype_id":{"_type":"ARCHETYPE_ID","value":"openEHR-EHR-EHR_STATUS.generic.v1"},
      "rm_version":"1.1.0"},
    "subject":{"_type":"PARTY_SELF"},
    "is_queryable":true,"is_modifiable":true,
    "other_details":{"_type":"ITEM_TREE",
      "name":{"_type":"DV_TEXT","value":"attachments"},
      "archetype_node_id":"at0001",
      "items":[{"_type":"ELEMENT",
        "name":{"_type":"DV_TEXT","value":"scan"},
        "archetype_node_id":"at0002",
        "value":{"_type":"DV_MULTIMEDIA",
          "media_type":{"_type":"CODE_PHRASE",
            "terminology_id":{"_type":"TERMINOLOGY_ID","value":"IANA_media-types"},
            "code_string":"application/octet-stream"},
          "size":' > "$out"
  printf '%s,"data":"%s"' "$bytes" "$b64" >> "$out"
  printf '%s' '}}]}}' >> "$out"
}

# Commit an EHR whose EHR_STATUS carries a 400 KiB DV_MULTIMEDIA; echo its id
# (empty when the commit did not succeed).
probe_commit_media_status() {
  local body="$PROBE_TMP/status.json" hdr="$PROBE_TMP/h.txt"
  probe_status_payload 409600 "$body"
  curl -s -u "$BASIC" -X POST -H 'Content-Type: application/json' \
    --data-binary "@$body" -D "$hdr" -o /dev/null "$API/ehr" || return 0
  grep -i '^location' "$hdr" 2>/dev/null | tr -d '\r' | awk -F/ '{print $NF}'
}

# The same commit, but reporting only its status code — for the states where the
# REFUSAL is the expected outcome.
probe_commit_media_status_code() {
  local body="$PROBE_TMP/status-broken.json"
  probe_status_payload 409600 "$body"
  curl -s -u "$BASIC" -X POST -H 'Content-Type: application/json' \
    --data-binary "@$body" -o /dev/null -w '%{http_code}' "$API/ehr"
}

# ── The families ──────────────────────────────────────────────────────────────

probes_shipped_config_boots() {
  bold "shipped configuration"

  # #2159: validate.sh rendered and linted; nothing ever STARTED what it
  # rendered, and none of the shipped values files produced a bootable server.
  probe "P-BOOT-01" "working" "compose" "#2159" \
    "the quickstart compose file boots to a serving CDR"
  if wait_http "$CDR/ferroehr/rest/status" 90; then
    local status; status="$(curl -s "$CDR/ferroehr/rest/status")"
    assert_contains "$status" '"status"' "the status document is served"
  else
    probe_fail "a serving CDR" "$( dc logs --tail 20 ferroehr 2>&1 | tail -5 )" \
      "the stack never answered /rest/status"
  fi
  probe_done

  probe "P-BOOT-02" "working" "image" "-" \
    "the health family answers without authentication"
  assert_eq "200" "$(http_code "$CDR/health/liveness")"
  assert_eq "200" "$(http_code "$CDR/health/readiness")"
  probe_done
}

probes_multimedia() {
  bold "multimedia (S3 externalization)"

  # #2169: the documented recipe exports FERROEHR__MULTIMEDIA__* in a shell.
  # Far end: the SERVER's own effective configuration, not the compose file.
  probe "P-MM-ENV" "working" "compose" "#2169" \
    "exported FERROEHR__MULTIMEDIA__* reaches the server"
  local env_doc; env_doc="$(curl -s -u "$BASIC" "$CDR/management/env")"
  assert_contains "$env_doc" '"endpoint":"http://seaweedfs:8333"' \
    "the compose file must pass the multimedia keys through from the shell"
  assert_contains "$env_doc" '"enabled":true'
  probe_done

  # #2168: the gateway ships with no bucket, and an S3 write into a missing one
  # answers 403 (not 404), so it reads as a credentials problem.
  probe "P-MM-BUCKET" "working" "compose" "#2168" \
    "the bucket exists with no manual step"
  assert_eq "200" "$(http_code -I "$S3/openehr-multimedia")" \
    "seaweedfs-init must create the bucket once the gateway is healthy"
  probe_done

  # The far-end observation the issue asks for: the blob is IN THE BUCKET,
  # under its content hash, and the stored record references it.
  probe "P-MM-OFFLOAD" "working" "server" "-" \
    "a large DV_MULTIMEDIA is externalized and the blob lands in the bucket"
  local uri
  PROBE_MEDIA_EHR="$(probe_commit_media_status)"
  local ehr="$PROBE_MEDIA_EHR" key=""
  if [[ -z "$ehr" ]]; then
    probe_fail "an EHR carrying a 400 KiB DV_MULTIMEDIA" "the commit did not return an id"
  else
    uri="$(curl -s -u "$BASIC" "$API/ehr/$ehr/ehr_status" \
      | tr ',' '\n' | grep -o 's3://openehr-multimedia/[0-9a-f]*' | head -1)"
    assert_contains "$uri" "s3://openehr-multimedia/" "the stored record must reference the blob"
    key="${uri##*/}"
    PROBE_MEDIA_KEY="$key"
    if [[ -n "$key" ]]; then
      assert_eq "200" "$(http_code "$S3/openehr-multimedia/$key")" \
        "the blob must be retrievable from the bucket by its content hash"
    fi
  fi
  probe_done

  # #2197: offload runs for every versioned object; re-inlining was wired to the
  # COMPOSITION read alone, so EHR_STATUS content had no way back.
  probe "P-MM-EXPAND" "working" "server" "#2197" \
    "?expand_multimedia=true returns the bytes on an EHR_STATUS read"
  if [[ -n "${ehr:-}" ]]; then
    local expanded data digest
    expanded="$(curl -s -u "$BASIC" "$API/ehr/$ehr/ehr_status?expand_multimedia=true")"
    data="$(printf '%s' "$expanded" \
      | jq -r '.other_details.items[0].value.data // empty' 2>/dev/null)"
    if [[ -z "$data" ]]; then
      probe_fail "a re-inlined DV_MULTIMEDIA.data" "no data member in the served value" \
        "the read must re-inline the blob, not answer with the compact reference"
    else
      # The strongest far-end check available: the bytes that came back must
      # hash to the content-addressed key the record references. That closes the
      # whole loop — committed, externalized under its SHA-256, fetched, and
      # re-inlined byte-identical — rather than trusting that a `data` member
      # appeared.
      digest="$(printf '%s' "$data" | base64 -d 2>/dev/null | shasum -a 256 | cut -d' ' -f1)"
      assert_eq "$key" "$digest" \
        "the re-inlined bytes must hash to the key the record references"
    fi
    # The `uri` deliberately SURVIVES expansion: RM DV_MULTIMEDIA's invariant is
    # `is_inline or is_external`, an OR, so carrying both is valid and keeps the
    # provenance reference. Asserting its absence would test a rule the spec
    # does not have.
  else
    probe_fail "an expandable record" "no EHR was committed"
  fi
  probe_done
}

probes_multimedia_restart() {
  bold "multimedia — persistence across a restart"

  # The case that separates a real object store from a temp directory: the
  # server process goes away and comes back, and the clinical content it
  # externalized is still retrievable byte-for-byte.
  #
  # It re-reads the record committed by P-MM-OFFLOAD rather than committing a
  # fresh one, because the property under test is that THAT content survived —
  # a new commit after the restart would prove only that the feature still
  # works, which is a different and weaker claim.
  probe "P-MM-RESTART" "working" "server" "-" \
    "externalized content survives a server restart, byte-for-byte"
  if [[ -z "${PROBE_MEDIA_EHR:-}" ]] || [[ -z "${PROBE_MEDIA_KEY:-}" ]]; then
    probe_fail "a record committed earlier in this run" "none was recorded" \
      "P-MM-OFFLOAD must run first — this probe deliberately re-reads its record"
    probe_done
    return 0
  fi

  dc restart ferroehr >/dev/null 2>&1
  if ! wait_http "$CDR/health/readiness" 120; then
    probe_fail "the CDR serving again after a restart" "readiness never returned"
    probe_done
    return 0
  fi

  # The blob is still in the store …
  assert_eq "200" "$(http_code "$S3/openehr-multimedia/$PROBE_MEDIA_KEY")" \
    "the object store outlives the server process"

  # … and the API still returns the bytes, verified against the same key.
  local data digest
  data="$(curl -s -u "$BASIC" "$API/ehr/$PROBE_MEDIA_EHR/ehr_status?expand_multimedia=true" \
    | jq -r '.other_details.items[0].value.data // empty' 2>/dev/null)"
  if [[ -z "$data" ]]; then
    probe_fail "the re-inlined bytes after a restart" "no data member in the served value" \
      "content committed before the restart must still be retrievable"
  else
    digest="$(printf '%s' "$data" | base64 -d 2>/dev/null | shasum -a 256 | cut -d' ' -f1)"
    assert_eq "$PROBE_MEDIA_KEY" "$digest" \
      "the bytes returned after the restart must hash to the original key"
  fi
  probe_done
}

probes_multimedia_off() {
  bold "multimedia — OFF (the default state)"

  # The default posture, and the state #2171 hid in: with externalization off a
  # large DV_MULTIMEDIA must be stored INLINE, byte-identical, with no
  # dependency on the object store at all. "Off" is the state most deployments
  # actually run, and it was never driven.
  #
  # Re-ups the CDR with the switch off; everything else is unchanged, so a
  # difference here is the switch and nothing else.
  probe "P-MM-OFF" "off" "server" "-" \
    "with externalization off, a large DV_MULTIMEDIA is stored inline"
  FERROEHR__MULTIMEDIA__ENABLED=false compose_up ferroehr
  if ! wait_http "$CDR/health/readiness" 90; then
    probe_fail "a serving CDR with multimedia off" "readiness never returned" \
      "turning an integration off must not stop the server starting"
    probe_done
    return 0
  fi

  local off_env; off_env="$(curl -s -u "$BASIC" "$CDR/management/env")"
  assert_contains "$off_env" '"enabled":false' "the switch must actually be off for this probe to mean anything"

  local ehr; ehr="$(probe_commit_media_status)"
  if [[ -z "$ehr" ]]; then
    probe_fail "a committed EHR" "the commit returned no id" \
      "an inline commit needs no object store and must succeed"
  else
    local stored; stored="$(curl -s -u "$BASIC" "$API/ehr/$ehr/ehr_status")"
    assert_contains "$stored" '"data"' "with the integration off the bytes stay in the record"
    assert_not_contains "$stored" 's3://' "nothing may be externalized while the switch is off"
  fi
  probe_done

  # Put the stack back the way the rest of the run expects it.
  compose_up ferroehr
  wait_http "$CDR/health/readiness" 90 || true
}

probes_multimedia_broken() {
  bold "multimedia — dependency broken"

  # The state least often tested, and where a system either fails loudly or
  # loses data quietly. With the store gone, a commit that must offload has to
  # be REFUSED, not half-stored.
  probe "P-MM-BROKEN" "broken" "server" "-" \
    "with the object store stopped, a commit that must offload is refused"
  dc stop seaweedfs >/dev/null 2>&1
  local code; code="$(probe_commit_media_status_code)"
  case "$code" in
    5*) : ;;
    *)  probe_fail "a 5xx refusal" "$code" \
          "an unreachable blob store must fail the commit, never store a half record" ;;
  esac
  dc start seaweedfs >/dev/null 2>&1
  wait_http "$S3/" 60 || true
  probe_done
}

probes_health_broken() {
  bold "health — dependency broken"

  # Readiness genuinely going unready. A probe that only ever returns UP is
  # worse than none, and this is the shape that keeps a dead pod in rotation.
  probe "P-HEALTH-BROKEN" "broken" "server" "-" \
    "database stopped: readiness 503, liveness still 200, no restart"
  dc stop ferroehr-postgres >/dev/null 2>&1
  if wait_status "$CDR/health/readiness" "503" 30; then
    assert_eq "200" "$(http_code "$CDR/health/liveness")" \
      "liveness must be process-local — restarting cannot fix a dependency"
    local body; body="$(curl -s "$CDR/health/readiness")"
    assert_contains "$body" '"status":"DOWN"' "readiness must name the failing component"
  else
    probe_fail "readiness 503 within 60s" "$(curl -s -o /dev/null -w '%{http_code}' "$CDR/health/readiness")" \
      "a readiness probe that never fails cannot remove a pod from rotation"
  fi
  dc start ferroehr-postgres >/dev/null 2>&1
  wait_http "$CDR/health/readiness" 90 || true
  probe_done
}

probes_management() {
  bold "management surface"

  # #2177: the per-endpoint levels are the ONLY authority, and they are
  # INDEPENDENT. The quickstart ships four different levels at once, which is
  # exactly the shape a per-endpoint guard gets wrong:
  #
  #   prometheus = public       env    = admin_only
  #   info       = private      (flamegraph names no level ⇒ off)
  #
  # A guard that applied the most permissive configured level to every route —
  # or the strictest — passes a one-endpoint-at-a-time test and fails here.
  probe "P-MGMT-LEVELS" "working" "server" "#2177" \
    "each management endpoint answers at its own level, anonymous and authenticated"
  assert_eq "200" "$(http_code "$CDR/management/prometheus")" \
    "a public endpoint is served OUTSIDE authentication"
  assert_eq "401" "$(http_code "$CDR/management/info")" \
    "a private endpoint must challenge an anonymous caller"
  assert_eq "401" "$(http_code "$CDR/management/env")" \
    "an admin_only endpoint must challenge an anonymous caller"
  # The quickstart's Basic user carries ADMIN, so both open for it.
  assert_eq "200" "$(http_code -u "$BASIC" "$CDR/management/info")"
  assert_eq "200" "$(http_code -u "$BASIC" "$CDR/management/env")"
  probe_done

  # An endpoint that names no level is not mounted — 404, not 401. Its absence
  # is not a credential problem, and there is no global default that could open
  # it by accident.
  probe "P-MGMT-OFF" "off" "server" "#2177" \
    "an endpoint naming no level is not mounted, even for an admin"
  assert_eq "404" "$(http_code -u "$BASIC" "$CDR/management/flamegraph")" \
    "an unnamed endpoint must answer 404 rather than fall back to a server default"
  probe_done
}

# The management surface on its OWN listener — the second of the two
# configurations #2162 asks for, and the one a production deployment actually
# uses, because it is what keeps ops introspection off the public port.
#
# The property is not "the port answers" but that the surface MOVED: served on
# the management port and NO LONGER on the API port. A probe that only checked
# the new port would pass just as happily on a server that exposed the surface
# on both, which is the exact misconfiguration this option exists to prevent.
probes_management_separate_listener() {
  bold "management surface — its own listener"

  local overlay="$PROBE_TMP/mgmt-port.yml"
  local mport="${PROBE_MGMT_PORT:-19090}"
  cat > "$overlay" <<YAML
services:
  ferroehr:
    ports:
      - "127.0.0.1:${mport}:9090"
    environment:
      FERROEHR__MANAGEMENT__PORT: "9090"
YAML

  probe "P-MGMT-PORT" "working" "server" "#2162" \
    "with management.port set, the surface is served from its own listener"
  dc -f docker-compose.yml -f "$overlay" up -d ferroehr >/dev/null 2>&1
  if ! wait_http "$CDR/health/readiness" 120; then
    probe_fail "a serving CDR with a separate management listener" \
      "$(dc logs --tail 5 ferroehr 2>&1 | tail -3)" \
      "a second listener that fails to bind takes the whole process down at boot"
    probe_done
    dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
    wait_http "$CDR/health/readiness" 120 || true
    return 0
  fi
  local mgmt="http://localhost:${mport}"
  if ! wait_http "$mgmt/management/prometheus" 30; then
    probe_fail "the management surface answering on its own port" \
      "$(http_code "$mgmt/management/prometheus")" \
      "management.port is documented to move the surface to a separate listener"
  fi
  probe_done

  # The half that makes the option worth having.
  probe "P-MGMT-PORT-MOVED" "broken" "server" "#2162" \
    "the surface is NO LONGER on the API listener once it has its own port"
  local on_api
  on_api="$(http_code "$CDR/management/prometheus")"
  case "$on_api" in
    404|403|401) : ;;
    200) probe_fail "the API port to stop serving /management" "$on_api" \
           "serving it on both ports defeats the point: the surface is still reachable from the public listener" ;;
    *)   probe_fail "404 from the API port" "$on_api" \
           "the API listener must not serve a surface that has moved" ;;
  esac
  probe_done

  # Back to the shipped posture for anything that follows.
  dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
  wait_http "$CDR/health/readiness" 120 || true
}

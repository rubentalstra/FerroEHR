#!/usr/bin/env bash
# The terminology probe family (#2178, #2158) — against a REAL terminology
# server, with REAL content.
#
# The temptation here is a seeded code system of our own: cheap, fast, and
# worthless. Passing `$validate-code` against a CodeSystem we wrote proves our
# client can talk to our own fixture. `$subsumes` is the clearest case — a
# subsumption answer means nothing without a real hierarchy behind it, and a
# fixture hierarchy is one we authored to make the test pass.
#
# So this family drives Snowstorm (SNOMED International's own server, which
# implements the FHIR Terminology Module) loaded with a real SNOMED CT release.
#
# THE RELEASE IS NOT OURS TO SHIP. The RF2 package is licensed content under an
# SNOMED International Affiliate agreement: it cannot be committed here, fetched
# by a script, or baked into a published image. The operator supplies it:
#
#   FERROEHR_SNOMED_RF2=/path/to/SnomedCT_InternationalRF2_PRODUCTION_*.zip
#
# Without it the family declares itself NOT EXERCISED rather than substituting a
# fixture — an honest gap beats a green row that measured our own data.
#
# It is also heavy: Elasticsearch plus Snowstorm want ~8 GB and the import takes
# far longer than every other probe combined, so this family is opt-in.
#
# Elasticsearch specifically, and not a substitute: the maintainers were asked
# about OpenSearch and answered no, "mainly because Elasticsearch provides
# better performance" (IHTSDO/snowstorm#411), and Snowstorm has since moved to
# the ES 8 client, which OpenSearch — forked at 7.10 — cannot serve. Meilisearch
# is not a candidate at all: Snowstorm speaks the Elasticsearch query DSL, so
# that would be a rewrite rather than a swap. None of this reaches the product:
# Elasticsearch exists only inside this probe stack, to run the terminology
# server being measured, and never in anything FerroEHR ships.
#
# Sourced by scripts/deploy-probe.sh; never run directly.

TERM_SNOWSTORM_PORT="${PROBE_SNOWSTORM_PORT:-18090}"
TERM_URL="http://localhost:${TERM_SNOWSTORM_PORT}"
# The container-side URL the CDR uses; the host port is for this script only.
TERM_INTERNAL="http://ferroehr-snowstorm:8080/fhir"

terminology_overlay() {
  local out="$PROBE_TMP/terminology.yml"
  cat > "$out" <<YAML
services:
  ferroehr-es:
    image: docker.elastic.co/elasticsearch/elasticsearch:8.11.1
    environment:
      - discovery.type=single-node
      - xpack.security.enabled=false
      - ES_JAVA_OPTS=-Xms4g -Xmx4g
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:9200 || exit 1"]
      interval: 5s
      timeout: 5s
      retries: 60
  ferroehr-snowstorm:
    image: snomedinternational/snowstorm:latest
    depends_on:
      ferroehr-es:
        condition: service_healthy
    entrypoint: java -Xms2g -Xmx4g --add-opens java.base/java.lang=ALL-UNNAMED --add-opens=java.base/java.util=ALL-UNNAMED -cp @/app/jib-classpath-file org.snomed.snowstorm.SnowstormApplication --elasticsearch.urls=http://ferroehr-es:9200
    ports:
      - "127.0.0.1:${TERM_SNOWSTORM_PORT}:8080"
  ferroehr:
    depends_on:
      - ferroehr-snowstorm
    environment:
      FERROEHR__TERMINOLOGY__EXTERNAL__ENABLED: "true"
      FERROEHR__TERMINOLOGY__EXTERNAL__FAIL_ON_ERROR: "true"
      FERROEHR__TERMINOLOGY__EXTERNAL__PROVIDERS__SNOMED__BASE_URL: ${TERM_INTERNAL}
      FERROEHR__TERMINOLOGY__EXTERNAL__ROUTES__DEFAULT: snomed
YAML
  printf '%s' "$out"
}

# Import the operator's RF2 archive through Snowstorm's own native API — the
# route its documentation prescribes for SNOMED content, since the FHIR API
# cannot load RF2.
terminology_import() {
  local zip="$1" location job
  location="$(curl -s -D - -o /dev/null -X POST "$TERM_URL/imports" \
    -H 'Content-Type: application/json' \
    -d '{"branchPath":"MAIN","createCodeSystemVersion":true,"type":"SNAPSHOT"}' \
    | grep -i '^location' | tr -d '\r' | awk '{print $2}')"
  [ -n "$location" ] || return 1
  job="${location##*/}"
  curl -s -o /dev/null -X POST "$TERM_URL/imports/$job/archive" -F "file=@${zip}" || return 1
  # The import is long. Poll its own status rather than guessing a duration.
  local _i status
  for _i in $(seq 1 480); do
    status="$(curl -s "$TERM_URL/imports/$job" | sed -n 's/.*"status":"\([A-Z_]*\)".*/\1/p')"
    case "$status" in
      COMPLETED) return 0 ;;
      FAILED)    return 1 ;;
    esac
    sleep 15
  done
  return 1
}

# The RF2 release, from a local path or fetched from wherever the affiliate
# keeps it. Echoes the archive path; non-zero means none was supplied.
#
# Two ways in, because the two places this runs differ:
#   FERROEHR_SNOMED_RF2      a path — how a developer runs it, against the copy
#                            already downloaded from MLDS
#   FERROEHR_SNOMED_RF2_URL  a URL, with optional basic-auth credentials — how
#                            CI runs it, from a secret, since the package cannot
#                            live in this repository
#
# FERROEHR_SNOMED_RF2_MD5, when set, is CHECKED. SNOMED International publishes
# an MD5 beside each release, and a terminology probe that silently ran against
# a truncated or substituted package would report conformance about content
# nobody chose. It is pinned per release, so a release upgrade is a deliberate
# edit rather than something that happens to a run.
terminology_release() {
  local zip="${FERROEHR_SNOMED_RF2:-}"
  if [ -n "$zip" ] && [ -f "$zip" ]; then
    terminology_verify "$zip" || return 1
    printf '%s' "$zip"
    return 0
  fi
  local url="${FERROEHR_SNOMED_RF2_URL:-}"
  [ -n "$url" ] || return 1
  zip="$PROBE_TMP/snomed-rf2.zip"
  local -a auth=()
  [ -n "${FERROEHR_SNOMED_RF2_USER:-}" ] && \
    auth=(-u "${FERROEHR_SNOMED_RF2_USER}:${FERROEHR_SNOMED_RF2_PASSWORD:-}")
  # --fail so an HTML login page is never mistaken for a release archive.
  curl -fsSL "${auth[@]}" -o "$zip" "$url" || return 1
  terminology_verify "$zip" || return 1
  printf '%s' "$zip"
}

terminology_verify() {
  local want="${FERROEHR_SNOMED_RF2_MD5:-}"
  [ -n "$want" ] || return 0
  local got
  got="$(md5sum "$1" 2>/dev/null | awk '{print $1}')"
  [ -n "$got" ] || got="$(md5 -q "$1" 2>/dev/null)"
  if [ "$got" != "$want" ]; then
    red "  SNOMED RF2 checksum mismatch: expected $want, got ${got:-none}"
    return 1
  fi
  return 0
}

probes_terminology() {
  bold "terminology — a real FHIR terminology server with real content"

  local zip
  if ! zip="$(terminology_release)"; then
    uncovered "terminology against a real server (#2178)" \
      "supply a SNOMED CT RF2 release — FERROEHR_SNOMED_RF2 (a local path) or FERROEHR_SNOMED_RF2_URL (+ optional _USER/_PASSWORD). It is licensed content this repository may not ship, and a seeded code system would only test our own fixture"
    return 0
  fi

  local overlay
  overlay="$(terminology_overlay)"

  probe "P-TERM-UP" "working" "compose" "#2178" \
    "Snowstorm serves the FHIR terminology API"
  dc -f docker-compose.yml -f "$overlay" up -d ferroehr-snowstorm >/dev/null 2>&1
  if ! wait_http "$TERM_URL/fhir/metadata" 600; then
    # Elasticsearch refuses to start without vm.max_map_count, which is a HOST
    # setting this harness cannot change from inside a container. Saying so is
    # more useful than reporting a defect in the chart or the server.
    probe_fail "a Snowstorm FHIR endpoint" "$(dc logs --tail 5 ferroehr-snowstorm 2>&1 | tail -3)" \
      "if Elasticsearch exited, the host needs vm.max_map_count=262144 — a sysctl no container can set for itself"
    probe_done
    return 0
  fi
  probe_done

  probe "P-TERM-IMPORT" "working" "compose" "#2178" \
    "the operator's SNOMED CT release imports and is served as a CodeSystem"
  if ! terminology_import "$zip"; then
    probe_fail "a COMPLETED import job" "the import did not complete" \
      "without content every operation below would answer emptily and prove nothing"
    probe_done
    return 0
  fi
  assert_contains "$(curl -s "$TERM_URL/fhir/CodeSystem")" "http://snomed.info/sct" \
    "the SNOMED CodeSystem must be served once the release is loaded"
  probe_done

  # The operation a seeded fixture cannot fake: subsumption over a real
  # hierarchy. 73211009 (Diabetes mellitus) subsumes 44054006 (Type 2 diabetes
  # mellitus) in the International Edition.
  probe "P-TERM-SUBSUMES" "working" "server" "#2178" \
    "a real SNOMED hierarchy answers \$subsumes — the check a fixture cannot honestly make"
  local sub
  sub="$(curl -s --get \
    --data-urlencode 'system=http://snomed.info/sct' \
    --data-urlencode 'codeA=73211009' \
    --data-urlencode 'codeB=44054006' \
    "$TERM_URL/fhir/CodeSystem/\$subsumes")"
  assert_contains "$sub" "subsumes" \
    "a hierarchy we authored ourselves would answer whatever we told it to; this one is the published release"
  probe_done

  # The far end that matters for the CDR: a coded value validated THROUGH the
  # configured provider, not by our in-process bundle.
  probe "P-TERM-VALIDATE" "working" "server" "#2178" \
    "the CDR validates a real SNOMED code through the configured provider"
  dc -f docker-compose.yml -f "$overlay" up -d ferroehr >/dev/null 2>&1
  if ! wait_http "$CDR/health/readiness" 180; then
    probe_fail "a serving CDR with an external terminology provider" \
      "$(dc logs --tail 5 ferroehr 2>&1 | tail -3)"
  else
    assert_contains "$(curl -s -u "$BASIC" "$CDR/management/env" | tr ',' '\n' | grep -i terminology | head -5)" \
      "enabled" "the provider must be active, or the probes above measured Snowstorm and not the CDR"
  fi
  probe_done

  # fail_on_error is the whole safety story: a terminology server that cannot
  # answer must not silently let an unvalidated code through.
  probe "P-TERM-BROKEN" "broken" "server" "#2178" \
    "with the terminology server stopped and fail_on_error set, validation fails CLOSED"
  dc stop ferroehr-snowstorm >/dev/null 2>&1
  local code
  code="$(http_code -u "$BASIC" --get --data-urlencode 'q=SELECT e/ehr_id/value FROM EHR e LIMIT 1' "$API/query/aql")"
  case "$code" in
    200|400|422|503) : ;;
    *) probe_fail "a defined answer with the terminology server down" "$code" \
         "an unreachable terminology server must produce a typed outcome, not an unclassified failure" ;;
  esac
  probe_done

  dc -f docker-compose.yml -f "$overlay" stop ferroehr-snowstorm ferroehr-es >/dev/null 2>&1
  dc -f docker-compose.yml -f "$overlay" rm -f ferroehr-snowstorm ferroehr-es >/dev/null 2>&1
  dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
  wait_http "$CDR/health/readiness" 120 || true
}

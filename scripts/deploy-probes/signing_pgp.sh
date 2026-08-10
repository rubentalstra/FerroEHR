#!/usr/bin/env bash
# The PGP signing probe family (#2163's second mode).
#
# #2163 is explicit that the PGP path must be exercised "with a real key,
# including the passphrase-file route, since that is the documented production
# shape". So this generates a real OpenPGP key, mounts it the way the
# configuration reference instructs — `signing.key_path` for the armored secret
# key, `signing.key_passphrase_file` for its passphrase — and drives the same
# assertions the digest family does, including tamper detection.
#
# The key is generated into a THROWAWAY keyring under the run's temp directory:
# a probe must never touch the operator's own GnuPG home.
#
# Sourced by scripts/deploy-probe.sh; never run directly.

PGP_PASS="probe-passphrase-not-a-secret"

# Generate an armored secret key + its passphrase file, echoing the directory
# that holds them. Empty output means gpg was unavailable or refused.
pgp_material() {
  local dir="$PROBE_TMP/pgp"
  mkdir -p "$dir" "$dir/gnupg"
  chmod 700 "$dir/gnupg"
  local fpr
  GNUPGHOME="$dir/gnupg" gpg --batch --pinentry-mode loopback \
    --passphrase "$PGP_PASS" --quick-generate-key \
    "FerroEHR Probe <probe@example.test>" default default never >/dev/null 2>&1 || return 1
  fpr="$(GNUPGHOME="$dir/gnupg" gpg --batch --list-secret-keys --with-colons 2>/dev/null \
    | awk -F: '/^fpr:/ {print $10; exit}')"
  [ -n "$fpr" ] || return 1
  GNUPGHOME="$dir/gnupg" gpg --batch --pinentry-mode loopback \
    --passphrase "$PGP_PASS" --armor --export-secret-keys "$fpr" > "$dir/signing.asc" 2>/dev/null || return 1
  [ -s "$dir/signing.asc" ] || return 1
  printf '%s' "$PGP_PASS" > "$dir/passphrase"
  # The container runs as the distroless nonroot uid; these are throwaway probe
  # credentials, so world-readable is the simplest way to guarantee the mount is
  # readable regardless of host uid mapping.
  chmod 644 "$dir/signing.asc" "$dir/passphrase"
  printf '%s' "$dir"
}

# An overlay mounting that material and switching the server to pgp mode. It is
# GENERATED rather than committed: the key does not exist until the run makes
# one, and a committed signing key in a repository is its own defect.
pgp_overlay() {
  local dir="$1" out="$PROBE_TMP/pgp-overlay.yml"
  cat > "$out" <<YAML
services:
  ferroehr:
    volumes:
      - $dir/signing.asc:/etc/ferroehr/signing.asc:ro
      - $dir/passphrase:/run/secrets/pgp-pass:ro
    environment:
      FERROEHR__SIGNING__ENABLED: "true"
      FERROEHR__SIGNING__MODE: pgp
      FERROEHR__SIGNING__KEY_PATH: /etc/ferroehr/signing.asc
      FERROEHR__SIGNING__KEY_PASSPHRASE_FILE: /run/secrets/pgp-pass
YAML
  printf '%s' "$out"
}

probes_signing_pgp() {
  bold "VERSION signing (PGP mode, key + passphrase from mounted files)"

  if ! command -v gpg >/dev/null 2>&1; then
    uncovered "signing in PGP mode (#2163)" "gpg is not available on this host"
    return 0
  fi
  local dir overlay
  if ! dir="$(pgp_material)" || [ -z "$dir" ]; then
    uncovered "signing in PGP mode (#2163)" "this host's gpg would not generate a probe key"
    return 0
  fi
  overlay="$(pgp_overlay "$dir")"

  # `pgp` mode fails CLOSED at boot when the key is missing or unusable, so the
  # server coming up at all is the first assertion — and the documented
  # production shape (a mounted key, a passphrase from its *_file sibling) is
  # what is being mounted here.
  probe "P-PGP-BOOT" "working" "server" "#2163" \
    "the server boots in pgp mode with the key and passphrase from mounted files"
  dc -f docker-compose.yml -f "$overlay" up -d ferroehr >/dev/null 2>&1
  if ! wait_http "$CDR/health/readiness" 120; then
    probe_fail "a serving CDR in pgp mode" "$(dc logs --tail 5 ferroehr 2>&1 | tail -3)" \
      "pgp mode fails closed at boot, so this is the key or the passphrase route"
    probe_done
    return 0
  fi
  probe_done

  probe "P-PGP-SIGN" "working" "server" "#2163" \
    "a version committed under pgp carries a signature and verifies on read"
  local ehr version
  ehr="$(curl -s -u "$BASIC" -X POST -D - -o /dev/null "$API/ehr" \
    | grep -i '^location' | tr -d '\r' | awk -F/ '{print $NF}')"
  if [ -z "$ehr" ]; then
    probe_fail "a committed EHR" "no id returned"
    probe_done
    return 0
  fi
  version="$(curl -s -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")"
  assert_contains "$version" '"signature"' "a pgp-signed version must carry its detached signature"
  # An OpenPGP detached signature is armored; digest mode's is not. Asserting
  # the armor is what distinguishes "pgp is really in use" from "signing is on".
  assert_contains "$version" "BEGIN PGP SIGNATURE" \
    "without the armor this is not an OpenPGP signature, whatever the mode says"
  assert_eq "200" "$(http_code -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")" \
    "an untampered pgp-signed version verifies on read"
  probe_done

  # Detection, in the mode where it is hardest to fake.
  probe "P-PGP-TAMPER" "broken" "server" "#2163" \
    "a tampered pgp-signed version is REFUSED on read"
  local matched after
  probe_psql "UPDATE ehr.node
                 SET data = jsonb_set(data, '{name,value}', '\"tampered\"')
               WHERE ehr_id = '$ehr'::uuid AND rm_type = 'EHR_STATUS';" >/dev/null
  matched="$(probe_psql "SELECT count(*) FROM ehr.node
                          WHERE ehr_id = '$ehr'::uuid
                            AND data #>> '{name,value}' = 'tampered';")"
  if [ "${matched:-0}" = "0" ]; then
    probe_fail "a tampered stored row" "the UPDATE matched nothing" \
      "detection was never tested — the probe could not reach the stored content"
  else
    after="$(http_code -u "$BASIC" "$API/ehr/$ehr/versioned_ehr_status/version")"
    case "$after" in
      5*) : ;;
      *)  probe_fail "a 5xx integrity refusal" "$after" \
            "verify_on_read is strict by default; a corrupt pgp-signed record must not be served" ;;
    esac
  fi
  probe_done

  # Put the stack back on the shipped posture for anything that follows.
  dc -f docker-compose.yml up -d ferroehr >/dev/null 2>&1
  wait_http "$CDR/health/readiness" 120 || true
}

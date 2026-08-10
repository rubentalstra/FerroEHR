#!/usr/bin/env bash
# The OIDC / Keycloak probe family.
#
# Driven through the PUBLISHED quickstart overlay and the recipe the book gives
# the reader — a password-grant token fetch, then an API call with it. #2176 was
# exactly that recipe returning 401 forever, because the demo realm never put
# `ferroehr` in the token's `aud` while the same overlay required that audience.
# Two halves of one shipped file contradicting each other, and no test could see
# it because neither half is code.
#
# Sourced by scripts/deploy-probe.sh; never run directly.

export KEYCLOAK_PORT="${PROBE_KC_PORT:-18081}"
KC="http://localhost:${KEYCLOAK_PORT}"
KC_REALM="$KC/auth/realms/ferroehr"

# The stack for this family: the quickstart PLUS the Keycloak overlay, which is
# how the book tells an operator to get an issuer.
oidc_up() {
  dc -f docker-compose.yml -f docker-compose.keycloak.yml up -d ferroehr keycloak >/dev/null 2>&1
}

# The middle segment of a JWT, decoded. `base64 -d` wants padding, and a JWT is
# base64url without it — so pad to a multiple of 4 and translate the alphabet.
jwt_claims() {
  local token="$1" payload
  payload="$(printf '%s' "$token" | cut -d. -f2 | tr '_-' '/+')"
  case $(( ${#payload} % 4 )) in
    2) payload="${payload}==" ;;
    3) payload="${payload}=" ;;
  esac
  printf '%s' "$payload" | base64 -d 2>/dev/null
}

probes_oidc() {
  bold "OIDC (Keycloak)"

  oidc_up
  if ! wait_http "$KC_REALM/.well-known/openid-configuration" 150; then
    probe "P-OIDC-UP" "working" "compose" "-" "the quickstart overlay starts an issuer"
    probe_fail "a Keycloak realm answering discovery" "no response after 300s" \
      "the overlay's own healthcheck gates the CDR, so this is the overlay, not the realm"
    probe_done
    return 0
  fi
  wait_http "$CDR/health/readiness" 120 || true

  # The documented recipe, verbatim: fetch a token with the password grant.
  local token
  token="$(curl -s -d client_id=ferroehr -d client_secret=ferroehr-quickstart-secret \
    -d username=ferroehr -d password=ferroehr -d grant_type=password \
    "$KC_REALM/protocol/openid-connect/token" | sed -n 's/.*"access_token":"\([^"]*\)".*/\1/p')"

  # #2176, far end: the CLAIM, not the status code. The realm must actually put
  # this server in the audience — a token that omits it is refused by every
  # correct resource server (RFC 7519 §4.1.3), which is what made the shipped
  # quickstart impossible rather than merely misconfigured.
  probe "P-OIDC-AUD" "working" "compose" "#2176" \
    "the demo realm mints a token carrying aud=ferroehr"
  if [ -z "$token" ]; then
    probe_fail "an access token from the password grant" "the token endpoint returned none" \
      "the realm's client must have the password grant enabled"
  else
    assert_contains "$(jwt_claims "$token")" '"aud":"ferroehr"' \
      "without an audience mapper the realm cannot mint a token this server accepts"
  fi
  probe_done

  # And the whole point: the documented flow SUCCEEDS.
  probe "P-OIDC-ACCEPT" "working" "server" "#2176" \
    "the API accepts that token — the documented quickstart works end to end"
  if [ -z "$token" ]; then
    probe_fail "201 from POST /ehr" "no token to present"
  else
    local code
    code="$(curl -s -o /dev/null -w '%{http_code}' -X POST \
      -H "Authorization: Bearer $token" "$API/ehr")"
    assert_eq "201" "$code" \
      "a token the issuer minted for this audience must be accepted"
  fi
  probe_done

  # A garbage bearer is refused — the negative twin, so the probe above cannot
  # pass because authentication is off altogether.
  probe "P-OIDC-REFUSE" "working" "server" "-" \
    "a bearer token that is not the issuer's is refused"
  assert_eq "401" "$(http_code -X POST -H 'Authorization: Bearer not-a-token' "$API/ehr")" \
    "acceptance means nothing if an invalid token is also accepted"
  probe_done
}

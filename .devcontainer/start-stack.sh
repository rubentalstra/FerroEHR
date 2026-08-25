#!/usr/bin/env bash
# SPDX-FileCopyrightText: FerroEHR contributors
# SPDX-License-Identifier: MIT
#
# Codespace start-time boot (#2709): bring the published quickstart stack up
# and wait until the server reports healthy, then print where everything is.
# Runs on every container start (postStartCommand), so a resumed Codespace
# comes back serving without any manual step.
set -Eeuo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

docker compose -f docker-compose.yml --profile admin-ui up -d --wait

cat <<'EOF'

FerroEHR is up.

  API + Swagger UI   http://localhost:8080/ferroehr/rest/swagger-ui
  Health             http://localhost:8080/health
  Admin console      http://localhost:3000
  Credentials        ferroehr / ferroehr

In a Codespace, open the forwarded ports 8080 and 3000 from the PORTS panel.
A two-minute walkthrough (create an EHR, upload a template, commit a
composition, query it back) is in the docs: https://ferroehr.eu/docs/latest/getting-started.html
EOF

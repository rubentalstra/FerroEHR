# Vendored openEHR ITS material (all from the individual spec repos' master, 2026-07-03)

Fetched directly from each `specifications-ITS-*` repo's `master` (NOT the
`specifications-ITS` umbrella submodules, which can lag years behind).

| Surface | Repo | master SHA | Vendored to |
|---|---|---|---|
| ITS-JSON schema | specifications-ITS-JSON | `5acae056…` (frozen 2021; development status, latest available) | `../schemas/openehr_rm_1.1.0_all.json` |
| ITS-XML XSDs    | specifications-ITS-XML  | `de8b37ba…` | `../schemas/xml/components/` (RM ≤1.1.0, BASE ≤1.2.0, AM 1.4, OET 1.0.1) |
| ITS-REST OAS    | specifications-ITS-REST | `e8a093e9…` | `rest-oas/*.openapi.yaml` (all 7 API groups × codegen/html/validation = 21 files) |
| ITS-BMM meta-model | specifications-ITS-BMM | `37b31739…` | `../../openehr-codegen/vendor/bmm/*.bmm.json` (codegen input) |

Related (verified at latest master the same day):
- QUERY grammar + examples @ `10cb73fe…` → `../../openehr-query/vendor/`
- TERM terminology XML @ `007d0dd…` → `../../openehr-term/assets/`

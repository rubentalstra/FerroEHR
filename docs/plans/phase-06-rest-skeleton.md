# Phase 06 — REST skeleton (axum)

- Status: not-started
- Started: -   Owner: Ruben
- Consumes (spec/layer): ITS-REST 1.0.3; RM, serde (Phases 03-05)
- Compile required: no (Phase A)

## Objectives

Stand up the axum route skeleton matching every ITS-REST 1.0.3 endpoint plus
EHRbase-specific additions (Admin API, item tags, EhrScape aliases), with
request/response DTOs and `utoipa` annotations generating an OpenAPI spec —
handlers can `todo!()` their bodies at this stage.

## Preconditions

- [ ] Phase 03 done: RM types exist for DTOs to reference
- [ ] Phase 04 done: canonical JSON available for response bodies

## Scope

In: route definitions and DTOs for EHR, EHR_STATUS, COMPOSITION,
DIRECTORY/FOLDER, CONTRIBUTION, QUERY (`/aql` + stored queries), DEFINITION
(`/template/adl1.4`, `/template/adl2`), Admin API (`/rest/admin`),
`/rest/status`, `/management/*`, Item Tags, `utoipa`-generated OpenAPI +
Swagger UI.
Out: actual handler logic (bodies are `todo!()` until Phases 11-16 wire real
behavior in), EhrScape endpoints proper (Phase 16 / `openehr-ehrbase-compat`).

## Tasks

- [ ] Scaffold the axum router in `openehr-rest` under base path `/ehrbase/rest/openehr/v1`
- [ ] Define EHR and EHR_STATUS routes + DTOs (create, get, update, versioned-object endpoints)
- [ ] Define COMPOSITION routes + DTOs (create, get, update, delete, versioned-object endpoints)
- [ ] Define DIRECTORY/FOLDER and CONTRIBUTION routes + DTOs
- [ ] Define QUERY routes + DTOs: ad-hoc `/aql` and stored-query CRUD/execute
- [ ] Define DEFINITION routes for `/template/adl1.4` and `/template/adl2` (the latter mostly 501 per the spec, matching EHRbase)
- [ ] Define EHRbase-specific routes: Admin API (`/rest/admin`), `/rest/status`, `/management/*`, experimental Item Tags
- [ ] Wire `utoipa` + `utoipa-axum` annotations on every route; serve Swagger UI via `utoipa-swagger-ui`
- [ ] Add PORT STATUS trailers referencing the EHRbase controller each route mirrors; note `todo!()` handler bodies with `// TODO(port):`

## Exit criteria

- [ ] Every ITS-REST 1.0.3 endpoint plus the EHRbase additions listed above has a route and DTO pair in `openehr-rest`
- [ ] `utoipa`-generated OpenAPI spec is servable and Swagger UI renders it
- [ ] Handler bodies are stubbed with `todo!()` and `// TODO(port):`, not silently omitted

## Decisions made this phase

- (none recorded yet)

## Handoff for next session

Not started. This phase receives EHRbase's `rest-openehr` Java (moved in
Phase 00's `git mv`); port controller-by-controller, keeping the Java file in
place next to its `.rs` counterpart until Phase 17/18 reach parity for that
controller.

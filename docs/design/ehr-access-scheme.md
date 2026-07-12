# EHR_ACCESS realization — the `ehrbase.access_control.v1` scheme

**Status:** designed 2026-07-12 (W-3b T4); implementation in the same phase.

## Spec grounding (what openEHR actually requires)

- `EHR_ACCESS` is a mandatory, versioned component of every EHR
  (`EHR.ehr_access: OBJECT_REF 1..1`, invariant `Ehr_access_valid`; RM UML
  `org.openehr.rm.ehr.ehr.adoc` §EHR Class), change-controlled like all other
  content ("All changes to the EHR Access object are versioned via the normal
  mechanism" — RM ehr master04 §EHR Access).
- `EHR_ACCESS.settings: ACCESS_CONTROL_SETTINGS [0..1]`;
  `scheme(): String` non-empty (`Scheme_valid`), naming the scheme of the
  concrete settings instance (RM UML `org.openehr.rm.ehr.ehr_access.adoc`).
- "All access decisions to data in the EHR must be made in accordance with
  the policies and rules in this object" (ibid.) — **but**
  `ACCESS_CONTROL_SETTINGS` is abstract and attribute-less, and **no
  published openEHR specification defines any concrete subtype or scheme**
  ("Currently implementation dependent" — RM UML
  `org.openehr.rm.ehr.access_control_settings.adoc`; the Architecture
  Overview itself notes "there is currently no published formal, proven
  model of access control for shared health information",
  BASE architecture_overview master07 §Access Control).
- The SM places authentication/authorisation out of band (SM openehr_platform
  master02, "Authentication and authorisation is assumed to have been dealt
  with before any particular call"); ITS-REST defines no EHR_ACCESS endpoint
  (the EHR resource carries the ref only) and mandates only 401/403
  discipline when an auth framework is present.
- The Architecture Overview's policy prose (master07 §Access Control)
  describes what a scheme should provide: an access list (identified
  individuals and categories), a **gate-keeper** who controls changes to the
  access settings, per-Composition **privacy levels** whose definitions are
  jurisdiction-defined, and "sensible defaults".

**Consequence:** everything below the store/version/audit obligation is an
extension. *No openEHR spec governs the concrete scheme — this document is
our own design*, realizing the master07 policy prose so that the EHR_ACCESS
gateway clause stops being dead weight.

## The scheme

`EHR_ACCESS.settings` instances of this scheme are canonical-JSON objects:

```json
{
  "_type": "EHRBASE_ACCESS_CONTROL_V1",
  "gate_keeper": "user:alice",
  "default_access": "open",
  "access_list": [
    { "principal": "user:bob",   "access": "full" },
    { "principal": "role:nurse", "access": "restricted_below", "max_level": 2 }
  ],
  "privacy": {
    "default_level": 0,
    "composition_overrides": [
      { "uid": "8849182c-82ad-4088-a07f-48ead4180515", "level": 3 }
    ]
  }
}
```

- `scheme()` = `ehrbase.access_control.v1` (derived from the settings
  `_type`; `Scheme_valid` holds — an EHR_ACCESS without settings reports the
  scheme name with `default` semantics).
- **Principals**: `user:<authenticated id>` (Basic username / OIDC subject)
  or `role:<name>` (matched against the authenticated principal's roles —
  master07's "categories"). Evaluation happens *after* authentication, in
  the REST access layer — consistent with the SM's out-of-band placement.
- **`default_access`**: `open` (default; every existing EHR keeps working —
  master07 "sensible defaults") or `restricted` (only access-list matches
  may touch the EHR).
- **`access_list`**: first match wins; `full` = no privacy ceiling,
  `restricted_below` = may read compositions with privacy level
  < `max_level` only.
- **`privacy`**: integer levels, meaning deliberately deployment-defined
  (master07: "the definition of the privacy levels is not hard-wired in the
  openEHR models but rather is defined by standards or agreements within
  jurisdictions of use"). `composition_overrides` pin levels per
  VERSIONED_COMPOSITION uid; everything else has `default_level`.
- **Gate-keeper** (master07 §Access Control): once `gate_keeper` is set,
  only that principal may commit a new EHR_ACCESS version (contribution
  seam, 403 otherwise). Changes remain CONTRIBUTION-wrapped and audited like
  all content (RM ehr master04 §EHR Access).

## Layering (rest → sm → ehrbase, strictly)

The decision **data** is the versioned EHR_ACCESS object (RM UML
`ehr_access.adoc`: "all access decisions to data in the EHR must be made in
accordance with the policies and rules in this object"); the decision
**point** is the protocol adapter (SM openehr_platform master02:
authorisation "is assumed to have been dealt with *before* any particular
call" — out of band, so never inside the SM traits, which stay the literal
SM catalog):

- **`ehrbase-sm`** — `EhrAccessAdapter`, a native-API extension trait beside
  the existing wire adapters (SM defines no `I_EHR_ACCESS` interface — no
  openEHR spec governs this adapter; our own extension):
  `current_ehr_access_settings(ehr_id)` returning the parsed scheme settings
  (or None → default-open).
- **`ehrbase` (Platform)** — implements the adapter over the normal
  versioned-object read path; `moka`-cached keyed by `ehr_id`, invalidated
  on any EHR_ACCESS commit. The REST layer never touches the database.
- **`ehrbase-rest`** — the `access` module owns the EHR-scoped policy
  engine, evaluated after authentication and before dispatch.

## Evaluation points

1. **Per-EHR gate** — every authenticated request on an `/ehr/{ehr_id}`-
   scoped route: settings absent, or `default_access = open` with no denying
   entry → allow. `restricted` → principal must match the access list.
   Fail → 403 (ITS-REST 401/403 discipline).
2. **Composition privacy** — composition read endpoints (by id / by
   version / versioned-composition): effective level = override(uid) else
   `default_level`; principal ceiling = `full` → ∞, `restricted_below` →
   `max_level`, no entry under `open` → `default_level` + 1 (i.e. default
   readable unless raised). Level ≥ ceiling → 403. Evaluated in the REST
   adapter from settings + the target uid alone (the overrides are
   uid-keyed), so no post-fetch inspection is needed.
3. **EHR_ACCESS writes** — gate-keeper preflight in the REST adapter on
   contribution commits whose version set targets EHR_ACCESS (the only
   write path; there is no dedicated EHR_ACCESS endpoint in ITS-REST).

## Explicit v1 scope boundaries (flagged, not silent)

- **AQL result filtering by privacy level is not evaluated in v1** — query
  execution has no principal context yet; the per-EHR gate still applies to
  the REST query surface where an `ehr_id` is bound. Recorded as a scheme
  limitation here and in the module docs.
- Time-limited access, notarisation forwarding, and read-access logs are
  respectively out of scope / deployment concerns / already provided by the
  ATNA system log (`app/ehrbase/src/system_log/`).
- The settings payload is an extension `_type`; canonical storage keeps it
  verbatim (node codec stores canonical JSON unmodified), so no generated
  type is added to the spec crates and wire fidelity is unaffected.

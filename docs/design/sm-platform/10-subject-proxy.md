# Subject Proxy Service (SPS) — complete redesign (W-3c)

Owner directive 2026-07-12: the SPS is the integration seam for connecting
external systems to EHRbase and must follow the vendored SM spec **properly and
completely** — the current SM-6 wave-1 implementation is a minimal skeleton
with the spec's defining behaviours deferred. This document is the full
redesign; the work item is **W-3c** in `docs/plans/WORKLIST.md`.

**Spec oracle** (read these before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
  (the SPS chapter: naming, service interface, data structures, samples,
  bindings, persistence, usage, YAML data-set/binding specifications)
- `docs/specs/openehr/SM/docs/UML/classes/` — `i_subject_proxy_service.adoc`,
  `i_data_binding.adoc`, `subject_proxy.adoc`, `subject_variable.adoc`,
  `subject_data_set.adoc`, `data_set_result.adoc`, `sample.adoc`,
  `data_frame_sample.adoc`, `openehr_sample.adoc`, `hl7v2_sample.adoc`,
  `hl7_fhir_sample.adoc`, `variable_sample.adoc`, `variable_value.adoc`
  (+ `_single`/`_list`/`_time_series`), `env_binding.adoc`, `data_frame.adoc`
- Adjacent: `master02-overview.adoc` (SPS in the platform component map);
  the openEHR PROC Task Planning spec defines `SYSTEM_CALL`/`API_CALL`/
  `QUERY_CALL` referenced by `DATA_FRAME` (PROC is **not vendored** — see
  gap G-6 and the PORT NOTE policy below).

**Current implementation** (verified 2026-07-12):

- Catalog trait + information structures:
  `app/ehrbase-sm/src/services/subject_proxy.rs` (608 lines)
- Service impl over the `sp_*` stores:
  `app/ehrbase/src/service/subject_proxy.rs` (609 lines)
- Schema: `sp_subject`, `sp_binding`, `sp_data_frame`, `sp_variable`,
  `sp_data_set` in `app/ehrbase/migrations/ehr/0001_baseline.sql`
- Wire: **none** (native-API-only; no `/subject_proxy` routes exist in
  `ehrbase-rest`)
- ECC: **zero SP cases**
- The doc comments cite `docs/design/sm-platform/04-…` and `08-…` which
  **no longer exist** — this document replaces them; scrub those references.

---

## 1. Gap register (what is not spec-true today)

Every gap cites the governing spec text. G-1..G-5 are the core of the owner's
"not correctly / completely following the specs" judgement.

| # | Gap | Spec citation | Today |
|---|-----|---------------|-------|
| G-1 | **No sample history — variables are not "tracked over time".** `SUBJECT_VARIABLE.history: List<VARIABLE_SAMPLE>` and `last_frame: DATA_FRAME_SAMPLE` are dropped entirely; every read re-executes the frame. The SPS's very purpose is "symbolic variables … to be retrieved **and tracked over time**" (master10 §Overview); `SUBJECT_VARIABLE` is "a proxy for a single subject variable, **including sample history over time**" (§Data Structures). | `subject_variable.adoc` (`history`, `last_frame`); master10 §Overview, §Data Structures | Omitted with a PORT NOTE reading §Persistence as permission to drop them. §Persistence only says results need not survive **system re-initialisation** — it does not license having no runtime sample state at all. |
| G-2 | **Currency/freshness semantics unimplemented.** `SUBJECT_VARIABLE.currency` is stored but never evaluated; `SAMPLE.effective_time` is never set or compared ("The optional `effective_time` … is **comparable to `currency` in order to determine the freshness** of the data", master10 §Samples). `register_application_data_set` skips the mandated "reducing the currency of existing subject variables, if the currency is lower" branch. | master10 §Samples; `i_subject_proxy_service.adoc` `register_application_data_set` | Every call re-executes; the currency-tightening branch is PORT-NOTEd away on ISO-duration-comparison grounds (solvable: anchor at the evaluation instant with `jiff`). |
| G-3 | **`fallback_method` never executed.** `DATA_FRAME.fallback_method`: "Alternative method to use **if primary retrieve method fails**". | `data_frame.adoc` | `get_frame` reads only `primary_method`; on failure it returns an unavailable sample without ever trying the fallback. |
| G-4 | **FHIR frames rejected `NotImplemented` — the external-systems seam is closed.** The SPS exists so callers need not "know about the particular standard, representational model, query language or API of the data source … there is no need to even assume that an openEHR back-end system is the source" (master10 §Overview); `HL7_FHIR_SAMPLE` is a first-class sample type. The workspace already has an outbound FHIR HTTP client (`reqwest`, B4 `FhirTerminologyProvider`) — the capability exists and is not wired here. | master10 §Overview, §Bindings; `hl7_fhir_sample.adoc` | `FrameMethod::Fhir` is carried in the store but `get_frame` rejects it. |
| G-5 | **No wire, no YAML/JSON ingestion.** "A data set specification **would be provided through a REST API** as a text specification, e.g. in JSON or YAML" (master10 §Specifying a Data-set; §Specifying a Binding shows the `ENV_BINDING` YAML). External systems cannot reach the SPS at all today. | master10 §§Specifying a Data-set / Specifying a Binding | Native-API-only PORT NOTE; ITS-REST indeed vendors no SPS endpoints, but the spec itself mandates a REST ingestion surface — an **extension API** (out of CORE/STANDARD scope) realizes it. |
| G-6 | **Frame methods diverge from the spec's `SYSTEM_CALL` shape.** `DATA_FRAME.primary_method: SYSTEM_CALL` (PROC), shown in §Specifying a Binding as `API_CALL`/`QUERY_CALL` with `system_id`, `call_name`, `parameters`, `query_text`. Our `FrameMethod` invents `_type` tags (`FHIR_API_CALL`, `HL7v2_CALL`) and drops `parameters`; `model_type` plays no role in dispatch. | `data_frame.adoc`; master10 §Specifying a Binding | Partially shaped; not round-trippable against the spec's own YAML example. |
| G-7 | **`VARIABLE_VALUE_TIME_SERIES` never produced** and `type_name` never used. `SUBJECT_VARIABLE.value()`: "Extract the value from the source retrieve frame, **reprocessing if necessary to obtain intended type (single, list, time_series)**"; `type_name [1]`: "Formal type name from defining model". | `subject_variable.adoc`; `variable_value_time_series.adoc` | Extraction yields Single/List only; the declared type is ignored (no coercion, no validation, no time-series pairing). |
| G-8 | **Subject-id resolution is a UUID sniff.** `I_DATA_BINDING.get_frame`: the subject id "might not be the primary identifier … but instead an identifier of an information resource …, e.g. an EHR identifier. TODO: … this service might need to **resolve it through another service**". The platform has that service — `I_EHR_INDEX` (`EhrIndexService`, `ehr_index` table) — and it is not consulted. | `i_data_binding.adoc`; master09 (EHR Index) | `Uuid::parse_str` on the subject id decides EHR scoping; external subject ids (MRN, FHIR Patient id) cannot resolve. |
| G-9 | **Data-set local aliases unused on read.** `SUBJECT_DATA_SET.variables` is keyed by *local* name ("a variable whose canonical name is `date_of_birth` may be known within a data set as `dob`", master10 §Subject Variable Naming); `get_variable` is documented "Get a single variable value **from a data-set**". | `subject_data_set.adoc`; master10 §Subject Variable Naming | Lookup is canonical-name-only; a data-set alias never resolves. |
| G-10 | **`using_app_ids` ref-counting seam dead.** "Optional list of identifiers of applications using this data set. May be used to track applications, **and dump the data set when empty**." | `subject_data_set.adoc` | Stored verbatim, never maintained or acted on. |
| G-11 | **`ask_user` / `is_manual` have no input channel.** A manual variable ("obtained by manual notification, typically from a worker observing the subject") needs a way to push a sample in; the SM interface defines no such call (spec gap). | `subject_variable.adoc` (`ask_user`, `is_manual`) | Both flags stored, nothing consumes them. |
| G-12 | **`SUBJECT_PROXY.create_time` not surfaced; `subject_category` "currently not controlled" (spec TBD) not documented.** | `subject_proxy.adoc` | `sp_subject` has a created-at column; no read path exposes the proxy object itself. |
| G-13 | **Spec-orphan classes unplaced.** `SP_VARIABLE_DEF` / `SP_VARIABLE_CATEGORY` / `S_PARTY_PROXY` exist in the SM UML index but are included by **no** chapter (`master10` includes neither) — a spec defect/TBD to record, not to implement. | `UML/class_index.adoc` vs `master10` includes | Unmentioned. |

---

## 2. Target design

### 2.1 Model layer (`ehrbase-sm/src/services/subject_proxy/`)

The single 608-line file splits into a module directory; every type keeps its
verbatim spec citation:

```
app/ehrbase-sm/src/services/subject_proxy/
├── mod.rs          # re-exports + module docs (spec map, PORT NOTE register)
├── value.rs        # VARIABLE_VALUE (SINGLE / LIST / TIME_SERIES)
├── sample.rs       # SAMPLE<T>, DATA_FRAME_SAMPLE payloads, VARIABLE_SAMPLE
├── variable.rs     # SUBJECT_VARIABLE (+ history/last_frame — G-1), naming fns
├── data_set.rs     # SUBJECT_DATA_SET, DATA_SET_RESULT
├── binding.rs      # ENV_BINDING, DATA_FRAME, SYSTEM_CALL model (G-6)
└── service.rs      # I_SUBJECT_PROXY_SERVICE + I_DATA_BINDING traits
```

Model corrections:

- **`SUBJECT_VARIABLE` regains `history: Vec<VariableSample>` and
  `last_frame: Option<DataFrameSample>`** (G-1) as *read-model* fields: the
  stored definition row stays configuration-only, and the service materialises
  history/last-frame from the sample store when returning a variable.
- **`SYSTEM_CALL`-faithful frame methods** (G-6): one `SystemCall` struct —
  `call_kind` (`API_CALL` | `QUERY_CALL`, the two PROC descendants shown in
  master10 §Specifying a Binding), `system_id: Option<String>`,
  `call_name: String`, `parameters: BTreeMap<String, Value>`,
  `query_text: Option<String>` — serialised with the spec's own `_type` tags
  so the §Specifying a Binding YAML round-trips verbatim. Dispatch is
  `model_type` × `call_name` (G-6), not an invented enum tag.
  PORT NOTE (kept): PROC/Task Planning is not vendored; `SystemCall` models
  exactly the attribute set master10's own examples exercise — flagged as the
  documented subset, re-checked on any future PROC vendoring.
- **Time-series value** (G-7): `VariableValue::TimeSeries` keyed
  `BTreeMap<String, Value>` (ISO-8601 date-time keys) stays; the *extraction*
  layer now produces it (§2.3).
- The trait surface (14 SM calls + `get_frame`) is already 1:1 with
  `i_subject_proxy_service.adoc` / `i_data_binding.adoc` — unchanged apart
  from returning the richer types. **One extension call** is added for G-11:
  `notify_variable_sample(subject_id, canonical_name, sample)` — flagged
  explicitly: *no openEHR spec defines this call — our own extension; it is
  the input channel `is_manual`/`ask_user` variables require*
  (`subject_variable.adoc` defines the flags but the SM interface defines no
  push operation).

### 2.2 Retrieval engine (`app/ehrbase/src/service/subject_proxy/`)

The impl splits the same way and gains the real engine:

```
app/ehrbase/src/service/subject_proxy/
├── mod.rs          # SubjectProxyService impl (preconditions, stores)
├── store.rs        # sp_* row mapping (subject, variable, data_set, binding)
├── frames.rs       # I_DATA_BINDING impl: dispatch, primary→fallback, samples
├── executors/
│   ├── mod.rs      # FrameExecutor seam: model_type × call → executor
│   ├── openehr.rs  # QUERY_CALL/aql_query → Query service → OPENEHR_SAMPLE
│   ├── fhir.rs     # API_CALL/fhir_get → reqwest → HL7_FHIR_SAMPLE (G-4)
│   └── hl7v2.rs    # typed rejection seam (kept; no transport exists)
├── freshness.rs    # currency evaluation + sample cache decisions (G-2)
└── extract.rs      # frame_path extraction → typed VARIABLE_VALUE (G-7)
```

1. **Frame pipeline** (G-3): `get_frame` executes `primary_method`; a failed
   or unavailable primary triggers `fallback_method` when present
   (`data_frame.adoc`). Every attempt — success or not — produces a `SAMPLE`
   ("Every retrieval attempt will generate a new Sample object, regardless of
   whether data was actually available or not", `sample.adoc`) and is recorded
   in the sample store.
2. **openEHR executor**: unchanged in substance (AQL through the Query
   service, `$subject_id` bound), now behind the executor seam and stamping
   `effective_time` from the result where derivable.
3. **FHIR executor** (G-4): `API_CALL` with `call_name: fhir_get` against
   `system_id` — a `reqwest` GET of `query_text` (a FHIR search/read URL
   template with `$subject_id` substitution) returning the resource JSON as
   `HL7_FHIR_SAMPLE`. Config: `EHRBASE_SUBJECT_PROXY__FHIR_*` (base-URL
   allowlist, timeout, auth header), opt-in like the B4 FHIR terminology
   provider; no allowlisted base URL ⇒ typed rejection (fail-closed).
   *No openEHR spec governs the transport specifics — our own design,
   consistent with `docs/design/terminology-server-integration.md`.*
4. **Subject-id resolution** (G-8): before executing an openEHR frame, resolve
   the subject id in order: (a) literal EHR id (UUID), (b) `I_EHR_INDEX`
   lookup by external subject ref (`EhrIndexService`, realising the spec's own
   "resolve it through another service" TODO), (c) unresolved ⇒ unavailable
   sample with reason. FHIR frames pass the subject id through verbatim
   (`$subject_id` substitution) — the remote system owns resolution.
5. **Freshness** (G-2): on `get_variable`/`get_data_set`, if the newest stored
   sample for the variable satisfies `currency` (sample `effective_time`
   falling back to `retrieve_time`, compared against `now - currency`
   anchored with `jiff` — nominal months/years resolve at the evaluation
   instant), return it **without** re-executing; otherwise execute the frame
   and store the new sample. `currency = Void` ⇒ most recent available is
   valid (`subject_variable.adoc`) — served from the store, refreshed when no
   sample exists.
   `register_application_data_set` now implements the full spec sentence:
   create missing variables; **tighten** an existing variable's currency when
   the data set declares a lower one; otherwise no change.
6. **Extraction** (G-7): `frame_path` semantics per sample family —
   - `OPENEHR_SAMPLE`: `RESULT_SET` column selector (as today), extended with
     an optional paired time column (`frame_path = "value @ time"` selector
     form) producing `VARIABLE_VALUE_TIME_SERIES`; multiple rows without a
     time pairing stay `LIST`, one row `SINGLE`.
   - `HL7_FHIR_SAMPLE`: a JSON pointer into the resource
     (fail-closed on a non-existent pointer ⇒ `SINGLE{None}`).
   - `type_name` is enforced: the extracted value is coerced/validated against
     the declared type (`Quantity`, `Date`, `Boolean`, …); a mismatch yields
     an unavailable variable sample with reason, never a silently wrong type.
   The exact selector grammar is documented in the module and in the website
   book page (§2.5). *PORT NOTE stays: the SM leaves `frame_path` semantics
   undefined ("Path within `last_frame` result") — this grammar is our
   documented realization.*
7. **Data-set reads** (G-9/G-10): `get_variable` resolves `var_name` first as
   a data-set-local alias (within the subject's data sets) and then as a
   canonical name; `remove_application_data_set`/`remove_application` maintain
   `using_app_ids` and drop a data set when its user list empties
   (`subject_data_set.adoc`).

### 2.3 Storage (`0001_baseline.sql`, re-authored section)

- `sp_variable` gains nothing (definitions stay configuration-only).
- **New `sp_sample`** — the sample store realizing G-1/G-2:
  `(id uuidv7 PK, subject_id FK, canonical_name, frame_id, retrieve_time
  timestamptz, effective_time timestamptz NULL, is_unavailable bool,
  unavailable_reason text NULL, payload_kind text, payload jsonb NULL)`,
  indexed `(subject_id, canonical_name, retrieve_time DESC)`; a retention
  cap per variable (config, default e.g. 100 samples) enforced on insert.
  master10 §Persistence requires only configuration to survive
  re-initialisation, and `reset()` truncates `sp_sample` too — persisting
  samples is *permitted* (nothing forbids it) and is what makes
  "tracked over time" + freshness real across restarts.
  *No openEHR spec governs the storage mechanics — our own design.*
- `sp_data_set.using_app_ids` becomes a maintained list (G-10).
- Schema comments cite `master10` sections only (never ADRs).

### 2.4 Wire (`ehrbase-rest`) — the external-systems surface (G-5)

A config-gated **extension** API (exactly the `/terminology` extension
pattern; out of CORE/STANDARD conformance scope, documented as an extension):

```
POST   /rest/subject_proxy/subjects                       register_subject
DELETE /rest/subject_proxy/subjects/{subject_id}          remove_subject
GET    /rest/subject_proxy/subjects/{subject_id}/variables        get_variable_defs
POST   /rest/subject_proxy/subjects/{subject_id}/variables        add_subject_variable
GET    /rest/subject_proxy/subjects/{subject_id}/variables/{name} get_variable
POST   /rest/subject_proxy/subjects/{subject_id}/variables/{name}/samples  notify_variable_sample (extension, G-11)
POST   /rest/subject_proxy/data_sets                      register_application_data_set
DELETE /rest/subject_proxy/subjects/{id}/data_sets/{app}  remove_application_data_set
GET    /rest/subject_proxy/subjects/{id}/data_sets/{ds}   get_data_set
POST   /rest/subject_proxy/bindings                       register_binding
POST   /rest/subject_proxy/bindings/{env_id}/frames       add_binding_frame
DELETE /rest/subject_proxy/applications/{app_id}          remove_application
POST   /rest/subject_proxy/admin/reset                    reset (admin-gated)
```

- **Ingestion is JSON and YAML** (master10 §Specifying a Data-set: "e.g. in
  JSON or YAML") — content negotiation on `application/json` /
  `application/yaml`, both deserialising into the same model types, so the
  spec's own YAML examples are accepted byte-for-byte.
- Auth: the standard authn stack; `reset` additionally requires the admin
  role. ATNA audit events for every mutating call (SM master02 system-log
  requirement).
- OAS: added to the extension OpenAPI (same assembly as `/terminology`,
  `scripts/assemble-oas.sh`), documented on the website book (same-PR rule).

### 2.5 Verification

- **Unit/integration**: per-executor tests (wiremock FHIR fixture server for
  `fhir.rs` — same pattern as B4); freshness matrix (currency unset/satisfied/
  expired/nominal-duration); fallback pipeline; alias resolution;
  time-series extraction; type coercion failures; `reset` truncates samples.
- **ECC**: a new `SP` area — cases over the extension wire (register binding →
  register subject → data set → get_variable/get_data_set, incl. the master10
  YAML examples verbatim, FHIR frame against the runner's wiremock, fallback
  and unavailability). Zero `skipped` outcomes (W-2 ruling); the wire makes
  the cases executable rather than native-API adjudications.
- Gates: workspace suites green, clippy clean, full ECC zero-drift.

---

## 3. Work plan (W-3c execution order)

1. **Model split + corrections** (`ehrbase-sm`): module directory, `history`/
   `last_frame` read-model fields, `SystemCall`, spec-tag serde, YAML
   round-trip test of the master10 examples. (G-1 model, G-6, G-7 model)
2. **Storage**: `sp_sample` + `using_app_ids` maintenance in the baseline
   (schema re-author; migrations test updated in the same change). (G-1, G-10)
3. **Engine**: executor seam, primary→fallback pipeline, openEHR executor
   port, subject-id resolution via EHR Index, sample recording. (G-3, G-8)
4. **Freshness**: currency evaluation, serve-from-store, data-set currency
   tightening. (G-2)
5. **FHIR executor** + config + wiremock tests. (G-4)
6. **Extraction v2**: typed coercion, time-series selector, FHIR JSON
   pointer, alias resolution on reads. (G-7, G-9)
7. **Wire**: extension routes + YAML/JSON ingestion + ATNA + OAS + book page.
   (G-5, G-11)
8. **ECC `SP` area** + gap-register closure sweep (every G-row: implemented,
   or a re-verified PORT NOTE with citation — G-13 recorded as spec defect).
   Scrub the dangling `docs/design/sm-platform/*` references (this document
   replaces them).

Exit: all 13 G-rows closed (code or cited PORT NOTE), suites + ECC zero-drift
green, extension surface documented on the website, WORKLIST row linked to
the merged PR.

---

## 4. Standing PORT NOTEs after the redesign (the honest residue)

- `SYSTEM_CALL` modelled as the master10-exercised subset (PROC not vendored).
- `frame_path` selector grammar is our documented realization of an
  undefined spec attribute.
- HL7v2 frames remain a typed-rejection seam (no HL7v2 transport in scope).
- `notify_variable_sample` + the whole REST surface are extensions (ITS-REST
  vendors no SPS endpoints; master10 explicitly anticipates a REST API).
- `SP_VARIABLE_DEF`/`SP_VARIABLE_CATEGORY`/`S_PARTY_PROXY`: orphaned in the
  SM UML index, included by no chapter — recorded as a spec defect/TBD.
- `subject_category` values: spec says "currently not controlled" — free
  text, default `"individual"`.

# Platform crate (W-3f) — Subject Proxy + Terminology + Validity Checker

Spec-first audit (read-only, 2026-07-12) of the `ehrbase` **platform crate**
service layer for three SM components: Subject Proxy (`I_SUBJECT_PROXY_SERVICE`
+ `I_DATA_BINDING`), Terminology (`I_TERMINOLOGY_SERVICE`), and Validity
Checking (`I_VALIDITY_CHECKER`). The method is the owner ruling: the spec is
mapped **onto** the code — the register skeleton is enumerated from the vendored
oracle first, then existing code is located against each item with a verdict.

The SM **trait surface is FIXED** (`app/ehrbase-sm/src/services/subject_proxy/`,
`.../terminology/`, and `.../common/validity.rs`). This document governs only
how the `ehrbase` crate **implements** those traits and where the impl files
sit.

**Spec oracles (read before any change):**

- Subject Proxy: `docs/specs/openehr/SM/docs/openehr_platform/master10-subject_proxy_service.adoc`
  + `docs/specs/openehr/SM/docs/UML/classes/i_subject_proxy_service.adoc`,
  `i_data_binding.adoc`, and the structure classes (`subject_variable.adoc`,
  `sample.adoc`, `data_frame.adoc`, `env_binding.adoc`, `variable_value*.adoc`).
- Terminology: `master12-terminology_service.adoc` +
  `UML/classes/i_terminology_service.adoc` and the extract model
  (`terminology_extract.adoc`, `terminology_description.adoc`,
  `term_code.adoc`, `defined_term.adoc`, `term_relationship.adoc`,
  `terminology_relation.adoc`).
- Validity: `master03-common_package.adoc` §Class Definitions →
  `UML/classes/i_validity_checker.adoc`.
- Context (not interface-defining): `BASE/docs/architecture_overview/master12-terminology.adoc`
  (RM value-sets, archetype internal terminology, external-terminology binding,
  the "terminology query server") and `BASE/docs/foundation_types/master07-terminology.adoc`
  (`Terminology_code`, `Terminology_term`, `Code_phrase` leaf types).
- Prior impl-side registers absorbed: `docs/design/sm-platform/10-subject-proxy.md`
  (W-3c, all 13 G-rows), `docs/design/sm-platform/12-terminology.md` (audit,
  G-1..G-8 open).

**Current implementation (verified 2026-07-12):**

| Component | Trait (FIXED, `ehrbase-sm`) | Impl (`ehrbase`) | Shape |
|---|---|---|---|
| Subject Proxy | `services/subject_proxy/` (7 files) | `service/subject_proxy/` — `mod.rs` 543, `store.rs` 420, `extract.rs` 342, `config.rs` 256, `frames.rs` 236, `freshness.rs` 105 | **directory (W-3c done)** — mirrors SM |
| Terminology | `services/terminology/{mod,service}.rs` | `service/terminology.rs` 438 (bundle mapping) **+** `service/api/terminology.rs` 273 (`TerminologyService` + `TerminologyExpander` impls) **+** top-level `src/terminology/` (`fhir.rs` 518, `config.rs` 255, `mod.rs` 19) | **scattered across 3 locations** — does NOT mirror SM |
| Validity | `common/validity.rs` (2 calls) | `service/api/mod.rs:47-80` (`impl ValidityChecker`) | **buried alongside `WebTemplateService`** |

Out of scope (do not touch): `service/fhir/` (`mapping.rs` 991, `mod.rs` 590)
is the **FHIR connector / inbound-ingest extension** (E3, spec-silent), unrelated
to terminology or subject-proxy despite the name collision.

---

## 1. Spec skeleton — Subject Proxy (`i_subject_proxy_service.adoc` / `i_data_binding.adoc`)

`I_SUBJECT_PROXY_SERVICE` — 15 calls, all present 1:1 on `impl SubjectProxyService
for EhrbaseService` (`service/subject_proxy/mod.rs`), verbatim call names,
parameters, and preconditions. Every unmet `__Pre_…__` → `SmError(precondition)`
(→ 400), design-filled (SM declares no error codes).

| SM call | Citation | Impl (file:line) | Verdict |
|---|---|---|---|
| `register_subject` (`Pre: not has_subject`) | i_subject_proxy §L15-22 | `mod.rs:158` | conformant |
| `add_subject_variable` (`Pre: has_subject`) | §L24-31 | `mod.rs:181` | conformant |
| `register_application_data_set` (create / currency-tighten / no-change) | §L33-41; master10 §Samples | `mod.rs:195` (tightening via `freshness::tighter_currency`) | conformant (W-3c G-2) |
| `remove_application_data_set` | §L43-51 | `mod.rs:262` (+ `using_app_ids` upkeep) | conformant (W-3c G-10) |
| `remove_subject` / `remove_application` | §L53-67 | `mod.rs:300` / `316` | conformant |
| `get_variable` (alias→canonical) | §L69-76; master10 §Variable Naming | `mod.rs:338` (`sp_resolve_alias`) | conformant (W-3c G-9) |
| `get_data_set` | §L78-85 | `mod.rs:366` | conformant |
| `has_subject` / `has_application` / `has_binding` | §L87-97, L124-128 | `mod.rs:397/401/461` | conformant |
| `get_variable_defs` ("name: Type") | §L99-103 | `mod.rs:405` | conformant |
| `register_binding` (`Pre: not has_binding`) | §L105-113 | `mod.rs:422` | conformant |
| `add_binding_frame` (frame typed `DataFrame`) | §L115-122 | `mod.rs:444` | conformant (PORT NOTE: SM leaves `frame` untyped) |
| `reset` (virgin state) | §L130-132; master10 §Persistence | `mod.rs:465` | conformant |
| `I_DATA_BINDING.get_frame` (primary→fallback) | i_data_binding §L22-35; `data_frame.adoc` | `frames.rs:47` | conformant (W-3c G-3) |
| `I_DATA_BINDING.bindings` attribute | i_data_binding §L15-17 | realized by binding store, not a getter | conformant (PORT NOTE, trait doc) |
| `notify_variable_sample`, `get_subject_variable` | none | `mod.rs:482/515` | **extension — flag** (spec-silent input/read channel for `is_manual`/`history`) |

Frame dispatch (`frames.rs:93`) routes `model_type × call_name` (master10
§Specifying a Binding): openEHR AQL executor + FHIR `API_CALL/fhir_get` executor
(config-gated `reqwest`, fail-closed); HL7v2 = typed reject. Subject-id
resolution = literal EHR uuid → `I_EHR_INDEX` (W-3c G-8). All W-3c G-1..G-13 are
closed; the standing residue is §5 below.

## 2. Spec skeleton — Terminology (`i_terminology_service.adoc`)

9 calls, all present on `impl TerminologyService for EhrbaseService`
(`service/api/terminology.rs:201`) delegating to the bundle mapping
(`service/terminology.rs`) or the FHIR provider (`src/terminology/fhir.rs`).

| SM call | Citation | Impl (file:line) | Verdict |
|---|---|---|---|
| `get_terminology_ids` (`[0..1]`) | i_terminology §L15-17 | `api/terminology.rs:202` → `term::terminology_ids` | conformant (opt. collapsed — G-8) |
| `has_terminology` | §L19-23 | `:206` → `term::has_terminology` | conformant |
| `get_terminology_description` (`Pre_has_terminology`) | §L25-31 | `:210` → `term::terminology_description` | conformant (`attributes` always `None` — G-2/G-3) |
| `has_term` (+`at_date`) | §L33-41 | `:217` (`_at_date` ignored) | conformant / bundle; `at_date` no-op — G-1 |
| `get_term` (+`attributes`,`at_date`; `Pre_has_term`) | §L43-53 | `:227` (`_attributes`,`_at_date` ignored) | conformant / no relationships — G-2/G-3 |
| `subsumes` (**strict**) | §L55-63 | `term::subsumes` (flat→false) / FHIR `$subsumes` | conformant (FAITHFUL, doc 12 §1) |
| `value_set_validate` (+`at_date`) | §L65-74 | `:248` → `term::value_set_validate` | conformant; `at_date` no-op — G-1 |
| `has_value_set` | §L76-81 | `:258` | conformant |
| `get_value_set` (`Pre_has_value_set`) | §L83-91 | `:266` | conformant / FHIR flattens hierarchy — G-5 |

Extract model (`terminology_extract.adoc` &c.) is fully present and faithful in
the FIXED `ehrbase-sm` trait crate (`TerminologyExtract`, `TermEntry`,
`TerminologyRelation.Inv_valid_definition`, `create_terminology_code`) — doc 12
§1. The `ehrbase`-side residue is provider coverage + extract fidelity (§4
register G-1..G-8), not a missing call or broken precondition.

## 3. Spec skeleton — Validity Checker (`i_validity_checker.adoc`)

2 calls (`master03-common_package.adoc` §Class Definitions). Impl in
`service/api/mod.rs:57`.

| SM call | Citation | Impl | Verdict |
|---|---|---|---|
| `definitions_valid(a_content: LOCATABLE)` | i_validity_checker §L15-19 | `api/mod.rs:58` — template-id lookup via `web_template_for` | conformant (PORT NOTE: template ids only — no ADL2 archetype store) |
| `content_valid(a_content: LOCATABLE)` | §L21-25 | `api/mod.rs:68` — per-`Kind` `validate_for_commit(strict)` | conformant |

## 4. Context oracles (what is *not* an interface obligation)

- `BASE/architecture_overview/master12-terminology`: RM coded attributes bound
  to the "openEHR" terminology + six code sets; archetype **internal** terminology
  (at-codes, flat, no server); external binding via ac-codes to a "terminology
  query server". This grounds the bundle's `openehr` + external-code-set split
  and confirms hierarchical/value-set queries are a **remote-server** concern —
  i.e. the interface/provider separation is the spec's own model, not our
  invention (bearing on the G-T2 provider decision below).
- `BASE/foundation_types/master07-terminology`: `Terminology_code`,
  `Terminology_term`, `Code_phrase` leaf types — realized by
  `openehr_base::TerminologyCode` (the `create_terminology_code` return). No
  service obligation here; documents why `DefinedTerm.language` /
  `TerminologyRelation.external_code` as bare `String` (not `Terminology_code`)
  is a faithful subset at this boundary.

## 5. Code mapping to no spec item (spec-silent / quarantine / delete)

- **Spec-silent, keep with flag** (no openEHR spec governs — our own design):
  `subject_proxy/store.rs` (`sp_*` schema), `subject_proxy/config.rs` +
  `SubjectProxyFhir` (FHIR-system config/transport), `subject_proxy/extract.rs`
  (`frame_path` selector grammar — SM leaves it undefined),
  `subject_proxy/freshness.rs` (currency arithmetic — realizes master10 §Samples),
  `service/terminology.rs` bundle-mapping decisions (`openehr` id, group↔value-set
  split, SPECPR-51 flat-any-group view, `at_date` single-version, published URI),
  `src/terminology/` FHIR provider (transport, grounded on
  `docs/design/terminology-server-integration.md` + `docs/terminology-validation.md`),
  the whole `/terminology` + subject-proxy extension wire.
- **No delete/quarantine candidates found** — every file maps to a spec
  obligation or a flagged extension. `service/fhir/` (connector) is a *separate*
  extension owned by another W-3 area; excluded here (no rename despite the
  name collision with `src/terminology/fhir.rs`).

## 6. G-row register

Layout rows (G-T*/G-V*) are the W-3f essence; conformance rows (G-1..G-8)
are absorbed from doc 12 (still open in the tree); SPS rows are W-3c residue.

| id | citation / flag | severity | disposition |
|---|---|---|---|
| **G-T1** | crate layout is spec-silent; convention = mirror SM `services/terminology/` | HIGH | **fix-in-rewrite** — collapse `service/terminology.rs` + `service/api/terminology.rs` into a `service/terminology/` directory |
| **G-T2** | interface/provider separation is the spec's own model (`arch-overview master12` "terminology query server") — layout still spec-silent | MED | **fix-in-rewrite** — **merge** top-level `src/terminology/` (FHIR provider + config) *into* `service/terminology/` as provider submodules (both providers realize the one `I_TERMINOLOGY_SERVICE`, so they belong with the interface realization; the top-level dir was only a B4 add-on) |
| **G-V1** | SM keeps `I_VALIDITY_CHECKER` in `common/`, not `services/` | LOW | **fix-in-rewrite** — extract `api/mod.rs:47-80` to a peer file `service/validity.rs` (a file, not a dir — mirror SM's `common/validity.rs` placement) |
| G-1 | i_terminology §L37-41,48-53,70 (`at_date`) | HIGH | **fix-in-rewrite** — forward `at_date` to FHIR `$validate-code`/`$lookup`/`$expand` (`date`); bundle keeps its single-version PORT NOTE |
| G-2 | terminology_extract §L9-16,34-39 (relationships/relations) | MED | **fix-in-rewrite** (FHIR: emit `contains` hierarchy as `Term_relationship`s) / **PORT NOTE** (bundle flat by nature) |
| G-3 | i_terminology §L47,53 (`attributes` allow-list) | MED | **fix-in-rewrite** (honour once relationships exist; surface as `?attribute=`) / PORT NOTE meanwhile |
| G-4 | i_terminology §L16-31 (enumeration) | MED | **fix-in-rewrite** — route `get_terminology_ids`/`has_terminology`/`get_terminology_description` to the bundle in the composing trait impl so a FHIR-only deployment answers them |
| G-5 | terminology_extract §L13-16 (structured value-set) | MED | **fix-in-rewrite** — FHIR `$expand` keep the `contains` tree, stop flattening |
| G-6 | i_terminology §L37,48,70 (`Iso8601_date`) | LOW | **fix-in-rewrite** — shape-validate `at_date` at the wire boundary → 400 (partly `ehrbase-rest` area) |
| G-7 | i_terminology §L62,73,89 (`Pre_has_terminology`) | LOW | **fix-in-rewrite** — pre-flight `has_terminology` on the FHIR provider, else PORT NOTE (404-map is equivalent on the happy path) |
| G-8 | i_terminology §L15-17 (`[0..1]`) | TRIVIAL | **PORT NOTE / already-correct** — providing it unconditionally is conformant |
| G-SP1 | doc 10 G-1..G-13 (SPS deep redesign) | — | **already-correct** — all closed by W-3c (PR #76); verify at rewrite |
| G-SP2 | master10 §Specifying a Data-set (REST ingestion) | MED | **cross-reference** — the extension wire is owned by the `ehrbase-rest` W-3 area; verify it shipped, do not build here |

## 7. Target design

Mirror `app/ehrbase-sm/src/services/` into `app/ehrbase/src/service/`; all files
≤ ~700 lines.

```
service/subject_proxy/          # already mirrors SM (W-3c) — keep as-is
├── mod.rs (543)  store.rs (420)  extract.rs (342)
├── config.rs (256)  frames.rs (236)  freshness.rs (105)

service/terminology/            # NEW directory (G-T1/G-T2) — the whole component
├── mod.rs        # impl TerminologyService + TerminologyExpander on EhrbaseService
│                 #   (from service/api/terminology.rs, 273) — provider routing
├── bundle.rs     # DB-free openehr-term mapping (from service/terminology.rs, 438)
├── fhir.rs       # FhirTerminologyProvider   (from src/terminology/fhir.rs, 518)
└── config.rs     # external-terminology figment config (from src/terminology/config.rs, 255)

service/validity.rs             # NEW peer file (G-V1) — impl ValidityChecker
                                #   (from service/api/mod.rs:47-80)
```

Delete after the move: `service/terminology.rs`, the terminology block of
`service/api/terminology.rs`, the top-level `src/terminology/` directory, and the
`ValidityChecker` block of `service/api/mod.rs` (leaving `WebTemplateService`
there or relocating it with the template/definition area). `service/mod.rs`
module list + the `external_terminology`/`subject_proxy_fhir` field wiring update
accordingly.

**Provider decision (G-T2), justified from the spec:** the SM defines a single
`I_TERMINOLOGY_SERVICE` interface and is silent on backing; `arch-overview
master12` models the concrete backend as an external "terminology query server".
The interface/provider split is therefore *logical*, realized by one trait impl
selecting among providers — so both the in-process bundle and the remote FHIR
client belong **under** `service/terminology/` with the interface realization,
not in a disconnected top-level module. Merge (not keep-separate).

## 8. Integration seams — TODO(w3f-integrate) candidates

- **AQL terminology family** — `service/api/terminology.rs` also carries `impl
  TerminologyExpander` (the AQL `TERMINOLOGY('expand'|'validate'|'subsumes')` +
  `matches {uri}` seam) consumed by `aql/terminology.rs`. It moves into
  `service/terminology/mod.rs`; the `aql/` consumer is a TODO(w3f-integrate) for
  the AQL W-3f area.
- **Validation terminology binding** — composition validation calls
  `value_set_validate` during `validate_for_commit`; the binding point is a
  TODO(w3f-integrate) for the validation W-3f area.
- **WebTemplateService** — the other `service/api/mod.rs` impl; belongs to the
  template/definition W-3f area, TODO(w3f-integrate) (not this area's to move).

## 9. PORT-NOTE residue disposition

- **Keep, re-verify citation:** SPS `SYSTEM_CALL` = master10-exercised subset
  (PROC unvendored); `frame_path` selector grammar (undefined attribute); HL7v2
  typed-reject seam; `notify_variable_sample`/`get_subject_variable` + the SPS
  REST surface as extensions; `subject_category` free-text ("currently not
  controlled"); terminology `at_date` single-version on the bundle; `openehr`
  `service_api` id; `DefinedTerm.language`/`external_code` as bare `String`;
  bundle flat-vocabulary `subsumes`/empty meta-model; no ITS-REST terminology
  contract (extension, drift-excluded).
- **Keep, recorded as spec defect/TBD:** `SP_VARIABLE_DEF` /
  `SP_VARIABLE_CATEGORY` / `S_PARTY_PROXY` orphaned in the SM UML index (included
  by no chapter).
- **Drop / upgrade:** the terminology `attributes`-"ignored" PORT NOTE
  (`api/terminology.rs:234`) becomes a G-3 code fix or the sharper "no meta-model
  attributes defined for the openEHR bundle" statement; the FHIR `at_date`-ignored
  note is G-1 work, not residue.

---

## W-3f closure (2026-07-13)

`service/terminology/` created (`mod.rs`, `bundle.rs`, `fhir.rs`, `config.rs`); `service/validity.rs` peer file created; `service/subject_proxy/` kept as-is (W-3c donor, re-grounded); the top-level `src/terminology/` merged in.

| G | Disposition | Evidence |
|---|---|---|
| G-1 | FIXED in code | `at_date` forwarded to FHIR `$validate-code`/`$lookup`/`$expand` — `service/terminology/fhir.rs` / `config.rs:68-82`; bundle keeps its single-version PORT NOTE (`bundle.rs:46`) |
| G-2 | PORT NOTE / fix | FHIR emits `contains` hierarchy as `Term_relationship`s; bundle flat by nature — `service/terminology/fhir.rs` |
| G-3 | fix / PORT NOTE | `attributes` allow-list — re-expressed as "no meta-model attributes for the openEHR bundle" — `service/terminology/mod.rs` |
| G-4 | FIXED in code | `get_terminology_ids`/`has_terminology`/`get_terminology_description` routed to the bundle in the composing impl — `service/terminology/mod.rs` |
| G-5 | PORT NOTE / fix | FHIR `$expand` keeps the `contains` tree — `service/terminology/fhir.rs` |
| G-6 | PORT NOTE | `at_date` shape-validation at the wire boundary (partly `ehrbase-rest`) — `service/terminology/mod.rs` |
| G-7 | PORT NOTE | `has_terminology` pre-flight (404-map equivalent on happy path) — `service/terminology/fhir.rs` |
| G-8 | already-correct | providing `[0..1]` unconditionally is conformant — `service/terminology/mod.rs` |
| G-SP1 | already-correct | SPS deep redesign closed by W-3c (PR #76); verified — `service/subject_proxy/**` |
| G-SP2 | Reassigned | SPS REST ingestion wire owned by the `ehrbase-rest` W-3 area (cross-reference) |

Open residue: none — G-1/G-4 fixed in code, G-3 re-expressed, G-SP2 reassigned to `ehrbase-rest`, the remaining terminology items kept as cited PORT NOTE / already-correct.

# Message Service (SM-5) — spec-compliance audit

Read-only audit (2026-07-12) of the SM **Message Service** chapter
(`I_MESSAGE_SERVICE` / `I_EHR_EXTRACT_SERVICE` / `I_TDD_SERVICE`) against the
implementation. Unlike the Subject-Proxy redesign this document mirrors (W-3c,
`10-subject-proxy.md`), the Message service is **substantively complete**: all
six SM operations are present and faithfully realised over the greenfield
versioned store. The open surface is the *wire* (WORKLIST **W-2 row (b)**) plus
a set of documented deviations, doc-drift, and deferred selectors recorded
below.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master09-message_service.adoc`
  — the chapter; it `include::`s three UML class files and adds no narrative of
  its own.
- `docs/specs/openehr/SM/docs/UML/classes/i_message_service.adoc` — "Generic
  message service"; **empty interface** (no functions).
- `docs/specs/openehr/SM/docs/UML/classes/i_ehr_extract_service.adoc` — the
  four export/import calls (all `0..1`, no pre/post/errors declared).
- `docs/specs/openehr/SM/docs/UML/classes/i_tdd_service.adoc` — `import_tdd`
  (signature `(an_ehr_id: UUID, tdd: String)`, no return) + `import_tdds`
  (no signature at all).
- **Substance** the SM points at: the RM **EHR Extract IM**
  (`docs/specs/openehr/RM/docs/ehr_extract/`) — `master05-openehr_extract_package.adoc`
  (`X_VERSIONED_OBJECT<T>` + the `X_VERSIONED_*` wrappers), `master09-semantics.adoc`
  §Creation Semantics (the extract-building algorithm), `master04-common_package.adoc`
  (`EXTRACT_SPEC`, `criteria`, `link_depth`); the RM class tables
  `org.openehr.rm.ehr_extract.x_versioned_object.adoc`,
  `…extract_version_spec.adoc`, `…extract_content_item.adoc`.
- **Import/clone semantics:** RM common `master06` §Copying / §Distributed
  versioning (`IMPORTED_VERSION`, Cases 1/2/3).
- Adjacent: the CNF schedule `master13-func_tc_messaging.adoc` (the
  `MESSAGE_SERVICE` suite — ships `aaaa`/`bbbb` placeholder headings with `TBD`
  bodies).

**Current implementation** (verified 2026-07-12, file:line):

- Native traits (`ehrbase-sm`): `EhrExtractService`
  (`app/ehrbase-sm/src/services/message.rs:52`), `TddService`
  (`app/ehrbase-sm/src/services/tdd.rs:45`). `I_MESSAGE_SERVICE` itself has no
  trait — correct: the vendored `i_message_service.adoc` declares no functions.
- Impl (`ehrbase`): `impl EhrExtractService for EhrbaseService`
  (`app/ehrbase/src/service/message.rs:592`); `impl TddService`
  (`app/ehrbase/src/service/tdd.rs:209`). TDD body conversion is
  `openehr_flat::from_tdd` (`crates/openehr-flat/src/tdd.rs:382`).
- Import replay + branching: `commit_import` / `commit_import_scoped`
  (`app/ehrbase/src/service/vobject.rs:1787` / `:1825`),
  `commit_demographic_import` (`:1802`).
- **Wire: none.** No `/message`, `/ehr_extract`, or TDD route exists in
  `ehrbase-rest` (verified: `grep -rln 'EhrExtractService|TddService|export_ehrs|import_tdd'
  app/ehrbase-rest/src/` returns nothing). Neither trait is part of the
  `Platform` union (native-API-only, by design — `message.rs:26-29`,
  `tdd.rs:22-27`).
- ECC: `tools/conformance/src/suites/message.rs` — 10 `Area::Msg` cases, **every
  one `SKIPPED(NativeApiOnly)`** (`:8-20`, `:144-212`).
- Tests (real PG18 testcontainers): `app/ehrbase/tests/service_extract.rs`,
  `service_import.rs`, `service_tdd.rs`, `service_branching.rs`,
  `service_events.rs`.

---

## 1. Operation parity — the six SM calls

Every SM operation is present with a faithful signature. This table is the
positive audit (realisations), not the gap register.

| SM call (spec) | Trait method | Impl | Verdict |
|---|---|---|---|
| `I_MESSAGE_SERVICE` (no functions) | — | — | Faithful: empty interface, nothing to realise (`i_message_service.adoc`). |
| `export_ehrs(an_ehr_id: UUID): List<EXTRACT>` | `message.rs:67` | `message.rs:593` | Faithful. Builds one whole-EHR `EXTRACT`, latest-only, every VO (`EHR_STATUS`/`EHR_ACCESS`/`FOLDER`/`COMPOSITION`) as a primary `OPENEHR_CONTENT_ITEM` wrapping an `X_VERSIONED_<kind>` (`build_openehr_content_item` `:190`). `has_ehr` precondition ⇒ `ehr_id_does_not_exist` (`:594`). |
| `export_ehr_extracts(extract_spec: EXTRACT_SPEC): List<EXTRACT>` | `message.rs:82` | `message.rs:602` | Faithful for the covered envelope: one `EXTRACT` per `EXTRACT_ENTITY_MANIFEST` entity, honouring `EXTRACT_VERSION_SPEC` (`include_all_versions`/`include_revision_history`/`include_data` — `version_selection` `:452`), `item_list` resolution (`:626`), `include_multimedia` (`strip_inline_multimedia` `:480`), `link_depth` DV_LINK following (`:664`), and the `Includes_revision_history_valid` invariant (`:463`, matches `extract_version_spec.adoc`). `extract_type` validated against the openEHR content-type group (`:543`). Deferred selectors ⇒ typed reject (G-2/G-3). |
| `import_ehr(an_ehr_id: UUID[0..1], an_extract: EXTRACT)` | `message.rs:98` | `message.rs:694` | Faithful. Clone into an **empty** target (master06 §Copying Case 1); fixed id else source id reused (`:713`, `source_ehr_id` `:802`); `IMPORTED_VERSION` replay preserving original identity/audit/lifecycle/data/signature verbatim; duplicate target ⇒ `ehr_create_fail_duplicate_id` (`:728`); requires an `EHR_STATUS` (`:702`). |
| `import_ehr_extract(an_ehr_id: UUID, an_extract: EXTRACT)` | `message.rs:106` | `message.rs:741` | Faithful. Land VOs into an existing EHR (Cases 2/3); first receipt clones, subsequent trunk versions append (`commit_import_scoped` `:1866-1902`); singleton `EHR_STATUS`/`EHR_ACCESS` guard (`:760`); unknown EHR ⇒ `ehr_id_does_not_exist` (`:746`). |
| `import_tdd(an_ehr_id: UUID, tdd: String)` | `tdd.rs:61` | `tdd.rs:210` | Faithful. Envelope parse (templates namespace + `template_id`, `:81`), `has_ehr` + `template_does_not_exist` preconditions (`:150-176`), OPT-guided body → COMPOSITION (`openehr_flat::from_tdd`), commit through the **validated** `create_composition` path, returns `OBJECT_VERSION_ID` (design-filled return). |
| `import_tdds` (SM: no signature) | `tdd.rs:74` | `tdd.rs:214` | Present; signature fully design-filled (G-11). Fail-fast all-or-nothing batch. |

Overall the operation set is **complete and spec-faithful**; the SM-declared
"no pre/post/errors" are filled by design and surfaced as `SmError` over
`CALL_STATUS_TYPE`, each choice flagged with a `PORT NOTE`.

---

## 2. Gap register

Every gap cites the governing spec text or the WORKLIST item. G-1 is the one
open *capability* gap (the wire); the rest are documented deviations,
doc-drift, and deferred selectors.

| # | Gap | Spec / worklist citation | Today |
|---|-----|--------------------------|-------|
| **G-1** | **No REST wire — the whole capability is off the ECC transport.** All 10 `Area::Msg` ECC cases report `SKIPPED(NativeApiOnly)`, and the owner ruling (W-2) is that **no ECC case may be "skipped"** — it must pass, fail, error, or be N/A with citation. W-2 row (b) names exactly these 11 native-API-only MSG cases: "wire an extract extension API or reclassify N/A pointing at the platform-suite evidence." | `docs/plans/WORKLIST.md` W-2 (b); ITS-REST vendors zero message endpoints; `suites/message.rs:8-20`; `message.rs:26-29` | Native-API-only. The trait impls are proven by testcontainer tests, but nothing drives them over HTTP, so every MSG case is a `SKIPPED`. |
| G-2 | **`EXTRACT_SPEC.criteria` (AQL primary-set selection) not applied.** A request with `criteria` set and no `item_list` fallback is rejected rather than executed. | `master04-common_package.adoc:49` (`criteria`: "queries defining the required content"); `master09-semantics.adoc:65` (the "primary Composition set"); `message.rs:76-81`, `:619-624` | Typed `precondition_violation` naming the unsupported selector; slated for the `$ehr`-bound AQL / query-integration wave. |
| G-3 | **`EXTRACT_VERSION_SPEC.commit_time_interval` not applied.** | `org.openehr.rm.ehr_extract.extract_version_spec.adoc` (`commit_time_interval`); `message.rs:456-460` | Typed `precondition_violation` rather than a silent over-broad export. |
| G-4 | **Export does not rewrite `OBJECT_REF.namespace` to `"local"`.** master09's algorithm step: "copy/serialise the Composition … **rewriting its `OBJECT_REFs` so that `namespace` = "local"**". The export serialises the stored `ORIGINAL_VERSION`s verbatim (`build_openehr_content_item:214-226`), so exported references keep their source namespaces. | `master09-semantics.adoc:77`; `message.rs:214-226` | Not done, and not flagged by a `PORT NOTE`. Either implement the rewrite or record a deliberate-deviation note. |
| G-5 | **Stale export `PORT NOTE` (doc-drift).** The header note claims demographic-chapter `PARTY` following, `DV_MULTIMEDIA` include/exclude, and `link_depth` DV_LINK following "land with the demographic/query-integration waves" — but all three are **already implemented**. | `message.rs:32-37` vs `demographic_chapter_items:266`, `strip_inline_multimedia:480`, the `link_depth` loop `:664` | Note describes unbuilt work that is in fact built; misleads a future reader about coverage. |
| G-6 | **Contradictory branching claim (doc-drift).** The module doc says "version-branching = typed rejection", but `parse_import_containers` states branches are "first-class" and `commit_import_scoped` actually lands per-lineage branch versions (multi-system trees, `uq_vo_version_tree`). Import branching **works**; the "typed rejection" line is wrong. | `message.rs:27` vs `message.rs:841`, `vobject.rs:1843-1902` (branch lineages) | The stale line understates the implementation; it should be scrubbed. (Export-side / commit-API branching may still be trunk-only elsewhere — verify per surface.) |
| G-7 | **Imported content is stored unvalidated and OPT-unlinked.** `vo_version.template_id` stays `NULL` on import and imported bodies bypass WebTemplate/RM-invariant/terminology validation (verbatim replay, like admin dump/load). | `message.rs:39-43` (`PORT NOTE (import scope)`) | Documented limitation. Imported COMPOSITIONs cannot be reliably template-scoped in AQL and are trusted, not re-checked. |
| G-8 | **`import_ehr` clone leaves `ehr.subject_id` unset.** A clone shares the source subject, which the one-EHR-per-subject promoted-column index cannot represent; the subject is preserved only inside `EHR_STATUS` content. | `message.rs:44-47` | Documented; promoted `subject_id` lookups will not find an imported clone. |
| G-9 | **Synthetic `archetype_node_id` on the extract skeleton.** `EXTRACT` / `EXTRACT_CHAPTER` / `OPENEHR_CONTENT_ITEM` are `LOCATABLE`s whose `archetype_node_id` is `1..1`, yet a programmatically-built skeleton has no generating archetype; the RM class token (`"EXTRACT"`, …) is emitted as a placeholder. | `message.rs:49-55`, `:253`, `:353`; `LOCATABLE.archetype_node_id [1..1]` | Documented deliberate deviation (no fake archetype id fabricated). |
| G-10 | **`GENERIC_CONTENT_ITEM` (ISO 13606 / CDA) import unsupported.** | `master06-generic_extract_package.adoc`; `message.rs:863-867` | Typed `precondition_violation`; only `OPENEHR_CONTENT_ITEM` chapters import. Consistent with the openEHR-only scope. |
| G-11 | **`import_tdds` signature entirely design-filled.** The SM gives the call no parameters, return, or semantics. | `i_tdd_service.adoc:22-24`; `tdd.rs:63-75` | Implemented as `(UUID, Vec<String>) → Vec<String>`, fail-fast. An extension by necessity — flagged. |
| G-12 | **Naming divergence.** SM spells `I_EHR_EXTRACT_SERVICE`; the (now-deleted) design digest and the CNF `master13` schedule reference a phantom singular `export_ehr()`/`export_ehr_extract()` pair. The trait uses the vendored `.adoc` spelling verbatim. | `message.rs:12-16`; CNF `master13-func_tc_messaging.adoc` (TBD bodies) | Documented; the CNF schedule is a placeholder (`TBD`), so nothing binds against the phantom names. |
| G-13 | **Dangling design-doc references.** `message.rs`, `tdd.rs` (both crates), and `suites/message.rs` cite `docs/design/sm-platform/10-message-integration.md`, which **no longer exists** (the `sm-platform/` dir holds only `10-subject-proxy.md` + `README.md`). Same class of orphan the SPS redesign flagged. | Code refs vs `ls docs/design/sm-platform/` | This document (`09-message.md`) supersedes those citations; scrub them when next touching each file. |

---

## 3. Target design

The Message service does not need a redesign — it needs a **wire** and a
**doc-hygiene pass**. Scope, in priority order (do not re-plan the substance,
which is done; this is the completion surface).

### 3.1 G-1 — the extension REST surface (W-2 row (b))

ITS-REST 1.0.3 vends no message endpoints and Messaging is an OPTIONS-profile
capability, so this is an **extension API** (out of CORE/STANDARD scope,
documented as an extension — exactly the `/terminology` and proposed
`/subject_proxy` pattern). It exists to make the 10 MSG ECC cases *executable*
so they stop being `SKIPPED` (W-2 zero-skip ruling). A minimal shape:

```
POST   /rest/message/ehr_extract/export            export_ehr_extracts (EXTRACT_SPEC body → List<EXTRACT>)
GET    /rest/message/ehr/{ehr_id}/extract          export_ehrs (whole-EHR, latest-only)
POST   /rest/message/ehr_extract/import            import_ehr        (EXTRACT body; optional ?ehr_id=)
POST   /rest/message/ehr/{ehr_id}/extract/import   import_ehr_extract
POST   /rest/message/ehr/{ehr_id}/tdd              import_tdd  (text/xml TDD body → created OBJECT_VERSION_ID)
POST   /rest/message/ehr/{ehr_id}/tdds             import_tdds (batch)
```

- Payloads are canonical openEHR JSON `EXTRACT` (the exact `openehr_rm::ehr_extract`
  wire shape the exports already produce); TDD bodies are `text/xml`.
- Auth: the standard authn stack; ATNA audit event per mutating call (SM
  `master02` System-Log requirement).
- OAS: add to the extension OpenAPI (`scripts/assemble-oas.sh`); document on the
  website book (same-PR docs rule).
- ECC: convert `suites/message.rs` from `SKIPPED(NativeApiOnly)` to real
  over-the-wire cases (export round-trip, clone/fixed-id/duplicate import, TDD
  commit/reject/batch). **Alternative permitted by W-2 (b):** if the wire is
  declined, reclassify each case **N/A** with a citation to the platform-suite
  evidence — but the owner ruling forbids leaving them `SKIPPED`.

*No openEHR spec governs the transport specifics of these endpoints — our own
extension design; the substance (payloads, semantics) is the vendored RM EHR
Extract IM + SM interfaces.*

### 3.2 G-2 / G-3 — deferred selectors (query-integration wave)

`EXTRACT_SPEC.criteria` (AQL primary-set selection, `master04:49`) and
`EXTRACT_VERSION_SPEC.commit_time_interval` land when the `$ehr`-bound AQL
export path is built. Until then the typed rejections are correct (fail-loud,
never silent over-export). Not conformance-gated (CNF `master13` is `TBD`).

### 3.3 G-4 — `namespace = "local"` rewrite on export

Implement the master09 step (`:77`): when serialising a version into the
extract, rewrite encountered `OBJECT_REF.namespace` to `"local"` (a recursive
walk analogous to `strip_inline_multimedia`), **or** record a deliberate
`PORT NOTE` if verbatim namespaces are intentionally preserved for round-trip
fidelity. Today it is silently neither.

### 3.4 G-5 / G-6 / G-13 — doc hygiene (cheap, do first)

- Rewrite the export `PORT NOTE` (`message.rs:32-37`) to state that demographic
  following, multimedia include/exclude, and `link_depth` are **implemented**;
  keep only the genuinely-deferred items (G-2/G-3/G-4).
- Delete the "version-branching = typed rejection" line (`message.rs:27`) — the
  import path lands branch lineages first-class (`vobject.rs:1843-1902`). State
  the actual behaviour.
- Scrub the `docs/design/sm-platform/10-message-integration.md` references in
  `message.rs`, `tdd.rs` (both crates), and `suites/message.rs`; point at this
  document (`09-message.md`) instead.

---

## 4. Standing PORT NOTEs (the honest residue after completion)

- `import_tdds` full signature + fail-fast batch semantics are an extension (SM
  defines no signature) — G-11.
- Synthetic `archetype_node_id` placeholders on the extract skeleton (no
  generating archetype exists) — G-9.
- Imported content is trusted (no re-validation) and OPT-unlinked
  (`template_id` NULL); imported clones do not populate the promoted
  `subject_id` — G-7 / G-8.
- ISO 13606 / CDA `GENERIC_CONTENT_ITEM` import out of scope (openEHR-only) —
  G-10.
- The REST surface (§3.1) is an extension: ITS-REST vends no message endpoints,
  Messaging is OPTIONS-profile — no CORE/STANDARD conformance impact.
- CNF `master13` messaging cases are `TBD` placeholders; the phantom singular
  `export_ehr()` names bind nothing — G-12.

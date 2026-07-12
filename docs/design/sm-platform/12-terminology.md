# Terminology Service (I_TERMINOLOGY_SERVICE) — spec-compliance audit

Read-only audit (2026-07-12) of our realization of the SM Terminology component
against the vendored spec. Unlike the SPS redesign (`10-subject-proxy.md`), the
Terminology interface is **substantially implemented and faithful**: nine calls,
a full extract model, two providers (in-process openEHR bundle + remote FHIR
R4), a config-gated extension wire, and the AQL `TERMINOLOGY()` family. This
document records what is spec-true, the residual gaps with citations, and the
target design for the gaps worth closing.

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master12-terminology_service.adoc`
  — the chapter (Overview + Class Definitions include list).
- `docs/specs/openehr/SM/docs/UML/classes/`:
  `i_terminology_service.adoc` (the 9 calls + preconditions),
  `terminology_description.adoc`, `terminology_extract.adoc`,
  `terminology_relation.adoc`, `term_relationship.adoc`, `term_code.adoc`,
  `defined_term.adoc` (the extract data model).
- Adjacent: `docs/specs/openehr/QUERY/docs/AQL/master03-syntax.adoc`
  §TERMINOLOGY (the AQL `TERMINOLOGY()`/`matches`-URI family, lines 748–767)
  — the only wire consumer of this service today.
- Reference (not spec): `docs/terminology-validation.md`,
  `docs/design/terminology-server-integration.md` (the FHIR-TS client + the
  Dockerised HAPI/Snowstorm server it points at).

**Current implementation** (verified 2026-07-12):

- **Interface + extract model**: `app/ehrbase-sm/src/services/terminology.rs`
  — `TerminologyService` trait (line 237, all 9 calls default to
  `NotImplemented`); `TerminologyDescription` (35), `TermCode` (52),
  `DefinedTerm` (61), `TermEntry` (85), `TermRelationship` (96),
  `TerminologyRelation` (113, `Inv_valid_definition` enforced by `new`, 143),
  `TerminologyExtract` (185) + `create_terminology_code` (208).
- **Provider A — openEHR-term bundle** (the local default): logic in
  `app/ehrbase/src/service/terminology.rs` (DB-free functions, 156–296),
  bound to `EhrbaseService` by the trait impl in
  `app/ehrbase/src/service/api/terminology.rs:201`.
- **Provider B — remote FHIR R4 TS** (opt-in): `FhirTerminologyProvider` in
  `app/ehrbase/src/terminology/fhir.rs:172` (impl), config in
  `app/ehrbase/src/terminology/config.rs`; held on `EhrbaseService` as
  `external_terminology: Option<Arc<FhirTerminologyProvider>>`
  (`app/ehrbase/src/service/mod.rs:98`), off by default.
- **Wire**: config-gated `/terminology` extension in
  `app/ehrbase-rest/src/dispatch/terminology.rs` (routes 53–89, gate 110). No
  ITS-REST contract exists for terminology — this is our own extension.
- **AQL family**: semantic pre-pass in `app/ehrbase/src/aql/terminology.rs`;
  the `TerminologyExpander` seam is implemented over both providers in
  `app/ehrbase/src/service/api/terminology.rs:38-162`.
- **Composed into `Platform`** via the `TerminologyService` supertrait bound
  (`app/ehrbase-sm/src/platform.rs:53`).
- **ECC**: a `TS` case area (ECC-TS-001..009, wiremock fixture +
  `--tx-server-url` real-server mode) shipped at B4.

---

## 1. Verified faithful realizations (evidence, not intent)

Every SM call and every extract type is present with spec-parity, so the audit
starts by banking what is correct — the gap register (§2) is the residue, not
the whole picture.

| Spec element | Spec citation | Our realization (file:line) | Verdict |
|---|---|---|---|
| **All 9 interface calls present** with matching signatures | `i_terminology_service.adoc` | trait `terminology.rs:237` → `240/249/258/271/287/302/316/331/344`; bundle impl `service/api/terminology.rs:201-273` | FAITHFUL |
| `Pre_has_terminology` (7 calls) | `i_terminology_service.adoc` | bundle guards via `has_terminology` → `unknown_terminology` (`service/terminology.rs:59-64,167-169,199,218,257,274,286`) | FAITHFUL |
| `Pre_has_term` (get_term) | `i_terminology_service.adoc` L51-52 | `service/terminology.rs:221-226` (unknown code → `VersionedObjectDoesNotExist`) | FAITHFUL |
| `Pre_has_value_set` (get_value_set) | `i_terminology_service.adoc` L89-90 | `service/terminology.rs:289-295` | FAITHFUL |
| `subsumes` = **strict** subsumption | `i_terminology_service.adoc` L63 "strict subsumption" | bundle: flat vocab ⇒ uniformly `false`, incl. identity (`service/terminology.rs:252-261`, test 385-397); FHIR: outcome `== "subsumes"` excludes `equivalent` (`fhir.rs:237-261`) | FAITHFUL |
| `Terminology_extract` shape (id, version, terms, relationships, relations) + `create_terminology_code` | `terminology_extract.adoc` | `terminology.rs:185-219`; `create_terminology_code` returns `openehr_base::TerminologyCode` (the BASE `Terminology_code`) | FAITHFUL |
| `Term_code` / `Defined_term` subtype choice in `terms` | `terminology_extract.adoc` L30-32; `defined_term.adoc` inherits `term_code` | `TermEntry` untagged enum (`terminology.rs:85-90`); a member with rubric → `Defined`, bare → `Bare` (`service/terminology.rs:120-146`, `fhir.rs:440-457`) | FAITHFUL |
| `Terminology_relation.Inv_valid_definition` (`local_code xor external_code`) | `terminology_relation.adoc` L28 | enforced by `TerminologyRelation::new` (`terminology.rs:143-156`, test 361-389) | FAITHFUL |
| `Term_relationship` (origin/relation_name/target_codes) | `term_relationship.adoc` | `terminology.rs:96-104` | FAITHFUL (type present; not yet emitted — see G-3) |
| `Terminology_description` (publisher/available_versions/attributes/uri) | `terminology_description.adoc` | `terminology.rs:35-47`; bundle populates publisher+uri+versions (`service/terminology.rs:172-195`) | FAITHFUL (`attributes` always `None` — see G-3) |
| AQL `TERMINOLOGY('expand'…)`, `matches`-URI, `validate`/`subsumes` boolean form | QUERY `master03-syntax.adoc` §TERMINOLOGY L748-767 | `aql/terminology.rs` semantic pre-pass; seam over both providers `service/api/terminology.rs:48-161` | FAITHFUL for the implemented staging; `lookup`/`map` typed-reject (`service/api/terminology.rs:121-125`) |

---

## 2. Gap register (what is not spec-true today)

Every gap cites the governing spec text. None is a missing call or a broken
precondition; the residue is temporal resolution, the extract meta-model, and
FHIR-provider coverage.

| # | Gap | Spec citation | Today (file:line) |
|---|-----|---------------|-------------------|
| G-1 | **`at_date` never changes any answer, on either provider.** The SM `has_term`/`get_term`/`value_set_validate` take an `Iso8601_date at_date` — "the response is the definition of the term as it was on a certain date" / "the code was present in the terminology at the date". For the **bundle** (single pinned TERM 3.1.0 version) this is defensible and PORT-NOTEd. For the **FHIR** provider it is a real gap: `at_date` is dropped (`_at_date`), never forwarded as the FHIR `date`/`version` param, so a versioned external server is queried at "now" regardless. | `i_terminology_service.adoc` L37-41, L48-53, L70; `terminology_extract.adoc` L9 ("single version or release") | bundle: accepted & ignored (`service/api/terminology.rs:217-225`; module PORT NOTE `service/terminology.rs:35-36`); FHIR: `_at_date` unused (`fhir.rs:185,269,287`) — **not** forwarded to `$lookup`/`$validate-code` |
| G-2 | **The extract meta-model (`relationships`, `relations`, `attributes`) is never populated.** `get_term`'s meaning is "Retrieve a term definition… **including particular attributes i.e. `Term_relationships`**"; `Terminology_extract` is explicitly designed to carry "a subsumption hierarchy below a specified concept" and `Terminology_description.attributes` lists "meta-model attributes that may be requested within extract requests." Both providers return term-only extracts; `relationships`/`relations` are always `None` and `description.attributes` is always `None`. | `i_terminology_service.adoc` L53; `terminology_extract.adoc` L9-16,34-39; `terminology_description.adoc` L24-25 | bundle `extract_from_members` sets `relationships: None, relations: None` (`service/terminology.rs:143-145`); FHIR `get_term` builds a single-term extract, no relationships (`fhir.rs:307-313`); `attributes: None` (`service/terminology.rs:180,187`) |
| G-3 | **`get_term.attributes` allow-list is accepted and silently ignored, and not surfaced on the wire.** The parameter selects which meta-model attributes to include in the extract; it is dropped on both providers and passed as `None` from the wire. Coupled to G-2: with no relationship model there is nothing to filter, but the parameter is not honoured or rejected — it is a no-op. | `i_terminology_service.adoc` L47, L53 | bundle impl ignores `_attributes` (`service/api/terminology.rs:227-237`); wire hard-codes `None` (`dispatch/terminology.rs:139-144`) |
| G-4 | **FHIR provider leaves 3 enumeration calls at the `NotImplemented` default.** `get_terminology_ids`, `has_terminology`, `get_terminology_description` are not overridden on `FhirTerminologyProvider` (the trait default 501s them). A deployment configured with **only** a FHIR TS therefore cannot enumerate or describe any terminology; those calls answer 501 rather than delegating to the bundle. | `i_terminology_service.adoc` L16-31 | PORT NOTE `fhir.rs:19-23`; only `get_terminology_description` is overridden (to a typed 501, `fhir.rs:328-336`); `get_terminology_ids`/`has_terminology` fall through to the trait default `terminology.rs:240,249` |
| G-5 | **FHIR `get_value_set` flattens hierarchy, losing the "structured value-set" / subsumption-hierarchy representation.** `Terminology_extract` is meant to represent "a structured value-set (aka 'ref set')" and "a subsumption hierarchy below a specified concept"; the FHIR `$expand` walker collapses the nested `expansion.contains` tree into a flat `terms` map, discarding the parent/child structure that `relationships` exists to carry. | `terminology_extract.adoc` L13-16, L34-39 | `FhirValueSet::into_extract` + `FhirContains::collect` recurse and flatten into one keyed map (`fhir.rs:409-457`) |
| G-6 | **`at_date` is not shape-validated as an `Iso8601_date`.** The trait models it as `Option<String>` (a deliberate weakening, module PORT NOTE) and no layer validates the string is an ISO-8601 date; the wire passes the raw query param straight through. A malformed `at_date` is silently accepted rather than rejected `400`. | `i_terminology_service.adoc` L37,48,70 (type `Iso8601_date`) | trait `terminology.rs:15-21` (Option<String>, "validated in shape by the caller" — but no caller validates); wire `dispatch/terminology.rs:138,168` passes `params::query_param` verbatim |
| G-7 | **`Pre_has_terminology` is not structurally enforced on the FHIR provider.** The bundle checks `has_terminology` first; the FHIR provider instead relies on a `404` from the operation mapping to `VersionedObjectDoesNotExist`. Equivalent for the happy/known-terminology path, but the abstract precondition is not evaluated before the operation, so a transport fault against an unknown terminology surfaces as `500`, not the precondition failure. | `i_terminology_service.adoc` L62,73,89 | FHIR ops map `404`→`VersionedObjectDoesNotExist`, else `500` (`fhir.rs:120-129,153-161`); no pre-flight `has_terminology` |
| G-8 | **`get_terminology_ids` optionality `[0..1]` collapsed.** The SM signature marks the call `0..1` (the server *may* not provide it); our trait returns `Vec<String>` unconditionally. Harmless (an implementation that provides it is conformant), recorded for completeness. | `i_terminology_service.adoc` L15-17 | `terminology.rs:240`; bundle always returns the list (`service/terminology.rs:156-164`) |

---

## 3. Target design (for the gaps worth closing)

G-6/G-7/G-8 are minor and mostly closed by a validation line or a note; G-1/
G-2/G-3/G-4/G-5 are the substantive items. None requires new SM surface — they
are provider-coverage and extract-fidelity work.

### 3.1 FHIR temporal parameter (G-1)

Forward `at_date` to the FHIR operations that accept it: `$validate-code` and
`$lookup` take a `date` parameter, `$expand` takes `valueSetVersion`/`date`.
`FhirTerminologyProvider::get` already builds a `&[(&str,&str)]` query list —
append `("date", at_date)` when `at_date` is `Some` and shape-valid. The bundle
keeps its PORT NOTE (single version). Verify with a wiremock case asserting the
`date` query param is present.

### 3.2 Extract meta-model + attribute filter (G-2, G-3, G-5)

- **FHIR structured value-sets**: when `$expand` returns hierarchical
  `contains`, emit the parent/child structure as `Term_relationship`s
  (`relation_name` = an `is_a`/`child` relation defined in `relations`) instead
  of flattening. `get_value_set`/`get_term` then carry the subsumption hierarchy
  the extract model is designed for (`terminology_extract.adoc` L13-16).
- **`get_term.attributes`**: once relationships exist, honour the allow-list by
  filtering the emitted relationship set to the requested attribute names;
  surface it on the wire as a repeatable `?attribute=` query param
  (replacing the hard-coded `None` at `dispatch/terminology.rs:144`). Until the
  bundle grows a relationship meta-model, keep its no-op behaviour but change
  the PORT NOTE from "ignored" to "no meta-model attributes defined for the
  openEHR bundle" (`terminology_description.attributes = None` is then correct
  by construction, not by omission).
- **Bundle**: openEHR's internal vocabulary is flat with no relationship
  meta-model, so `relationships`/`relations` legitimately stay empty — record
  it as a structural fact, not a deferral.

### 3.3 FHIR enumeration fallback (G-4)

Route `get_terminology_ids`/`has_terminology`/`get_terminology_description` so a
FHIR-only deployment still answers them. Options, in preference order:
(a) delegate enumeration to the bundle (the bundle is always compiled in and is
the enumerable terminology — the FHIR TS is a validation/expansion backend);
(b) probe the FHIR `CodeSystem`/`ValueSet` metadata endpoints where the server
exposes them. Simplest faithful realization: the composing `EhrbaseService`
trait impl (`service/api/terminology.rs:201`) already sits over *both*
providers — make the enumeration calls read the bundle and the validation/
expansion calls prefer the configured FHIR provider, so no single provider is
asked a call it cannot answer.

### 3.4 `at_date` validation (G-6)

Validate `at_date` against the BASE `Iso8601_date` lexical rule at the wire
boundary (`dispatch/terminology.rs`), rejecting a malformed value `400` before
it reaches the backend. Keep the trait's `Option<String>` (the SM date type is
partial-precision; a strong type buys nothing while the bundle is single-version).

### 3.5 Verification

- Per-provider unit tests already exist (`service/terminology.rs:298-438`,
  `fhir.rs:459-518`); add: FHIR `date` forwarding, FHIR hierarchy→relationships,
  attribute-filter on the wire, FHIR-only enumeration fallback, malformed
  `at_date` → `400`.
- Extend the `TS` ECC area (ECC-TS-001..009) with an `at_date` case and a
  structured-value-set case against the wiremock fixture.
- Gates: workspace suites green, clippy clean, full ECC zero-drift.

---

## 4. Standing PORT-NOTE residue (the honest boundary)

These are spec-silent or spec-defect points; each stays a documented note, not
open work:

- **No ITS-REST terminology contract** — the `/terminology` surface is our own
  extension, config-gated off by default, excluded from the ITS-REST drift
  check (`dispatch/terminology.rs:10-31`). *No openEHR spec governs a
  terminology REST wire — our own design/extension.* Boolean `has_*` calls are
  surfaced as `200`/`404` of their `get` counterparts, not separate endpoints.
- **`DefinedTerm.language` and `TerminologyRelation.external_code` carried as
  `String`, not `Terminology_code`** (`terminology.rs:67-70,119-121`) — the
  native API resolves rubrics/relations directly against the bundle; the strong
  BASE type buys nothing at this boundary. Faithful subset.
- **openEHR bundle is flat** — `subsumes` is identity-excluding-false and the
  extract meta-model is empty *by the terminology's nature* (SPECPR-51 group
  collision handled by the flat-any-group `has_term`/`get_term` view,
  `service/terminology.rs:19-26,202-209`). Hierarchical subsumption is the FHIR
  provider's job.
- **`service_api` identifier for the in-process bundle is `"openehr"`** — QUERY
  master03 §TERMINOLOGY defines only external-server examples; the bundle id is
  our adopted value (`service/api/terminology.rs:24-29`).
- **AQL `lookup`/`map` operations typed-reject** — no boolean/list comparison
  semantics in AQL (`service/api/terminology.rs:121-125`; module note
  `aql/terminology.rs:33-34`).
- **`at_date` single-version answer on the bundle** — TERM 3.1.0 is pinned;
  legitimate, kept (`service/terminology.rs:35-36`). (The FHIR forwarding gap
  G-1 is the part that is *not* residue but work.)
</content>
</invoke>

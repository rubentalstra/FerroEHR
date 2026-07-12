# Architecture Overview — full-document checklist (W-3a)

Oracle: the vendored openEHR **Architecture Overview** (BASE component),
`docs/specs/openehr/BASE/docs/architecture_overview/` — chapters
`master01-preface.adoc` … `master16-implementation.adoc` plus the amendment
record (`master00-amendment_record.adoc`), included in that order by
`master.adoc`. Numbering below follows the rendered document (`:sectnums:`
with `leveloffset=+1`): chapter 1 = Preface … chapter 16 = Implementation
Technology Specifications.

Row discipline (owner directive 2026-07-12, `docs/plans/arch-overview-PROMPT.md`):
one row per load-bearing statement under every numbered heading; verdicts are

- **verified** — realized in ehrbase-rs, with file-path evidence;
- **gap** — missing; each gap has a `WORKLIST.md` row (referenced inline);
- **informative** — context/narrative with nothing to implement (purely
  narrative subsections get one such row, not one per sentence).

A box is ticked only when the verdict is written and evidence-backed.

---

## 1 Preface (`master01-preface.adoc`)

### 1.1 Purpose

- [x] The Architecture Overview is the key technical overview of openEHR and
  should be read before all other technical documents; component-specific
  semantics live in the component specs (`master01-preface.adoc` §Purpose).
  **informative** — this checklist is the reading record; per-component
  semantics are audited by the blueprint chapters (`docs/blueprint/01-rm.md`
  … `07-cnf.md`).

### 1.2 Status

- [x] Status/TBD-paragraph conventions of the document
  (`master01-preface.adoc` §Status). **informative**.

### 1.3 Feedback

- [x] Feedback/PR-tracker channels (`master01-preface.adoc` §Feedback).
  **informative**.

## 2 Overview (`master02-overview.adoc`)

- [x] The abstract specifications (PIM) are split from the Implementation
  Technology Specifications (ITS/PSM): abstract components BASE, LANG/AM/QUERY,
  TERM/RM, PROC/CDS, Platform Services; ITSs are their concrete
  JSON/XML/BMM/REST expressions and constitute the interoperability
  specifications (`master02-overview.adoc` §The openEHR Specification
  Program). **verified** — the repo mirrors the split exactly: abstract
  models are generated from the vendored machine-readable specs into
  `crates/openehr-base`, `openehr-rm`, `openehr-am`, `openehr-term`,
  `openehr-lang`, `openehr-query`; the ITS surfaces live in
  `crates/openehr-its` (canonical JSON/XML + generated ITS-REST contract).
- [x] The CNF component defines conformance criteria — a formal definition of
  the notional openEHR Platform and how to test it — based primarily on ITS
  artefacts (`master02-overview.adoc` §The openEHR Specification Program).
  **verified** — the vendored CNF schedule is the acceptance oracle
  (`docs/specs/openehr/CNF/`), executed by the ECC runner
  (`tools/conformance/`, reports in `docs/conformance/`).
- [x] The published specifications constitute the primary reference for all
  openEHR semantics (`master02-overview.adoc` §The openEHR Specification
  Program). **verified** — standing repo rule: the vendored spec text is the
  only oracle (`.claude/rules/spec-adherence.md`; spec-only citations in
  code).
- [x] Specification program history, SEC change control, Asciidoctor sourcing
  (`master02-overview.adoc` §The openEHR Specification Program).
  **informative**.

## 3 Aims of the openEHR Architecture (`master03-aims.adoc`)

### 3.1 Overview

- [x] The reference model embodies only concepts relating to 'service and
  administrative events relating to a subject of care'; specifics of care
  events and subjects are defined in archetypes and templates — the
  architecture is generic across scope (episodic → population) and subject
  kind (`master03-aims.adoc` §Overview). **verified** — the RM is consumed
  as-generated with no domain hard-wiring (`crates/openehr-rm/`); domain
  content arrives only via uploaded OPTs/templates
  (`app/ehrbase/src/service/template.rs`).

#### 3.1.1 Generic Care Record Requirements

- [x] Medico-legal faithfulness, traceability and audit-trailing are core
  record requirements (`master03-aims.adoc` §Generic Care Record
  Requirements). **verified** — every write emits contribution + audit rows
  in the same transaction (`app/ehrbase/src/service/vobject.rs`,
  `app/ehrbase/migrations/`; change-control semantics audited against RM
  common in full).
- [x] Support for clinical data structures: lists, tables, time-series
  including point and interval events (`master03-aims.adoc` §Generic Care
  Record Requirements). **verified** — generated `data_structures` package
  incl. `HISTORY`/`POINT_EVENT`/`INTERVAL_EVENT`
  (`crates/openehr-rm/src/data_structures/`).
- [x] Technology & data-format independence; maintainable software
  (`master03-aims.adoc` §Generic Care Record Requirements). **informative**
  (design rationale).

#### 3.1.2 Health Care Record (EPR)

- [x] Support for pathology data incl. normal ranges and alternative unit
  systems (`master03-aims.adoc` §Health Care Record (EPR)). **verified** —
  `DV_QUANTITY`/`DV_INTERVAL` reference ranges are generated RM types
  (`crates/openehr-rm/src/data_types/quantity/`), validated at commit
  (`app/ehrbase/src/service/opt_validation/`).
- [x] Supports all natural languages and translations in the record
  (`master03-aims.adoc` §Health Care Record (EPR)). **verified** — language
  is carried per COMPOSITION/ENTRY (generated RM), and text encoding is
  canonical JSON/XML UTF-8 (`crates/openehr-its/`); see also row 6.7.
- [x] Integrates with any/multiple terminologies (`master03-aims.adoc`
  §Health Care Record (EPR)). **verified** — `TerminologyService` trait with
  openEHR-bundle and external FHIR-TS providers
  (`app/ehrbase-sm/src/services/terminology.rs`,
  `app/ehrbase/src/terminology/`).

#### 3.1.3 Shared Care EHR

- [x] Support for patient privacy including anonymous EHRs
  (`master03-aims.adoc` §Shared Care EHR). **verified** — EHR creation
  without subject id is supported; ECC evidences AnonymousEhrs as a CORE
  capability (`docs/conformance/`; see row 7.3.3 for the PARTY_SELF
  mechanism).
- [x] Sharing via interoperability at data and knowledge levels; 13606/
  messaging compatibility; distributed workflow support
  (`master03-aims.adoc` §Shared Care EHR). **informative** — realized
  concretely by the Extract/versioning rows (ch 8, ch 9) where normative.

### 3.2 Clinical Aims

- [x] Clinical motivations (integrated care record, decision support, safety)
  (`master03-aims.adoc` §Clinical Aims). **informative**.

### 3.3 Deployment Environments

- [x] The architecture supports many system categories (shared-care EHR,
  hospital EMR, GP systems, gateways, web EHR systems); demographic links in
  the EHR are optional, enabling anonymised/pseudonymised deployments
  (`master03-aims.adoc` §Deployment Environments). **informative** — the
  normative mechanism (optional subject reference) is row 7.3.3.

## 4 Design Principles (`master04-design_principles.adoc`)

### 4.1 Ontological Separation

- [x] Information models, domain content models and terminologies are three
  separated categories, each with limited scope and clear interfaces —
  domain-level semantics must not be hard-wired into software or databases
  (`master04-design_principles.adoc` §Ontological Separation). **verified**
  — RM = generated crates (`crates/openehr-rm/`); domain content = runtime
  OPT/template store (`app/ehrbase/src/service/template.rs`); terminology
  behind its own service seam (`app/ehrbase-sm/src/services/terminology.rs`).

#### 4.1.1 Multi-level Modelling and Archetypes

- [x] Three model levels: stable reference model; reusable archetypes;
  context-specific templates — only the RM (plus stable representation
  languages) is implemented in software; archetypes/templates are consumed
  at runtime (`master04-design_principles.adoc` §Multi-level Modelling and
  Archetypes). **verified** — the RM/AM/BASE types are the only spec models
  compiled in (generated, `crates/openehr-*`); archetypes/templates are data
  ingested at runtime (`app/ehrbase/src/service/template.rs`, cached
  WebTemplates), never code.
- [x] Archetypes are expressed in the generic Archetype Definition Language
  (ADL) (`master04-design_principles.adoc` §Multi-level Modelling and
  Archetypes). **gap (tracked)** — OPT 1.4 XML ingestion exists
  (`app/ehrbase/src/service/opt_validation.rs`); ADL2 source parsing is the
  W-4 mandate (`docs/plans/WORKLIST.md` W-4); ADL 1.4 source parsing is not
  a platform-conformance requirement (OPT is the deployment artefact) — ADL2
  is where the language lands.

#### 4.1.2 Consequences for Software Engineering

- [x] Under multi-level modelling, the core system (storage, querying,
  caching) is generic and stable; domain semantics are delegated to
  archetype/template/terminology authors outside the software process
  (`master04-design_principles.adoc` §Consequences for Software
  Engineering). **verified** — storage is RM-generic (one `node` +
  `vo_version` design, no per-archetype schema:
  `app/ehrbase/migrations/`); AQL is archetype-driven at query time
  (`app/ehrbase/src/aql/`).

### 4.2 Separation of Responsibilities

- [x] Functionality is partitioned into coarse-grained services (SOA) with
  defined interfaces (`master04-design_principles.adoc` §Separation of
  Responsibilities). **verified** — one trait per SM Platform Service
  interface (`app/ehrbase-sm/src/services/`), REST as a protocol adapter
  (`app/ehrbase-rest/`).
- [x] openEHR's service scope: patient-centric data/process services,
  enterprise-centric services, knowledge services; openEHR sometimes adapts
  existing standards rather than redefining them
  (`master04-design_principles.adoc` §Separation of Responsibilities).
  **informative**.

### 4.3 Separation of Viewpoints

- [x] openEHR separates an information viewpoint (the Reference Model) from a
  computational viewpoint (the Service Model), with ITS as the engineering
  viewpoint; no 1:1 relationship between models across viewpoints is assumed
  (`master04-design_principles.adoc` §Separation of Viewpoints). **verified**
  — RM (`crates/openehr-rm/`) vs SM traits (`app/ehrbase-sm/`) vs ITS
  (`crates/openehr-its/` + `app/ehrbase-rest/` adapter) are distinct layers
  with downward-only dependencies (root `Cargo.toml` workspace layout).

## 5 openEHR Specification Structure (`master05-package_structure.adoc`)

### 5.1 Overview

- [x] Specifications come as language specs (grammars), information models
  (UML) and service models (UML); ITS artefacts are API definitions, XML/JSON
  schemas, BMM schemas (`master05-package_structure.adoc` §Overview).
  **verified** — codegen consumes exactly these artefact classes: BMM
  (`crates/openehr-codegen/vendor/bmm/`), XSDs
  (`crates/openehr-its/schemas/xml/`), OAS
  (`crates/openehr-its/vendor/rest-oas/`), grammars for AQL
  (`crates/openehr-query/`).

### 5.2 Consolidated Package Structure

- [x] Top-level UML packages are `base`, `lang`, `rm`, `am`, `proc`, `sm`
  within the `org.openehr` namespace; detailed models all appear inside one
  of them (`master05-package_structure.adoc` §Consolidated Package
  Structure). **verified** — crate-per-component with matching internal
  package modules (`crates/openehr-base/src/{foundation_types,base_types,resource}`,
  `crates/openehr-rm/src/*`, `crates/openehr-am/src/{am14,am24}`); `proc`
  (Task Planning/GDL) is out of product scope — no openEHR platform-
  conformance requirement attaches to it (CNF schedule has no PROC cases).

### 5.3 Base Component (BASE)

- [x] The `base` package defines identifiers, data types, data structures and
  common design patterns reused by `rm`, `am`, `sm`
  (`master05-package_structure.adoc` §Base Component (BASE)). **verified** —
  `crates/openehr-base/` (BASE 1.3.0, generated; `docs/VERSIONS.md`).

#### 5.3.1 Foundation Types

- [x] Foundation Types defines the primitive types assumed in external type
  systems plus basic structures (`Array<T>`, `Hash<K,V>`), time types and
  functional types, as the mapping basis into implementation languages
  (`master05-package_structure.adoc` §Foundation Types). **verified** —
  `crates/openehr-base/src/foundation_types/` with the documented primitive
  mappings (`Integer`→`i32`, `Real`→`f64`, etc.) applied by the emitter
  (`crates/openehr-codegen/src/emit.rs`).

#### 5.3.2 Base Types

- [x] Base Types comprises `definitions`, `identification`, `terminology`
  and `measurement` sub-packages, giving all other models identifiers and
  access to knowledge services (`master05-package_structure.adoc` §Base
  Types). **verified** — `crates/openehr-base/src/base_types/{definitions,identification}`
  plus the terminology classes emitted under
  `crates/openehr-base/src/foundation_types/terminology/` (BMM package
  placement); the `measurement` package content is the abstract
  `MEASUREMENT_SERVICE` interface only — **informative** residue, no data
  semantics to implement (units validation is row 15.4).

#### 5.3.3 Resource Model

- [x] A generic authored-resource class carries authorship, licence,
  language/translation and annotation meta-data, inherited by resource-like
  types (`master05-package_structure.adoc` §Resource Model). **verified** —
  `crates/openehr-base/src/resource/` (AUTHORED_RESOURCE family), inherited
  by the AM archetype/template types (`crates/openehr-am/`).

### 5.4 Languages Component (LANG)

- [x] LANG holds ODIN (object data syntax), BMM (meta-model language) and EL
  (expression language) (`master05-package_structure.adoc` §Languages
  Component (LANG)). **verified** — `crates/openehr-lang/src/{bmm,bmm3,bmm_persistence,beom}`
  (BEOM = the expression object model); ODIN parsing lives with its consumer
  (see 5.4.2).

#### 5.4.1 Basic Meta-Model (BMM)

- [x] BMM formally expresses object-oriented models for tool consumption;
  understanding/using BMM is not required to implement openEHR, but is a
  convenient format for model processing such as code generation
  (`master05-package_structure.adoc` §Basic Meta-Model (BMM)). **verified**
  — BMM is this repo's codegen input: vendored `*.bmm.json`
  (`crates/openehr-codegen/vendor/bmm/`) → `openehr-lang::bmm` loader →
  emitted spec crates; plus the AQL planner's BMM-generated RM model
  (`crates/openehr-rm/src/model/`).

#### 5.4.2 Object Data Instance Notation (ODIN)

- [x] ODIN implements faithful machine (de)serialisation of object graphs,
  with leaf types, in-built typing and paths; used in ADL archetypes and BMM
  schemas (`master05-package_structure.adoc` §Object Data Instance Notation
  (ODIN)). **verified** — ODIN section parsing is implemented where ADL2
  artefacts are consumed (`app/ehrbase/src/service/adl2_validation.rs` and
  its `adl2_validation/` module tree); full ADL2-grade ODIN coverage is in
  scope of W-4 (`docs/plans/WORKLIST.md`).

#### 5.4.3 Expression Language (EL)

- [x] EL is a subset of first-order predicate logic expressions underpinning
  ADL rules, GDL and Task Planning (`master05-package_structure.adoc`
  §Expression Language (EL)). **verified (types) / gap-scoped** — the
  expression object model is generated (`crates/openehr-lang/src/beom/`);
  evaluation of ADL2 `rules` sections belongs to the W-4 ADL2 mandate
  (`docs/plans/WORKLIST.md` W-4).

### 5.5 Reference Model Component (RM)

- [x] The `rm` package splits into domain-related (`ehr`, `demographic`,
  `ehr_extract`, `composition`, `integration`) and generic (`common`,
  `data_structures`, `data_types`, `support`) packages; the package
  structure is normally replicated in ITS expressions
  (`master05-package_structure.adoc` §Reference Model Component (RM)).
  **verified** — `crates/openehr-rm/src/{ehr,demographic,ehr_extract,composition,integration,common,data_structures,data_types,support}` (RM 1.2.0, generated).

#### 5.5.1 Package Overview

- [x] Sub-package roles as listed below (`master05-package_structure.adoc`
  §Package Overview). **informative** (roll-up; details in 5.5.1.1–5.5.1.9).

##### 5.5.1.1 Support Information Model

- [x] The former RM `support` package moved to BASE
  (`master05-package_structure.adoc` §Support Information Model).
  **verified** — identifiers live in
  `crates/openehr-base/src/base_types/identification/`; the residual RM
  `support` module (`crates/openehr-rm/src/support/`) holds what RM 1.2.0
  still defines there.

##### 5.5.1.2 Data Types Information Model

- [x] The data types IM defines basic types, text (plain/coded/paragraph),
  quantities (incl. ordinals, date/times, partial dates), encapsulated data
  (multimedia, parsable), time specification, and URIs
  (`master05-package_structure.adoc` §Data Types Information Model).
  **verified** — `crates/openehr-rm/src/data_types/{basic,text,quantity,encapsulated,time_specification,uri}`.

##### 5.5.1.3 Data Structures Information Model

- [x] Generic structures Single/List/Table/Tree plus History time-series
  (point and interval samples) express archetype-defined content
  (`master05-package_structure.adoc` §Data Structures Information Model).
  **verified** — `crates/openehr-rm/src/data_structures/` (`ITEM_SINGLE`,
  `ITEM_LIST`, `ITEM_TABLE`, `ITEM_TREE`, `history` with
  `POINT_EVENT`/`INTERVAL_EVENT`).

##### 5.5.1.4 Common Information Model

- [x] `LOCATABLE`/`ARCHETYPED` link information to archetypes; `ATTESTATION`
  and `PARTICIPATION` document professional involvement incl. signing
  (`master05-package_structure.adoc` §Common Information Model). **verified**
  — `crates/openehr-rm/src/common/archetyped/`,
  `crates/openehr-rm/src/common/generic/` (participation, attestation,
  audit types).
- [x] The `change_control` package is a formal model of change management and
  versioning for any service that must supply previous states (EHR,
  demographics) (`master05-package_structure.adoc` §Common Information
  Model). **verified** — `crates/openehr-rm/src/common/change_control/`
  (types) realized by the versioned-object storage (ch 8 rows).

##### 5.5.1.5 Security Information Model

- [ ] The Security IM defines access control and privacy-setting semantics
  for EHR information (`master05-package_structure.adoc` §Security
  Information Model). *(verdict pending — see ch 7 rows)*

##### 5.5.1.6 EHR Information Model

- [x] The EHR IM (`ehr` + `composition` packages) defines the containment and
  context semantics of `EHR`, `COMPOSITION`, `SECTION`, `ENTRY`
  (`master05-package_structure.adoc` §EHR Information Model). **verified** —
  `crates/openehr-rm/src/{ehr,composition}/`; served at the ITS-REST surface
  (`app/ehrbase-rest/`).

##### 5.5.1.7 EHR Extract Information Model

- [x] The Extract IM defines how an EHR extract is built from Compositions,
  demographic and access-control information, in several variations
  (`master05-package_structure.adoc` §EHR Extract Information Model).
  **verified** — generated `crates/openehr-rm/src/ehr_extract/`; export/
  import implemented in the message service (ch 8/ch 14 rows;
  `app/ehrbase/src/service/message.rs`).

##### 5.5.1.8 Integration Information Model

- [x] `GENERIC_ENTRY` represents free-form legacy/external data as a tree,
  archetyped by integration archetypes (`master05-package_structure.adoc`
  §Integration Information Model). **verified (types)** —
  `crates/openehr-rm/src/integration/`; behavioural use is ch 14 rows.

##### 5.5.1.9 Demographics Information Model

- [x] The demographic model defines generic `PARTY`, `ROLE` and contact
  details, with archetypes constraining any person/organisation/role type
  (`master05-package_structure.adoc` §Demographics Information Model).
  **verified** — `crates/openehr-rm/src/demographic/`; demographic service
  (`app/ehrbase/src/service/demographic.rs`).

### 5.6 Archetype Model Component (AM)

- [x] Two extant major archetype-technology versions — ADL 1.4 and ADL 2 —
  are maintained side by side; implementers may work with either
  (`master05-package_structure.adoc` §Archetype Model Component (AM)).
  **verified (models) / gap (ADL2 behaviour)** — both are generated
  (`crates/openehr-am/src/am14/`, `crates/openehr-am/src/am24/`); ADL 1.4
  OPT ingestion is implemented; the full ADL2 pipeline is W-4
  (`docs/plans/WORKLIST.md`).
- [x] The AM consists of ADL (syntax) and AOM (object model of archetypes);
  archetype identification/versioning/lifecycle semantics come from the
  Archetype Identification specification
  (`master05-package_structure.adoc` §Archetype Model Component (AM)).
  **verified** — AOM generated (`crates/openehr-am/`); archetype-id
  parsing/validation in the template/definition services
  (`app/ehrbase/src/service/definition.rs`).

### 5.7 Service Model (SM)

- [x] The SM defines basic services centred on the EHR; the included set
  evolves (`master05-package_structure.adoc` §Service Model (SM)).
  **verified** — one trait per SM Platform Service interface, complete
  SM-1..SM-6 catalogue (`app/ehrbase-sm/src/services/`; component map in
  `docs/architecture.md`).

#### 5.7.1 Definitions Service

- [x] The Definitions Service is the interface to repositories of archetypes,
  templates and AQL queries (`master05-package_structure.adoc` §Definitions
  Service). **verified** — `DefinitionAdl14Service`/`DefinitionAdl2Service`/
  `DefinitionQueryService` (`app/ehrbase-sm/src/services/`), wired at
  `/definition/*` (`app/ehrbase-rest/src/dispatch/definition.rs`).

#### 5.7.2 EHR Service

- [x] The EHR Service is the coarse-grained interface at
  Contribution/Composition granularity — a version-control/change-set
  interface (`master05-package_structure.adoc` §EHR Service). **verified** —
  `EhrService`/`EhrCompositionService`/`EhrContributionService` etc.
  (`app/ehrbase-sm/src/services/`), REST dispatch
  (`app/ehrbase-rest/src/dispatch/ehr.rs`).
- [x] Part of the model covers server-side querying returning small
  aggregated answers (`master05-package_structure.adoc` §EHR Service).
  **verified** — AQL aggregates execute server-side
  (`app/ehrbase/src/aql/`).

#### 5.7.3 Query Service

- [x] The Query Service executes stored or ad-hoc AQL queries
  (`master05-package_structure.adoc` §Query Service). **verified** —
  `QueryService` (`app/ehrbase-sm/src/services/`), stored queries
  (`app/ehrbase/src/service/stored_query.rs`), `/query/*` wire.

#### 5.7.4 Terminology Interface

- [x] The Terminology Service abstracts underlying terminology architectures
  and is the gateway to terminology/ontology knowledge services
  (`master05-package_structure.adoc` §Terminology Interface). **verified** —
  `TerminologyService` trait with openEHR-bundle + external FHIR providers
  (`app/ehrbase-sm/src/services/terminology.rs`,
  `app/ehrbase/src/terminology/`).

### 5.8 Global View

- [x] Dependencies exist only from higher components to lower ones; CNF and
  ITS are derivative of the primary specifications
  (`master05-package_structure.adoc` §Global View). **verified** — workspace
  dependency arrows point downward only (`tools/* → app/* →
  crates/openehr-*`; root `Cargo.toml`).

---

*(Chapters 6–16 + amendment record follow in subsequent commits of this
checklist — the codebase-verification sweep for those rows is in flight.)*

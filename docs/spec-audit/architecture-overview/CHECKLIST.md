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

- [x] The Security IM defines access control and privacy-setting semantics
  for EHR information (`master05-package_structure.adoc` §Security
  Information Model). **verified (types) / gap (evaluation)** — openEHR has
  never published the concrete Security IM; what the RM 1.2.0 BMM defines is
  generated (`crates/openehr-rm/src/ehr/access_control_settings.rs` — the
  abstract, attribute-less `ACCESS_CONTROL_SETTINGS` extension point, plus
  `ehr_access.rs`/`versioned_ehr_access.rs`); per-EHR access-control
  *evaluation* is the ch 7 gap → WORKLIST **W-9**.

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

## 6 Design of the openEHR EHR (`master06-design_of_the_ehr.adoc`)

### 6.1 The EHR System

- [x] An EHR *system* is a distinct logical repository under one legally
  responsible entity; the technical criterion identifying it is that it is
  the entity assigning version identifiers within the repository
  (`master06-design_of_the_ehr.adoc` §The EHR System). **verified** — one
  `system_id` per EHR repository, immutable, and version identity is minted
  by the server (`app/ehrbase/migrations/ehr/0001_baseline.sql:60`
  `ehr.system_id`; `app/ehrbase/src/service/vobject.rs` assigns
  `creating_system_id`/`version_tree_id`).

#### 6.1.1 System Identity

- [x] `system_id` is recorded in each EHR (`EHR` class), in the audit of
  every commit (`AUDIT_DETAILS`), and in feeder-system audits
  (`FEEDER_AUDIT_DETAILS`) (`master06-design_of_the_ehr.adoc` §System
  Identity). **verified** — `ehr.system_id`
  (`0001_baseline.sql:60`), `audit.system_id NOT NULL`
  (`0001_baseline.sql:91`), FEEDER_AUDIT round-tripped as an addressable
  node (`app/ehrbase/src/storage/codec.rs:51`) and stamped on FHIR import
  (`app/ehrbase/src/service/fhir/mapping.rs:35`).
- [x] The system identifier may be of any form (reverse domain, GUID, OID)
  and is not assumed directly processable
  (`master06-design_of_the_ehr.adoc` §System Identity). **verified** —
  stored as opaque `text`, never dereferenced
  (`0001_baseline.sql:60`; `app/ehrbase/src/service/ehr.rs:39`).

#### 6.1.2 Information Architecture

- [x] A minimal openEHR system consists of an EHR repository, archetype
  repository, terminology and demographic/identity information; EHR and
  demographic information are completely separated — an EHR in isolation
  carries little or no clue to patient identity
  (`master06-design_of_the_ehr.adoc` §Information Architecture).
  **verified** — EHR content references the subject only via `PARTY_SELF`
  (`app/ehrbase/src/service/ehr.rs:716` subject must be PARTY_SELF);
  demographics are a standalone versioned repository with `NULL ehr_id`
  (`0001_baseline.sql:244`; `app/ehrbase/src/service/demographic.rs`).

### 6.2 Top-level Information Structures

- [x] The top-level structures are Composition, EHR Access, EHR Status,
  Folder hierarchies, Party, and EHR Extract; all persistent content lives
  within top-level structures (`master06-design_of_the_ehr.adoc` §Top-level
  Information Structures). **verified** — the versioned-object kinds cover
  exactly these: `vo_version.kind IN ('COMPOSITION','EHR_STATUS',
  'EHR_ACCESS','FOLDER', 'PERSON','ORGANISATION','GROUP','AGENT','ROLE',
  'PARTY_RELATIONSHIP')` (`0001_baseline.sql:244`); EHR_EXTRACT is the
  transmission unit built by the message service
  (`app/ehrbase/src/service/message.rs`).

### 6.3 The EHR

- [x] The EHR comprises: a root EHR object with a globally unique EHR id;
  versioned EHR_access; versioned EHR_status (optionally holding the subject
  id); an optional versioned directory; versioned Compositions; and
  Contributions as the change-set records referencing the versions committed
  together (`master06-design_of_the_ehr.adoc` §The EHR). **verified** —
  `ehr` table + generated `Ehr` type
  (`0001_baseline.sql:60`, `crates/openehr-rm/src/ehr/ehr.rs:12`);
  EHR_ACCESS auto-created and versioned per EHR
  (`app/ehrbase/src/service/ehr.rs:75`, default at `ehr.rs:607`);
  EHR_STATUS versioned with full version/revision-history wire
  (`app/ehrbase-rest/src/dispatch/ehr.rs:230-372`); directory versioned
  (`app/ehrbase/src/service/directory.rs`); contributions FK'd from every
  version (`0001_baseline.sql:128`, `vo_version.contribution_id NOT NULL`).
- [x] Additional optional folder hierarchies (`EHR.folders`) beyond
  `directory` can logically organise Compositions
  (`master06-design_of_the_ehr.adoc` §The EHR). **gap** — the generated RM
  carries `Ehr.folders: Vec<ObjectRef>`
  (`crates/openehr-rm/src/ehr/ehr.rs:31`), but the server enforces a single
  root FOLDER per EHR (`app/ehrbase/src/service/directory.rs:25`;
  `app/ehrbase/src/service/contribution.rs:481` "at most one directory");
  no service/wire path manages multiple named hierarchies (ITS-REST defines
  only `/directory`). → WORKLIST **W-6**.
- [x] The 21 data types provide for all clinical/administrative data; a
  typical Composition nests Sections → Entries → data structures → data
  types (`master06-design_of_the_ehr.adoc` §The EHR). **verified** —
  generated `crates/openehr-rm/src/data_types/` +
  `crates/openehr-rm/src/composition/`.

### 6.4 Entries and Clinical Statements

#### 6.4.1 Entry Subtypes

- [x] An Entry is logically a single clinical statement; there are five
  concrete subtypes — `ADMIN_ENTRY`, `OBSERVATION`, `EVALUATION`,
  `INSTRUCTION`, `ACTION` — the latter four being kinds of `CARE_ENTRY`
  (`master06-design_of_the_ehr.adoc` §Entry Subtypes). **verified** — all
  five generated (`crates/openehr-rm/src/composition/content/entry/`),
  accepted as composition content and invariant-validated on commit
  (`crates/openehr-flat/src/validation/mod.rs:307-341`;
  `app/ehrbase/src/service/composition.rs:433`).

##### 6.4.1.1 Ontology of Entry Types

- [x] The Entry model derives from the clinical investigation process
  (observation / opinion / instruction / action + administrative
  information); it imposes no process model, only information types
  (`master06-design_of_the_ehr.adoc` §Ontology of Entry Types).
  **informative** — modelling rationale; category fidelity is delivered by
  archetypes, not server logic.

##### 6.4.1.2 Clinical Statement Status and Negation

- [x] 'Status' variants (history-of, risk-of, fear-of) and negations are
  modelled as different information categories (e.g. exclusion archetypes on
  the appropriate Entry type), not status flags — so querying does not match
  wrong categories (`master06-design_of_the_ehr.adoc` §Clinical Statement
  Status and Negation). **informative** — archetype-authoring discipline;
  the server's role (typed Entry subtypes + archetype-path querying) is
  covered by rows 6.4.1 and 10.6.

### 6.5 Managing Interventions

- [x] Interventions are specified with `INSTRUCTION`/`ACTIVITY` and recorded
  with `ACTION`; every intervention's state is knowable in terms of the
  standard Instruction state machine, with careflow steps mappable to
  machine states, so the EHR is queryable for e.g. "all active medications"
  (`master06-design_of_the_ehr.adoc` §Managing Interventions). **verified**
  — `instruction.rs`, `activity.rs`, `action.rs`, `ism_transition.rs`,
  `instruction_details.rs` generated
  (`crates/openehr-rm/src/composition/content/entry/`);
  `ISM_TRANSITION.current_state`/`transition` are validated against the
  openEHR `instruction_states`/`instruction_transition` terminology groups
  on every commit (`crates/openehr-flat/src/validation/terminology.rs:229`).
  Transition-*legality* checking (a full careflow state machine) is not a
  platform-conformance requirement — the RM/CNF mandate is valid states and
  transitions as terminology, which is enforced.

### 6.6 Time in the EHR

- [x] Times that are a by-product of the investigation process (sampling,
  measurement, business event, committal) are concretely modelled in the RM;
  content-specific times are archetyped over generic data attributes
  (`master06-design_of_the_ehr.adoc` §Time in the EHR). **verified** —
  generated RM time attributes (`HISTORY.origin`, `EVENT.time`,
  `COMPOSITION.context.start_time`, audit `time_committed`:
  `crates/openehr-rm/src/data_structures/`,
  `crates/openehr-rm/src/composition/`; `audit.time_committed`
  `0001_baseline.sql:91`).

### 6.7 Language

- [x] Language is mandatorily indicated in Compositions and in Entries,
  allowing mixed-language records; text/coded-text items may optionally
  carry their own language (`master06-design_of_the_ehr.adoc` §Language).
  **verified** — `language` is non-optional on the generated
  `COMPOSITION`/ENTRY types
  (`crates/openehr-rm/src/composition/composition.rs:39`), and codes are
  validated against ISO 639-1 on commit
  (`crates/openehr-flat/src/validation/terminology.rs:223-249`);
  `DV_TEXT.language` is optional per the RM.
- [x] Translations can conveniently be recorded as branch versions attached
  to the version they translate; this is not mandatory
  (`master06-design_of_the_ehr.adoc` §Language). **verified** — branch
  versions are fully supported in storage and service
  (`0001_baseline.sql:196` branch columns + constraints;
  `app/ehrbase/src/service/vobject.rs:1212` branch forking;
  `app/ehrbase/tests/service_branching.rs`).

## 7 Security and Confidentiality (`master07-security.adoc`)

### 7.1 Requirements

#### 7.1.1 Privacy, Confidentiality and Consent

- [x] Data sharing must be controlled by patient consent; differential access
  to record parts is a requirement, complicated by the interrelatedness of
  health information (`master07-security.adoc` §Privacy, Confidentiality and
  Consent). **informative** — requirements framing; the openEHR-specified
  mechanisms are the 7.3/7.4 rows.

#### 7.1.2 Requirements of Healthcare Providers

- [x] Fast, faithful access; emergency access consented only generally;
  research access — all must coexist with consent
  (`master07-security.adoc` §Requirements of Healthcare Providers).
  **informative**.

#### 7.1.3 Specifying Access Control

- [x] Access control must be expressible in terms of categories/role types,
  not only identified individuals (`master07-security.adoc` §Specifying
  Access Control). **verified** — role/attribute-based policies via the
  Cedar/remote-PDP authz engine (`app/ehrbase-rest/src/access/authz/`).

#### 7.1.4 The Problem of Roles

- [x] Role evaluation into identities must happen in the care-delivery
  environment, not in the EHR (`master07-security.adoc` §The Problem of
  Roles). **informative** — consistent with the out-of-band authorization
  placement (`app/ehrbase-rest/src/access/mod.rs:18`).

#### 7.1.5 Usability

- [x] Security mechanisms must be usable (sensible defaults, exceptions)
  (`master07-security.adoc` §Usability). **informative**.

### 7.2 Threats to Security and Privacy

- [x] Assumed threat model (mis-identification, inappropriate access, theft,
  integrity/availability threats, software failure)
  (`master07-security.adoc` §Threats to Security and Privacy).
  **informative**.

### 7.3 Solutions Provided by openEHR

#### 7.3.1 Overview

- [x] openEHR directly specifies: EHR/demographic separation, an EHR-wide
  access-control object, mandatory commit audits, and digital
  signatures/hashes at the versioned-object level; other mechanisms
  (authentication, encryption, concrete access control) belong to
  deployments (`master07-security.adoc` §Overview). **verified** — each
  element evidenced in the rows below; authn is deployment-level
  (`app/ehrbase-rest/src/access/authn/` Basic argon2 + OAuth2/OIDC JWT).

#### 7.3.2 Security Policy

##### 7.3.2.1 General

- [x] **Indelibility**: health record information cannot be deleted; logical
  deletion is marking via version control (`master07-security.adoc`
  §General). **verified** — logical delete = content-less version,
  lifecycle 523, prior versions retained
  (`app/ehrbase/migrations/ehr/0001_baseline.sql:201`;
  `app/ehrbase/src/service/vobject.rs`).
- [x] **Audit trailing**: all changes — content, EHR status and access
  objects alike — are audit-trailed with user identity, time-stamp, reason,
  optional signature and version information; the subject-as-modifier may
  use the symbolic `PARTY_SELF` (`master07-security.adoc` §General).
  **verified** — mandatory audit per version incl. EHR_ACCESS/EHR_STATUS
  kinds (`0001_baseline.sql:91`, `vo_version.audit_id NOT NULL`;
  `app/ehrbase/src/service/contribution.rs`).
- [x] **Anonymity**: record content is separate from identifying
  demographics, configurable so EHR theft yields no direct identity clue
  (`master07-security.adoc` §General). **verified** — subject only via
  `PARTY_SELF` with optional external_ref
  (`app/ehrbase/src/service/ehr.rs:716`); demographics in separate stores
  (`app/ehrbase/src/service/subject_proxy.rs`,
  `app/ehrbase/src/service/demographic.rs`).

##### 7.3.2.2 Access Control

- [x] An EHR access-control list (identified individuals and categories) with
  a gate-keeper controlling changes to it, and patient-settable per-
  Composition privacy levels (levels defined by jurisdictions, not
  hard-wired) (`master07-security.adoc` §Access Control). **gap** — the
  versioned EHR_ACCESS container exists (row 6.3) but no access-list/
  gate-keeper/privacy-mark evaluation is implemented (no "privacy" mechanism
  in `crates/`/`app/`); authorization is server-level RBAC/ABAC
  (`app/ehrbase-rest/src/access/authz/`). openEHR publishes no concrete
  `ACCESS_CONTROL_SETTINGS` scheme, so realization requires a scheme
  decision. → WORKLIST **W-9**.
- [x] Sensible-default usability posture for access settings
  (`master07-security.adoc` §Access Control). **informative** (design
  guidance for the W-9 scheme).
- [x] Policy items *not* directly specified by openEHR: read-access logging,
  record de/merging, time-limitation of access, non-repudiation via
  mandatory signing, key certification (`master07-security.adoc` §Access
  Control). **verified where implemented** — read/API access logging exists
  via the ATNA system log (`app/ehrbase/src/system_log/`,
  `app/ehrbase-rest/src/audit.rs`); non-repudiation via stored PGP
  signatures (`app/ehrbase/src/signing/`); the rest are deployment concerns
  the spec itself places outside openEHR — no openEHR spec governs them.

#### 7.3.3 Integrity

##### 7.3.3.1 Versioning

- [x] Integrity is grounded in change-set versioning: no content is ever
  physically modified, only new Versions created; Contributions are the
  unit-of-work integrity boundary; audits (and optional signed attestations)
  cover every write (`master07-security.adoc` §Versioning). **verified** —
  ch 8 rows (`vo_version` append-only temporal model, atomic contributions,
  mandatory audits, `vo_attestation`).

##### 7.3.3.2 Digital Signature

- [x] Each Version may be digitally signed: a private-key encryption of a
  hash of a canonical representation of the Version, openPGP (RFC 4880)
  format being the candidate; without a key infrastructure the encryption
  step may be omitted leaving a digest; the signature is stored within the
  Version (`master07-security.adoc` §Digital Signature). **verified** —
  exactly this design: `SignerMode::Pgp` (RFC 4880 detached signature via
  rPGP) or `Digest` (sha256), computed over the Version's RFC 8785 canonical
  JSON, stored in `vo_version.signature`
  (`app/ehrbase/src/signing/signer.rs:38`;
  `app/ehrbase/src/service/vobject.rs:581`;
  `0001_baseline.sql:222`); verify-on-read policy
  (`app/ehrbase/src/signing/verify.rs`).
- [x] Signatures *can* be forwarded to a trusted notarisation service; small
  per-Version signing localises corruption impact
  (`master07-security.adoc` §Digital Signature). **informative** —
  permissive ("can be"); notarisation forwarding is a deployment option, not
  implemented; per-Version granularity is what the signing module does.

#### 7.3.4 Anonymity

- [x] `PARTY_SELF` carries only an optional external reference; three
  configurable separation levels — no subject reference anywhere, once only
  in EHR_STATUS.subject, or in every PARTY_SELF instance
  (`master07-security.adoc` §Anonymity). **verified** — blank `PARTY_SELF`
  accepted (anonymous EHR) and external_ref validated when present
  (`app/ehrbase/src/service/ehr.rs:662-770` + regression tests
  `ehr.rs:919-976`); the level is per-record data, not server-forced.

### 7.4 Access Control

#### 7.4.1 Overview

- [x] Access control is completely specified in the `EHR_ACCESS` object,
  which acts as the gateway for all information access; alternative access-
  control models are accommodated as subtypes of `ACCESS_CONTROL_SETTING`,
  the scheme in use always indicated in the EHR Access object
  (`master07-security.adoc` §Overview). **gap** — EHR_ACCESS is stored,
  versioned, defaulted at EHR creation and contribution-writable
  (`app/ehrbase/src/service/ehr.rs:75,607`;
  `app/ehrbase/src/service/contribution.rs:609`), and the abstract
  `ACCESS_CONTROL_SETTINGS` type is generated
  (`crates/openehr-rm/src/ehr/access_control_settings.rs`), but no access
  decision consults it — authorization runs out-of-band per the SM placement
  (`app/ehrbase-rest/src/access/mod.rs:18`). The spec itself notes no
  formal, proven access-control model for shared health records exists.
  → WORKLIST **W-9** (scheme decision + evaluation or a cited PORT NOTE).

## 8 Versioning (`master08-versioning.adoc`)

### 8.1 Overview

- [x] An openEHR repository is managed as a change-controlled collection of
  version containers (`VERSIONED_OBJECT<T>`, `common.change_control`), one
  per top-level content structure (`master08-versioning.adoc` §Overview).
  **verified** — `vo_version` keyed by `vo_id` with successive versions
  (`app/ehrbase/migrations/ehr/0001_baseline.sql:187`); generated
  change-control types (`crates/openehr-rm/src/common/change_control/`).
- [x] Changes are made to the repository as change-sets ("Contributions")
  that act like transactions, taking the repository from one consistent
  state to another (`master08-versioning.adoc` §Overview). **verified** —
  one CONTRIBUTION per change set, all versions committed in one DB
  transaction (`app/ehrbase/src/service/contribution.rs:4`;
  `vo_version.contribution_id NOT NULL` `0001_baseline.sql:229`).

### 8.2 The Configuration Management Paradigm

#### 8.2.1 Organisation of the Repository

- [x] A controlled repository consists of uniquely identified configuration
  items, an optional directory system, and environmental information
  (`master08-versioning.adoc` §Organisation of the Repository). **verified**
  — versioned objects (CIs) + optional versioned FOLDER directory
  (`0001_baseline.sql:244`; `app/ehrbase/src/service/directory.rs`).

#### 8.2.2 Change Management

- [x] Change occurs to the repository as a whole; CM must ensure the
  repository is always valid, any previous state can be reconstructed, and
  all changes are audit-trailed (`master08-versioning.adoc` §Change
  Management). **verified** — atomic contributions (above); previous states
  reconstructible via `version_at` time-travel
  (`app/ehrbase/src/service/vobject.rs:2047`) and REVISION_HISTORY
  (`app/ehrbase/src/service/versioned.rs:17`); every version carries a
  mandatory audit (`audit` table `0001_baseline.sql:91`).

### 8.3 Managing Changes in Time

- [x] The kinds of change to items in a Contribution are: addition (new
  version container + first version); deletion (new version with data set to
  Void); modification (new version with updated content); import (a new
  'import' Version incorporating the received Version); attestation (a new
  Attestation added to an existing Version's attestations list)
  (`master08-versioning.adoc` §Managing Changes in Time). **verified** — all
  five: creation/modification via `commit_contribution`
  (`app/ehrbase/src/service/contribution.rs`); logical delete = content-less
  version, lifecycle_state 523, never physical
  (`0001_baseline.sql:201`); import preserves the received identity tuple
  (`app/ehrbase/src/service/vobject.rs:1751` `commit_import`,
  `app/ehrbase/tests/service_import.rs:196`); attestation appended without a
  new version (`vo_attestation` `0001_baseline.sql:379`;
  `vobject.rs:927` `attest`).
- [x] A Contribution is the set of Versions created or attested at one time;
  whether deltas are used internally is an implementation matter
  (`master08-versioning.adoc` §Managing Changes in Time). **verified** —
  contribution = the version set committed together
  (`contribution` table `0001_baseline.sql:128`); storage is whole-version
  decomposed nodes, an implementation choice the spec leaves open (no
  openEHR spec governs storage mechanics — our own design).

#### 8.3.1 General Model of a Change-controlled Repository

- [x] A change-controlled repository = versioned CIs + CONTRIBUTIONs + an
  optional folder directory which, if used, must itself be versioned as a
  unit (`master08-versioning.adoc` §General Model of a Change-controlled
  Repository). **verified** — FOLDER is a versioned kind, one root FOLDER
  versioned as a unit (`0001_baseline.sql:244`;
  `app/ehrbase/src/service/directory.rs`).

### 8.4 The Virtual Version Tree

- [x] Version identification must make all copies of a Versioned object
  across systems compatible with one "virtual" version tree — no
  inconsistencies due to sharing, logical copies explicitly represented
  (mirroring/synchronisation and shared longitudinal Compositions are the
  driving scenarios) (`master08-versioning.adoc` §The Virtual Version Tree).
  **verified** — full 3-part version identity with per-version
  `creating_system_id` (`0001_baseline.sql:208`), strict 3-part
  `OBJECT_VERSION_ID` parsing
  (`crates/openehr-base/src/base_types/identification/object_version_id_impl.rs:26`),
  and automatic branch-forking when a foreign-system version is modified
  locally (`app/ehrbase/src/service/vobject.rs:1212`;
  `app/ehrbase/tests/service_branching.rs`).

## 9 Identification (`master09-identification.adoc`)

### 9.1 Identification of the EHR

- [x] Each EHR has a unique EHR id (strong global identifier) in the root
  EHR object; **no single system should contain two EHRs for the same
  subject** (`master09-identification.adoc` §Identification of the EHR).
  **verified** — `ehr.id uuid` PK (`0001_baseline.sql:60`); partial unique
  index `uq_ehr_subject (subject_id, subject_namespace)` with
  unique-violation mapped to 409 Conflict
  (`0001_baseline.sql:80`; `app/ehrbase/src/service/vobject.rs:1082`).
- [x] In integrated distributed environments the same EHR id is reused
  across locations for one patient (clone all or part of the existing EHR,
  or create empty with the same id)
  (`master09-identification.adoc` §Identification of the EHR). **verified**
  — clone-EHR import with reused ehr_id via the extract import path
  (`app/ehrbase/src/service/message.rs`;
  `app/ehrbase/tests/service_import.rs`).

### 9.2 Identification of Items within the EHR

#### 9.2.1 General Scheme

- [x] Identification distinguishes identifiers proper (written into the
  object; `OBJECT_ID` descendants) from references/locators (used by
  exterior objects; `OBJECT_REF` descendants), both in
  `support.identification` (`master09-identification.adoc` §General
  Scheme). **verified** — full generated family: `object_id.rs`,
  `hier_object_id.rs`, `object_version_id.rs`, `object_ref.rs`,
  `locatable_ref.rs`, `party_ref.rs`, etc.
  (`crates/openehr-base/src/base_types/identification/`).

#### 9.2.2 Levels of Identification

- [x] Identification operates at three levels: version containers (UIDs —
  `HIER_OBJECT_ID`, UUIDs preferred); versions (the globally unique 3-part
  tuple `versioned_object.uid` + `creating_system_id` + `version_tree_id`,
  formalised as `OBJECT_VERSION_ID`); interior nodes (paths)
  (`master09-identification.adoc` §Levels of Identification). **verified** —
  UUID `vo_id` (`0001_baseline.sql:187`); 3-part id stored (`trunk_version`/
  `branch_number`/`branch_version` + `creating_system_id`) and served as
  `uuid::system::tree`
  (`app/ehrbase/src/service/vobject.rs:1197`,
  `app/ehrbase/src/service/ehr.rs:578`); node paths materialized per node
  (`node.path` `0001_baseline.sql:307`).
- [x] The contained top-level content item's `uid` (from `LOCATABLE`) is
  strongly recommended to be populated with a copy of the containing
  VERSION's `OBJECT_VERSION_ID`
  (`master09-identification.adoc` §Levels of Identification). **verified** —
  `with_uid` injects the OBJECT_VERSION_ID into served content
  (`app/ehrbase/src/service/ehr.rs:565`; same for demographic parties,
  `app/ehrbase/src/service/demographic.rs:665`).
- [x] A VERSION is referred to with an `OBJECT_REF` carrying its
  `OBJECT_VERSION_ID`; an interior node from outside requires version
  locator + path (`LOCATABLE_REF`), expressible as a `DV_EHR_URI` in the
  `ehr:` scheme; every `LOCATABLE_REF` converts to a `DV_EHR_URI` but not
  vice versa (`master09-identification.adoc` §Levels of Identification).
  **verified (types + scheme)** — `locatable_ref.rs` generated;
  `DV_EHR_URI` with `Scheme_valid` invariant enforced
  (`crates/openehr-rm/src/data_types/uri/dv_ehr_uri_impl.rs:17`). Full
  `ehr:` URI grammar/resolution is assessed at rows 11.4.x.

---

## 10 Archetypes and Templates (`master10-archetypes.adoc`)

### 10.1 Overview

- [x] All RM-conformant information is archetypable; archetypes are separate
  from data, stored in their own repository, and deployed at runtime via
  templates (`master10-archetypes.adoc` §Overview). **verified** — template/
  archetype store + runtime WebTemplate cache
  (`app/ehrbase/src/service/template.rs`,
  `app/ehrbase/src/service/definition.rs`); no archetype semantics compiled
  into the server.
- [x] The archetypes used at creation time are written into the data: the
  multipart archetype identifier at root nodes and `[atNNNN]` node ids as
  normative node names — the basis for paths — enabling later modification
  to retrieve and respect the original archetypes
  (`master10-archetypes.adoc` §Overview). **verified** —
  `archetype_node_id` mandatory and validated
  (`crates/openehr-rm/src/validate.rs:100`;
  `crates/openehr-flat/src/validation/mod.rs:259`); stored per node with
  nearest-archetype scoping (`app/ehrbase/src/storage/codec.rs:150`;
  `node.archetype` `0001_baseline.sql:309`).
- [x] Queries are expressed in a synthesis of SQL and XPath extracted from
  archetypes (`master10-archetypes.adoc` §Overview). **verified** — AQL
  front end + engine (`crates/openehr-query/`, `app/ehrbase/src/aql/`).

### 10.2 Archetype Formalisms and Models

#### 10.2.1 Overview

- [x] The AOM is the definitive statement of archetype semantics; ADL is the
  normative lossless serialisation; XML/ODIN serialisations exist; ADL2
  templates are ODIN documents conforming to the AOM
  (`master10-archetypes.adoc` §Overview). **verified (models) / gap (ADL2
  pipeline)** — AOM 1.4 + AOM 2 generated
  (`crates/openehr-am/src/{am14,am24}/`); OPT 1.4 XML ingestion
  (`crates/openehr-its/src/opt14/`); the ADL2 parser/flattener is W-4
  (`app/ehrbase/src/service/adl2_validation.rs:1-49` states the current
  registration-surface-only scope; `docs/plans/WORKLIST.md` W-4).

#### 10.2.2 Design-time Relationships between Archetypes

- [x] Specialisation rule: a specialised archetype only narrows parent
  constraints, so **data created with a specialised archetype is always
  matched by queries based on the parent archetype** (subsumption);
  specialised ids derive from the parent id with a '-'-separated sub-element
  (`master10-archetypes.adoc` §Design-time Relationships between
  Archetypes). **gap (query side)** — specialised ids lex/parse fine
  (`crates/openehr-query/src/lexer.rs:205,453`) and AOM carries
  `parent_archetype_id`/`specialisation_depth`
  (`crates/openehr-am/src/am14/aom14/archetype/archetype.rs:44`), but AQL
  archetype matching is exact case-folded equality
  (`app/ehrbase/src/aql/sql.rs:632` `lower(archetype) = lower(value)`) — a
  parent-archetype query does not match specialised-child data.
  → WORKLIST **W-7**.
- [x] Composition via slots: an `allow_archetype` constraint names the
  archetypes usable at a chaining point, simplest form being regular
  expressions on archetype identifiers; templates choose among allowed
  archetypes (`master10-archetypes.adoc` §Design-time Relationships between
  Archetypes). **verified** — slot admission with includes/excludes regexes
  + RM-type conformance + occurrences
  (`crates/openehr-flat/src/validation/mod.rs:904` `slot_admits`,
  `mod.rs:514-575`).

### 10.3 Relationship of Archetypes and Templates to Data

- [x] Every top-level type is an archetype root point; hierarchies of
  archetypes create interior root points (ENTRY instances almost always;
  top SECTION/FOLDER instances; potentially lower structures); data in any
  top-level object conforms to the template-chosen archetype composition
  including optionality, value and terminology constraints
  (`master10-archetypes.adoc` §Relationship of Archetypes and Templates to
  Data). **verified** — per-node archetype scoping through interior root
  points (`app/ehrbase/src/storage/codec.rs:150`); commit-time conformance
  walk (`app/ehrbase/src/service/composition.rs:433`;
  `crates/openehr-flat/src/validation/`).

### 10.4 Archetype-enabling of Reference Model Data

- [x] `LOCATABLE` supplies `archetype_node_id` and `archetype_details`: a
  root point carries the generating archetype's multipart id plus an
  `ARCHETYPED` object; a non-root node carries the at-code with
  `archetype_details` void (`master10-archetypes.adoc` §Archetype-enabling
  of Reference Model Data). **verified** — non-root-must-not-carry arm
  enforced (`crates/openehr-flat/src/validation/mod.rs:259-288`); the
  converse arm (root must carry `archetype_details`) is a recorded
  deviation with citation (PORT NOTE `mod.rs:266`, A1
  rm-common-change-control-R46) pending stricter enforcement.
- [x] Sibling nodes may carry the same `archetype_node_id` (archetypes are
  patterns, not exact templates) (`master10-archetypes.adoc`
  §Archetype-enabling of Reference Model Data). **verified** — storage keys
  nodes positionally, not by at-code
  (`app/ehrbase/src/storage/codec.rs:214`; node identity
  `(vo_id, sys_version, num)` `0001_baseline.sql:323`).

### 10.5 Archetypes, Templates and Paths

- [x] Paths are constructed from attribute names + archetype node ids in an
  Xpath-compatible syntax; node ids embedded in data make archetype paths
  the basis for extraction/querying (`master10-archetypes.adoc` §Archetypes,
  Templates and Paths). **verified** — path parsing
  (`crates/openehr-query/src/parser.rs:250-331`) and execution against
  stored nodes (`app/ehrbase/src/aql/`); per-node `aql_path` in the
  WebTemplate (`crates/openehr-flat/src/webtemplate/builder.rs`).

### 10.6 Archetypes and Templates at Runtime

#### 10.6.1 Overview

- [x] Archetypes/templates have two runtime functions: data validation at
  capture/import, and the design basis for queries
  (`master10-archetypes.adoc` §Overview). **verified** — commit validation
  (`app/ehrbase/src/service/composition.rs:433`) + archetype-path querying
  (`app/ehrbase/src/aql/`).

#### 10.6.2 Deploying Archetypes and Templates

- [x] Archetypes come from quality-assured repositories; templates are local;
  deployments may compile archetypes/templates into a near-runtime form
  incorporating the relevant archetypes (`master10-archetypes.adoc`
  §Deploying Archetypes and Templates). **verified** — OPT upload is the
  deployment artefact; the built WebTemplate (cached) is exactly the
  compiled near-runtime form (`app/ehrbase/src/service/template.rs`,
  `crates/openehr-flat/src/webtemplate/builder.rs`).

#### 10.6.3 Validation during Data Capture

- [x] By committal time the mediating template is fully specified and
  committed data are guaranteed to conform to the template/archetype
  definitions, carrying the semantic imprint (node ids on every node)
  (`master10-archetypes.adoc` §Validation during Data Capture). **verified**
  — template-driven conformance validation on every create/update
  (`app/ehrbase/src/service/composition.rs:433-448`); at-code imprint
  validated (`crates/openehr-rm/src/validate.rs:100`).

#### 10.6.4 Querying

- [x] AQL = SQL-style SELECT/FROM/WHERE + archetype paths, e.g. querying BMI
  observations by archetype and path predicate
  (`master10-archetypes.adoc` §Querying). **verified** — full pipeline
  parse → analyze → IR → SQL → RESULT_SET
  (`crates/openehr-query/`; `app/ehrbase/src/aql/{analyze,ir,sql,exec}.rs`).

### 10.7 The openEHR Archetypes

- [x] The CKM archetype library (`master10-archetypes.adoc` §The openEHR
  Archetypes). **informative**.

## 11 Paths and Locators (`master11-paths.adoc`)

### 11.1 Overview

- [x] Any node in a top-level structure is addressable by an archetype-based
  X-path-compatible path; path + version identifier = globally qualified
  node reference (`LOCATABLE_REF`), expressible as a `DV_EHR_URI` locator
  (`master11-paths.adoc` §Overview). **verified (reference types + paths)** —
  `locatable_ref.rs` generated
  (`crates/openehr-base/src/base_types/identification/`); node paths
  materialized per node (`node.path` `0001_baseline.sql:315`); URI form —
  see 11.3 rows.

### 11.2 Paths

#### 11.2.1 Basic Syntax

- [x] Paths are slash-separated attribute-name segments; relative (starts
  with a name) and absolute (starts with `/`) forms
  (`master11-paths.adoc` §Basic Syntax). **verified** — AQL object paths
  (`crates/openehr-query/src/parser.rs:324` `objectPath`); archetype paths
  in the WebTemplate (`crates/openehr-flat/src/webtemplate/builder.rs`).
- [x] The `//` notation defines a path *pattern* matching any number of
  segments (`master11-paths.adoc` §Basic Syntax). **gap** — no `//`
  descendant construct anywhere (the AQL 1.1 grammar has none;
  `parser.rs:324-331` requires non-empty segments). → WORKLIST **W-8**.

#### 11.2.2 Predicate Expressions

##### 11.2.2.1 Overview

- [x] Bracketed predicates select among container siblings (omitting the
  predicate selects the whole container) and can express boolean conditions
  over paths, operators and values — a subset of Xpath predicates with
  shortcuts (`master11-paths.adoc` §Overview). **verified** — predicate AST
  with And/Or trees + standard comparisons
  (`crates/openehr-query/src/ast.rs:247-325`,
  `parser.rs:244-295`).

##### 11.2.2.2 Archetype path Predicate

- [x] `[atNNNN]` is the shortcut for `[@archetype_node_id = 'atNNNN']`; an
  archetype path unique in the archetype may match multiple data items
  (`master11-paths.adoc` §Archetype path Predicate). **verified** — node-code
  predicates (`ast.rs:282` `NodePredicate::Code`); multi-match handled by
  positional node storage (`app/ehrbase/src/storage/codec.rs:214`).

##### 11.2.2.3 Name-based Predicate

- [x] `[at0001 and name/value='standing']` and the comma shortcut
  `[at0001, 'standing']` select by node id + name
  (`master11-paths.adoc` §Name-based Predicate). **verified** — both forms
  parsed (`parser.rs:265-295` `NodeNameConstraint`) and executed against the
  stored `node.name` (`app/ehrbase/src/aql/analyze.rs:473-541`;
  `0001_baseline.sql:314`).

##### 11.2.2.4 Other Predicates

- [x] Predicates over other attribute values (e.g. `time >= …`,
  `value/defining_code/…`) combine with node-id predicates
  (`master11-paths.adoc` §Other Predicates). **verified** — general
  path-comparison predicates (`parser.rs:244-248` standard predicates;
  lowered in `app/ehrbase/src/aql/analyze.rs`).

#### 11.2.3 Paths within Top-level Structures

- [x] Paths strictly follow RM attribute names; at archetype chaining points
  the predicate carries the archetype id (same `[xxx]` shorthand as
  at-codes) to distinguish sibling structures
  (`master11-paths.adoc` §Paths within Top-level Structures). **verified** —
  archetype-HRID predicates on path segments
  (`crates/openehr-query/src/ast.rs:271-296`,
  `lexer.rs:205`); storage records the governing archetype per node
  (`app/ehrbase/src/storage/codec.rs:150`).

#### 11.2.4 Data Paths and Uniqueness

##### 11.2.4.1 Using a Uid-based Predicate

- [x] Populating `LOCATABLE.uid` with UUIDs gives reliably unique node paths
  (`[uid='…']`, optionally with the at-code)
  (`master11-paths.adoc` §Using a Uid-based Predicate). **verified** —
  expressible as a standard path-comparison predicate
  (`crates/openehr-query/src/parser.rs:244-248`); top-level uid injection is
  row 9.2.2.
- [x] In general, sibling property-value uniqueness is not required; only
  positional predicates guarantee unique paths
  (`master11-paths.adoc` §Data Paths and Uniqueness). **informative** —
  storage-internal uniqueness uses positional indices
  (`app/ehrbase/src/storage/codec.rs:214`); the *query-language* positional
  form is the next row.

##### 11.2.4.2 Using a Name-based Predicate

- [x] Name-based unique paths require the system to ensure sibling `name`
  uniqueness; the name/value predicate forms map to standard Xpath
  (`master11-paths.adoc` §Using a Name-based Predicate). **verified** —
  name predicates work (row 11.2.2.3); sibling-name uniqueness is not
  server-forced, matching the spec's conditional ("if … known to be reliably
  populated").

##### 11.2.4.3 Using Positional Parameters

- [x] Xpath positional parameters (`items[1]`) give guaranteed-unique paths
  where container order is preserved (`master11-paths.adoc` §Using
  Positional Parameters). **gap** — integer-index predicates are not in the
  AQL 1.1 grammar and not parsed (`crates/openehr-query/src/parser.rs`
  `PathPredicate` has no positional alternative); container order itself is
  preserved in storage (`node.num` ordering). → WORKLIST **W-8**.

### 11.3 EHR URIs

- [x] `DV_EHR_URI` (RM `data_types`) carries `ehr:`-scheme references that
  can only refer to entities within an openEHR EHR
  (`master11-paths.adoc` §EHR URIs). **verified** — generated type with the
  `Scheme_valid` invariant enforced
  (`crates/openehr-rm/src/data_types/uri/dv_ehr_uri_impl.rs:17`).

#### 11.3.1 EHR Reference URIs

- [x] The `ehr:` URI model —
  `ehr://system_id/ehr_id/top_level_structure_locator/path`, with EHR
  location (11.3.1.1), top-level structure locators by VERSIONED_OBJECT uid
  (latest trunk assumed) or exact 3-part version id (11.3.1.2), item URIs
  with full paths (11.3.1.3), and relative URIs (11.3.1.4)
  (`master11-paths.adoc` §EHR Reference URIs). **gap** — beyond the scheme
  check, no `ehr:` URI grammar parser/resolver exists (no dereferencing of
  system_id/ehr_id/structure-locator/path). The spec itself notes `ehr:`
  name resolution infrastructure does not yet exist and ad-hoc means are
  expected; still, parse/validate + local resolution is implementable.
  → WORKLIST **W-8**.
- [x] Plain-text URIs with RFC-3986-forbidden characters are allowed for
  readability but must be RFC-3986-encoded before use in REST APIs
  (`master11-paths.adoc` §EHR Reference URIs). **verified** — the REST layer
  handles percent-encoded paths/params via `urlencoding` throughout
  (`app/ehrbase-rest/`; workspace-wide codec rule), and `DV_URI` value
  validation is invariant-checked
  (`crates/openehr-rm/src/data_types/uri/`).

## 12 Terminology in openEHR (`master12-terminology.adoc`)

### 12.1 Overview

- [x] Terminology is used four ways: openEHR terminology for coded RM
  attributes; archetype-internal terminology; bindings to external
  terminologies; querying via those bindings
  (`master12-terminology.adoc` §Overview). **verified** — rows 12.2–12.5.

### 12.2 Terminology to Support the Reference Model

- [x] Six code sets (meaningful codes, `CODE_PHRASE`-typed attributes) plus
  group-based value sets (meaningless codes + rubrics, `DV_CODED_TEXT`
  attributes, code in `defining_code`) supply RM attribute values
  (`master12-terminology.adoc` §Terminology to Support the Reference
  Model). **verified** — all six code-set identifiers defined and bound
  (`crates/openehr-rm/src/support/terminology/openehr_code_set_identifiers_impl.rs:11`;
  `crates/openehr-flat/src/validation/terminology.rs:58-124` incl.
  `compression_algorithms`/`integrity_check_algorithms` at 116-119 and the
  multimedia attributes at 259-263); bundle assets byte-identical
  (`crates/openehr-term/src/bundle.rs`).

### 12.3 Archetype Internal Terminology

- [x] Each archetype carries its own flat internal terminology: at-codes
  either name data nodes or provide leaf value sets; the archetype path is
  the alternating pattern of RM attribute names and node codes
  (`master12-terminology.adoc` §Archetype Internal Terminology).
  **verified** — ontology term definitions drive WebTemplate labels/rubrics
  (`crates/openehr-flat/src/webtemplate/builder.rs:989-1040`) and coded/
  ordinal value validation (`crates/openehr-flat/src/validation/leaf.rs:134`
  `check_ordinal`; `validation/terminology.rs:278`).

### 12.4 Binding to External Terminologies

#### 12.4.1 Binding External Terminology Codes to Archetype Codes

- [x] Internal codes bind to external-terminology codes, grouped per
  terminology; bindings may be path-scoped (pre-coordinated mapping holds
  only on that path) or atomic (holds everywhere)
  (`master12-terminology.adoc` §Binding External Terminology Codes to
  Archetype Codes). **verified (carried)** — `term_bindings` parsed from OPT
  and surfaced on WebTemplate coded values
  (`crates/openehr-flat/src/webtemplate/builder.rs:1066`
  `collect_term_bindings`; `webtemplate/inputs.rs:28`); bindings are
  informational mappings by design here — validation constraints are the
  RM-fixed bindings (row 12.2) and template value lists.

##### 12.4.1.1 Binding Terminology Value-sets to Archetypes

- [x] "ac" constraint codes bind to *queries* against external terminologies
  whose result is a value set; the spec notes **no standard exists for such
  queries** — archetypes hold only an identifier resolved by a terminology
  query server (`master12-terminology.adoc` §Binding Terminology Value-sets
  to Archetypes). **verified (by recorded policy)** — `CONSTRAINT_REF`/
  constraint_definitions are parsed into the AOM and deliberately treated as
  open (no node/rejection) in the WebTemplate walk
  (`crates/openehr-flat/src/webtemplate/builder.rs:314`;
  `crates/openehr-its/src/opt14/types.rs:71`); external value-set resolution
  is available at query time via `TERMINOLOGY('expand', …)` (row 12.5).

### 12.5 Querying using External Terminologies

- [x] Coding of data and code use in queries must be governed by common
  models; openEHR querying uses archetype paths plus terminology relations
  (e.g. equal-to-or-subsumed-by) over path-constrained values
  (`master12-terminology.adoc` §Querying using External Terminologies).
  **verified (envelope) / informative (subsumption operator)** — path-based
  querying with terminology value-set expansion is implemented
  (`app/ehrbase/src/aql/terminology.rs:106` `expand_matches`; FHIR
  `$subsumes` primitive `app/ehrbase/src/terminology/fhir.rs:237`); a native
  "subsumed-by" AQL operator does not exist in the AQL 1.1 grammar — value-
  set expansion is the spec'd AQL 1.1 mechanism, so nothing further is owed
  at the platform surface (the openEHR-bundle provider's identity-only
  `subsumes` carries its own PORT NOTE,
  `app/ehrbase/src/service/terminology.rs:30`).

## 13 Deployment (`master13-deployment.adoc`)

### 13.1 5-tier System Architecture

- [x] The 5 tiers — persistence, back-end services, virtual EHR
  (middleware + archetype kernel), application logic, presentation — and the
  approximate mapping: RM+AM → the kernel; `common.change_control` → the
  versioning logic of versioned services; SM packages → exposed service
  interfaces (`master13-deployment.adoc` §5-tier System Architecture).
  **verified** — the server realizes tiers 1–3 with exactly that mapping:
  persistence (`app/ehrbase/migrations/`, `app/ehrbase/src/storage/`),
  services behind SM traits (`app/ehrbase-sm/`,
  `app/ehrbase/src/service/`), the archetype kernel = WebTemplate build +
  validation walk (`crates/openehr-flat/`); applications/presentation are
  out of product scope (a CDR serves tier 3 downward).
- [x] A future abstract persistence API / optimised persistence models may be
  published by openEHR (`master13-deployment.adoc` §5-tier System
  Architecture). **informative** — none published; storage mechanics remain
  implementation-defined (no openEHR spec governs them — our own design).

## 14 Integrating openEHR with other Systems (`master14-integration.adoc`)

### 14.1 Overview

- [x] Legacy data conversion must handle scope, structure, terminology and
  data-type mismatches into a single standardised patient-centric EHR
  (`master14-integration.adoc` §Overview). **informative** — requirements
  framing for the rows below.

### 14.2 Integration Archetypes

- [x] Two archetype categories: "designed" archetypes (Entry subtypes,
  designed from scratch) define target semantics; "integration" archetypes
  (same high-level types but `GENERIC_ENTRY`) mimic legacy/external
  structures, one per message/source type
  (`master14-integration.adoc` §Integration Archetypes). **verified
  (platform side)** — `GENERIC_ENTRY` generated and accepted as composition
  content (`crates/openehr-rm/src/integration/generic_entry.rs`;
  `crates/openehr-rm/src/composition/content/content_item.rs:20`;
  validation `crates/openehr-flat/src/validation/mod.rs:337`); authoring
  integration archetypes is a modelling activity, not server logic.

### 14.3 Data Conversion Architecture

- [x] Import is two steps: (1) syntactic conversion into
  COMPOSITION/SECTION/GENERIC_ENTRY structures with `FEEDER_AUDIT`
  meta-data; (2) semantic transformation via integration→designed archetype
  mappings (`master14-integration.adoc` §Data Conversion Architecture).
  **verified (step 1) / informative (step 2)** — step-1 machinery exists:
  FEEDER_AUDIT stored round-trip (`app/ehrbase/src/storage/codec.rs:51`),
  FHIR-inbound conversion stamping FEEDER_AUDIT_DETAILS
  (`app/ehrbase/src/service/fhir/mapping.rs:35`), TDD import
  (`app/ehrbase/src/service/tdd.rs`); step-2 mapping is tool/authoring
  territory the spec assigns to archetype authors, not the CDR.

## 15 Relationship to Standards (`master15-standards.adoc`)

- [x] Standards for evaluation (ISO 20514, ISO 18308) and design influences
  (OMG HDTF, ISO 13606, CEN HISA/13940)
  (`master15-standards.adoc` §§Standards by which openEHR can be
  evaluated / …influenced the design). **informative**.
- [x] Standards used *inside* openEHR: ISO 8601 (Quantity package date/times),
  UCUM (Quantity units), HL7v3 GTS (time specification), RFC 4880 openPGP
  (`master15-standards.adoc` §Standards which are used "inside" openEHR).
  **verified** — ISO 8601 types
  (`crates/openehr-base/src/foundation_types/time/`); UCUM units with a
  syntax validator (`crates/openehr-rm/src/data_types/quantity/dv_quantity.rs:9`;
  `crates/openehr-term/src/measurement.rs` — commit-time unit checking is
  template-list-driven by recorded PORT NOTE); GTS time-specification types
  generated (`crates/openehr-rm/src/data_types/time_specification/`);
  RFC 4880 signing (`app/ehrbase/src/signing/`).
- [x] Conversion-gateway standards (13606, CDA, HL7v2/v3) and generic
  technology standards (RM/ODP, UML, XSD, XPath)
  (`master15-standards.adoc` §§Standards which require a conversion
  gateway / Generic Technology Standards). **informative** — gateway
  conversions are integration-layer concerns (ch 14); FHIR in/out exists as
  our extension (`app/ehrbase/src/service/fhir/`,
  `app/ehrbase/src/fhir_outbound/`) — no openEHR spec governs it.

## 16 Implementation Technology Specifications (`master16-implementation.adoc`)

### 16.1 Overview

- [x] ITSs apply transformation rules from the abstract models to a
  technology: class/attribute name mapping, signature mapping, basic-type
  mapping, multiple inheritance, generics, covariance, attribute-vs-function
  choice, invariant expression, assumed-type mapping; implementers should
  use an existing ITS where one exists
  (`master16-implementation.adoc` §Overview). **verified** — this is
  precisely what the codegen implements deterministically from the published
  machine-readable expressions: BMM→Rust emission with documented mapping
  decisions (flattened inheritance, bound-filled generics, boxed recursion,
  primitive mappings) in `crates/openehr-codegen/src/emit.rs`, plus the
  ITS-XML/ITS-REST/ITS-JSON artefacts consumed as published
  (`crates/openehr-its/`); invariants hand-written in `*_impl.rs` siblings
  (`crates/openehr-rm/src/**/**_impl.rs`).

## Amendment Record (`master00-amendment_record.adoc`)

- [x] Document change history (BASE 1.3.0 latest, 2025-01-10)
  (`master00-amendment_record.adoc`). **informative** — matches the vendored
  BASE pin (`docs/VERSIONS.md`).

---

## Gap → worklist mapping

| Checklist row | Gap | Worklist |
|---|---|---|
| 6.3 | `EHR.folders` multiple hierarchies not managed (single root FOLDER enforced) | W-6 |
| 10.2.2 | AQL archetype matching is exact — parent-archetype queries miss specialised-child data | W-7 |
| 11.2.1 / 11.2.4.3 / 11.3.1 | `//` path patterns, positional predicates, `ehr:` URI grammar+resolution | W-8 |
| 5.5.1.5 / 7.3.2.2 / 7.4.1 | No EHR_ACCESS-based access evaluation, access list/gate-keeper, or Composition privacy levels | W-9 |
| 4.1.1 / 10.2.1 (pre-existing) | ADL2 pipeline (parser, AOM2 semantic validation, flattening, OPT2) | W-4 |

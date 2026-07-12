# SM Platform Overview (master02) — conformance audit

Read-only audit of the SM Platform Service Model **overview** chapter against
the implementation. Unlike the per-service chapters (which specify concrete
calls), this chapter is **architectural and normative-about-structure**: it
fixes the platform's service catalogue, the native-API/protocol-adapter split,
the abstract-call semantics (command/query, pre/post/exception form), the
call-status error protocol, the list-cursor convention, the global parameter
naming, the package structure, and where authn/authz sits. The audit question
is therefore: *does our crate/component decomposition and service catalogue
faithfully realize the platform architecture this chapter defines?*

**Verdict: largely compliant.** All ten platform services are present as
native-API traits; the native-API/adapter architecture, the CALL_STATUS error
model, the list cursor, the global naming, and the out-of-band-auth placement
are all faithful and cited in the code. The residue is minor and, where it
diverges structurally, is expressly sanctioned by this chapter's own
*formal-equivalence* clause. There is one small documentation drift (a
component-map trait name that does not exist in code).

**Spec oracle** (read before any change):

- `docs/specs/openehr/SM/docs/openehr_platform/master02-overview.adoc`
  (this chapter: §General Assumptions, §openEHR Platform Model, §Interface
  Calls, §Anatomy of an Abstract Call Specification, §Global Conventions
  [Functional Style / List Handling / Global Naming Conventions], §Package
  Structure)
- No `include::` of `UML/classes/*.adoc` — the chapter references only SVG
  package/architecture diagrams (`assumed_model.svg`,
  `SM-platform.definition.svg`, `SM-platform-packages.svg`). The concrete call
  detail lives in the per-service chapters (master03–master12), audited in the
  sibling documents in this directory.
- Cross-referenced supporting text: `master03-common_package.adoc` §Representing
  Call Status (the `CALL_STATUS` / `I_STATUS` model this chapter's §Functional
  Style invokes).

**Implementation surface** (verified 2026-07-12):

- Native-API traits: `app/ehrbase-sm/src/services/` (one module per SM
  interface; catalogue in `services/mod.rs:15-72`).
- `Platform` supertrait (the "platform is the set of all its services" union):
  `app/ehrbase-sm/src/platform.rs:28-60`.
- Call-status error model: `app/ehrbase-sm/src/error.rs`.
- Shared service types incl. the list cursor: `app/ehrbase-sm/src/types.rs`.
- REST protocol adapter over the native API: `app/ehrbase-rest/src/`
  (`lib.rs`, `dispatch/`, `router.rs`).
- DB-backed `Platform` impl: `app/ehrbase/src/service/`.

---

## 1. Requirement register — the overview's normative surface vs the code

Each row cites the governing master02 section and the file:line that realizes
it. R-rows that are gaps/divergences are flagged **GAP** / **DIVERGENCE**;
the rest are confirmed compliant.

| # | Requirement (master02 §) | Spec text | Realized by | State |
|---|--------------------------|-----------|-------------|-------|
| R-1 | **Ten platform services, standardised naming** (§openEHR Platform Model, service table) | "standardised _naming_ of components … so that platform procurers and users can unambiguously refer to the 'Admin service' or the 'EHR service'". Services: Definitions, EHR, Demographic, EHR Index, Query, Terminology, Message, System Log, Subject Proxy, Admin. | All ten present as traits: Definitions → `DefinitionAdl14Service`/`DefinitionAdl2Service`/`DefinitionQueryService` (`services/mod.rs:46`); EHR → `EhrService`+`EhrStatusService`+`EhrCompositionService`+`EhrDirectoryService`+`EhrContributionService` (`mod.rs:44-49`); Demographic → `DemographicService` (`mod.rs:47`); EHR Index → `EhrIndexService` (`mod.rs:54`); Query → `QueryService` (`mod.rs:57`); Terminology → `TerminologyService` (`mod.rs:67-70`); Message → `EhrExtractService`+`TddService` (`mod.rs:56,66`); System Log → `SystemLog` (`mod.rs:63`); Subject Proxy → `SubjectProxyService` (`mod.rs:59`); Admin → `AdminService` (`mod.rs:40`). | **Compliant** |
| R-2 | **Native API reached through protocol adapters** (§General Assumptions) | "native APIs are network-accessible via one or more communications protocols, each with an appropriate _protocol adapter_ … The focus of this specification is the nominal 'native API'." | The native API is the protocol-free `ehrbase-sm` crate (`lib.rs:1-14` cites this exact section); `ehrbase-rest` is the REST protocol adapter, generic over `S: Platform` (`ehrbase-rest/src/lib.rs:1-10`, `build_with:74`). The service crate carries no `openehr-its` (wire) types (`platform.rs:11-15`). | **Compliant** — architecture matches the assumed model 1:1 |
| R-3 | **Component = one or more logical interfaces, each a set of typed calls** (§openEHR Platform Model / §Interface Calls) | "Each component has one or more associated interfaces … Each interface consists of a number of _calls_ … a callable routine with a formal, typed signature." | One Rust trait per SM interface, each method the SM call with its spec name/params/types transcribed (`services/ehr.rs:22-133` transcribes `I_EHR_SERVICE`; module docs cite the `i_*.adoc` source). | **Compliant** |
| R-4 | **Formal equivalence, not structural mirroring** (§Interface Calls, §Functional Style) | "even if three calls in an implementation are required to achieve the effect of a single call … the conditions described here prior and after the call(s) are the same … the functional style used in this specification does not need to be exactly replicated … only the resulting semantics." | Our decomposition (per-interface traits, `IEhr` handle for the `I_EHR` accessor `ehr.rs:87-99`, adapter-support extension methods clearly PORT-NOTEd `ehr.rs:101-132`) relies on this clause and stays within it. | **Compliant** (this clause licenses R-8/R-9 below) |
| R-5 | **Abstract call = name + args + pre/post/exception** (§Anatomy of an Abstract Call Specification) | The `create_ehr_with_id` worked example: pre `Valid_id: not has_ehr(an_id)`, post `Ehr_created: has_ehr(an_id)`, exceptions `Ehr_already_exists`, `Auth_error`. | `EhrService::create_ehr_with_id` (`ehr.rs:46-50`) carries pre `Id_available: not has_ehr(an_ehr_id)`, post `has_ehr(Result)`, error `ehr_create_fail_duplicate_id` in its doc-comment — the anatomy example realized verbatim. Pre/post/exception documented per call across the traits. | **Compliant** |
| R-6 | **Nearly-stateless functional style; I_STATUS mapped to typed status** (§Functional Style) | "a _nearly_ stateless approach … return types reflecting successful execution … Failures … by calling `last_call_failed()` and … `last_call_status()` which returns a structured error object … Either style can be used, and can be trivially mapped from one to the other." | Every trait method returns `Result<T, SmError>`; `SmError` = a `CallStatusType` + message (`error.rs:170-177`), with `CallStatus` modelling `CALL_STATUS` (`error.rs:216-233`). The doc explicitly invokes this section as sanction for the stateless mapping (`error.rs:9-13,163-169`). | **Compliant** |
| R-7 | **List-cursor convention** (§List Handling) | Optional `item_offset` (0-based; "Zero signifies … from the first item") and `items_to_fetch` ("A zero means 'all'"). | `types.rs:28-59` `Page { item_offset, items_to_fetch }` with `all()`/`offset()`/`limit()` normalising `Some(0)`→all, citing this section. Threaded through unbounded-list calls (`services/contribution.rs:60`, `services/definition.rs`). | **Compliant** |
| R-8 | **Global naming conventions** (§Global Naming Conventions) | `ehr_id`, `versioned_object_uid`, `version_uid`, `preceding_version_uid`, `object_id`, `time`. | Trait parameters use the SM names (`ehr.rs`: `ehr_id`, `an_ehr_id`, `a_subject_id`; `types.rs` `ObjectVersionId` for version uids); per-service audits track the exact per-call mapping. | **Compliant** (surface; per-call detail is the per-service chapters' scope) |
| R-9 | **Command/query separation** (§Interface Calls) | "Good practice usually dictates … pure _functions_ or pure _procedures_ … side-effect producing functions should generally be avoided." | Not strictly separated: `create_ehr` both mutates and returns the new `UUID` (`ehr.rs:39`). **This mirrors the spec itself** — the chapter's own `I_EHR_SERVICE` IDL (`master02` §Functional Style lines 102-107) declares `UUID create_ehr()` as a state-changing function, and the text frames CQS as "good practice usually dictates," not a MUST. | **Compliant** (follows the spec's own worked example) |
| R-10 | **Package structure: `common` / `definition` / `interface`** (§Package Structure) | "It consists of the packages `common`, `definition` and `interface`. The second contains the service components, while the third contains the interfaces attached to each service component." | Our layering is `common` → `error.rs` + `types.rs`; `interface` → `services/*` traits. The `definition` layer (the *service component* as an object distinct from its interfaces) is **collapsed**: the component IS the trait(s); there is no separate component object. | **DIVERGENCE (sanctioned)** — see G-1 |
| R-11 | **Authn/authz handled out of band, before any call** (§Functional Style) | "Authentication and authorisation is assumed to have been dealt with **before any particular call** … by a combination of standard authentication technologies (e.g. OAuth, RFC 7235) **and role-based access control**." | Auth is one middleware over the API router in the adapter, ahead of dispatch (`ehrbase-rest/src/lib.rs:12-15`; `access::authn`/`access::authz` re-exported `lib.rs:33-34`); the native API carries only an `AuthFailure` status (`error.rs:48`). Placement is exactly "before the call," in the protocol adapter, off the native API — as the chapter prescribes. | **Compliant** — see G-2 for the RBAC-depth nuance |
| R-12 | **System Log is a first-class platform service, IHE ATNA-compliant** (§openEHR Platform Model, service table) | "System Log | IHE ATNA-compliant system log." | `SystemLog` trait in the catalogue (`services/mod.rs:63`, `services/system_log.rs`); it is a `Platform` supertrait member (`platform.rs:54`); ATNA emission wired through it (`ehrbase-rest/src/lib.rs:86-89`). | **Compliant** |
| R-13 | **The platform = the set of all its services** (§openEHR Platform Model) | Components + their interfaces constitute the platform; conformance is per-component/per-call. | `Platform` supertrait unions every SM service trait + the adapter-support extensions; implemented once on the concrete DB service, a missing impl is a compile error not a runtime 501 (`platform.rs:1-10,28-94`). | **Compliant** |

---

## 2. Gap / divergence register

Only two items are worth recording, plus one documentation-drift finding.
Neither G-row is a conformance defect — both are structural choices this
chapter's own text permits.

| # | Item | Spec citation | Today | Assessment |
|---|------|---------------|-------|------------|
| G-1 | **The `definition` (service-component) package layer is collapsed into the interface traits.** master02 §Package Structure defines three packages, with `definition` holding *service components* distinct from the `interface` package's interfaces. We have no component object separate from its interface trait(s) — e.g. the "EHR service" component is realized directly as five `I_EHR_*` traits + the `IEhr` handle, not as an `EHR` component object owning those interfaces. | `master02-overview.adoc` §Package Structure; §openEHR Platform Model ("This view does not attempt to define a real product architecture … but … a _formal equivalent_"; §Functional Style: the functional style "does not need to be exactly replicated … only the resulting semantics do") | `services/*` = interfaces; `common` = `error.rs`+`types.rs`; the component tier is absent. | **Sanctioned divergence.** The chapter explicitly declines to mandate a product architecture and requires only formal equivalence of *calls and their pre/post conditions*, which R-3/R-5 satisfy. No component-level call exists that we fail to place. Record as a documented structural choice, not a gap to close. *No openEHR spec text mandates a distinct component object — the packaging is illustrative (§Package Structure diagrams).* |
| G-2 | **Fine-grained RBAC/ABAC is a Stage-2 track, not part of the native-API surface.** master02 §Functional Style names "role-based access control" among the technologies assumed to handle authorisation *before* a call. Coarse RBAC exists in the adapter (`access::authz`, `ehrbase-rest/src/lib.rs:12-14,34`); fine-grained ABAC is deferred (`docs/enterprise/access-control.md`). | `master02-overview.adoc` §Functional Style ("… and role-based access control") | Authn + coarse role gate in the adapter middleware; ABAC deferred. | **Compliant placement; depth is out of this chapter's scope.** The chapter places *all* authz out of band, before the call — it does not specify authorisation granularity or make RBAC a native-API call. Our placement (adapter middleware, ahead of dispatch) matches; depth is an enterprise track, not a master02 conformance item. No native-API call is missing. |
| D-1 | **Documentation drift: `docs/architecture.md` SM component map names a `MessageService` trait that does not exist.** The Message-component row in the architecture component map lists native trait "MessageService"; the code exports **`EhrExtractService`** (+ `TddService`), and there is no `MessageService` trait (`grep` in `ehrbase-sm/src` returns only `trait EhrExtractService` `services/message.rs:52` and `trait TddService` `services/tdd.rs:45`). | master02 §openEHR Platform Model names only the *component* "Message"; the interfaces (`I_MESSAGE_SERVICE`/`I_EHR_EXTRACT_SERVICE`/`I_TDD_SERVICE`) are master09's scope. | Component realized by `EhrExtractService`+`TddService`; `docs/architecture.md` map cell says `MessageService`. | **Doc-only drift** (not a code defect; the Message component IS realized). The `docs/architecture.md` SM component-map row should be corrected to name the actual traits. Non-blocking; flagged for the next architecture-doc touch. |

---

## 3. PORT-NOTE residue (the honest, permanent notes)

These are correct-as-designed and should stay noted, not "fixed":

- **`I_STATUS` stateful protocol → stateless typed `Result`.** master02
  §Functional Style sanctions the mapping explicitly ("Either style can be
  used, and can be trivially mapped from one to the other"); documented at
  `error.rs:9-20,163-169`. Permanent.
- **The service-component package tier is not modelled** (G-1) — the interface
  trait is the component; formal-equivalence clause covers it. Permanent
  structural choice; keep a one-line note where the module layout is described.
- **Adapter-support methods on the native traits** (e.g.
  `EhrService::ehr_object`/`ehr_created_object`/`ehr_object_for_subject`,
  `ehr.rs:101-132`) are ITS-REST wire-assembly seams, not SM calls; each is
  already PORT-NOTEd as an adapter extension. Correct — the wire body shapes
  are master05/ITS-REST concerns pushed to the seam so the adapter builds them
  from one place. Permanent.
- **Command/query separation is not enforced** (R-9) — the spec's own examples
  return values from state-changing calls; we follow the spec, not the "good
  practice" aside. No note needed beyond this record.

---

## 4. Conclusion

master02 defines *structure and conventions*, and our decomposition realizes
them faithfully: ten named services as protocol-free native-API traits
(`ehrbase-sm`), a REST protocol adapter generic over `Platform`
(`ehrbase-rest`), the `CALL_STATUS`/`I_STATUS` error model mapped to typed
`Result`s exactly as §Functional Style sanctions, the `item_offset`/
`items_to_fetch` list cursor, the global parameter naming, the anatomy-example
pre/post/exception form on the calls, and out-of-band auth in the adapter. The
only structural divergence (collapsing the `definition` component tier into the
interface traits, G-1) is expressly permitted by the chapter's formal-
equivalence clause; the RBAC-depth point (G-2) is an enterprise track the
chapter never scopes into the native API; and D-1 is a one-cell documentation
correction in `docs/architecture.md`. No conformance work is required against
this chapter. Per-call fidelity is audited in the per-service documents in
this directory (the concrete calls live in master03–master12, not master02).

# Compliance overview

FerroEHR is software. It is not a controller, not a processor and not a
certified organisation, so nothing on this page says that a deployment
complies with anything. What this page does say is which technical controls
the product ships today, which are planned and tracked in public, and which
obligations stay with the organisation that runs it.

The page is written for three readers. A privacy officer wants the legal
sources and the split of duties. A hospital CISO wants the controls and their
status. A developer wants to know where the boundary between clinical and
identifying data runs. Each of those three readings takes about ten minutes.

<!-- toc -->

## What FerroEHR claims, and what it does not

FerroEHR aims to be the first openly developed, source-available openEHR CDR
with a published, tracker-backed EU compliance posture, and an EHDS conformity
self-assessment is on its roadmap.

It holds no certification, no declaration of conformity and no third-party
assessment. No page on this site will tell you that FerroEHR is "GDPR
compliant", "NEN 7510 certified" or "EHDS conformant", because none of those
statements would be true of a piece of software on its own.

What the project does instead is publish the record. A shipped control has a
feature page in this book and an issue in the tracker that delivered it. A
planned control has an open issue and appears here as planned, with its
number. The [control matrix](control-matrix.md) is generated from the tracker,
so a control cannot sit on this site as "planned" after it has shipped, or as
"shipped" before it has.

> [!WARNING]
> The legal texts linked here change, and some of them are not yet in force.
> The EHDS obligations phase in over several years on the dates its own final
> provisions carry, and NEN republishes its standards on its own cycle
> (NEN 7510 was reissued in December 2024). Check each publisher directly
> before you rely on a statement here: [EUR-Lex](https://eur-lex.europa.eu/)
> for EU law, [wetten.overheid.nl](https://wetten.overheid.nl/) for Dutch law,
> the [EDPB](https://www.edpb.europa.eu/) for guidelines, and
> [NEN](https://www.nen.nl/zorg-welzijn/ict-in-de-zorg/informatiebeveiliging-in-de-zorg)
> for the 7510 family. This page is a summary for evaluators and deployers,
> not legal advice.

## The pseudonymisation boundary

A clinical record is identifying as soon as the record and the person can be
put back together by whoever holds the database. Separating the two, and
controlling who may rejoin them, is what
[GDPR Art. 4(5)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) calls
pseudonymisation, and it is the control the
[EDPB Guidelines 01/2025](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en)
expect a supplier to describe rather than assert.

openEHR anticipated this. The Reference Model's
[PARTY_SELF and Referring to the Patient from the EHR](https://specifications.openehr.org/releases/RM/Release-1.1.0/common.html#_party_self_and_referring_to_the_patient_from_the_ehr)
section names three schemes for pointing at the record subject, and calls the
one that never sets `external_ref` anywhere in the EHR "the most secure
approach", because the link between record and patient is then held outside
the EHR. The second scheme sets it once, in
[`EHR_STATUS.subject`](https://specifications.openehr.org/releases/RM/Release-1.1.0/ehr.html#_ehr_status_class),
whose own class description says the association "may be done elsewhere for
security reasons". The reference itself is a
[`PARTY_REF`](https://specifications.openehr.org/releases/BASE/Release-1.2.0/base_types.html#_party_ref_class),
described by the specification as an "identifier for parties in a demographic
or identity service", so it carries a namespace, a party type and an id and no
demographic content of its own. The specification leaves the choice of scheme
to the implementation. Where the schemas, the database roles and the resolve
path live is FerroEHR's own design, and no openEHR spec governs it.

### What ships today

Clinical content and demographic parties live in separate PostgreSQL schemas,
`ehr` and `demographic`, each with its own archival tier and its own runtime
database roles. `ferroehr_ehr` and `ferroehr_demographic`, and a read-only twin
of each, are `NOINHERIT`, hold explicit grants on one domain only, and carry an
explicit revoke on the other. The server refuses to start if that does not
hold: a self-check enumerates every table, view, sequence and function in each
domain and names the role and the object it can reach. The database refuses the
mix as well, in both directions, so a code path that missed the split fails as
a write error rather than leaking quietly.

The separation of schemas is unconditional. Pointing the demographic pool at
its own DSN (`[db] demographic_url`) makes it a separation of credentials too,
which is what stops one leaked connection string from reaching both.

The controls that apply across both domains are the same: role- and
attribute-based authorization, per-EHR access settings, tenant row-level
security, and the audit trail. Both sides carry openEHR's own provenance,
because every write commits a contribution and its audit in the same
transaction, which is the versioning discipline the
[Change Control Package](https://specifications.openehr.org/releases/RM/Release-1.1.0/common.html#_change_control_package)
defines.

What is **not** yet separated is the resolve map that rejoins the two. Today the
subject reference on the clinical side is whatever a client supplied; there is
no third domain holding the mapping under its own role and its own audit, and
no constraint keeping a national identifier off the clinical side.

```mermaid
flowchart LR
    client["API client"] --> server["FerroEHR server"]
    server -->|ferroehr_ehr| ehr[("ehr schema:<br/>clinical versions and nodes")]
    server -->|ferroehr_demographic| demo[("demographic schema:<br/>parties and identifiers")]
    server -->|audit writer| audit[("audit schema:<br/>ATNA record repository")]
```

### What is planned

The schema and role split is done. What remains is the rest of the domain: a
resolve map with its own role and its own audit, a clinical side that refuses
identifying data outright, encrypted national identifiers, per-domain access
logging, and per-domain keys and backups. The whole programme is
[#3152](https://github.com/rubentalstra/FerroEHR/issues/3152).

```mermaid
flowchart LR
    client2["API client"] --> server2["FerroEHR server"]
    server2 -->|clinical role| ehr2[("ehr schema:<br/>clinical versions and nodes")]
    server2 -->|demographic role| demo[("demographic schema:<br/>parties and identifiers")]
    server2 -->|linkage role| link[("linkage schema:<br/>party to EHR resolve map")]
```

Each row below is an open issue.

| Planned control | Issue |
|---|---|
| The clinical side refuses identifying data, and the subject reference is constrained to a pseudonym namespace | [#3154](https://github.com/rubentalstra/FerroEHR/issues/3154) |
| National identifiers stored encrypted, looked up by key, resolved under audit | [#3155](https://github.com/rubentalstra/FerroEHR/issues/3155) |
| Per-domain access logging for reads and queries | [#3156](https://github.com/rubentalstra/FerroEHR/issues/3156) |
| Separate encryption keys and per-schema backup handling | [#3157](https://github.com/rubentalstra/FerroEHR/issues/3157) |
| The linkage service: the party-to-EHR resolve map as its own schema and role | [#3158](https://github.com/rubentalstra/FerroEHR/issues/3158) |
| Cross-domain cohort queries with a demographic predicate and a clinical selection | [#3159](https://github.com/rubentalstra/FerroEHR/issues/3159) |
| A secondary-use read model as a separate pseudonymisation domain | [#3160](https://github.com/rubentalstra/FerroEHR/issues/3160) |

Until those land, a FerroEHR database still holds the information that rejoins
a record to a person: the two domains are separated, and nothing yet stops a
client putting a directly identifying value on the clinical side, nor holds the
resolve map apart from either. Size your access control, backup handling and
risk assessment on that, not on the schema split alone. The
[threat model](../threat-model.md) states the residual risk at each boundary
the product does defend.

## GDPR

[Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj)
places its duties on the controller and the processor. A CDR can only supply
the technical measures those duties are met with. These are the articles a
repository actually touches.

| What the article asks for | What FerroEHR ships | Tracker | What the deploying organisation must do |
|---|---|---|---|
| **Art. 4(5)** pseudonymisation: identifying data kept separately, under technical measures | Separate schemas with non-overlapping `NOINHERIT` roles, enforced by grants, by a boot-time self-check and by a database constraint in both directions; optionally separate credentials | partly shipped, [#3153](https://github.com/rubentalstra/FerroEHR/issues/3153); the resolve map and the identifier constraints are [#3158](https://github.com/rubentalstra/FerroEHR/issues/3158) and [#3154](https://github.com/rubentalstra/FerroEHR/issues/3154) | Point the demographic pool at its own DSN, and keep the additional information out of the clinical side until those land |
| **Art. 5(1)(f)** integrity and confidentiality | TLS 1.3 with optional mutual authentication, [authentication and authorization](../security.md), per-version digest [signing](../signing/index.md), a tamper-evident audit chain | shipped | Terminate TLS correctly, run the identity provider, hold the keys |
| **Art. 5(2)** accountability: being able to demonstrate compliance | An audit trail of every access, [retrievable over ITI-81](../audit.md#retrieving-audit-records-iti-81), plus openEHR's own contribution and audit chain on every write | shipped | Keep the records, define retention, be able to produce them |
| **Art. 9** special categories of data | Object-level [`EHR_ACCESS`](../security.md#per-ehr-access-control-ehr_access) settings, RBAC, ABAC, tenant row-level security | shipped | Establish the Art. 9(2) condition and the national derogation that permits the processing |
| **Art. 25** data protection by design and by default | Deny-by-default authorization, an `EHR_ACCESS` default that can be set to restricted, tenancy that fails closed, audit on by default | shipped | Choose the restrictive settings. Two defaults favour compatibility instead: the per-EHR access default is `open`, and the audit fail mode is `open` |
| **Art. 30** records of processing activities | The effective configuration as a redacted JSON tree at `GET {base}/admin/config`, and this book as a description of what the software does | shipped | Write and maintain the record itself; the software cannot know your purposes or recipients |
| **Art. 32** security of processing | The controls listed in [Security & multi-tenancy](../security.md) and the residual risk in the [threat model](../threat-model.md) | shipped | Assess whether they are appropriate to your risk, and supply everything below the application |
| **Art. 35** data protection impact assessment | Published control and boundary documentation to assess against | guidance planned, [#3161](https://github.com/rubentalstra/FerroEHR/issues/3161) | Run the DPIA; it is the controller's, and no supplier document replaces it |

## EDPB Guidelines 01/2025 on pseudonymisation

The [EDPB guidelines](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en)
ask for something more specific than "we pseudonymise": a named
pseudonymisation domain, a stated attacker, and additional information kept
where that attacker cannot reach it.

| What the guidelines ask for | What FerroEHR ships | Tracker | What the deploying organisation must do |
|---|---|---|---|
| A pseudonymisation domain stated explicitly | The clinical and demographic domains are separate schemas with their own roles, and the server refuses to boot if a role reaches across | shipped, [#3153](https://github.com/rubentalstra/FerroEHR/issues/3153) | State the domain for your deployment, including the parts outside FerroEHR |
| The additional information held separately from the pseudonymised data | Nothing yet; the resolve map is planned as its own schema and role | planned, [#3158](https://github.com/rubentalstra/FerroEHR/issues/3158) | Hold your own identity mapping outside the CDR if you need the separation today |
| A written attacker model, including the insider holding a credential | The [threat model](../threat-model.md) names actors, boundaries and the risk surviving each control | shipped | Extend it with the actors your environment adds: operators, backups, the network |
| Resolution of a pseudonym recorded and controlled | Every access is audited today; the audited resolve path is planned with the linkage service | partly shipped, [#3155](https://github.com/rubentalstra/FerroEHR/issues/3155) | Restrict who may resolve, and review the trail |

## EHDS

[Regulation (EU) 2025/327](https://eur-lex.europa.eu/eli/reg/2025/327/oj)
splits into chapters that reach a CDR differently. Chapter III is the one that
speaks to an EHR system as a product, and its obligations apply from a date in
the regulation's own final provisions rather than today.

| Chapter | What FerroEHR ships | Tracker | What the deploying organisation must do |
|---|---|---|---|
| **Chapter II**, primary use, including the patient's access to their data and to a record of who accessed it | An [access trail](../audit.md) of every read, write and refusal, searchable by patient and by agent | shipped | Build the patient-facing access route; the CDR exposes the trail to an admin caller, not to the patient |
| **Chapter III**, EHR systems: a European interoperability software component and a European logging software component, with published technical documentation | A [conformance record](../conformance.md) and an [audit model](../audit.md) to map onto those components | readiness planned, [#3168](https://github.com/rubentalstra/FerroEHR/issues/3168), [#3169](https://github.com/rubentalstra/FerroEHR/issues/3169), [#3170](https://github.com/rubentalstra/FerroEHR/issues/3170), [#3171](https://github.com/rubentalstra/FerroEHR/issues/3171) | Decide whether you are the manufacturer of the EHR system you put into service |
| **Chapter IV**, secondary use | [AQL](../querying-aql.md) over the stored record, and a [change-event outbox](../beyond-core/amqp.md) | a separate pseudonymisation domain for secondary use is planned, [#3160](https://github.com/rubentalstra/FerroEHR/issues/3160) | Deal with the health data access body; a CDR is not a data-holder process |

## Dutch law: UAVG and Wabvpz

Two Dutch acts sit on top of the GDPR for a care provider. The
[UAVG](https://wetten.overheid.nl/BWBR0040940) is the national implementation
act; the [Wabvpz](https://wetten.overheid.nl/BWBR0023864) governs the
burgerservicenummer in care and the patient's electronic access to their
record.

| Provision | What FerroEHR ships | Tracker | What the deploying organisation must do |
|---|---|---|---|
| **UAVG Art. 30**, exceptions for health data | Access control at the record and the attribute level, and an audit trail of who used it | shipped | Establish that your processing falls inside the exception, per role and per purpose |
| **UAVG Art. 46**, processing a national identification number | A party identifier is stored in the demographic domain, reachable only by that domain's role; it is not yet encrypted, key-looked-up or resolved under its own audit | partly shipped, [#3155](https://github.com/rubentalstra/FerroEHR/issues/3155) | Hold the legal authorisation before a BSN enters the store, and keep it out until #3155 lands |
| **Wabvpz Art. 4 to 9**, use and verification of the BSN by care providers | Nothing specific: FerroEHR performs no BSN verification and consults no index | not planned | Verify identity and the BSN in your own systems before data reaches the CDR |
| **Wabvpz Art. 15d**, electronic access and copy for the patient | The full record over the openEHR REST API, and [EHR Extract export](../beyond-core/messaging.md) for a whole record | shipped | Build the patient-facing route and authenticate the patient |
| **Wabvpz Art. 15e**, a record of who made data available and who consulted it | The ATNA trail records reads, writes and refusals with the agent, the patient, the action and the outcome, and answers a per-patient search | shipped | Turn the trail into something a patient can read, and set retention |

## NEN 7510, NEN 7512 and NEN 7513

The [NEN 7510 family](https://www.nen.nl/zorg-welzijn/ict-in-de-zorg/informatiebeveiliging-in-de-zorg)
governs information security in Dutch healthcare. NEN 7510 is a
management-system standard, which no product can be certified against.
NEN 7512 governs what exchanging parties promise each other. NEN 7513 is the
one that states requirements a piece of software meets directly.

| Standard | What FerroEHR ships | Tracker | What the deploying organisation must do |
|---|---|---|---|
| **[NEN 7510-1](https://www.nen.nl/nen-7510-1-2024-nl-331311)** and **[7510-2](https://www.nen.nl/nen-7510-2-2024-nl-331314)**, the management system and its controls | Technical controls an ISMS can point at, documented per control with their residual risk | shipped | Run the ISMS and hold the [certification](https://www.nen.nl/certificatie-en-keurmerken-nen-7510); a product cannot be certified against a management-system standard |
| **[NEN 7512](https://www.nen.nl/nen-7512-2022-nl-297137)**, the trust basis for data exchange | Mutually authenticated TLS ([IHE ITI-19](../audit.md#node-authentication-iti-19-mutual-tls)), OAuth2 and OIDC with an [enterprise identity provider](../identity-providers.md), signed and verifiable [releases](../verifying-releases.md) | shipped | Agree the trust basis with each counterparty and operate the certificate estate |
| **[NEN 7513](https://www.nen.nl/nen-7513-2018-nl-245399)**, logging actions on electronic patient records | An IHE ATNA trail that records every operation including refusals, in FHIR `AuditEvent` and DICOM PS3.15 form, hash-chained in the database and retrievable per patient | shipped | Map the recorded fields onto the standard's own list, set retention, and review the trail |

## Where to go next

- **[Control matrix](control-matrix.md):** the machine-generated status of every
  declared control, straight from the tracker.
- **[Shared responsibility](shared-responsibility.md):** which obligation is
  the software's and which is yours, obligation by obligation.
- **[Security & multi-tenancy](../security.md):** how each control is
  configured.
- **[Threat model](../threat-model.md):** what survives each control.
- **[Audit trail](../audit.md):** what is recorded, in which formats, and how
  to read it back.
- **DPIA guidance, records of processing and a go-live checklist:** planned,
  [#3161](https://github.com/rubentalstra/FerroEHR/issues/3161).

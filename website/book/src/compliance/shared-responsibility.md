# Shared responsibility

Almost every obligation in EU and Dutch health-data law rests on the
controller, and where FerroEHR is operated on that controller's behalf, on the
processor. A repository supplies technical measures. It cannot hold a legal
basis, sign a processing agreement, notify a supervisory authority or run a
management system.

This page draws that line obligation by obligation, so you can see at a glance
which part of the work the software has already done and which part is still
yours. No openEHR specification governs any of it; the division below follows
the legal texts each row links.

<!-- toc -->

## How to read the tables

Each row names one obligation and links its official source. The middle column
is what FerroEHR provides, linked to the page that documents it, or to the open
issue when the control is planned rather than shipped. The right column is the
work that stays with the deploying organisation.

"Nothing" in the middle column is a real answer and appears wherever it is
the true one.

> [!NOTE]
> The FerroEHR project is not your processor. It publishes software; it
> operates nothing on your behalf and holds none of your data. Where a row says
> "the processor", it means whoever runs the deployment, which may be you.

## GDPR

Every row below cites
[Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj). The
duties in it belong to the controller and the processor. The middle column is
only the technical measure FerroEHR supplies toward one of them.

| Obligation | What FerroEHR provides | What the deploying organisation does |
|---|---|---|
| [Art. 5(1)(e)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) storage limitation | Audit-trail retention as a configured period, and irreversible [physical deletion](../operations-admin-apis.md#physical-deletion) of an EHR through the admin API | Set the retention schedule and execute it; openEHR versions are append-only until you delete the record |
| [Art. 5(2)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) accountability | An [audit trail](../audit.md) of every access, openEHR's own contribution and audit chain on every write, and a published [conformance record](../conformance.md) | Retain the evidence and be able to produce it on demand |
| [Art. 6 and Art. 9(2)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) legal basis and the condition for health data | Nothing. Software cannot hold a legal basis | Establish the basis and the Art. 9(2) condition, per purpose, before data is entered |
| [Art. 24 and Art. 25](https://eur-lex.europa.eu/eli/reg/2016/679/oj) responsibility, and protection by design and by default | Deny-by-default [authorization](../security.md#authorization), per-EHR access settings, tenancy that fails closed, audit on by default | Choose the restrictive settings, and document why the chosen configuration is appropriate |
| [Art. 28](https://eur-lex.europa.eu/eli/reg/2016/679/oj) processor terms and sufficient guarantees | Published control documentation, a [threat model](../threat-model.md) with named residual risk, and verifiable [release artifacts](../verifying-releases.md) | Conclude the processing agreement with whoever operates the deployment, and audit them |
| [Art. 30](https://eur-lex.europa.eu/eli/reg/2016/679/oj) records of processing activities | The effective configuration as a redacted tree at `GET {base}/admin/config`, and this book as a description of what the software does | Write and maintain the record; only you know the purposes, the recipients and the transfers |
| [Art. 32](https://eur-lex.europa.eu/eli/reg/2016/679/oj) security of processing | TLS 1.3 with optional [mutual authentication](../audit.md#node-authentication-iti-19-mutual-tls), [authentication and access control](../security.md), per-version [signing](../signing/index.md), a tamper-evident audit chain, [tenant row-level security](../security.md#multi-tenancy) | Supply everything below the application: the database, its backups, the network, the platform. See [Cluster hardening](../installation/kubernetes-hardening.md) |
| [Art. 32(1)(d)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) regularly testing the measures | A [storage-integrity sweep and rebuild](../operations-admin-apis.md#storage-integrity), an [audit-chain verification query](../audit.md#tamper-evidence), and a [conformance suite](../conformance.md) that runs against your own server | Schedule the checks, alert on their output, and test your restore |
| [Art. 33 and 34](https://eur-lex.europa.eu/eli/reg/2016/679/oj) breach notification | The evidence a breach assessment needs: who read what, when, and whether the trail itself is intact | Detect, assess and notify within the deadlines. No software does this for you |
| [Art. 15 and 20](https://eur-lex.europa.eu/eli/reg/2016/679/oj) access and portability | The full record over the openEHR REST API in canonical JSON or XML, and [EHR Extract export](../beyond-core/messaging.md) for a whole record | Authenticate the data subject and build the patient-facing route |
| [Art. 16 and 17](https://eur-lex.europa.eu/eli/reg/2016/679/oj) rectification and erasure | Versioned correction with the prior version retained, and [physical, irreversible deletion](../operations-admin-apis.md#physical-deletion) of an EHR for a legal erasure request | Decide how an erasure request interacts with the medical record-keeping duty, and record the decision |
| [Art. 18 and 21](https://eur-lex.europa.eu/eli/reg/2016/679/oj) restriction and objection | `EHR_STATUS.is_queryable` and `is_modifiable`, both enforced by the server: a restricted record leaves population queries and refuses content writes | Decide when to set them, and record why |
| [Art. 35](https://eur-lex.europa.eu/eli/reg/2016/679/oj) data protection impact assessment | Control and boundary documentation to assess against; DPIA guidance is planned in [#3161](https://github.com/rubentalstra/FerroEHR/issues/3161) | Run the DPIA and keep it current. It is the controller's, and no supplier document replaces it |
| [Art. 4(5)](https://eur-lex.europa.eu/eli/reg/2016/679/oj) pseudonymisation | Nothing yet. Clinical and demographic data share one schema and one runtime role; the boundary is planned in [#3152](https://github.com/rubentalstra/FerroEHR/issues/3152) | Keep identifying data outside the CDR if you need the separation today |

## EHDS

[Regulation (EU) 2025/327](https://eur-lex.europa.eu/eli/reg/2025/327/oj)
is in force, and its operative obligations apply from the dates its own final
provisions carry. No row below claims conformity with any of them.

| Obligation | What FerroEHR provides | What the deploying organisation does |
|---|---|---|
| [Chapter II](https://eur-lex.europa.eu/eli/reg/2025/327/oj), primary use and the patient's sight of who accessed their data | An access trail of every read, write and refusal, [searchable by patient and by agent](../audit.md#retrieving-audit-records-iti-81) | Build the patient-facing access route; the trail is exposed to an admin caller, not to the patient |
| [Chapter III](https://eur-lex.europa.eu/eli/reg/2025/327/oj), EHR systems: the European interoperability and logging software components, and published technical documentation | Readiness work is planned in [#3168](https://github.com/rubentalstra/FerroEHR/issues/3168), [#3169](https://github.com/rubentalstra/FerroEHR/issues/3169), [#3170](https://github.com/rubentalstra/FerroEHR/issues/3170) and [#3171](https://github.com/rubentalstra/FerroEHR/issues/3171) | Decide whether you are the manufacturer of the EHR system you put into service, and carry the manufacturer's duties if so |
| [Chapter IV](https://eur-lex.europa.eu/eli/reg/2025/327/oj), secondary use | [AQL](../querying-aql.md) over the stored record and a [change-event outbox](../beyond-core/amqp.md); a separate pseudonymisation domain for secondary use is planned in [#3160](https://github.com/rubentalstra/FerroEHR/issues/3160) | Deal with the health data access body and carry the data holder's duties |

## Dutch law

| Obligation | What FerroEHR provides | What the deploying organisation does |
|---|---|---|
| [UAVG Art. 30](https://wetten.overheid.nl/BWBR0040940), the exception for health data | Access control at the record and attribute level, with every use audited | Establish that your processing falls inside the exception, per role and per purpose |
| [UAVG Art. 46](https://wetten.overheid.nl/BWBR0040940), processing a national identification number | Nothing specific yet; encrypted storage and audited resolution of national identifiers is planned in [#3155](https://github.com/rubentalstra/FerroEHR/issues/3155) | Hold the statutory authorisation before a BSN enters the store |
| [Wabvpz Art. 4 to 9](https://wetten.overheid.nl/BWBR0023864), use and verification of the BSN | Nothing. FerroEHR performs no BSN verification and consults no index | Verify identity and the BSN in your own systems before data reaches the CDR |
| [Wabvpz Art. 15d](https://wetten.overheid.nl/BWBR0023864), electronic access and copy for the patient | The full record over the REST API, and [EHR Extract export](../beyond-core/messaging.md) | Authenticate the patient and build the route; the CDR has no patient-facing interface |
| [Wabvpz Art. 15e](https://wetten.overheid.nl/BWBR0023864), a record of who made data available and who consulted it | An [ATNA trail](../audit.md) recording the agent, the patient, the action, the outcome and the time, retrievable per patient | Render it for the patient, set retention, and review it |
| [BW Book 7, Art. 454](https://wetten.overheid.nl/BWBR0005290), the medical treatment contract's record-keeping duty | Append-only version history, so a correction never destroys the prior version | Set the retention schedule the article requires, and reconcile it with erasure requests |

## NEN

The [NEN 7510 family](https://www.nen.nl/zorg-welzijn/ict-in-de-zorg/informatiebeveiliging-in-de-zorg)
is where the split is sharpest. A management-system standard cannot be met by
a product at all.

| Obligation | What FerroEHR provides | What the deploying organisation does |
|---|---|---|
| [NEN 7510-1](https://www.nen.nl/nen-7510-1-2024-nl-331311), the information security management system | Technical controls an ISMS can point at, each documented with its residual risk in the [threat model](../threat-model.md) | Run the ISMS: scope, risk assessment, policy, internal audit, management review |
| [NEN 7510-2](https://www.nen.nl/nen-7510-2-2024-nl-331314), the controls | [Access control](../security.md), [audit logging](../audit.md), cryptography in transit and for [version signatures](../signing/index.md), [supply-chain verification](../verifying-releases.md) | Everything organisational: personnel, physical security, supplier management, continuity |
| [Certification](https://www.nen.nl/certificatie-en-keurmerken-nen-7510) against NEN 7510 | Nothing. A product cannot be certified against a management-system standard, and FerroEHR makes no such claim | Obtain and maintain the certificate for your organisation |
| [NEN 7512](https://www.nen.nl/nen-7512-2022-nl-297137), the trust basis for data exchange | [Mutually authenticated TLS](../audit.md#node-authentication-iti-19-mutual-tls), OAuth2 and OIDC with an [enterprise identity provider](../identity-providers.md), [SMART App Launch](../smart-app-launch.md) | Agree the trust basis with each counterparty, and operate the certificate estate |
| [NEN 7513](https://www.nen.nl/nen-7513-2018-nl-245399), logging actions on electronic patient records | An [audit trail](../audit.md) of every operation including refusals, in FHIR `AuditEvent` and DICOM PS3.15 form, hash-chained in the database | Map the recorded fields onto the standard's own list, set retention, and review the trail |

## What this page does not do

It does not tell you whether your deployment satisfies any of these
obligations. That answer depends on your legal basis, your organisation, your
infrastructure and your operating practice, none of which a supplier can see.

The companion guidance is planned rather than written:
[DPIA guidance, records of processing and a go-live checklist](https://github.com/rubentalstra/FerroEHR/issues/3161).
Until it lands, the [compliance overview](index.md) carries the legal sources,
the [control matrix](control-matrix.md) carries the live status of every
declared control, and the [threat model](../threat-model.md) carries the risk
that survives each one.

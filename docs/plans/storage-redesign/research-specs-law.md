# Research report 3: what the openEHR specs and the vendored law require of storage

Prefixes: SPEC = docs/specs/openehr, LAW = docs/law.

## A.0 Where the spec leaves the physical layout free
- RM common master06 §Overview: "Although the figure implies physical containment of Versions by a Versioned object, this is only one possible implementation. Other implementations (e.g. using orthodox relational structures) might use references, separate compressed copies, or any other mechanism."
- master06 §Versioned Objects: "How the representation of this collection is implemented inside the VERSIONED_OBJECT is not defined by this specification, only the form of any given version is. Implementations ... might range from the simple (all versions stored as full copies in a list) to a sophisticated compressed versioning approach".
- RM ehr master04 §Versioning of Compositions: "how it stores successive versions in time is an implementation concern ... what is important is that its functional interface enables any version to be retrieved, whether it be the latest, the first, or any in between."
- BASE arch overview master08 §Managing Changes in Time: "The internal versioning implementation may or may not generate deltas as a way of efficient storage."
- BASE master13 §5-tier: tier 1 = persistence; "an abstract persistence API and optimised persistence models ... are likely to be published by openEHR in the future". None vendored.
- BASE master07 §Integrity: "the simplest possible implementations (1 Version = 1 copy) can provide very good safety due to being write-once systems."
- RM common master03 §The PATHABLE Class: "_parent_ ... may be implemented in any way convenient."
- Silent: no vendored text names a table, index, storage tier, encryption at rest, backup, replication, or physical deletion mechanics.

## A.1 Version container and identity (master06; BASE base_types master05)
1. VERSIONED_OBJECT {uid HIER_OBJECT_ID (no extension), owner_id OBJECT_REF, time_created}; owner_id "helps ensure that in storage systems, Versioned objects are always correctly allocated to their enclosing repository"; uid "the same in all copies ... in a distributed system".
2. VERSION.uid = OBJECT_VERSION_ID = object_id '::' creating_system_id '::' version_tree_id; version_tree_id = trunk_version[.branch_number.branch_version]; Owner_id_valid: owner_id.value = uid.object_id.value; the triple is globally unique.
3. Trunk starts at 1, single increments; branch pairs start at 1. Store the three parts.
4. preceding_version_uid stored; first xor preceding present; may carry a different creating_system_id.
5. creating_system_id is PER VERSION and can change along one container's trunk after a move. Never a container constant.
6. Local modification of a copied version must branch.
7. Branch copies require their preceding + trunk versions; never create a new container with an existing uid.
8. VERSIONED_OBJECT must answer: version_count, all_version_ids, all_versions, has_version_at_time, has_version_id, version_with_id, is_original_version, version_at_time, revision_history, latest_version (ANY branch), latest_trunk_version, trunk_lifecycle_state.
9. Composite identifiers: case-preserving, case-insensitive equality.

## A.2 Indelibility, logical deletion, lifecycle
10. "a versioned repository ... is by definition indelible, all logical changes including deletions ... are achieved by physically committing new Versions". Change types 249 creation, 523 deleted, 250 amendment, 251 modification, 249 import(?), 666 attestation.
11. Logical deletion: new Version, data Void, lifecycle_state deleted; "information can only ever be logically deleted"; causes include "patient direction to remove material".
12. lifecycle_state ∈ {532 complete, 553 incomplete, 523 deleted, 800 inactive, 801 abandoned}; any transition = new version.
13. Incomplete content persisted server-side with mandatory attributes absent: content never NOT NULL at row level.
14. BASE master07 §Security Policy: Indelibility; Audit trailing of "all changes made to the EHR including content objects as well as the EHR status and access control objects".
15. Disjoint merge ends with a 523 trunk version on the source and other_input_version_uids on the target.

## A.3 Contributions, audits, time
16. CONTRIBUTION {uid, versions List<OBJECT_REF>, audit}; every VERSION has contribution 1..1 and commit_audit 1..1.
17. "Contributions are similar to nested transactions. An attempt to commit a Contribution should only succeed if each Version and/or Attestation in the Contribution is committed successfully."
18. The list of all Contributions "provides a complete history of the change-sets ... basis for performing 'rollback'".
19. Contribution audit system_id/committer/time_committed copied into each VERSION.commit_audit.
20. time_committed = server time of availability; ITS-REST: "The time_committed attribute is always set by the server"; system_id defaults to the server's.
21. AUDIT_DETAILS.system_id = the logical EHR system, "distinct from any application, or any hosting infrastructure".
22. IMPORTED_VERSION wraps an ORIGINAL_VERSION verbatim; time queries use LOCAL commit time ("a key requirement for supporting medico-legal and historical investigations").
23. Attestations append to an existing ORIGINAL_VERSION without a new version; not assumed valid for later versions.
24. VERSION.signature 0..1; canonical_form serialisation "not yet defined by openEHR"; signatures may be forwarded to a notarisation service.

## A.4 EHR, EHR_STATUS, EHR_ACCESS, subject association
25. Root EHR records three immutables: system_id, ehr_id, time_created; references, not containment, to VERSIONED_X.
26. EHR invariants: ehr_access, ehr_status, compositions all VERSIONED_COMPOSITION, folders.item(1) = directory, tags within the same EHR.
27. EHR creation = root + EHR_STATUS + EHR_ACCESS "plus any other house-keeping information the versioning implementation requires"; a cloned EHR re-uses the source ehr_id.
28. EHR_STATUS: subject PARTY_SELF 1..1, is_queryable ("included in population queries"), is_modifiable ("the EHR, other than the EHR_STATUS object"), other_details archetyped. SM i_query_service: population query "on all EHRs whose status has the is_queryable flag set to True".
29. Subject association: external_ref "can be used ... Alternatively, the association between patients and their records may be done elsewhere for security reasons"; anonymous form → "a cross-reference table".
30. Three schemes (RM common master04 §PARTY_SELF): (1) no external_ref anywhere, link outside the EHR by ehr_id ↔ PMI id, "the most secure approach"; (2) once in EHR_STATUS.subject; (3) everywhere. BASE master07 §Anonymity: "a hacker has to steal not just EHR data but also separate demographic records and an identity cross-reference database, both of which can be located on different machines ... The identity cross-reference database would be easy to encrypt".
31. PARTY_REF = OBJECT_REF {namespace, type, id}, type ∈ PERSON/ORGANISATION/GROUP/AGENT/ROLE/PARTY/ACTOR.
32. EHR_ACCESS: "All access decisions to data in the EHR must be made in accordance with the policies and rules in this object"; versioned so past views are reconstructable.
33. Deactivation: is_modifiable=False for death/duplicate/opt-out/move; "will remain queryable, unless is_queryable is also set to False".
34. Historical views = previous informational states, for medico-legal purposes.
35. Read-access logging endorsed but outside EHR content: "openEHR does not specify models of such logs"; demerging uses access logs.
36. Folders reference compositions; each tree versioned. Tags: not content, no re-versioning, "indexing would be required".

## A.5 Locatable, paths, node identity
37. LOCATABLE {name, archetype_node_id, uid 0..1, links, archetype_details, feeder_audit}.
38. Node addressing by PATH not GUID: LOCATABLE_REF = OBJECT_VERSION_ID + absolute path from VERSION.data. Storage must resolve (OBJECT_VERSION_ID, path) → node.
39. Top-level uid = VERSION.uid.object_id recommended; PARTY uid mandatory.
40. FEEDER_AUDIT is content, may attach to any LOCATABLE, supports duplicate detection via "hash or content signature".

## A.6 Demographic
41. "Every Party is stored in its own Version container"; a PARTY version includes identities, contacts, relationships of which it is the source.
42. State identifiers are content in PARTY.details.
43. PARTY_RELATIONSHIPs stored in the source party; source/target OBJECT_REF with HIER_OBJECT_IDs; reverse_relationships implies a reverse index.
44. Demographic model may wrap an existing PMI.

## A.7 ITS-REST 1.1.0
45. PUT/POST/DELETE on change-controlled resources executed "the native way" (versioning under the hood).
46. Resource identifiers stable for life.
47. composition_get by version_uid or versioned_object_uid (+version_at_time → extant at t, else latest); 200 / 204 (deleted at time) / 404 (no version at t). Storage must distinguish no-version-at-t, 523-at-t, exists.
48. composition_delete needs the LATEST version uid; 409 with ETag latest when mismatched; 400 when already deleted. Cheap "current latest" needed.
49. If-Match preceding version → 412 with latest ETag.
50. ETag from VERSION.uid etc., weak allowed; Last-Modified from commit_audit.time_committed.
51. versioned_* quartets (get, revision_history, version at time, version by id) for COMPOSITION, EHR_STATUS, PARTY; directory_get_at_time with path.
52. ehr_create: EHR_STATUS always created; 409 on an existing EHR with the same (subject id, namespace) → the pair unique per service.
53. ehr_get_by_subject matches EHR_STATUS.subject.external_ref.id.value + namespace (wire binds subject lookup to EHR_STATUS content; SM get_ehrs_for_subject admits a list, REST narrows to one).
54. contribution_create: client uid accepted if unused; time_committed always server; system_id validated.
55. admin_ehr_delete: "All resources associated with or owned by the specified EHR (such as COMPOSITION, EHR_STATUS, ITEM_TAG, CONTRIBUTION, and their historical versions) will also be permanently and physically deleted, in compliance with applicable data protection regulations (e.g., the GDPR in the European Union). The server may execute this operation asynchronously ... 202 Accepted." delete_all may be disabled in production (405). Silent on backups, logs, derived stores.
56. Datetime fidelity (Resources.md): body date/time values "will be preserved as it was sent by the client ... Retrieval or querying those resources SHOULD return date, datetime, or time values in the (original) format". Storage keeps the literal.
57. Query API: population queries take no ehr_id; offset/fetch; ETag = result set id.

## A.8 AQL 1.1
58. CONTAINS = parent/child; AND/OR/parentheses; NOT CONTAINS.
59. Archetype predicate ≡ archetype_node_id equality; node predicates on archetype_node_id, name/value, name/defining_code/code_string + terminology_id/value, and general path op value.
60. ORDER BY over comparable (primitives + Ordered); no default ordering; LIMIT/OFFSET determinism needs ORDER BY; DISTINCT before LIMIT.
61. TERMINOLOGY() resolved at planning time.
62. LATEST_VERSION / ALL_VERSIONS: released PROSE IS SILENT; only the grammar (AqlParser.g4 versionPredicate: LATEST_VERSION | ALL_VERSIONS | standardPredicate). Semantics (trunk only? branches? deleted?) unspecified. RM latest_version = any branch; latest_trunk_version separate.
63. RESULT_SET 2-D; NULL for missing.

## A.9 SM
64. I_EHR_SERVICE: has_ehr_for_subject, create_ehr_for_subject, get_ehrs_for_subject: List<EHR_SUMMARY>.
65. I_EHR_STATUS: at_time (default latest), set/clear queryable/modifiable, at_version, versioned.
66. I_EHR_COMPOSITION: latest/at_time/at_version/versioned/create/update/delete (523 version).
67. I_EHR_CONTRIBUTION: commit_contribution, list_contributions(time_range, offset, fetch), contribution_count.
68. I_ADMIN_SERVICE: list_contributions, counts, physical_ehr_delete, physical_party_delete.
69. I_ADMIN_ARCHIVE: archive_ehrs "Move selected EHRs to archival storage"; archive_parties. No storage form.
70. I_ADMIN_DUMP_LOAD: export_ehrs, load_ehrs (duplicate ids fail).
71. I_EHR_INDEX = "EHR id / demographic subject cross-reference service": add_ehr_subject(ehr_id, subject_id OBJECT_REF, status, loc_desc), update_ehr_subject_status, remove_ehr_subject, remove_subject.
72. Multi-call realisations must be "transactionally protected".

## B. Law × storage columns: (a) separation (b) erasure reach (c) restriction marking (d) retention (e) access logging (f) encryption (g) residency (h) derived-store pseudonymisation

### B.1 GDPR
- Art. 4(5): (a) "additional information is kept separately and is subject to technical and organisational measures"; (h) definitional.
- Art. 5(1)(b),(c),(e): further processing for research per 89(1); minimisation; "kept in a form which permits identification ... for no longer than is necessary" (the FORM is the limit; non-identifying form satisfies it).
- Art. 5(1)(f): availability ("accidental loss, destruction"); security.
- Art. 11: controller need not hold identifiers merely to comply; 11(2) Arts 15-20 not applicable where the subject cannot be identified.
- Art. 17(1),(3): erasure without undue delay; copies named only in 17(2) for public data; SILENT on backups/internal copies; 17(3)(d) research exception via 89(1).
- Art. 18 + 4(3): "restriction of processing means the marking of stored personal data with the aim of limiting their processing in the future"; 18(2): only storage + named exceptions; 18(3): inform before lifting.
- Art. 25(1)-(2): pseudonymisation as the example measure from design time; by-default applies to "the period of their storage and their accessibility".
- Art. 30(1)(f),(g): record envisaged time limits for erasure per category; description of security measures.
- Art. 32(1)(a)-(c),(2): pseudonymisation and encryption; CIA + resilience; restore availability "in a timely manner"; risks incl. "unauthorised disclosure of, or access to personal data ... stored".
- Art. 33(3)(a): notification names categories/approximate numbers of subjects and records (implies the store can count per breach scope).
- Art. 34(3)(a): no subject communication if data rendered unintelligible "such as encryption".
- Art. 89(1): pseudonymisation where purposes allow; anonymisation where purposes can be fulfilled that way.

### B.2 EHDS (2025/327) — addressees are health data access bodies / holders / entities acting for them
- Art. 66(2)-(3): anonymised where possible, else pseudonymised; reversal info only at the access body or a trusted third party.
- Art. 73 SPE: unique identities; minimise copying; identifiable access logs kept for the verification period, "at least one year"; only anonymised statistical output leaves.
- Art. 87: store and process in the Union for pseudonymisation/anonymisation and Art. 67-72 operations; third country only under adequacy.
- Art. 51, 60: availability duties, no mechanics.

### B.3 EDPB Guidelines 01/2025
- §35-41: the pseudonymisation domain; §40 "ensure that additional information allowing the attribution to data subjects does not enter the pseudonymisation domain", "limit the resources available"; §41 within a controller the domain is "only the persons processing the pseudonymised data under its authority ..., the information they have at their disposal, and the systems and services they employ"; §39 pseudonymised data must not leave the domain.
- §88-91: keyed one-way functions (MAC) preferred; lookup tables kept secret; sufficient entropy; §91 plan to replace weak algorithms "without having to reconstitute the original personal data".
- §116-120: person pseudonyms = "risk of unauthorised attribution is comparatively high", admissible only if linking is necessary and lawful; §117 relationship secrets kept only for the relationship's duration; §118 relationship pseudonyms in separate tables per relationship; §119 transaction pseudonyms minimise most.
- Example 5: Trust Centre holds the lookup table; Data Centre staff cannot reach medical data; study groups get only query results from a secure processing environment.

### B.4 Netherlands
- UAVG Art. 46: a legally prescribed personal number used only for that law's purposes (use limit; silent on storage form).
- Wabvpz Art. 15e: patient may receive who made data available / who consulted, with dates.
- Begz Art. 5: logging per NEN 7513; retention set per NEN 7513.
- Besluit bewaartermijn logging: "ten minste 5 jaar bewaard vanaf het moment dat de logregel wordt geschreven".

### B.5 Germany
- BDSG § 22 Abs. 2 (measures menu): Nr. 2 traceability of input/change/removal "ob und von wem"; Nr. 5 access restriction within the controller; Nr. 6 pseudonymisation; Nr. 7 encryption; Nr. 8 restore availability.
- BDSG § 27 Abs. 3 (research): identifying features "gesondert zu speichern", merged only when the research purpose requires; anonymise as soon as possible.
- BDSG § 35: restriction instead of erasure — Abs. 1 scoped to "nicht automatisierter Datenverarbeitung wegen der besonderen Art der Speicherung"; Abs. 2/3 apply "entsprechend" where erasure would harm the subject's interests or retention periods bar it. Scope for automated CDR storage NOT stated: re-adjudicate.
- SGB V § 309 (TI applications): access logs reviewable for the 3-year § 195 BGB period; Abs. 3 delete "unverzüglich" after (a CEILING); Abs. 4 personenbeziehbar from 2030.
- SGB V § 363 Abs. 2-3 (ePA): pseudonymisation split FDZ / Vertrauensstelle; transmission documented in the ePA; only reliably automatically pseudonymised data transmitted; encryption.
- GDNG § 4 Abs. 5: pseudonymised records without visible pseudonyms inside a public-body secure processing environment; copying prevented.
- GDNG § 6: institutions' own further processing: pseudonymise, anonymise when possible, roles concept, log further processing, delete at latest 30 years after start.

### B.6 Switzerland
- DSG Art. 6 Abs. 4: "vernichtet oder anonymisiert, sobald sie zum Zweck der Bearbeitung nicht mehr erforderlich sind".
- DSG Art. 7, 8: privacy by design/default; security delegated to DSV.
- DSG Art. 25 Abs. 2 lit. d/e/g: access right includes retention period OR criteria, origin, recipients.
- DSG Art. 31 Abs. 2 lit. e: research — anonymise as soon as possible; sensitive data disclosed to third parties non-identifiable.
- DSV Art. 2: confidentiality, availability, traceability.
- DSV Art. 3: Zugriffskontrolle, Datenträgerkontrolle, Speicherkontrolle, Wiederherstellung, Eingabekontrolle ("welche Personendaten zu welcher Zeit und von welcher Person ... eingegeben oder verändert"), Bekanntgabekontrolle.
- DSV Art. 4: large-scale sensitive data → log storing/changing/disclosing/deleting/destroying/accessing; Abs. 4 actor identity, type, date, time, recipient; Abs. 5 logs kept "während mindestens einem Jahr", "getrennt vom System, in welchem die Personendaten bearbeitet werden", readable only by oversight roles.
- EPDG Art. 10: communities log every processing.
- EPDV Art. 10 (communities): medical EPD data "von anderen Datenbeständen getrennt gespeichert"; encryption for storage and transfer "nach aktuellem Stand der Technik"; destroy after 20 years; destroy all on dissolution; on request: not record certain data, exempt certain data from destruction (a per-datum mark), destroy certain data.
- EPDV Art. 12 Abs. 5: "Die Datenspeicher müssen sich in der Schweiz befinden und dem Schweizer Recht unterstehen."

### B.7 Silent / addressee-limited
GDPR 5(2), 30 (docs only), 33(3)(a); EHDS 51, 60; UAVG 46; DSG 7, 8, 28. Addressee limits: EHDS 66/73/87, GDNG § 4, SGB V § 309, § 363, EPDG 10 / EPDV 10, 12 bind a CDR only when the deployment is or acts for that addressee.

## C. Cross-check of the current design

### C.1 Served (evidence)
Three-part OBJECT_VERSION_ID, per-version creating_system_id, stored preceding_version_uid (ehr/0001 vo_version); 523 logical delete, nullable content; contribution + audit transactional, server time_committed; IMPORTED_VERSION verbatim (wrapped_original), local sys_period; vo_attestation; signature stored; immutable EHR triple, promoted is_queryable/is_modifiable; separation scheme 2 with opaque pseudonym; linkage.party_ehr realises the cross-reference database; audit schema separate from ehr, append-only; uq_ehr_subject partial unique; physical delete over both tiers via FK cascade + cold purge + audit rows; cold tier for I_ADMIN_ARCHIVE; nested-set CONTAINS, promoted predicate columns, ext.openehr_magnitude; national identifier sealed AES-256-GCM + HMAC lookup under per-tenant subkey with RLS; NL 5-year floor as 1830 days; per-domain dumps; FORCE RLS incl. cold.

### C.2 Gaps
1. Restriction marking (Art. 18/4(3), BDSG § 35, EPDV 10(2)(a)-(b)): none at any grain; is_queryable narrows population AQL only, is_modifiable writes only; point reads, exports, get_by_subject unaffected. #3324 admits the grain.
2. Erasure reach: delete_ehr does not touch linkage.party_ehr (linkage role has no DELETE) nor audit rows naming the patient; delete.rs "every trace of it" overstates; backups the controller's; demographic party a separate delete.
3. Content retention: no per-record/per-category retention datum in ehr/demographic/linkage; only audit store and outbox have retention_days; DSG 25(2)(d) wants a stated period or criteria.
4. Log separation (DSV 4(5)): audit is a schema in the same cluster; separation only via optional forwarding sinks; SGB V § 309(3) ceiling has no counterpart.
5. Encryption of clinical content: only national_identifier sealed; threat-model says no payload encryption before PG; EPDV 10(1)(c) demands storage encryption for EPD data.
6. Derived stores for secondary use: no separate domain; cohort queries on the primary store; the subject reference is ONE opaque UUID across the whole ehr domain = a PERSON pseudonym (EDPB §116 high risk); no relationship/project pseudonym scheme, no re-pseudonymisation plan.
7. A second cross-reference INSIDE the clinical domain: ehr.ehr_index (ehr_id, subject_id, subject_namespace, subject_type) and sp_* tables in the ehr schema; whether the pseudonym guard binds ehr_index.subject_id is not visible from the migrations. Verify.
8. Cluster-wide artefacts: one WAL archive/base backup holds all domains (shared_cluster gap acknowledged).
9. Residency: deployment matter; nothing recorded in storage.
10. latest_version vs latest_trunk_version: vo_version comment equates LATEST_VERSION with latest trunk; the RM's latest_version is any-branch; AQL prose defines neither token.
11. Datetime literal fidelity: vo_version.body text verbatim; node.data jsonb re-normalises scalars; AQL leaf projections come from node.data; original-format return not evidenced.

### C.3 Overstated page claims
- compliance index Art. 18 "shipped" at the whole record: is_queryable limits population queries only.
- Art. 17 "everything it owns ... over the primary and cold tier alike": linkage and audit rows persist; dpia §5 says linkage "Never deleted".
- BDSG § 35 presented as a general rule; its Abs. 1 is scoped to non-automated processing.
- DSV Art. 4 "shipped": separation from the processing system is optional forwarding.
- storage.md: "ALL_VERSIONS is the unfiltered table; LATEST_VERSION is that partial index": no released prose defines the tokens.
- storage.md: temporal single table "cheaper than current/history pairs": no measurement cited.
- storage.md: "seven schemas", ext/audit CREATE SCHEMA not in migrations read (migrator?).
- linkage header: SM get_ehrs_for_subject admits several EHRs per subject; REST 409 makes one-per-pair hard.
- demographic/linkage headers: "EDPB requires separation to hold against operators with database access" is a reading of §40-41, not its words.

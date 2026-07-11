# Spec-audit requirements — chapter: rm-integration

- **Date:** 2026-07-11
- **Component:** RM (Reference Model) — Integration package + the FEEDER_AUDIT / FEEDER_AUDIT_DETAILS common-package duties it depends on.
- **Spec files read:**
  - `docs/specs/openehr/RM/docs/integration/master02-integration_package.adoc` (whole chapter; class descriptions are `include::`d)
  - `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.integration.generic_entry.adoc` (GENERIC_ENTRY class table)
  - `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.feeder_audit.adoc` (FEEDER_AUDIT class table)
  - `docs/specs/openehr/RM/docs/UML/classes/org.openehr.rm.common.feeder_audit_details.adoc` (FEEDER_AUDIT_DETAILS class table + invariant)

Note: The `master02` integration chapter is largely design-basis prose (syntactic/semantic transformation strategy, ISO 13606 gateway) that is not machine-checkable. The machine-checkable normative content is the GENERIC_ENTRY structural contract and the FEEDER_AUDIT/FEEDER_AUDIT_DETAILS attribute/invariant tables the chapter pulls in.

| id | requirement | citation | category | risk |
|---|---|---|---|---|
| rm-integration-R1 | `GENERIC_ENTRY.data` is mandatory (`1..1`) and typed `ITEM`; a GENERIC_ENTRY with a missing/null `data` must be rejected. | generic_entry.adoc lines 18–20 (`*1..1* data: ITEM`) | mandatory-attr | high |
| rm-integration-R2 | `GENERIC_ENTRY.data` is typed to `ITEM` (abstract, closed subtype set `CLUSTER`/`ELEMENT`); a payload whose `_type` is not a valid `ITEM` descendant must be rejected. | generic_entry.adoc line 19 (`data: ITEM`) | rejection-duty | high |
| rm-integration-R3 | `GENERIC_ENTRY` has NO hard-wired attributes other than the single generic `data` attribute; it inherits only from `CONTENT_ITEM`/`LOCATABLE`. Any additional required own-field would be non-conformant. | master02 lines 62–64 ("contains no hard-wired attributes at all, only one generic attribute, `data`"); generic_entry.adoc lines 11–12, 18–20 | invariant | low |
| rm-integration-R4 | `GENERIC_ENTRY` inherits from `CONTENT_ITEM` (hence `LOCATABLE`), so `archetype_node_id` and the other LOCATABLE mandatory attributes apply to it. | generic_entry.adoc lines 11–12 (`Inherit CONTENT_ITEM`); master02 lines 72–76 | invariant | medium |
| rm-integration-R5 | As a subtype of `CONTENT_ITEM`, `GENERIC_ENTRY` is a valid value for `COMPOSITION.content`; the composition validator must accept GENERIC_ENTRY entries in `content`. | master02 lines 77–79 ("as a subtype of `CONTENT_ITEM`, `GENERIC_ENTRY` is a valid value for `COMPOSITION.content`") | behaviour | medium |
| rm-integration-R6 | GENERIC_ENTRY instances can only be committed to the record as part of a `COMPOSITION` instance (same commit/versioning/audit-trail rule as other content); they must not be committed standalone. | master02 lines 78–80 ("instances can only be committed to the record as part of a `COMPOSITION` instance") | behaviour | medium |
| rm-integration-R7 | The `LOCATABLE.feeder_audit` attribute is inherited by GENERIC_ENTRY (and every LOCATABLE node) and may carry source-system meta-data; a `feeder_audit` present anywhere on a locatable node must be validated as a `FEEDER_AUDIT`. | master02 lines 74–76 ("The `LOCATABLE` attribute feeder_audit is also inherited … may be used to mark every node of data") | mandatory-attr | low |
| rm-integration-R8 | `FEEDER_AUDIT.originating_system_audit` is mandatory (`1..1`) and typed `FEEDER_AUDIT_DETAILS`; a FEEDER_AUDIT missing `originating_system_audit` must be rejected. | feeder_audit.adoc lines 27–29 (`*1..1* originating_system_audit: FEEDER_AUDIT_DETAILS`) | mandatory-attr | high |
| rm-integration-R9 | `FEEDER_AUDIT.originating_system_audit` is typed to the concrete class `FEEDER_AUDIT_DETAILS` (no subtypes); a value with a foreign `_type` must be rejected. | feeder_audit.adoc line 28 | rejection-duty | high |
| rm-integration-R10 | `FEEDER_AUDIT.feeder_system_audit` is optional (`0..1`) and, when present, must be a `FEEDER_AUDIT_DETAILS`. | feeder_audit.adoc lines 31–33 (`*0..1* feeder_system_audit: FEEDER_AUDIT_DETAILS`) | mandatory-attr | medium |
| rm-integration-R11 | `FEEDER_AUDIT.originating_system_item_ids` is optional (`0..1`) and, when present, is a `List<DV_IDENTIFIER>`; every element must be a `DV_IDENTIFIER`. | feeder_audit.adoc lines 15–17 | mandatory-attr | low |
| rm-integration-R12 | `FEEDER_AUDIT.feeder_system_item_ids` is optional (`0..1`) and, when present, is a `List<DV_IDENTIFIER>`. | feeder_audit.adoc lines 19–21 | mandatory-attr | low |
| rm-integration-R13 | `FEEDER_AUDIT.original_content` is optional (`0..1`) and, when present, is a `DV_ENCAPSULATED` (abstract; subtypes `DV_MULTIMEDIA`/`DV_PARSABLE`) — reject a foreign `_type`. | feeder_audit.adoc lines 23–25 | rejection-duty | medium |
| rm-integration-R14 | `FEEDER_AUDIT_DETAILS.system_id` is mandatory (`1..1`) and typed `String`; a FEEDER_AUDIT_DETAILS missing `system_id` must be rejected. | feeder_audit_details.adoc lines 15–17 (`*1..1* system_id: String`) | mandatory-attr | high |
| rm-integration-R15 | Invariant `System_id_valid`: `not system_id.is_empty` — a FEEDER_AUDIT_DETAILS with an empty-string `system_id` must be rejected (fail-closed, not silently accepted). | feeder_audit_details.adoc lines 43–44 (`__System_id_valid__: not system_id.is_empty`) | rejection-duty | high |
| rm-integration-R16 | `FEEDER_AUDIT_DETAILS.location`, `.provider` are optional (`0..1`) and, when present, are `PARTY_IDENTIFIED` (validate accordingly; `PARTY_RELATED` is a permitted subtype). | feeder_audit_details.adoc lines 19–21, 27–29 | mandatory-attr | low |
| rm-integration-R17 | `FEEDER_AUDIT_DETAILS.subject` is optional (`0..1`) and, when present, is a `PARTY_PROXY` (abstract; `PARTY_IDENTIFIED`/`PARTY_SELF`/`PARTY_RELATED`) — validate against the proxy subtype set. | feeder_audit_details.adoc lines 23–25 | mandatory-attr | low |
| rm-integration-R18 | `FEEDER_AUDIT_DETAILS.time` is optional (`0..1`) and, when present, is a `DV_DATE_TIME`. | feeder_audit_details.adoc lines 31–33 | mandatory-attr | low |
| rm-integration-R19 | `FEEDER_AUDIT_DETAILS.version_id` is optional (`0..1`) and, when present, is a `String`; `other_details` is optional (`0..1`) and, when present, is an `ITEM_STRUCTURE`. | feeder_audit_details.adoc lines 35–41 | mandatory-attr | low |
| rm-integration-R20 | GENERIC_ENTRY data carries no built-in semantic coherence guarantee; the spec forbids treating a GENERIC_ENTRY store as a reliable clinical/queryable record — implementations must not silently promote GENERIC_ENTRY content into designed-archetype query paths without the semantic-transformation step. | master02 lines 87–100 | behaviour | low |

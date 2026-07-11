# A1 Spec Audit — Phase 1 (Extract) — chapter `base-base-types`

- **Date:** 2026-07-11
- **Component:** openEHR BASE — `base_types` (definitions, builtins, identification) + `foundation_types.Terminology_code`
- **Spec files read (relative to `docs/specs/openehr/`):**
  - `BASE/docs/base_types/master03-definitions_package.adoc`
  - `BASE/docs/base_types/master04-builtins_package.adoc`
  - `BASE/docs/base_types/master05-identification_package.adoc`
  - **Correction:** master03/04/05 are thin include shells. The normative class
    tables they `include::` live under `BASE/docs/UML/classes/`. The audited
    class files are:
    `org.openehr.base.base_types.{uid,iso_oid,uuid,internet_id,object_id,uid_based_id,hier_object_id,object_version_id,version_tree_id,archetype_id,template_id,terminology_id,generic_id,object_ref,party_ref,locatable_ref,basic_definitions,openehr_definitions,validity_kind,version_status}.adoc`
    and `org.openehr.base.foundation_types.terminology_code.adoc`.
    Citations below reference these class files plus the `<<Syntaxes>>` EBNF and the
    "Composite Identifiers and Case" section in `master05`.

## Chapter note follow-up (case-insensitive composite-identifier equality)

master05 §"Composite Identifiers and Case" (lines 164–177) makes case-insensitive
equality a normative duty for **all** composite (`OBJECT_ID`-family) identifiers.
B2 claims this landed; the audit must verify it holds at **every** comparison seam
— storage lookups, `If-Match`/OVID compare, and AQL identifier equality — not just
one. Captured as R31/R32 (high).

## Requirements

| id | requirement | citation | category | risk |
|---|---|---|---|---|
| base-base-types-R1 | `UID.value` is mandatory (1..1) and invariant `Value_valid: not value.empty` — an empty `UID` value string must be rejected. | `UML/classes/…uid.adoc` (UID Class: Attributes `value` 1..1; Invariants `Value_valid`) | rejection-duty | high |
| base-base-types-R2 | `UID` is abstract with exactly three concrete subtypes (`UUID`, `ISO_OID`, `INTERNET_ID`) whose string patterns are mutually exclusive; when materialising a `UID` from its string form, the correct subtype must be selected by pattern (a value must match exactly one). | master05 §"Primitive Identifiers" (lines 69–73); `<<Syntaxes>>` `uid = iso_oid \| uuid \| internet_id` | rejection-duty | high |
| base-base-types-R3 | `ISO_OID` value form is `number, { '.', number }` (dot-separated integers, ≥1 component). | master05 `<<Syntaxes>>` `iso_oid`; `…iso_oid.adoc` | validity-fn | medium |
| base-base-types-R4 | `UUID` value form is five hyphen-separated hex groups following the 8-4-4-4-12 pattern (`hex-number, '-', hex-number, '-', hex-number, '-', hex-number, '-', hex-number`). | master05 `<<Syntaxes>>` `uuid`; `…uuid.adoc` | validity-fn | medium |
| base-base-types-R5 | `INTERNET_ID` value form is a reverse-domain `subdomain` = dot-separated `label`s, each `label` an alphanumeric string optionally with internal `-`/`_` (per the relaxed RFC1034/1123 grammar). | master05 `<<Syntaxes>>` `internet_id`/`subdomain`/`label`; `…internet_id.adoc` | validity-fn | medium |
| base-base-types-R6 | `OBJECT_ID.value` is mandatory (1..1); direct (concrete) `OBJECT_ID` instances are permitted when no subtype is suitable. | `…object_id.adoc` (Attributes `value` 1..1; Description) | mandatory-attr | medium |
| base-base-types-R7 | `UID_BASED_ID.root()` returns the substring left of the first `'::'` separator, or the whole string if none. | `…uid_based_id.adoc` (Functions `root`) | validity-fn | medium |
| base-base-types-R8 | `UID_BASED_ID.extension()` returns the substring right of the first `'::'` separator, or the empty String if none. | `…uid_based_id.adoc` (Functions `extension`) | validity-fn | medium |
| base-base-types-R9 | `UID_BASED_ID.has_extension()` is true iff `extension` is non-empty; invariant `Has_extension_valid: extension.is_empty xor has_extension`. | `…uid_based_id.adoc` (Functions `has_extension`; Invariants) | invariant | medium |
| base-base-types-R10 | `OBJECT_VERSION_ID` lexical form is exactly three `'::'`-separated parts: `object_id '::' creating_system_id '::' version_tree_id`; a version id lacking any part must be rejected. | `…object_version_id.adoc` (Description); master05 `<<Syntaxes>>` `object_version_id`; §"Identifying Versions…" (lines 140–144) | rejection-duty | high |
| base-base-types-R11 | `OBJECT_VERSION_ID.object_id` and `.creating_system_id` are each a `UID` (must parse as a valid `UID`); `.version_tree_id` is a `VERSION_TREE_ID`. | `…object_version_id.adoc` (Functions); `<<Syntaxes>>` `object_id = uid`, `creating_system_id = uid` | mandatory-attr | high |
| base-base-types-R12 | `VERSION_TREE_ID.value` is mandatory (1..1); invariant `Value_valid: not value.is_empty`. | `…version_tree_id.adoc` (Attributes `value`; Invariants `Value_valid`) | invariant | medium |
| base-base-types-R13 | `VERSION_TREE_ID` invariant `Trunk_version_valid`: `trunk_version` must be a non-Void integer ≥ 1 (reject `0`, non-numeric trunk). | `…version_tree_id.adoc` (Invariants `Trunk_version_valid`) | rejection-duty | high |
| base-base-types-R14 | `VERSION_TREE_ID` invariants `Branch_number_valid` / `Branch_version_valid`: when present, `branch_number` and `branch_version` must each be an integer ≥ 1. | `…version_tree_id.adoc` (Invariants) | rejection-duty | high |
| base-base-types-R15 | `VERSION_TREE_ID` invariant `Branch_validity`: `branch_number` and `branch_version` are both present or both absent (`(bn=Void and bv=Void) xor (bn≠Void and bv≠Void)`) — a 2-part id must be rejected. | `…version_tree_id.adoc` (Invariants `Branch_validity`) | rejection-duty | high |
| base-base-types-R16 | `VERSION_TREE_ID` invariant `Is_branch_validity`: `is_branch xor branch_number = Void` (is_branch() true iff branch parts present). | `…version_tree_id.adoc` (Invariants `Is_branch_validity`; Functions `is_branch`) | invariant | low |
| base-base-types-R17 | `VERSION_TREE_ID` invariant `Is_first_validity`: `is_first()` is true iff `trunk_version` equals `"1"`. | `…version_tree_id.adoc` (Invariants `Is_first_validity`; Functions `is_first`) | invariant | low |
| base-base-types-R18 | `VERSION_TREE_ID` lexical form `trunk_version [ '.' branch_number '.' branch_version ]` — 1-part or 3-part dot-separated numbers only. | `…version_tree_id.adoc` (Description); master05 `<<Syntaxes>>` `version_tree_id` | validity-fn | medium |
| base-base-types-R19 | `ARCHETYPE_ID` lexical form `rm_originator '-' rm_name '-' rm_entity '.' concept_name { '-' specialisation }* '.v' version_id`: outer sections `'.'`-delimited, first-section parts `'-'`-delimited. | `…archetype_id.adoc` (Description); master05 §"Archetype Identifiers" (lines 93–113); `<<Syntaxes>>` `archetype_id` | validity-fn | medium |
| base-base-types-R20 | `ARCHETYPE_ID` `version_id = '0' \| non-zero-digit, [ number ]` — numeric only; a lifecycle-suffixed version part like `.v1draft` is nonconforming. | master05 `<<Syntaxes>>` `version_id`; §"Archetype Identifiers" WARNING (line 115) | validity-fn | medium |
| base-base-types-R21 | `TERMINOLOGY_ID` lexical form `name [ '(' version ')' ]`; `name()` and `version_id()` parse those parts (empty version → empty string). | `…terminology_id.adoc` (Description, Functions); master05 `<<Syntaxes>>` `terminology_id` | validity-fn | medium |
| base-base-types-R22 | `GENERIC_ID.scheme` is mandatory (1..1) — a `GENERIC_ID` without a scheme name must be rejected. | `…generic_id.adoc` (Attributes `scheme` 1..1) | mandatory-attr | medium |
| base-base-types-R23 | `OBJECT_REF.namespace` is mandatory (1..1) and its legal values are `"local"`, `"unknown"`, or a string matching `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*`; a namespace not matching must be rejected. | `…object_ref.adoc` (Attributes `namespace`) | rejection-duty | high |
| base-base-types-R24 | `OBJECT_REF.type` is mandatory (1..1); RM class name (concrete/abstract) or `"ANY"`. | `…object_ref.adoc` (Attributes `type`) | mandatory-attr | medium |
| base-base-types-R25 | `OBJECT_REF.id` is mandatory (1..1) and typed `OBJECT_ID`. | `…object_ref.adoc` (Attributes `id`) | mandatory-attr | medium |
| base-base-types-R26 | `PARTY_REF` invariant `Type_validity`: `type` must be one of `PERSON`, `ORGANISATION`, `GROUP`, `AGENT`, `ROLE`, `PARTY`, `ACTOR`; any other type value must be rejected. | `…party_ref.adoc` (Invariants `Type_validity`) | rejection-duty | high |
| base-base-types-R27 | `LOCATABLE_REF.id` is redefined (narrowed) from `OBJECT_ID` to `UID_BASED_ID` — a `LOCATABLE_REF` whose `id` is a non-`UID_BASED_ID` (e.g. `TERMINOLOGY_ID`, `GENERIC_ID`) must be rejected. | `…locatable_ref.adoc` (Attributes `id` 1..1 redefined → `UID_BASED_ID`) | rejection-duty | high |
| base-base-types-R28 | `LOCATABLE_REF.path` is optional (0..1); an empty/absent path means the object referred to by `id` itself is specified. | `…locatable_ref.adoc` (Attributes `path` 0..1) | mandatory-attr | low |
| base-base-types-R29 | `LOCATABLE_REF.as_uri()` = concatenation of scheme (derived from `namespace`, e.g. `ehr:`) + `id.value` + (`/` + `path` when `path` non-empty). | `…locatable_ref.adoc` (Functions `as_uri`) | behaviour | low |
| base-base-types-R30 | `Terminology_code`: `terminology_id` (1..1) and `code_string` (1..1) mandatory; `terminology_version` (0..1) and `uri` (0..1) optional — a term code missing terminology_id or code_string must be rejected. | `…foundation_types.terminology_code.adoc` (Attributes) | mandatory-attr | medium |
| base-base-types-R31 | Composite (`OBJECT_ID`-family) identifier equality is **case-insensitive**: two identifiers identical apart from case are the same identifier and identify the same thing — must hold at every comparison seam (storage lookup, If-Match/OVID compare, AQL). | master05 §"Composite Identifiers and Case" (lines 164–169) | behaviour | high |
| base-base-types-R32 | Composite identifiers are **case-preserving**: persistence, copying, transfer, or other computation must not alter the case of the identifier as created. | master05 §"Composite Identifiers and Case" (lines 166–168) | behaviour | medium |
| base-base-types-R33 | The 'meaningful' identifier types' human-readable sections use only the basic latin set (+ per-production special chars); accented/diacritical letters are not allowed. | master05 §"Composite Identifiers and Language" (lines 179–181); `<<Syntaxes>>` `letter` (A–Z, a–z only) | validity-fn | low |
| base-base-types-R34 | `BASIC_DEFINITIONS` constants have fixed values: `CR='\015'`, `LF='\012'`, `Any_type_name="Any"`, `Regex_any_pattern=".*"`, `Default_encoding="UTF-8"`, `None_type_name="None"`. | `…basic_definitions.adoc` (Constants) | mandatory-attr | low |
| base-base-types-R35 | `OPENEHR_DEFINITIONS.Local_terminology_id = "local"` (predefined local terminology id). | `…openehr_definitions.adoc` (Constants); cf. `OBJECT_REF` `"local"` namespace | mandatory-attr | low |
| base-base-types-R36 | `VALIDITY_KIND` enumeration values are exactly `mandatory`, `optional`, `prohibited`, `disallowed` (`disallowed` deprecated, AOM 1.4 only; AOM 2 uses `prohibited`). | `…validity_kind.adoc` (Constants) | serialization | low |
| base-base-types-R37 | `VERSION_STATUS` enumeration values are exactly `alpha`, `beta`, `release_candidate`, `released`, `build`. | `…version_status.adoc` (Constants) | serialization | low |

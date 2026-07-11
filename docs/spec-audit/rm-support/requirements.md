# A1 Spec Audit — Phase 1 (Extract) — Chapter: rm-support

- **Date:** 2026-07-11
- **Component:** RM support package + identification / measurement / terminology interfaces (RM 1.2.0 / BASE 1.3.0 / TERM 3.1.0)

## Spec files read

- `RM/docs/support/master02-support_package.adoc` (overview only; class defs via include)
- `RM/docs/support/master03-assumed_types.adoc` (pointer → BASE foundation types)
- `RM/docs/support/master04-identification_package.adoc` (**pointer → BASE**; see correction)
- `RM/docs/support/master05-terminology_package.adoc` (terminology interface duties + binding invariant patterns)
- `RM/docs/support/master06-measurement_package.adoc` (units validity service)
- `RM/docs/support/master07-definition_package.adoc` (**pointer → BASE**; see correction)
- `RM/docs/UML/classes/org.openehr.rm.support.{terminology_service,terminology_access,code_set_access,measurement_service,external_environment_access,openehr_terminology_group_identifiers,openehr_code_set_identifiers}.adoc`

### File corrections (listed files that redirect)

- **`master04-identification_package.adoc`** contains only a NOTE that the identification classes are now in the BASE component. Authoritative content read from **`BASE/docs/base_types/master05-identification_package.adoc`** (narrative + EBNF `Syntaxes` grammar) and the class-def includes under **`BASE/docs/UML/classes/org.openehr.base.base_types.*.adoc`** (`object_id`, `uid_based_id`, `hier_object_id`, `object_version_id`, `version_tree_id`, `archetype_id`, `template_id`, `terminology_id`, `generic_id`, `internet_id`, `iso_oid`, `object_ref`, `party_ref`, `locatable_ref`).
- **`master07-definition_package.adoc`** is likewise a pointer to the BASE Definitions package; no machine-checkable identification rules are unique to it beyond what the identification grammar covers.

---

## Requirements

| ID | Requirement | Citation | Category | Risk |
|----|-------------|----------|----------|------|
| rm-support-R1 | A UID string parsed/serialized as a UUID MUST match the lexical form `hex-number '-' hex-number '-' hex-number '-' hex-number '-' hex-number` (5 hex groups, `-`-separated); reject malformed UUIDs. | BASE base_types master05, §Syntaxes EBNF `uuid` (L231, L290–293) | rejection-duty | high |
| rm-support-R2 | An `ISO_OID` value MUST match `number { '.' number }` (dot-separated non-negative integers); reject anything else in that slot. | BASE base_types master05, §Syntaxes EBNF `iso_oid` (L230), ISO_OID class desc | rejection-duty | medium |
| rm-support-R3 | An `INTERNET_ID` value MUST match `subdomain = label \| subdomain '.' label`, `label = alphanum \| alphanum-ext-str alphanum` (letters/digits, `-`, `_`; each label starts+ends alphanumeric). | BASE base_types master05, §Syntaxes EBNF `internet_id`/`subdomain`/`label` (L240–242, L279) | validity-fn | low |
| rm-support-R4 | The three `UID` subtypes (`UUID`, `ISO_OID`, `INTERNET_ID`) have mutually exclusive string patterns; a UID string MUST resolve to exactly one subtype on deserialization (structure-driven typing). | BASE base_types master05 §Primitive Identifiers (L69–73) | behaviour | medium |
| rm-support-R5 | `UID_BASED_ID` invariant `Has_extension_valid`: `extension.is_empty xor has_extension` — `has_extension` MUST be true iff extension is non-empty. | BASE base_types `uid_based_id.adoc` Invariants | invariant | medium |
| rm-support-R6 | `UID_BASED_ID.root()` returns the substring left of the first `'::'` (or whole string if none); `extension()` returns the substring right of the first `'::'` (or empty). Lexical form `root '::' extension`. | BASE base_types `uid_based_id.adoc` functions; §Syntaxes `uid_based_id` (L246) | validity-fn | medium |
| rm-support-R7 | An `OBJECT_VERSION_ID` value MUST have exactly the 3-part form `object_id '::' creating_system_id '::' version_tree_id`; reject values with fewer/more `'::'`-delimited parts. | BASE base_types `object_version_id.adoc` desc; §Syntaxes `object_version_id` (L251); master05 §Identifying Versions (L140–152) | rejection-duty | high |
| rm-support-R8 | In `OBJECT_VERSION_ID`, both `object_id` and `creating_system_id` MUST each be a valid `UID` (UUID/ISO_OID/INTERNET_ID); reject non-UID values in these parts. | BASE base_types `object_version_id.adoc` functions (object_id: UID, creating_system_id: UID); §Syntaxes (L252–253) | rejection-duty | high |
| rm-support-R9 | `VERSION_TREE_ID` invariant `Value_valid`: `not value.is_empty` — reject an empty version-tree id. | BASE base_types `version_tree_id.adoc` Invariants | invariant | high |
| rm-support-R10 | `VERSION_TREE_ID` invariant `Trunk_version_valid`: `trunk_version` is an integer and `>= 1`. | BASE base_types `version_tree_id.adoc` Invariants | invariant | high |
| rm-support-R11 | `VERSION_TREE_ID` invariant `Branch_number_valid`: if `branch_number` present it is an integer `>= 1`. | BASE base_types `version_tree_id.adoc` Invariants | invariant | high |
| rm-support-R12 | `VERSION_TREE_ID` invariant `Branch_version_valid`: if `branch_version` present it is an integer `>= 1`. | BASE base_types `version_tree_id.adoc` Invariants | invariant | high |
| rm-support-R13 | `VERSION_TREE_ID` invariant `Branch_validity`: `(branch_number=Void and branch_version=Void) xor (branch_number/=Void and branch_version/=Void)` — the id is either 1-part or 3-part, never 2-part; reject a 2-part value. | BASE base_types `version_tree_id.adoc` Invariants; §Syntaxes `version_tree_id` (L256) | rejection-duty | high |
| rm-support-R14 | `VERSION_TREE_ID` invariant `Is_branch_validity`: `is_branch xor branch_number = Void` — `is_branch()` true iff a branch part exists. | BASE base_types `version_tree_id.adoc` Invariants | invariant | medium |
| rm-support-R15 | `VERSION_TREE_ID` invariant `Is_first_validity`: `not is_first xor trunk_version.is_equal("1")` — `is_first()` true iff trunk_version = "1". | BASE base_types `version_tree_id.adoc` Invariants | invariant | medium |
| rm-support-R16 | `VERSION_TREE_ID` lexical form `trunk_version [ '.' branch_number '.' branch_version ]` with every part a `number`; reject non-numeric / wrong-arity dot forms. | BASE base_types master05 §Syntaxes `version_tree_id` (L255–259) | rejection-duty | high |
| rm-support-R17 | An `ARCHETYPE_ID` value MUST match `qualified_rm_entity '.' domain_concept '.v' version_id` where `qualified_rm_entity = rm_originator '-' rm_name '-' rm_entity`; reject malformed archetype ids. | BASE base_types `archetype_id.adoc` desc; §Syntaxes `archetype_id`/`qualified_rm_entity` (L262–263) | rejection-duty | high |
| rm-support-R18 | `ARCHETYPE_ID` `version_id` MUST be numeric: `'0' \| non-zero-digit [number]`; reject nonconforming version parts such as `.v1draft` (lifecycle-status suffix). | BASE base_types master05 §Syntaxes `version_id` (L270); WARNING at L115 | rejection-duty | high |
| rm-support-R19 | `ARCHETYPE_ID` parts `rm_originator`, `rm_name`, `rm_entity`, `concept_name`, `specialisation` MUST each be `alphanum-str` = `letter { letter \| digit \| '_' }` (no leading digit, no `-` inside a part). | BASE base_types master05 §Syntaxes `alphanum-str` (L278), archetype productions (L264–269) | validity-fn | medium |
| rm-support-R20 | `ARCHETYPE_ID.domain_concept` = `concept_name { '-' specialisation }` — specialisation segments are `-`-delimited within the domain-concept section. | BASE base_types master05 §Syntaxes `domain_concept` (L267) | validity-fn | medium |
| rm-support-R21 | A `TERMINOLOGY_ID` value MUST match `name-str [ '(' name-str ')' ]` with `name-str = letter { letter \| digit \| '_' \| '-' \| '/' \| '+' }`; the optional parenthesised part is the version. | BASE base_types `terminology_id.adoc` desc; §Syntaxes `terminology_id`/`name-str` (L273, L277) | rejection-duty | medium |
| rm-support-R22 | `OBJECT_ID.value` is a mandatory (1..1) String; every concrete OBJECT_ID subtype MUST carry a non-null value in the defined lexical form. | BASE base_types `object_id.adoc` Attributes (value 1..1) | mandatory-attr | medium |
| rm-support-R23 | `OBJECT_REF.namespace` (1..1) legal values are `"local"`, `"unknown"`, or a string matching `[a-zA-Z][a-zA-Z0-9_.:\/&?=+-]*`; reject namespaces not matching. | BASE base_types `object_ref.adoc` namespace attribute | rejection-duty | high |
| rm-support-R24 | `OBJECT_REF.type` (1..1) is a mandatory String naming a concrete/abstract RM class or `"ANY"`. | BASE base_types `object_ref.adoc` type attribute | mandatory-attr | medium |
| rm-support-R25 | `OBJECT_REF.id` (1..1) is a mandatory `OBJECT_ID`. | BASE base_types `object_ref.adoc` id attribute | mandatory-attr | medium |
| rm-support-R26 | `PARTY_REF` invariant `Type_validity`: `type` MUST be one of `PERSON`, `ORGANISATION`, `GROUP`, `AGENT`, `ROLE`, `PARTY`, `ACTOR`; reject any other type string in a PARTY_REF. | BASE base_types `party_ref.adoc` Invariants | rejection-duty | high |
| rm-support-R27 | `LOCATABLE_REF.id` is redefined (narrowed) to `UID_BASED_ID` — a LOCATABLE_REF MUST reject an `id` that is not a UID_BASED_ID (`HIER_OBJECT_ID`/`OBJECT_VERSION_ID`); e.g. a `TERMINOLOGY_ID`/`GENERIC_ID`/`ARCHETYPE_ID` in this slot is invalid. | BASE base_types `locatable_ref.adoc` id (1..1 redefined: UID_BASED_ID) | rejection-duty | high |
| rm-support-R28 | `LOCATABLE_REF.path` is optional (0..1); an empty/absent path means the reference targets the object identified by `id` itself. | BASE base_types `locatable_ref.adoc` path attribute | mandatory-attr | low |
| rm-support-R29 | `LOCATABLE_REF.as_uri()` MUST concatenate: scheme (derived from `namespace`, e.g. `ehr:`) + `id.value` + (`/` + `path` when path non-empty). | BASE base_types `locatable_ref.adoc` as_uri function | serialization | medium |
| rm-support-R30 | Composite identifiers are **case-insensitive**: two identifiers identical apart from case MUST be treated as identifying the same thing (equality must fold case). | BASE base_types master05 §Composite Identifiers and Case (L166–169) | behaviour | medium |
| rm-support-R31 | Composite identifiers are **case-preserving**: persistence/copy/transfer MUST NOT alter the stored case of an identifier. | BASE base_types master05 §Composite Identifiers and Case (L166–168) | behaviour | low |
| rm-support-R32 | Meaningful identifier textual parts use only the basic latin character set (per each production); accented/diacritical letters MUST NOT be accepted. | BASE base_types master05 §Composite Identifiers and Language (L179–181) | rejection-duty | low |
| rm-support-R33 | `TERMINOLOGY_SERVICE.code_set(name)` has precondition `has_code_set(name)` — MUST NOT return a code set for an unknown internal name. | RM support `terminology_service.adoc` code_set (Pre: has_code_set) | validity-fn | medium |
| rm-support-R34 | `TERMINOLOGY_SERVICE.code_set_for_id(id)` has precondition `valid_code_set_id(id)` — MUST reject an id not in the defined internal code-set id set. | RM support `terminology_service.adoc` code_set_for_id (Pre: valid_code_set_id) | validity-fn | medium |
| rm-support-R35 | For `DV_CODED_TEXT`-typed coded attributes, the `defining_code` (`CODE_PHRASE`) MUST be a member of the mandated openEHR terminology group (e.g. `Change_type_valid: terminology(openehr).has_code_for_group_id(Group_id_audit_change_type, change_type.defining_code)`); reject codes outside the bound group. | RM support master05 §Terms and Codes (L49–61) | rejection-duty | high |
| rm-support-R36 | For `CODE_PHRASE`-typed attributes bound to a code set, the code MUST be present in that code set (e.g. ENTRY `Language_valid: code_set(Code_set_languages).has_code(language)`); reject codes not in the code set. | RM support master05 §Terms and Codes (L63–68) | rejection-duty | high |
| rm-support-R37 | `valid_code_set_id` MUST accept exactly the internal code-set ids defined by `OPENEHR_CODE_SET_IDENTIFIERS`: character sets, compression algorithms, countries, integrity check algorithms, languages, media types, normal statuses. | RM support `openehr_code_set_identifiers.adoc` constants + valid_code_set_id | validity-fn | medium |
| rm-support-R38 | The mandated openEHR terminology group ids (`audit change type`, `attestation reason`, `composition category`, `event math function`, `instruction states`, `instruction transitions`, `null flavours`, `property`, `participation function`, `participation mode`, `setting`, …) MUST be recognised for group-based code validation. | RM support `openehr_terminology_group_identifiers.adoc` constants | behaviour | low |
| rm-support-R39 | `MEASUREMENT_SERVICE.is_valid_units_string(units)` is true iff `units` is a valid string per the HL7 **UCUM** specification; `DV_QUANTITY.units` validation MUST use UCUM validity (reject non-UCUM unit strings). | RM support master06 (L11–12) + `measurement_service.adoc` is_valid_units_string | validity-fn | high |
| rm-support-R40 | `MEASUREMENT_SERVICE.units_equivalent(units1, units2)` is true iff both unit strings correspond to the same measured property (dimensional equivalence). | RM support `measurement_service.adoc` units_equivalent | validity-fn | low |

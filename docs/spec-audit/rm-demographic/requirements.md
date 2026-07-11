# A1 Spec Audit — Phase 1 (Extract) — chapter: rm-demographic

- **Chapter:** rm-demographic
- **Date:** 2026-07-11
- **Spec files read (all under `docs/specs/openehr/`):**
  - `RM/docs/demographic/master02-demographic_package.adoc` (overview, party-relationship serialisation rule, versioning semantics)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.party.adoc` (PARTY)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.actor.adoc` (ACTOR)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.person.adoc` (PERSON)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.organisation.adoc` (ORGANISATION)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.group.adoc` (GROUP)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.agent.adoc` (AGENT)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.role.adoc` (ROLE)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.capability.adoc` (CAPABILITY)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.contact.adoc` (CONTACT)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.address.adoc` (ADDRESS)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.party_identity.adoc` (PARTY_IDENTITY)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.party_relationship.adoc` (PARTY_RELATIONSHIP)
  - `RM/docs/UML/classes/org.openehr.rm.demographic.versioned_party.adoc` (VERSIONED_PARTY)

**Chapter note:** the demographic REST/wire is our own extension design (blueprint row 5); this audit targets RM **semantics** (invariants, mandatory slots, types, versioning duties), not the wire shape.

**Note on file listing:** all listed files exist; no corrections needed. The
class-detail files are `org.openehr.rm.demographic.*.adoc` (as expected).

---

| id | requirement | citation | category | risk |
|---|---|---|---|---|
| rm-demographic-R1 | PARTY.identities is mandatory (`1..1`, `List<PARTY_IDENTITY>`) — every PARTY (PERSON/ORGANISATION/GROUP/AGENT/ROLE) must carry an identities list; absence must be rejected. | party.adoc L21-23 (`*1..1* identities: List<PARTY_IDENTITY>`) | mandatory-attr | high |
| rm-demographic-R2 | PARTY invariant `Identities_valid`: `not identities.is_empty` — an empty identities list must be rejected on write. | party.adoc L55 (`Identities_valid: not identities.is_empty`) | invariant | high |
| rm-demographic-R3 | PARTY invariant `Contacts_valid`: `contacts /= Void implies not contacts.is_empty` — when a contacts list is present it must be non-empty; a present-but-empty contacts list must be rejected. | party.adoc L58 | invariant | high |
| rm-demographic-R4 | PARTY invariant `Relationships_validity`: `relationships /= Void implies (not relationships.is_empty and then relationships.for_all(r | r.source = self))` — a present relationships list must be non-empty and every relationship's `source` must reference this Party. | party.adoc L61 | invariant | high |
| rm-demographic-R5 | PARTY invariant `Type_valid`: `type = name` — the `type()` function value must equal the inherited `name` attribute value. | party.adoc L52, L42-43 | invariant | medium |
| rm-demographic-R6 | PARTY invariant `Is_archetype_root`: `is_archetype_root` — every PARTY must be an archetype root (its `archetype_details`/root marker must be set); a non-root PARTY must be rejected. | party.adoc L67 | invariant | high |
| rm-demographic-R7 | PARTY invariant `Uid_mandatory`: `uid /= Void` — the inherited `uid` must be populated on every PARTY; a missing uid must be rejected. | party.adoc L70; L11-12 (uid copied from enclosing VERSION `object_id()`) | invariant | high |
| rm-demographic-R8 | PARTY.contacts is optional (`0..1`, `List<CONTACT>`); PARTY.details is optional (`0..1`, `ITEM_STRUCTURE`); PARTY.relationships is optional (`0..1`, `List<PARTY_RELATIONSHIP>`). | party.adoc L25-35 | cardinality | low |
| rm-demographic-R9 | PARTY function `reverse_relationships(): List<LOCATABLE_REF>` — post-condition `Post_reverse_relationships_validity`: if non-Void, non-empty and every referenced party-relationship's `target` equals self (references to relationships where this Party is target). | party.adoc L46-49, L64 | validity-fn | low |
| rm-demographic-R10 | PARTY_IDENTITY.details is mandatory (`1..1`, `ITEM_STRUCTURE`) — a PARTY_IDENTITY with no details must be rejected. | party_identity.adoc L18-20 | mandatory-attr | high |
| rm-demographic-R11 | PARTY_IDENTITY invariant `Purpose_valid`: `purpose = name` — the `purpose()` function value must equal the inherited `name`. | party_identity.adoc L31, L27-28 | invariant | medium |
| rm-demographic-R12 | ACTOR.languages optional (`0..1`, `List<DV_TEXT>`); ACTOR.roles optional (`0..1`, `List<PARTY_REF>`) — roles list holds identifiers of the Version container of each Role played. | actor.adoc L18-24 | cardinality | low |
| rm-demographic-R13 | ACTOR invariant `Roles_valid`: `roles /= Void implies not roles.is_empty` — a present roles list must be non-empty. | actor.adoc L27 | invariant | high |
| rm-demographic-R14 | ACTOR.roles is typed `List<PARTY_REF>` (monomorphic ref slot) — each entry must be a PARTY_REF; a foreign `_type` ref must be rejected. | actor.adoc L22-24 | rejection-duty | high |
| rm-demographic-R15 | PERSON inherits ACTOR with no additional attributes/invariants — a PERSON is an ACTOR (and thus a PARTY) and carries all ACTOR/PARTY invariants. | person.adoc L11-13 | invariant | low |
| rm-demographic-R16 | ORGANISATION inherits ACTOR with no additional attributes/invariants — carries all ACTOR/PARTY invariants. | organisation.adoc L11-13 | invariant | low |
| rm-demographic-R17 | GROUP inherits ACTOR with no additional attributes/invariants — carries all ACTOR/PARTY invariants. | group.adoc L11-13 | invariant | low |
| rm-demographic-R18 | AGENT inherits ACTOR with no additional attributes/invariants — carries all ACTOR/PARTY invariants (agent = device/software system, not human/organisation). | agent.adoc L11-13 | invariant | low |
| rm-demographic-R19 | ROLE.performer is mandatory (`1..1`, `PARTY_REF`) — reference to the Version container of the Actor playing the role; a ROLE with no performer must be rejected. | role.adoc L22-24 | mandatory-attr | high |
| rm-demographic-R20 | ROLE.performer is typed `PARTY_REF` (monomorphic ref slot) — must be a PARTY_REF; a foreign `_type` ref must be rejected. | role.adoc L22-24 | rejection-duty | high |
| rm-demographic-R21 | ROLE.time_validity optional (`0..1`, `DV_INTERVAL<DV_DATE>`); ROLE.capabilities optional (`0..1`, `List<CAPABILITY>`). | role.adoc L18-28 | cardinality | low |
| rm-demographic-R22 | ROLE invariant `Capabilities_valid`: `capabilities /= Void implies not capabilities.empty` — a present capabilities list must be non-empty. | role.adoc L31 | invariant | high |
| rm-demographic-R23 | ROLE inherits PARTY — thus subject to R1-R7 (identities mandatory & non-empty, uid mandatory, is_archetype_root, type=name), and is independently versioned (R37). | role.adoc L11-12; master02 L47-48 | invariant | high |
| rm-demographic-R24 | CAPABILITY.credentials is mandatory (`1..1`, `ITEM_STRUCTURE`) — a CAPABILITY with no credentials must be rejected. | capability.adoc L18-20 | mandatory-attr | high |
| rm-demographic-R25 | CAPABILITY.time_validity optional (`0..1`, `DV_INTERVAL<DV_DATE>`). | capability.adoc L22-24 | cardinality | low |
| rm-demographic-R26 | CONTACT.addresses is mandatory (`1..1`, `List<ADDRESS>`) — a CONTACT with no addresses list must be rejected. | contact.adoc L18-20 | mandatory-attr | high |
| rm-demographic-R27 | CONTACT invariant `Purpose_valid`: `purpose = name` — the `purpose()` function value must equal the inherited `name`. | contact.adoc L35, L31-32 | invariant | medium |
| rm-demographic-R28 | CONTACT.time_validity optional (`0..1`, `DV_INTERVAL<DV_DATE>`). | contact.adoc L22-24 | cardinality | low |
| rm-demographic-R29 | ADDRESS.details is mandatory (`1..1`, `ITEM_STRUCTURE`) — an ADDRESS with no details must be rejected. | address.adoc L18-20 | mandatory-attr | high |
| rm-demographic-R30 | ADDRESS invariant `Type_valid`: `type = name` — the `type()` function value must equal the inherited `name`. | address.adoc L31, L27-28 | invariant | medium |
| rm-demographic-R31 | PARTY_RELATIONSHIP.target is mandatory (`1..1`, `PARTY_REF`) — a relationship with no target must be rejected. | party_relationship.adoc L22-24 | mandatory-attr | high |
| rm-demographic-R32 | PARTY_RELATIONSHIP.source is mandatory (`1..1`, `PARTY_REF`) — a relationship with no source must be rejected. | party_relationship.adoc L30-32 | mandatory-attr | high |
| rm-demographic-R33 | PARTY_RELATIONSHIP.source and .target are typed `PARTY_REF` (monomorphic ref slots) — each must be a PARTY_REF; a foreign `_type` must be rejected. | party_relationship.adoc L22-24, L30-32 | rejection-duty | high |
| rm-demographic-R34 | PARTY_RELATIONSHIP invariant `Source_valid`: `source /= Void and then source.relationships.has(self)` — the source Party's relationships list must contain this relationship (relationship stored by value under its source Party). | party_relationship.adoc L43; master02 L40, L44 | invariant | high |
| rm-demographic-R35 | PARTY_RELATIONSHIP invariant `Target_valid`: `target /= Void and then not target.reverse_relationships.has(self)` — the target must be set and must NOT list this relationship as a stored reverse-relationship (reverse links are computed, not stored). | party_relationship.adoc L46 | invariant | medium |
| rm-demographic-R36 | PARTY_RELATIONSHIP invariant `Type_validity`: `type = name` — the `type()` function value must equal the inherited `name`. | party_relationship.adoc L49, L38-40 | invariant | medium |
| rm-demographic-R37 | PARTY and its descendants ACTOR and ROLE are the versioned entities; each Party is stored in its own Version container (`VERSIONED_PARTY`), and a Version of a PARTY includes all compositional parts (identities, contacts, party-relationships of which it is source). | master02 L46-48; versioned_party.adoc L8-9 | behaviour | medium |
| rm-demographic-R38 | VERSIONED_PARTY = `VERSIONED_OBJECT<PARTY>` — the version container binds the generic parameter to PARTY; it carries all VERSIONED_OBJECT versioning duties (change control, audit). | versioned_party.adoc L8-12 | behaviour | medium |
| rm-demographic-R39 | PARTY_RELATIONSHIPs are stored by value under the source Party (`relationships` attribute is by value), while `source`/`target` are by reference — a Party must be a self-contained serialisable hierarchy; a relationship must not embed the target Party by value. | master02 L16, L44 | behaviour | medium |
| rm-demographic-R40 | The references in PARTY_RELATIONSHIP.source/.target must denote the Version *container* of a Party via `OBJECT_REF` carrying a `HIER_OBJECT_ID` (continuant), NOT an `OBJECT_VERSION_ID` (a particular version). | master02 L44 | behaviour | medium |
| rm-demographic-R41 | PARTY.uid should be copied from the enclosing VERSION's `uid.object_id()` (e.g. `ORIGINAL_VERSION.uid` `<uuid>::<system>::2` → PARTY.uid) — the Party uid ties to its version container identity. | party.adoc L11-12 | behaviour | low |
| rm-demographic-R42 | Party identifiers assigned by external organisations/state are NOT `identities` (which are self-owned names, `PARTY_IDENTITY`) — they belong in `PARTY.details`; conflating them into `identities` misrepresents the model. | master02 L26, L28-30 | behaviour | low |

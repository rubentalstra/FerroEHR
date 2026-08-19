---
name: object-ref-resolvability-location
description: Where (and whether) the released specs say an OBJECT_REF / LINK / tag target must resolve to an existing object — the cross-component silence map
metadata:
  type: reference
---

OBJECT_REF **resolvability** (does a committed reference have to point at something that
exists?) — the cross-component navigation map. Short answer the text supports: NO released
clause requires resolvability anywhere; the only scope constraint that exists is on
`EHR.tags`.

Where the load-bearing sentences live:

- **BASE, the general statement** —
  `BASE/docs/UML/classes/org.openehr.base.base_types.object_ref.adoc` Description:
  "a reference to another object, which may exist locally or be maintained outside the
  current namespace, e.g. in another service". Prose companion:
  `BASE/docs/base_types/master05-identification_package.adoc §References` (L183-185, the
  primary-key/foreign-key analogy, "distributed referencing"); architecture framing in
  `BASE/docs/architecture_overview/master09-identification.adoc §Levels of Identification`
  (L51-93). **None of these state a resolution obligation.**
- **RM, per-holder attribute meanings + invariants** (the invariants constrain `.type`
  ONLY, never existence):
  - `RM/docs/UML/classes/org.openehr.rm.ehr.ehr.adoc` — invariants
    Contributions_valid / Ehr_access_valid / Ehr_status_valid / Compositions_valid /
    Directory_valid / Folders_valid / Directory_in_folders: all `is_equal("<TYPE>")` checks.
    Its `tags` Meaning is the ONE scope rule in the RM: "Tag `_target_` values can only be
    within the same EHR" (nothing equivalent for folders/compositions/directory).
  - `org.openehr.rm.common.folder.adoc` — `items` = "references to other (usually)
    versioned objects logically in this folder"; **empty invariant section**.
  - `org.openehr.rm.common.link.adoc` — LINK.type doc explicitly contemplates links
    "which must be followed and which can be broken when the extract is created" — the RM's
    clearest admission that a reference may not resolve.
  - `org.openehr.rm.common.item_tag.adoc` — only Inv_key_valid / Inv_value_valid.
- **RM, the nearest thing to a general integrity clause** —
  `RM/docs/ehr/master04-ehr_package.adoc §Change Control in the EHR` L159-163: "the record
  should always be in a consistent informational state" (vague, non-operationalized).
  §Folders L110: folders "may be _added or removed contemporaneously or subsequently_ to the
  committal of the referenced Compositions"; same-Contribution grouping is "normally", not
  a requirement.
- **RM, EHR Extract** — `RM/docs/ehr_extract/master09-semantics.adoc §Creation Semantics`
  L76-77: on export, "for each instance of `OBJECT_REF` encountered … obtain the target of
  the reference from the relevant service" and rewrite `_namespace_` = "local". This is the
  only algorithm anywhere that assumes resolvability, and it is an EXTRACT-BUILD step, not a
  commit-time rule.
- **SM** — `SM/docs/UML/classes/i_validity_checker.adoc` is the whole commit-time validation
  vocabulary: `definitions_valid` (archetype/template ids) + `content_valid` (RM-class
  validity). Neither mentions references. See [[directory-api-location]] for the
  `valid_content` vs `content_valid` naming defect.
- **ITS-REST** — grep-verified: no "referential", "resolvable", "dangling", "must exist"
  anywhere in `ITS-REST/specifications/docs/**`. The only refs-related clause is
  `docs/overview/Requests_and_responses.md §Prefer resolving Object references`
  (`Prefer: return=representation, resolve_refs`) — a READ-side option with no
  unresolvable-ref branch. `responses/422.yaml` scopes semantic errors to "the underlying
  template is not known or is not validating the supplied resource".

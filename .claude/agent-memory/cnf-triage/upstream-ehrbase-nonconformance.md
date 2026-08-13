---
name: upstream-ehrbase-nonconformance
description: upstream EHRbase 2.34.0 (comparison SUT) upstream divergences, all reproduced on the wire 2026-07-28 against the full catalogue
metadata:
  type: project
---

Reproduced first-hand against `docker/sut-ehrbase.yml` (2.34.0, declares
ITS-REST 1.0.3). Each is a divergence from a RELEASED source at or below the
party's own declared version unless marked otherwise. Never "fix" our
pack/catalogue toward any of these.

1. **Quoted `If-Match` → 400** "UUID string too large"; the unquoted value
   works. The double-quoted entity-tag form dates to Release-**1.0.2**
   (SPECITS-41) and is the overview §If-Match example, so this is floor-proof.
   Whole `precondition_missing` class (27 in-scope rows).
2. **`POST /definition/template/adl1.4` with `Accept: application/json` → 406**;
   only `application/xml` is served. Released OAS `Accept_Template.yaml` enums
   json/xml/`openehr.wt+json` and `201_Template_adl1_4_upload.yaml` declares an
   `application/json` (TemplateIdentifier) entry. Ground is the 1.1.0 OAS — the
   1.0.3 OAS is not vendored, so caveat the version.
3. **Semantic RM violations answered 400, not 422** (`Missing attribute
   EHR_STATUS/subject`). 422-for-semantic dates to Release-1.0.1 (SPECITS-38).
4. **RM-invalid EHR_STATUS accepted (201)**: missing `archetype_details` on an
   archetype root, and `other_details` with no `_type`. See
   [[ehr-status-archetype-root-invariant]].
5. **`openehr-audit-details` ignored on `POST /ehr`** (both the 1.1.0 spelling
   and the deprecated `openEHR-AUDIT_DETAILS`): committer stays the auth user
   and `description` comes back as `{"_type":"DV_TEXT"}` with NO `value` — an
   RM-invalid DV_TEXT. Overview §openehr-version and openehr-audit-details:
   "services MUST accept …" / "MUST be merged with the default VERSION and
   VERSION.audit_details attributes on commit runtime".
6. **Canonical XML is served UNNAMESPACED**: `<composition …
   xmlns:ns2="http://schemas.openehr.org/v1">` — the root is in no namespace and
   ns2 is unused, while `Composition.xsd` is `targetNamespace=…/v1`
   `elementFormDefault="qualified"`. §XML Format: responses "MUST conform to the
   [published XSDs]". NO catalogue case catches this (see
   [[amb167-documented-family-root-name-gap]]).
7. **`GET /definition/query` with `Accept: application/xml` → 200 `<List/>`**
   (Content-Type application/xml) — an XML document conforming to no published
   XSD, where the release offers JSON only on every query operation.
8. **Unqualified/dotted stored-query name → 400** (`cnf.ward_dashboard-probe`).
   Query `Qualified_query_name.md`: "The `namespace` is optional"; `query-name`
   pattern `[a-zA-Z0-9_.-]`; `my_compositions` is a listed VALID example.
9. **Stale `If-Match` on `DELETE /ehr/{id}/directory` → 404** (409 in the run),
   where §If-Match makes 412 a MUST. 412 responses carry no `ETag` (that part is
   only a SHOULD, so not gating).
10. **405 responses carry no `Allow`** (RFC 9110 §15.5.6 MUST; the overview
    status table incorporates RFC 9110).
11. **Malformed path uuid → 404**, catalogue expects 400 — check the OAS before
    calling this a divergence; not adjudicated.
12. **`VERSIONED_EHR_STATUS.owner_id.type` = `"ehr"`**, not the RM class name
    `EHR` (BASE `object_ref.adoc` §type: "class names are from the relevant
    reference model").
13. **500** on `commit_contribution` with invalid change types.
14. **Admin EHR delete served at `/ferroehr/rest/admin/ehr/{id}`** (204), 404 at
    the released `…/openehr/v1/admin/ehr/{id}` — but SPECITS-80 dates the
    operation to 1.1.0, so it is OUT OF SCOPE for a 1.0.3 party, not a finding.
15. **The query `ehr_id` scope is DISCARDED on both released carriers** (the
    `ehr_id` query parameter and the `openehr-ehr-id` header): 200 + a full
    population result, disclosed by EHRbase's own `meta._executed_aql`. Grounds +
    reproduction in [[ehr-id-scope-semantics-is-sm-grounded]]; two red rows
    (`execute_ad_hoc_query-empty_db_bare_ehr`, `-unknown_ehr_scope`).

**CORRECTION to the earlier note:** the item-tag 404s are NOT an upstream
non-conformance for this party — SPECITS-77 dates ITEM_TAGs to Release-**1.1.0**
([[its-rest-1-1-0-dated-surfaces]]), and the party declares 1.0.3. Same for the
Demographic API (SPECITS-70) and Simplified Formats (SPECITS-61).

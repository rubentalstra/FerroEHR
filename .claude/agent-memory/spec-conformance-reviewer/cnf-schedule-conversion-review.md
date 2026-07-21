---
name: cnf-schedule-conversion-review
description: Pitfalls when reviewing a CNF Platform Test Schedule → machine-readable-catalogue design (case-model completeness against master06-09/17.x)
metadata:
  type: feedback
---

When reviewing a design that converts the vendored CNF Platform Test Schedule
(`docs/specs/openehr/CNF/docs/platform_test_schedule/`) into a machine-readable
case model, verify these against the ACTUAL chapter text (not the design's own
extraction summary):

**Why:** designs tend to extract only the "easy" fleshed chapters
(master06 EHR create matrix, master04 upload_opt, master07 update_composition,
master17.3 DV_QUANTITY) and then assert a "lossless representability" design law
over ALL fleshed chapters — including master08 (CONTRIBUTION) and master09
(DIRECTORY) that were never mined. master08/09 carry the constructs a
step/matrix/decision-table model most often CANNOT express.

**How to apply — the high-risk constructs to test the case model against:**
- master08: a single `commit_contribution` call whose payload is a LIST of
  VERSIONs, each with its own change_type/lifecycle_state/category/validity, one
  aggregate accept/reject (transaction atomicity), and MULTIPLE returned version
  uids to assert. A `parameters.matrix` with `reset_per_row`/`single_pass`
  models repeated SEPARATE calls, not members of one payload; `decision_table`
  is single-instance content; scalar `capture` can't hold a version LIST. This
  is usually an UNREPRESENTABLE-GAP. Also mixed-RM-type version sets
  (COMPOSITION+EHR_STATUS+FOLDER in one CONTRIBUTION).
- master09: folder hierarchies + has_path (path,result) tables ARE expressible
  as fixtures+matrix, but watch for: no `directory` prerequisite type;
  get_directory_at_time before/between/after needs relative-time refs between
  captured commit times (no vocab for it); empty-vs-error ambiguity (F.1/G.1/L.1
  NOTEs) missing from the ambiguity register; has_path returns a scalar boolean
  the RM-body-oriented assertion vocab doesn't cleanly cover.
- master06 create_ehr-main: the 16-row table is VALID data-set class **1.b**
  (EHR_STATUS provided) only — the spec caption "class 1.a" at line 45
  contradicts its own list at line 40 (a spec defect). A "verbatim" pilot that
  encodes only the 16 rows DROPS class 1.a (no EHR_STATUS → server defaults),
  even if its test_purpose claims default behavior.
- Cross-reference ids in `verified_by`: check they exist in the real schedule
  (e.g. the empty-server OPT case is `get_opts-retrieve_all_no_opts`, NOT
  `get_opts-empty_server`).
- Simplified-formats MIME reject-lists: `simplified_formats/master02` defines
  ONLY `application/openehr.wt.flat+json` + `…wt.structured+json`. Legacy types
  (`openehr.nc.flat+json`, `openehr.tds2+xml`, `.schema+json`) are EHRbase/Better
  prior-art, NOT spec — asserting a server MUST reject them is not spec-grounded.

**What usually checks out (verify anyway):** the ITS-REST 1.1.0 OAS is
decomposed (`specifications/operations|responses|parameters/**`) with NO per-API
prose status tables; status sets ehr_create 201/400/409, composition
create 201/400/404/422, update 200/204/400/404/412/422, delete 204/400/404/409,
template adl1.4 upload 201/400/409 (Content-Type application/xml only), adhoc
query 200/400/408; If-Match `required: true`; weak-ETag W/ MUST
(Requests_and_responses.md); "Additional status codes MAY be used"; Error.yaml =
{message, validationErrors[string]} wired only to 400-family while prose example
uses {message, code, errors[DV_CODED_TEXT]} (a real intra-1.1.0 divergence);
fetch default implementation-defined + cannot combine with AQL TOP
(query/Request.md); ctx/setting default "other care"=238, ctx/time→now()
(simplified_formats master06).

---
name: etag-matcher-placeholder-refusal
description: RUNNER bin — #1865's loud refusal of unresolvable `<name>` matcher placeholders reddens 338 spec-correct cases; validate has no placeholder-resolvability gate
metadata:
  type: project
---

Veredictum's `src/exec/headers.rs::resolve_placeholders` (#1865, commit
907a22a87) refuses any `pattern:` placeholder that is neither a case variable nor
one of its TWO structural tokens (`<n>`, `<system_id>`), and books the refusal as
a law-(b) conformance FAILURE.

**Why that is the runner's defect, not the catalogue's** (confirmed 2026-08-04,
845 rows / 370 cases):

- 43 bindings spell `pattern:W/"<versioned_object_uid>::<system_id>::1"` — a
  SHAPE assertion the docs text requires (`Requests_and_responses.md` §ETag and
  Last-Modified, example `W/"…::openEHRSys.example.com::2"`). On a CREATE the
  object-id is server-assigned, and the binding declares
  `versioned_object_uid` as a capture OF THAT SAME response, so no case can
  supply it. The segment has a released grammar (BASE base_types master05
  §OBJECT_VERSION_ID/UID) exactly like the two tokens that ARE modelled — the
  vocabulary is incomplete, so the catalogue could not author it correctly.
- The merged `with:` scope also POISONS matchers: a step passing a full
  version uid as the `{versioned_object_uid}` PATH argument (spec-legal —
  `operations/composition_get.yaml` `uid_based_id`) resolves the ETag matcher to
  a doubled tail and fails a correct ETag.
- `veredictum validate` reports **0 findings** on the same tree: the parse-time
  probe (`model/binding.rs:226,354`) only compiles the pattern with placeholders
  wildcarded, never checks resolvability. The class is run-time-only.

**How to apply:** before attributing any `refusing the vacuous wildcard (#1852)`
row, reproduce the ETag on the wire — the SUT was conformant on composition
create/get/update, ad-hoc query, demographic create and ADL2 upload
(`W/"<uuid>::ferroehr.local::<n>"`, `W/"<ARCHETYPE_HRID>"`). Fix is runner-side;
never edit the bindings to dodge it.

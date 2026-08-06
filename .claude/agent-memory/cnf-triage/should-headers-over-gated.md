---
name: should-headers-over-gated
description: Last-Modified presence and the 412 ETag are SHOULDs in the ITS-REST overview, but many bindings declare them as bare gating matchers
metadata:
  type: project
---

`ITS-REST/specifications/docs/overview/Requests_and_responses.md`:

- §ETag and Last-Modified, closing sentence: "Both `ETag` and `Last-Modified`
  **SHOULD** be included in responses for VERSION, VERSIONED_OBJECT, or other
  resources that have versioning or unique state identifiers."
- §If-Match: on a false precondition the service "MUST respond with HTTP status
  code `412 Precondition Failed`, and **SHOULD** return also latest
  `version_uid` in the `ETag` response headers."

So on those two, PRESENCE is a SHOULD; only the FORM (`W/"…"`, 1.1.0-dated) is a
MUST. The runner supports exactly this via `HeaderExpectation.optional` (authored
`present?`). But ~20 bindings declare bare `Last-Modified: present` and the 412
outcomes declare bare `ETag: latest-version-uid` — gating assertions. That cost
8 in-scope red rows on the 2026-07-28 ehrbase record
(`I_EHR_DIRECTORY.delete_directory-*`, `get_directory*-deleted*`,
`I_EHR_STATUS.get_versioned_ehr_status-*`,
`I_EHR_DIRECTORY.update_directory-stale_if_match`).

**How to apply:** CATALOGUE bin. A header whose presence the released text makes
a SHOULD is authored `present?`; the FORM matcher (and its `applies` floor) stays
gating. Same family as [[query-etag-presence-is-should]] and
[[etag-weak-indicator-is-1-1-0-scoped]] — the floor/optionality is applied
inconsistently across bindings, so sweep, don't patch one file.

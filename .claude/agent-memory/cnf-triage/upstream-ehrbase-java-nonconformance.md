---
name: upstream-ehrbase-java-nonconformance
description: EHRbase Java 2.34.0 upstream (comparison SUT) non-conformances found reproducing #266 measured-window error classes
metadata:
  type: project
---

Reproducing #266's upstream measured error classes (EHRbase Java 2.34.0,
`docker/sut-ehrbase-java.yml`) confirmed two UPSTREAM non-conformances vs the
released ITS-REST 1.1.0 docs text — NOT our-side defects (our driver/pack are
spec-valid; the identical exchanges pass 100% on ehrbase-rs).

**1. Quoted If-Match → 400 "UUID string too large".** EHRbase rejects the
RFC-9110 / spec-standard double-quoted entity-tag `If-Match: "uid::system::N"`
(the form in `ITS-REST/specifications/docs/overview/Requests_and_responses.md`
line 207) and only accepts the non-standard UNquoted value. It even 400s on its
OWN returned ETag echoed back verbatim. This is the whole composition_update
(77/77) + ehr_status_update (26/26) 400 class. Our driver
(`perf_run/client.rs:120-122`) sends the quoted form = spec-correct.

**2. Item-tag surface unimplemented → 404 "No resource found at path".**
`/ehr/{id}/tags`, `/ehr/{id}/composition/{uid}/tags` (GET+PUT) are part of the
STABLE EHR API (`ehr.openapi.yaml` x-status: STABLE, L5; paths L95-113) but
EHRbase 2.34.0 has no routes. The tags_put (34/34) + tags_read (34/34) 404 class.
Released-STABLE, so upstream non-conformance, NOT N/A.

**template_get 3/85 = transient** (endpoint returns 200 JSON+XML; load-window
noise), not deterministic, no defect.

Disposition for all: outcome (b) — measured record stands; note upstream
non-conformances in the public comparison narrative. NEVER "fix" our pack/driver
to match upstream (that would break the spec-correct ehrbase-rs path).

**Why: `If-Match` echoing the server ETag verbatim (quoted) is the textbook
RFC flow; deriving the expectation from the released docs text (not the SUT)
made this an upstream verdict, not a pack fix.**
**How to apply:** when a comparison SUT 400s a versioned PUT, check If-Match
quoting before suspecting the payload; when it 404s item tags, it's a
STABLE-surface gap.

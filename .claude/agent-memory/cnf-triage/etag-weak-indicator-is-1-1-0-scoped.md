---
name: etag-weak-indicator-is-1-1-0-scoped
description: The ETag W/ weakness indicator is a MUST introduced BY ITS-REST 1.1.0 — asserting it against a party declaring its_rest 1.0.3 needs an applies floor, and the catalogue applies that floor inconsistently
metadata:
  type: project
---

`ITS-REST/specifications/docs/overview/Requests_and_responses.md`:

- §Deprecated headers (L63-64): "The `ETag` response header was used without a
  weakness indicator `W/`. This is now deprecated, all `ETag` headers that hold
  a resource identifier MUST include a weakness indicator `W/`." — a MUST.
- §ETag and Last-Modified (L174, the DEPRECATION block): "**Prior to Release
  1.1.0** … the `ETag` header was used without a weakness indicator `W/`. This
  usage is now deprecated, but implementations MAY still support it **alongside**
  the updated header format" + L185 "Servers MAY add **additional** `ETag`
  response headers".

Reading that keeps both operative: the identifier-bearing ETag MUST be `W/`-
prefixed; a strong tag is only permitted as an ADDITIONAL header. A strong-only
ETag is non-conformant **to 1.1.0** — and conformant to 1.0.3, which the text
itself says.

**Therefore the assertion is version-scoped and needs `applies: { its_rest:
">=1.1.0" }` on the case.** The catalogue applies that floor inconsistently: in
the 2026-07-28 ehrbase-java run the SAME `pattern:W/"…"` matcher produced 141
in-verdict-scope failures (cases with no its_rest floor) and 108 out-of-scope
ones (cases carrying the floor). The matcher lives in the BINDING outcome
headers, so it rides into every case realizing the operation regardless of the
case's floor.

**Presence vs form:** presence of `ETag` is only SHOULD (§ETag and Last-Modified:
"Both `ETag` and `Last-Modified` SHOULD be included in responses for VERSION,
VERSIONED_OBJECT, or other resources that have versioning or unique state
identifiers"); the `W/` MUST is conditional on the header being emitted. A
`pattern:` matcher asserts BOTH — see [[query-etag-presence-is-should]].

**How to apply:** before attributing an ETag row, split form (MUST, 1.1.0+) from
presence (SHOULD, always) and check the case's `applies` floor against the
party's `spec_versions.its_rest`. Same shape for `Location` on GET/DELETE
(§Location L136/L140 "MUST NOT … via `GET`" / "MUST ONLY be used for resource
creation … or redirect") — also 1.1.0-dated by its own DEPRECATION block.

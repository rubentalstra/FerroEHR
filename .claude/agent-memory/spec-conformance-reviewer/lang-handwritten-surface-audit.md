---
name: lang-handwritten-surface-audit
description: openehr-lang hand-written *_impl/parser audit (#2256) — citation hygiene is genuinely high; the real defects are silent-default conversions and doc-comment NOTE essays
metadata:
  type: project
---

Audited 2026-08-11 (issue #2256), ~46 hand-written files under
`crates/openehr-lang/src/{v1_0,v1_1}/`.

**Do NOT go hunting for fabricated quotations here.** A normalized-text sweep
of every ≥35-char quoted phrase in the crate's comments against the vendored
LANG+AM adoc corpus + the vendored `.g4` grammars found ZERO fabrications;
every apparent miss was an elision (`…`), asciidoc markup, or a quote from a
non-vendored-but-legitimate source (BASE foundation_types, the openEHR
release-strategy page, RFC 2781/3986). This crate is materially cleaner than
`openehr-base`/`openehr-rm` on that axis.

**The recurring real defect classes:**
1. **Silent typed defaults in the P_BMM→BMM transforms.**
   `bmm_persistence/create_bmm3_model.rs` turns a non-integer or
   out-of-`i32` enumeration item value into `value: 0`
   (`.and_then(|v| i32::try_from(v).ok()).unwrap_or_default()`), and
   `check_enumeration_validity` (`create_model.rs`) checks only ancestor
   count + list 1:1, never item-value TYPE.
2. **`.chars().next().unwrap_or('\u{fffd}')`** in all four
   `decode_char` twins (v1_0/v1_1 odin, bel, el) — unreachable, but the
   exact shape `reliability.md` bans beside a `#[expect(expect_used)]`.
3. **`// NOTE:` essays relocated into `///` doc comments.**
   `scripts/checks/comment-style.sh` applies the 3-line NOTE budget ONLY to
   `is_line` (`//`) comments, never to doc comments — so 15–20-line
   `/// NOTE (adjudicated …)` blocks pass the guard. Review-enforced by
   `comments.md`; this crate is where they cluster.

**Two spec-internal facts worth remembering:**
- `conformance_type_name` was REMOVED from `BMM_CLASSIFIER` by amendment
  2.2.1 (`LANG/docs/bmm/master00-amendment_record.adoc`) yet
  `master05-core.adoc` §Basics still describes it as a `BMM_CLASSIFIER`
  feature; only `BMM_OPEN_TYPE` declares it in a §Functions table.
- `BMM_GENERIC_TYPE.is_open` is NAMED for openness but DEFINED as closure
  ("True if all generic parameters … have been substituted"), and
  `master06-core-types.adoc` §Generic Type L81 calls the same property
  `is_closed` — a function no class declares. Unregistered upstream (no
  `upstream-report` issue, no `ambiguities.yaml` row) as of 2026-08-11.

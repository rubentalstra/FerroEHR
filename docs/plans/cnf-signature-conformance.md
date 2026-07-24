# CNF version-signature conformance — design (Bucket 3 of #231)

*Working plan (delete-on-implementation, linked from #231). The durable
framework-normative pin (§ "The agreed canonical form") is destined for
`docs/conformance/cnf-design.md` §8.15 — that file is under active owner
rewrite, so this plan states the pin precisely for the owner to fold in; this
plan does NOT edit cnf-design.md.*

## Framing — a portable, deterministic capability

The CNF framework is language-agnostic: the catalogue (SM cases), the ITS-REST
operation bindings, the corpus, the vocabularies, the ambiguity register, and
the IXIT/statement schemas are the **portable artifacts** that define
conformance against the openEHR spec; the Rust `cnf-runner` is only the
**reference interpreter**. Version-signature conformance is therefore expressed
entirely at that portable layer so any openEHR CDR — in any language — is tested
by the identical catalogue. No expectation is ever derived from what our SUT
emits (`.claude/rules/cnf-triage.md`).

## The agreed canonical form (framework-normative — the "agreed format" openEHR defers to the ecosystem)

openEHR RM common `master06-change_control_package.adoc` §Digital Signature
says the Version is "serialised into canonical form" over "all attributes
except signature", then hashed/signed — but leaves the exact serialization
"an agreed XML, ODIN or other text format", i.e. **implementation-defined for
the JSON ITS**. A cross-implementation `verifiable` conformance point requires
that agreement, so the framework pins it:

> **The signed canonical form is RFC 8785 (JCS) of the `ORIGINAL_VERSION`
> ITS-JSON representation with the `signature` member removed, UTF-8 bytes.**

- Spec basis: master06 §Digital Signature + RM common `version.adoc`
  (`canonical_form`: "all attributes except signature") + ITS-JSON (the
  canonical JSON representation) + RFC 8785 / JCS (deterministic bytes so any
  language reproduces them identically).
- This is a framework-normative pin, NOT a spec silence — it supplies the
  ecosystem agreement openEHR explicitly defers. (Register it as a cited
  framework decision, not an `ambiguities.yaml` disposition.)
- Reference-SUT note: `openehr_rm::common::change_control::version_impl::
  canonical_form_of_json` already computes exactly this
  (`serde_jcs::to_string` of the version minus `signature`), so the reference
  SUT conforms by construction — verification is a pure recompute/verify, no
  app change.

## Enabling capability — the version-envelope read (general, not signature-only)

A specific version's `ORIGINAL_VERSION` envelope (carrying `signature`,
`commit_audit`, `lifecycle_state`, `data`, `uid`) is read via
`GET /ehr/{ehr_id}/versioned_composition/{vo_uid}/version/{version_uid}`
(confirmed live: returns `_type: ORIGINAL_VERSION` + all of the above). Every
ITS-REST CDR exposes it. Adding its SM op + binding unblocks **both** the
`signature` family **and** the `version` family (`change_type` /
`lifecycle_state` / `uid_pattern`) — both are silently no-op'd in the driver
today (`driver.rs:547`), so the SIG-VERSION cases currently pass without
verifying anything (false-green; not even registered exceptions). This is the
coverage-mandate defect to close (`.claude/rules/testing.md` §CNF coverage).

Binding captures: `signature`, `commit_audit.change_type`, `lifecycle_state`,
`preceding_version_uid`, and the full envelope body (for canonical-form
reconstruction).

## IXIT signing posture (per-SUT, portable)

The IXIT declares the SUT's signing posture; the framework tests whatever is
declared:

```
signing:
  enabled: bool
  mode: digest | pgp
  # digest mode (self-description of the plain-digest wire form):
  digest_algorithm: sha256        # e.g.
  digest_encoding: base64
  digest_prefix: "sha256:"        # engine self-description prefix, or ""
  # pgp mode:
  public_key: <RFC 4880 armored public key>
```

Reference SUT (digest mode): `{ enabled: true, mode: digest,
digest_algorithm: sha256, digest_encoding: base64, digest_prefix: "sha256:" }`.

## Assertion evaluation (deterministic, spec-derived; against the envelope read)

- **present** — `.signature` is non-empty. Mode-agnostic (digest and pgp).
- **equals** — the stored `.signature` equals a client-supplied value verbatim
  (master06 Copying / `IMPORTED_VERSION` carries its own signature).
- **verifiable** —
  1. reconstruct `canonical = JCS(envelope with "signature" removed)`;
  2. **digest mode**: recompute `digest_algorithm(canonical)`, apply
     `digest_encoding` + `digest_prefix`, compare to `.signature`;
  3. **pgp mode**: verify the RFC 4880 detached signature `.signature` over the
     `canonical` bytes against the IXIT `public_key`.
- **version facts** (`change_type` / `lifecycle_state` / `uid_pattern`) —
  evaluated against the same envelope (closes the version-family no-op too).

Evaluation runs in the verification pass over a version-read STEP the case's
own flow provides (the existing `driver.rs:547` "in-case verification" seam),
so it is wire-observed and deterministic.

## Dual-mode run

`present` / `equals` / `verifiable` hold under whichever mode the IXIT
declares. Validating BOTH modes = one run per mode: the default compose stack
(digest) + a pgp-configured compose/IXIT variant. Mode-specific cases are
guarded by the declared mode (the other cited-N/A, like the existing `Signing`
capability guard).

## Coverage (the mandate — each behaviour its own isolated case)

- `present` on a creation VERSION;
- `present` across version kinds (creation + modification + deletion);
- `verifiable` (digest + pgp);
- `equals` (client-supplied / imported signature stored verbatim);
each isolated, both modes, `Signing`-capability-guarded. Closes the current
false-greens.

## Implementation plan

1. **Binding** — `artifacts/bindings/its-rest/
   I_EHR_COMPOSITION.get_versioned_composition_version.yaml` (path, captures,
   OAS citation). Directory/party analogs only if in scope.
2. **Runner** (`tools/cnf-runner/src`) — evaluate `Signature` + `Version` in
   the verification pass against the version-read step's captured envelope; a
   `verify` module (JCS reconstruct + digest recompute + RFC 4880 verify) keyed
   off the IXIT `signing` posture; IXIT `signing`-block parsing.
3. **IXIT + schema** — the `signing` block in
   `party/ehrbase-rs/ixit.json` + `schemas/ixit.schema.json`.
4. **Cases** — `schedule/security/SIG-VERSION-*.yaml`: add the version-read
   step; wire-assert `present` / `equals` / `verifiable`; guard by `Signing`.
5. **Register / citation** — record the agreed-canonical-form pin as a
   spec-cited framework decision (NOT a spec-silence disposition).
6. **cnf-design.md §8.15** — the durable pin text, handed to the owner's active
   rewrite (not edited here).

## Verification

Live run in **both** modes: digest (default compose) + pgp (key-configured
variant). SIG-VERSION cases wire-assert (no longer no-op); the false-greens /
coverage exceptions close; validator green; workspace gates green.

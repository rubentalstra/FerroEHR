# ECC upstream fairness adjudication registers

These committed TOML files (`<sut>.toml`) let the ECC runner report a fair
verdict for a **non-`ehrbase-rs`** SUT. The ECC catalogue is *our own*
instrument, authored against the pinned specs (RM 1.2.0, ITS-REST
Release-1.1.0) with adjudicated skips for *our* server; running it
unmodified against another product would unfairly fail it on version skew and
on our own extensions. A register **only reclassifies with a citation — it
never edits, weakens, or skips a case** (honesty rule 10 / standing rule 3).

A register is loaded with `--adjudications <file>` and applied **only** when the
SUT product is not `ehrbase-rs` (set with `--sut-name`). For an `ehrbase-rs`
SUT the register is ignored, so our own conformance baseline is never touched
(the zero-drift gate). Running with no register is today's behaviour,
byte-for-byte.

## Dispositions

| Disposition | Effect | When |
|---|---|---|
| `extension` | case → **NotApplicable** (excluded from pass/fail + capability math) | the case exercises an `ehrbase-rs` extension the SUT does not implement (demographic REST API, version signing, our TERMINOLOGY() AQL family, `/terminology`) |
| `rm-version-sensitive` | case → **NotApplicable** | the request payload or response comparison depends on RM 1.2.0 shapes the SUT's older RM/ITS surface cannot be expected to produce |
| `defect` | case **runs**; its natural outcome (a failure) **stands** | a genuine spec gap that survived triage — reported plainly with the spec citation |

Every entry carries a **non-empty `reason` and `citation`** — the loader
rejects a register that omits either.

## File format

```toml
[meta]
sut = "ehrbase-java"            # informational; the authoritative product identity is in results.json
version = "2.34.0"
description = "one line"

# Area-wide rule: every case in the ECC area gets this disposition.
# `area` is the ECC area tag (uppercase): EHR STA COM CTB DIR TPL SQR QRY VAL
#   REST DEM ADM SEC SIG MSG TS  (see `catalog::Area::tag`).
[[area]]
area = "DEM"
disposition = "extension"
reason = "why this reclassification is fair"
citation = "docs/…#… (a spec/research citation)"

# Per-case rule (keyed by ECC id) — wins over an area-wide rule for that case.
[[case]]
ecc_id = "ECC-QRY-014"
disposition = "rm-version-sensitive"
reason = "…"
citation = "docs/VERSIONS.md §RM-version divergence"
```

## Populating a register (the X1.2 triage process)

The register is grown from a **real triage run**, never guessed:

1. Run ECC against the upstream SUT (`--sut-name`, `--admin-base-url`, no
   register yet). Every failure is a candidate.
2. Triage each failure into exactly one disposition with a citation, or fix a
   genuine runner-tolerance bug (the register never routes around a runner
   defect).
3. Commit the register; the *second* run is the published one.

`extension` entries for whole areas the SUT structurally lacks (e.g. upstream
EHRbase has no demographic REST API, and version signing is an `ehrbase-rs`
feature) are the only entries knowable **without** a run — they are seeded here.
`rm-version-sensitive` and `defect` entries require the triage run and are added
in X1.2.

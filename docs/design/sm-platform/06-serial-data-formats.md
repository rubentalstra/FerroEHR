# SM digest 3/3 — Serial Data Formats (SDF)

Part of the SM-platform design set (`docs/design/sm-platform/README.md`).
Source: `docs/specs/openehr/SM/docs/serial_data_formats/` (all files read in
full). Spec status: **DEVELOPMENT**, SM Release 1.0.0 (unreleased), latest
issue 0.5.0 (SPECSM-5, 2020-08-27, "adapted from the Better Platform 'Web
Templates' specification and EtherCIS documentation").

## 1. What this spec is

"This document describes the Serial data formats, a set of serialisations for
openEHR RM data, intended to be used to generate simplified Data Templates."
(`master01-preface.adoc` §Purpose.) SDF defines compact serialisations for the
openEHR Foundation Types and the RM data types, to (a) reduce size / increase
human-readability of JSON-serialised openEHR data and (b) make generation of
openEHR data easier (`master02-overview.adoc` §Design Concept).

It is the normative openEHR standardisation *seed* of the Better
"web-template"/EhrScape FLAT wire conventions (Original-IP note in
`master.adoc` §Acknowledgements). Vendor behaviour is captured per type as
"EhrScape Variants".

## 2. Normative content (what IS defined)

### 2.1 JSON-native primitives (`master03-data_values.adoc` §JSON Primitives)

| Type | Form | Examples |
|---|---|---|
| `Boolean` | JSON `true`/`false` | |
| `Integer` | JSON integer | `5`, `124000` |
| `Real` | JSON number | `5.0`, `6.023e23` |
| `String` | JSON string | `"mild anaemia"` |

### 2.2 Primitives as JSON strings (§openEHR Primitives Represented as JSON String)

| Type | Form | Examples |
|---|---|---|
| `Character` | JSON 1-char string | `"k"`, `"\t"` |
| `Iso8601_date` | ISO 8601 date incl. partial (`valid_iso8601_date()`) | `"2020-04-01"`, `"2020-04"`, `"2020"` |
| `Iso8601_time` | ISO 8601 time incl. partial | `"13:45:00"`, `"13"`, `"13:45:00.722+03:00"` |
| `Iso8601_date_time` | ISO 8601 date-time incl. partial | (spec's own examples are erroneously duration strings — source defect, see §4) |
| `Iso8601_duration` | ISO 8601 duration | `"P2Y4M10D"`, `"P1DT3H"`, `"PT2h5m0s"` |
| `Uri` | RFC 3986 | `"https://www.openEHR.org"` |
| `Terminology_code` | ODIN `TERM_CODE_REF`: `"[<terminology_id>::<code>]"` \| `"[<terminology_id>(<version_id>)::<code>]"` | `"[icd10AM::F60.1]"`, `"[snomed_ct(2020_06_01)::3415004]"` |
| `Terminology_term` | `"[<terminology_id>::<code>\|<text>\|]"` (text pipe-delimited inside brackets) | `"[icd10AM::F60.1\|Schizoid personality disorder\|]"` |

Open TBD in the spec: version_id support in `Terminology_code`/`Terminology_term`
and a "value" in `Terminology_term`.

EhrScape variant (object form, pipe-prefixed keys):
`Terminology_code` → `{"|code": "238", "|terminology": "openehr"}`;
`Terminology_term` adds `"|value"`.

### 2.3 Interval strings (§openEHR Intervals Represented as JSON String)

`Interval<T: Ordered>` → ODIN interval strings:

```
|N .. M|      |> N .. M|     |N .. <M|     |> N .. <M|
|< N|         |> N|          |>= N|        |<= N|
|N +/-M|      |N±M|
```

Examples: `|0 .. 5|`, `|0.0 .. <1000.0|`, `|08:02 .. 09:10|`,
`|>= 1939-02-01|`, `|5.0 ±0.5|`.

### 2.4 Lists (§Lists of Primitive Type and Intervals)

Standard JSON arrays of the above encodings, e.g.
`["[icd10AM::F60.1]", "[icd10AM::F64.2]"]`.

### 2.5 DV types mapped 1:1 to a foundation encoding (§RM DATA_VALUE Types, table 1)

`DV_BOOLEAN`→Boolean, `CODE_PHRASE`→Terminology_code, `DV_TEXT`→String,
`DV_CODED_TEXT`→Terminology_term, `DV_COUNT`→Integer, `DV_DATE`/`DV_DATE_TIME`/
`DV_TIME`/`DV_DURATION`→ISO strings, `DV_EHR_URI`→Uri. Instantiation to the
correct RM type relies on the model context (the template), not the wire.

### 2.6 DV types with specific SDF string forms (table 2)

| Type | Form | Example |
|---|---|---|
| `DV_ORDINAL` | `"<ordinal_value>\|<terminology_code-or-term>"` | `"1\|[snomed_ct::313267000\|Stroke\|]"` |
| `DV_SCALE` | `"<scale_value>\|<terminology_code-or-term>"` (real value) | `"1.5\|[snomed_ct::127840596\|minor difficulty\|]"` |
| `DV_QUANTITY` | `"<value>,<unit>"` (**comma**, singular unit) | `"78.500,kg"` |
| `DV_PROPORTION` | `"<numerator>/<denominator>;<kind>"`, kind ∈ RATIO\|UNITARY\|PERCENT\|FRACTION\|INTEGER_FRACTION | `"25.3/100;PERCENT"` |
| `DV_MULTIMEDIA` | JSON object (`integrityCheckAlgorithm`, `mediaType`, `compressionAlgorithm`, `uri`) | |
| `DV_IDENTIFIER` | JSON object (`id`, `issuer`, `assigner`, `type`) | |
| `DV_PARSABLE` | **`CHECK` — undefined in spec** | |
| `DV_INTERVAL<T>` | **`CHECK` — undefined in spec** (generic interval string exists, binding does not) | |

EhrScape variants: same objects with `|`-prefixed keys; `DV_PARSABLE`
EhrScape form is `{"|value": ..., "|formalism": ...}`.

### 2.7 Syntax chapter (`master04-syntax.adoc`)

Defines only: (a) a citation of the standard JSON.org EBNF grammar, verbatim
and unmodified, and (b) a "second-level string parser" for the SDF string
encodings whose body is literally `TBD: define` (ANTLR4 files also `TBD`).

## 3. What is NOT in this spec (load-bearing absences)

Confirmed absent from the vendored text — the design must not pretend the SDF
spec provides these:

1. **No flat-path / key syntax at all.** No segment grammar, no `:n` indexing,
   no node counting, no `ctx/` prefixes, no flat-vs-structured distinction.
2. **No MIME types, no format tokens, no format-naming scheme.**
3. `DV_PARSABLE`, `DV_INTERVAL<T>` canonical forms = `CHECK`.
4. **`DV_QUANTITY` is `"value,unit"` (comma)** — the `|magnitude`/`|unit`
   suffix vocabulary of Better/EHRbase FLAT is *not* defined here.
5. §Variant forms and Rules for Mixing, §EhrScape Format, §Parsing Model:
   all `TBD: xxxx`.
6. Conformance + Tooling sections: `TBD`.
7. No explicit textual binding to `simplified_im_b` (the linkage is implicit
   via "simplified Data Templates").

## 4. Design consequences for ehrbase-rs

- **The FLAT/STRUCTURED path layer stays grounded in the Better `web-template`
  prior art** (as `openehr-flat` already is, per `PORT_MASTER_PLAN.md` §7.4):
  the SDF spec simply does not define it. Record per-construct decisions with
  `// PORT NOTE:` + SDF citation where SDF *does* speak.
- **Where SDF is normative (the leaf-value string encodings, interval strings,
  ordinal/scale/quantity/proportion forms), `openehr-flat` should accept the
  SDF forms** in addition to the Better forms, and the divergences
  (`value,unit` vs `|magnitude`/`|unit`) must be documented in the converter.
- The `Iso8601_date_time` example defect and the two `CHECK` types are spec
  defects to track upstream; do not invent semantics for them silently.
- CNF/ECC note: FLAT is not conformance-gated (CNF tests OPT provisioning +
  canonical JSON/XML only), so SDF alignment is interop quality, not a
  conformance blocker.

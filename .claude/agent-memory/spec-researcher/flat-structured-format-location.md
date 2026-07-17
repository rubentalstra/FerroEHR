---
name: flat-structured-format-location
description: Where the FLAT / STRUCTURED (Simplified Formats / SDT) spec lives, and the authority split between ITS-REST and SM
metadata:
  type: reference
---

# FLAT / STRUCTURED (simSDT / structSDT) spec locations

**Authoritative wire format = ITS-REST `simplified_formats/` (STABLE).**
- `docs/specs/openehr/ITS-REST/docs/simplified_formats/`
  - `master02-overview.adoc` — MIME types (`application/openehr.wt.flat+json`, `application/openehr.wt.structured+json`); scope
  - `master03-design_rationale.adoc` — history (Better WT, EtherCIS ECISFLAT, Ocean TDS), full flat/structured examples
  - `master04-basic_concepts.adoc` — **path syntax**: node-id generation algorithm, `:i` indices, `|attr` suffixes, `_`-prefix RM attrs, `|raw`, flat vs structured syntax rules, **Level Removal** (elided container attrs + collapsed wrapper types incl. conditional EVENT collapse), validation, `|other` open-value-set rule
  - `master05-rm_mapping.adoc` (3216 lines) — **per-RM-type flat mapping tables** (every DV_*, PARTY_*, COMPOSITION/ENTRY/ELEMENT/CLUSTER, etc.), one `[cols=5*]` table per class with Flat Path / Flat type / RM Path / Required / Note
  - `master06-context_information.adoc` — full **`ctx/` vocabulary** with RM landing sites + defaults

**SDT** (`ITS-REST/docs/simplified_data_template/`) = **RETIRED** at ITS-REST Release 1.1.0, superseded by Simplified Formats. Only preface/amendment stubs remain.

**SM side = abstract model + rules, DEVELOPMENT status (many TBD):**
- `SM/docs/simplified_im_b/` (SIM-B, DEVELOPMENT) — the `S_XXX` class model; `master07-transformation_rules.adoc` = RM↔SIM path/collapse/copy rule tables; most chapters are `include::` of UML class .adoc stubs
- `SM/docs/serial_data_formats/` (SDF, DEVELOPMENT, many TBD) — `master03-data_values.adoc` = terse leaf encodings (`"125 mm[Hg]"`, `"1|[snomed_ct::..|..|]"`, ODIN intervals) + EhrScape variants; `master04-syntax.adoc` = JSON EBNF only, string parser TBD

**Disagreements/overlap:** SDF terse `DV_QUANTITY` = `"<value>,<unit>"` and ordinal/interval string forms are NOT what ITS-REST simplified_formats uses (ITS-REST uses `|magnitude`/`|unit` suffixes). ITS-REST simplified_formats is the STABLE authority for the REST wire; SM SIM-B/SDF are DEVELOPMENT and abstract. `SM_MASTER` component-map row (README line 33) mislabels SM as "SDT: FLAT/STRUCTURED semantics — P14/P17 authority".

**CNF:** no schedule prose; only the **legacy Robot suite** (`CNF/tests/platform/robot/_resources/keywords/composition_keywords.robot` ~L268-390 + `test_data_sets/compositions/FLAT/`) — EHRbase/EhrScape prior art, FLAT commit uses `Content-Type: application/json` at `composition?format=FLAT&ehrId=&templateId=` returning `{compositionUid}`, STRUCTURED uses `application/openehr.wt.structured+json`. Not the ECC (our own framework).

# Control matrix

FerroEHR is software. It is not a controller, not a processor and not a
certified organisation, so this page makes no compliance claim on anyone's
behalf. It lists the technical controls the product ships or plans, and the
article or clause each one is designed to support. Whether a deployment
satisfies a legal obligation depends on how the deploying organisation runs
it.

Every row comes from the tracker. A control is declared on the issue that
delivers it, as a line in the issue body:

```text
Control: <legal source> <article or clause>
```

The short name resolves to an official publisher URL from a registry inside
the generator, so a legal citation on this page is never free text. An issue
may declare several controls, one per line.

## How this page is built

`scripts/render/control-matrix.sh` queries the tracker with the GitHub CLI,
joins each declared control to its legal source and to its current state, and
writes this file. A CI job re-runs the generator with `--check` and fails the
build when the committed page no longer matches the tracker, which is what
keeps a shipped control from sitting here as "planned".

- **Shipped:** the issue is closed as completed. The merged pull request that
  closed it is linked in the last column. The one pull request that closes a
  control regenerates this page as it will read after the merge, so the page
  never lags a shipped control.
- **Planned:** the issue is open. Whether work has started is the issue's
  column on the [public roadmap board](https://github.com/users/rubentalstra/projects/4),
  which this page does not copy: a status that lives in two places disagrees
  the day one of them moves.
- **Not planned:** the issue was closed without the control being built. The
  row stays visible so the record does not quietly lose it.

The page carries no generation timestamp and no build commit. Both change on
every run or every push while the tracker has not moved, which would make the
CI staleness check fail on days when nothing was wrong. When this page was
last regenerated, and from which commit, is the file's own git history.

## Controls

| Legal source | Applies to | Article or clause | Control | Issue | Status | Closing PR |
|---|---|---|---|---|---|---|
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 15(1) | A natural person cannot obtain their own access log: the only retrieval is admin-gated over the whole repository | [#3240](https://github.com/rubentalstra/FerroEHR/issues/3240) | Planned | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 17(1) | physical_delete_party leaks externalized multimedia blobs | [#3180](https://github.com/rubentalstra/FerroEHR/issues/3180) | Shipped | [#3201](https://github.com/rubentalstra/FerroEHR/pull/3201) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 25(1) | Contributor rules for personal data handling and a PR guard for privacy-boundary changes | [#3167](https://github.com/rubentalstra/FerroEHR/issues/3167) | Shipped | [#3172](https://github.com/rubentalstra/FerroEHR/pull/3172) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 25(2) | Refuse identifying data on the clinical side and constrain the subject reference to a pseudonym namespace | [#3154](https://github.com/rubentalstra/FerroEHR/issues/3154) | Shipped | [#3187](https://github.com/rubentalstra/FerroEHR/pull/3187) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 25(2) | The compliance layer assumes the Netherlands; it must be jurisdiction-pluggable | [#3185](https://github.com/rubentalstra/FerroEHR/issues/3185) | Shipped | [#3187](https://github.com/rubentalstra/FerroEHR/pull/3187) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 25(2) | The identifier scanner does not run on verbatim-replay writes (EHR-Extract import, admin load), and the book says every clinical write is scanned | [#3237](https://github.com/rubentalstra/FerroEHR/issues/3237) | Shipped | [#3251](https://github.com/rubentalstra/FerroEHR/pull/3251) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 25(2) | No database constraint holds the subject-reference shape: the CHECK #3154 promised was never built | [#3241](https://github.com/rubentalstra/FerroEHR/issues/3241) | Planned | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1) | Deploy artifacts do not provision the demographic role split | [#3179](https://github.com/rubentalstra/FerroEHR/issues/3179) | Shipped | [#3193](https://github.com/rubentalstra/FerroEHR/pull/3193) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1) | demographic.national_identifier has no row-level security policy | [#3219](https://github.com/rubentalstra/FerroEHR/issues/3219) | Shipped | [#3223](https://github.com/rubentalstra/FerroEHR/pull/3223) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1) | The documented two-DSN posture cannot boot: schema preparation runs as the clinical runtime credential | [#3224](https://github.com/rubentalstra/FerroEHR/issues/3224) | Shipped | [#3229](https://github.com/rubentalstra/FerroEHR/pull/3229) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1) | A declared deployment profile: production refuses the postures it cannot prove, research says so in red | [#3226](https://github.com/rubentalstra/FerroEHR/issues/3226) | Planned | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1)(a) | Split demographic parties into a `demographic` schema with a non-overlapping runtime role | [#3153](https://github.com/rubentalstra/FerroEHR/issues/3153) | Shipped | [#3182](https://github.com/rubentalstra/FerroEHR/pull/3182) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1)(a) | National identifiers in the demographic schema: encrypted storage, keyed lookup, audited resolution | [#3155](https://github.com/rubentalstra/FerroEHR/issues/3155) | Shipped | [#3189](https://github.com/rubentalstra/FerroEHR/pull/3189) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1)(c) | Separate encryption keys and per-schema backup handling for the clinical and demographic domains | [#3157](https://github.com/rubentalstra/FerroEHR/issues/3157) | Shipped | [#3198](https://github.com/rubentalstra/FerroEHR/pull/3198) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1)(c) | Nothing backs up the linkage schema, and no probe covers its boundary | [#3220](https://github.com/rubentalstra/FerroEHR/issues/3220) | Shipped | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1)(d) | The storage-parity sweep and node rebuild do not cover the demographic domain | [#3178](https://github.com/rubentalstra/FerroEHR/issues/3178) | Shipped | [#3184](https://github.com/rubentalstra/FerroEHR/pull/3184) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 32(1)(d) | Nothing tests the two-DSN posture: credential separation is documented, configured and unexercised | [#3222](https://github.com/rubentalstra/FerroEHR/issues/3222) | Shipped | [#3231](https://github.com/rubentalstra/FerroEHR/pull/3231) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 4(5) | Split demographic parties into a `demographic` schema with a non-overlapping runtime role | [#3153](https://github.com/rubentalstra/FerroEHR/issues/3153) | Shipped | [#3182](https://github.com/rubentalstra/FerroEHR/pull/3182) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 4(5) | Refuse identifying data on the clinical side and constrain the subject reference to a pseudonym namespace | [#3154](https://github.com/rubentalstra/FerroEHR/issues/3154) | Shipped | [#3187](https://github.com/rubentalstra/FerroEHR/pull/3187) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 4(5) | Separate encryption keys and per-schema backup handling for the clinical and demographic domains | [#3157](https://github.com/rubentalstra/FerroEHR/issues/3157) | Shipped | [#3198](https://github.com/rubentalstra/FerroEHR/pull/3198) |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 4(5) | Linkage service: the party to EHR resolve map as its own schema and role | [#3158](https://github.com/rubentalstra/FerroEHR/issues/3158) | Shipped | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 4(5) | The subject pseudonym is opt-in: nothing mints it, and an empty subject_namespaces is silent | [#3232](https://github.com/rubentalstra/FerroEHR/issues/3232) | Planned | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 5(1)(b) | Cross-domain cohort queries with a demographic predicate and a clinical selection | [#3159](https://github.com/rubentalstra/FerroEHR/issues/3159) | Planned | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 5(1)(e) | Audit retention has no jurisdictional floor: retention_days=1 erases the access log daily while the chain still verifies | [#3242](https://github.com/rubentalstra/FerroEHR/issues/3242) | Planned | — |
| [GDPR](https://eur-lex.europa.eu/eli/reg/2016/679/oj) | EU | Art. 89(1) | Secondary-use read model as a separate pseudonymisation domain fed from the outbox | [#3160](https://github.com/rubentalstra/FerroEHR/issues/3160) | Planned | — |
| [EHDS](https://eur-lex.europa.eu/eli/reg/2025/327/oj) | EU | Annex II 3.2 | European logging software component: map EHDS logging requirements onto the access event model and ATNA trail | [#3170](https://github.com/rubentalstra/FerroEHR/issues/3170) | Shipped | [#3205](https://github.com/rubentalstra/FerroEHR/pull/3205) |
| [EHDS](https://eur-lex.europa.eu/eli/reg/2025/327/oj) | EU | Annex II 3.2 | Auditing off is one info-level line, and fail_mode=open is the silent default: make both visible where an operator looks | [#3238](https://github.com/rubentalstra/FerroEHR/issues/3238) | Shipped | [#3260](https://github.com/rubentalstra/FerroEHR/pull/3260) |
| [EHDS](https://eur-lex.europa.eu/eli/reg/2025/327/oj) | EU | Annex II 3.2(a) | The access log records no accessing organisation (EHDS Annex II 3.2(a)) | [#3204](https://github.com/rubentalstra/FerroEHR/issues/3204) | Shipped | [#3213](https://github.com/rubentalstra/FerroEHR/pull/3213) |
| [EHDS](https://eur-lex.europa.eu/eli/reg/2025/327/oj) | EU | Art. 9 | A natural person cannot obtain their own access log: the only retrieval is admin-gated over the whole repository | [#3240](https://github.com/rubentalstra/FerroEHR/issues/3240) | Planned | — |
| [EHDS](https://eur-lex.europa.eu/eli/reg/2025/327/oj) | EU | Chapter IV | Secondary-use read model as a separate pseudonymisation domain fed from the outbox | [#3160](https://github.com/rubentalstra/FerroEHR/issues/3160) | Planned | — |
| [EDPB 01/2025](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en) | EU | pseudonymisation domain | Split demographic parties into a `demographic` schema with a non-overlapping runtime role | [#3153](https://github.com/rubentalstra/FerroEHR/issues/3153) | Shipped | [#3182](https://github.com/rubentalstra/FerroEHR/pull/3182) |
| [EDPB 01/2025](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en) | EU | pseudonymisation domain | Linkage service: the party to EHR resolve map as its own schema and role | [#3158](https://github.com/rubentalstra/FerroEHR/issues/3158) | Shipped | — |
| [EDPB 01/2025](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en) | EU | pseudonymisation domain | Cross-domain cohort queries with a demographic predicate and a clinical selection | [#3159](https://github.com/rubentalstra/FerroEHR/issues/3159) | Planned | — |
| [UAVG](https://wetten.overheid.nl/BWBR0040940) | NL | Art. 46 | Refuse identifying data on the clinical side and constrain the subject reference to a pseudonym namespace | [#3154](https://github.com/rubentalstra/FerroEHR/issues/3154) | Shipped | [#3187](https://github.com/rubentalstra/FerroEHR/pull/3187) |
| [UAVG](https://wetten.overheid.nl/BWBR0040940) | NL | Art. 46 | National identifiers in the demographic schema: encrypted storage, keyed lookup, audited resolution | [#3155](https://github.com/rubentalstra/FerroEHR/issues/3155) | Shipped | [#3189](https://github.com/rubentalstra/FerroEHR/pull/3189) |
| [Wabvpz](https://wetten.overheid.nl/BWBR0023864) | NL | Art. 15e | A natural person cannot obtain their own access log: the only retrieval is admin-gated over the whole repository | [#3240](https://github.com/rubentalstra/FerroEHR/issues/3240) | Planned | — |
| [NEN 7513](https://www.nen.nl/nen-7513-2018-nl-245399) | NL | actor role | The access record carries no actor role or authorisation basis, which NEN 7513 requires and the RBAC layer already holds | [#3239](https://github.com/rubentalstra/FerroEHR/issues/3239) | Planned | — |
| [NEN 7513](https://www.nen.nl/nen-7513-2018-nl-245399) | NL | event content | Per-domain access logging for reads and queries aligned with NEN 7513 and EHDS Art. 9 | [#3156](https://github.com/rubentalstra/FerroEHR/issues/3156) | Shipped | [#3188](https://github.com/rubentalstra/FerroEHR/pull/3188) |
| [NEN 7513](https://www.nen.nl/nen-7513-2018-nl-245399) | NL | event content | Domain-level access events discard their EmitOutcome, so fail_mode=closed does not cover them | [#3235](https://github.com/rubentalstra/FerroEHR/issues/3235) | Shipped | [#3246](https://github.com/rubentalstra/FerroEHR/pull/3246) |
| [NEN 7513](https://www.nen.nl/nen-7513-2018-nl-245399) | NL | retention | Audit retention has no jurisdictional floor: retention_days=1 erases the access log daily while the chain still verifies | [#3242](https://github.com/rubentalstra/FerroEHR/issues/3242) | Planned | — |

## Legal sources

The short names above resolve to these publishers. The linked text is the
authority; nothing on this page restates it.

| Short name | Applies to | Source |
|---|---|---|
| GDPR | EU | [https://eur-lex.europa.eu/eli/reg/2016/679/oj](https://eur-lex.europa.eu/eli/reg/2016/679/oj) |
| EHDS | EU | [https://eur-lex.europa.eu/eli/reg/2025/327/oj](https://eur-lex.europa.eu/eli/reg/2025/327/oj) |
| EDPB 01/2025 | EU | [https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en) |
| UAVG | NL | [https://wetten.overheid.nl/BWBR0040940](https://wetten.overheid.nl/BWBR0040940) |
| Wabvpz | NL | [https://wetten.overheid.nl/BWBR0023864](https://wetten.overheid.nl/BWBR0023864) |
| NEN 7510 | NL | [https://www.nen.nl/nen-7510-1-2024-nl-331311](https://www.nen.nl/nen-7510-1-2024-nl-331311) |
| NEN 7512 | NL | [https://www.nen.nl/nen-7512-2022-nl-297137](https://www.nen.nl/nen-7512-2022-nl-297137) |
| NEN 7513 | NL | [https://www.nen.nl/nen-7513-2018-nl-245399](https://www.nen.nl/nen-7513-2018-nl-245399) |
| IHE ATNA | INT | [https://profiles.ihe.net/ITI/TF/Volume1/ch-9.html](https://profiles.ihe.net/ITI/TF/Volume1/ch-9.html) |

`EU` and `INT` apply to every deployment. A two-letter country code is
national law or a national standard, and applies to a deployment in that
country: FerroEHR is an openEHR CDR, openEHR is not a Dutch standard, and a
deployment elsewhere answers to its own equivalents rather than to these.
Adding a jurisdiction is a registry entry plus the controls that cite it.

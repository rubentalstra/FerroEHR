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

- **Shipped:** the issue is closed as completed. The closing pull request is
  linked in the last column.
- **In progress:** the issue is open and its card on the public roadmap board
  is in the In Progress column.
- **Planned:** the issue is open and work has not started.
- **Not planned:** the issue was closed without the control being built. The
  row stays visible so the record does not quietly lose it.

The page carries no generation timestamp and no build commit. Both change on
every run or every push while the tracker has not moved, which would make the
CI staleness check fail on days when nothing was wrong. When this page was
last regenerated, and from which commit, is the file's own git history.

## Controls

No issue in the tracker declares a control yet, so this table is empty. It
fills itself as the compliance program lands: the first issue to carry a
`Control:` line appears here on the next regeneration.

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

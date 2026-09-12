# The regulation corpus

The legal texts the compliance documentation cites, vendored verbatim at a
named consolidation so a citation resolves to bytes this repository carries
rather than to whatever the publisher's page says on the day it is read.

Two scripts produce the whole tree and nothing here is written by hand:

| script | destination |
|---|---|
| `scripts/vendor/law-eu.sh` | `docs/law/eu/` |
| `scripts/vendor/law-nl.sh` | `docs/law/nl/` |

Every act directory carries the publisher's own file, a `PROVENANCE.md` naming
the identifier, the consolidation, the source URL, the fetch date and the
licence, and a `SHA256SUMS` with one line per vendored file.
`scripts/checks/law-corpus.sh` verifies the digests and refuses a directory
whose record is missing or names a file that is not there.

**Never hand-edit anything under this tree.** A hand edit makes the recorded
digest a lie, which is the one thing a vendored legal text may never be. To
refresh or re-pin, change the script and re-run it.

**This is reference material, not a build input.** Nothing compiles against it
and no test reads the act text; it is what a reviewer checks a compliance claim
against. The openEHR specification oracle is a separate tree,
`docs/specs/openehr/`.

## European Union

The act text is fetched from the Publications Office CELEX content-negotiation
URI, which answers the same document byte for byte on every request; the
eur-lex.europa.eu page linked below renders that document for a human reader
but wraps it in per-request site chrome, so it is not the fetch source. Each
record names both, and the exact manifestation the CELEX resolved to.

| Act | Identifier | Consolidation vendored | Read it at | Licence | Cited by |
|---|---|---|---|---|---|
| [Regulation (EU) 2016/679, General Data Protection Regulation](eu/gdpr/) | CELEX `02016R0679-20160504`, ELI `reg/2016/679` | 2016-05-04 | [EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:02016R0679-20160504) | `LicenseRef-EUR-Lex-Reuse AND CC-BY-4.0` | `compliance/index.md`, `compliance/control-matrix.md`, `compliance/shared-responsibility.md`, `security.md`, `security/dpia.md`, `security/records-of-processing.md`, `audit.md`, `operations.md`, `concepts/storage.md`, `installation/config-privacy.md`, `installation/config-auth.md`, `installation/config-audit.md`, `installation/configuration.md`, `contributing.md` |
| [Regulation (EU) 2025/327, European Health Data Space](eu/ehds/) | CELEX `32025R0327`, ELI `reg/2025/327` | none, the OJ text | [EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:32025R0327) | `LicenseRef-EUR-Lex-Reuse` | `compliance/ehds-readiness.md`, `compliance/technical-documentation.md`, `compliance/index.md`, `compliance/control-matrix.md`, `compliance/shared-responsibility.md`, `audit.md`, `security/dpia.md`, `security/records-of-processing.md`, `beyond-core/fhir.md`, `comparison.md`, `why-ferroehr.md`, `operations.md`, `installation/config-audit.md`, `installation/config-auth.md`, `installation/configuration.md` |
| [Directive (EU) 2022/2555, NIS2](eu/nis2/) | CELEX `32022L2555`, ELI `dir/2022/2555` | none, the OJ text | [EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:32022L2555) | `LicenseRef-EUR-Lex-Reuse` | no page yet; vendored for the cybersecurity obligations a deployment answers to |
| [Regulation (EU) 2024/2847, Cyber Resilience Act](eu/cra/) | CELEX `32024R2847`, ELI `reg/2024/2847` | none, the OJ text | [EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:32024R2847) | `LicenseRef-EUR-Lex-Reuse` | `compliance/ehds-readiness.md`, `compliance/technical-documentation.md` (as the regulation EHDS amends and cross-references) |
| [Regulation (EU) 2017/745, Medical Device Regulation](eu/mdr/) | CELEX `02017R0745-20260719`, ELI `reg/2017/745` | 2026-07-19 | [EUR-Lex](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX:02017R0745-20260719) | `LicenseRef-EUR-Lex-Reuse AND CC-BY-4.0` | `compliance/ehds-readiness.md` (the Article 27 interoperability question, left open) |
| [EDPB Guidelines 01/2025 on pseudonymisation](eu/edpb-guidelines-01-2025-pseudonymisation/) | EDPB Guidelines 01/2025, version 1.0 | adopted 2025-01-16 | [EDPB](https://www.edpb.europa.eu/our-work-tools/documents/public-consultations/2025/guidelines-012025-pseudonymisation_en) | `LicenseRef-EDPB-Reuse` | `compliance/index.md`, `compliance/control-matrix.md`, `security.md`, `security/dpia.md`, `operations.md` |

Guidelines are not law. The EDPB document is here because the pseudonymisation
boundary is designed against it, and its record says plainly that it binds
nobody.

## The Netherlands

| Act | Identifier | Consolidation vendored | Read it at | Licence | Cited by |
|---|---|---|---|---|---|
| [Uitvoeringswet AVG (UAVG)](nl/uavg/) | BWB `BWBR0040940` | 2026-09-01 | [wetten.overheid.nl](https://wetten.overheid.nl/BWBR0040940/2026-09-01) | `LicenseRef-Auteurswet-Art11-Public-Domain` | `compliance/index.md`, `compliance/control-matrix.md`, `compliance/shared-responsibility.md` |
| [Wabvpz](nl/wabvpz/) | BWB `BWBR0023864` | 2025-07-05 | [wetten.overheid.nl](https://wetten.overheid.nl/BWBR0023864/2025-07-05) | `LicenseRef-Auteurswet-Art11-Public-Domain` | `compliance/index.md`, `compliance/control-matrix.md`, `compliance/shared-responsibility.md`, `audit.md` |
| [Burgerlijk Wetboek Boek 7, geneeskundige behandelingsovereenkomst (Wgbo)](nl/bw7-geneeskundige-behandelingsovereenkomst/) | BWB `BWBR0005290`, Art. 7:446-468 | 2026-07-01 | [wetten.overheid.nl](https://wetten.overheid.nl/BWBR0005290/2026-07-01) | `LicenseRef-Auteurswet-Art11-Public-Domain` | `compliance/shared-responsibility.md` |
| [Besluit vaststelling bewaartermijn logging](nl/besluit-bewaartermijn-logging/) | BWB `BWBR0042391` | 2019-09-01 | [wetten.overheid.nl](https://wetten.overheid.nl/BWBR0042391/2019-09-01) | `LicenseRef-Auteurswet-Art11-Public-Domain` | `audit.md`, `security/dpia.md`, `security/go-live-checklist.md` |
| [Besluit elektronische gegevensverwerking door zorgaanbieders (Begz)](nl/begz/) | BWB `BWBR0040238` | 2020-10-01 | [wetten.overheid.nl](https://wetten.overheid.nl/BWBR0040238/2020-10-01) | `LicenseRef-Auteurswet-Art11-Public-Domain` | `audit.md` |

The whole of Boek 7 is vendored rather than the cited articles alone: the
publisher serves the book as one document, and cutting Art. 446-468 out of it
would be an edit.

## The standards that cannot be vendored

NEN sells its standards under copyright, so no text of one is in this tree.
Each directory holds a record naming the edition, the shop URL and the clauses
the compliance pages cite, so a reviewer holding a licensed copy can follow
every citation.

| Standard | Record | What the pages cite |
|---|---|---|
| NEN 7510-1:2024 and NEN 7510-2:2024, information security in healthcare | [nl/nen-7510/](nl/nen-7510/) | The part (-1 the management system, -2 the controls) and the control family; no numbered clause |
| NEN 7512:2022, the trust basis for data exchange | [nl/nen-7512/](nl/nen-7512/) | The standard whole; no numbered clause |
| NEN 7513:2018, logging actions on electronic patient records | [nl/nen-7513/](nl/nen-7513/) | §5, the content of a logged event; plus actor role, event content and retention |

## Licensing

The declarations live in `REUSE.toml` and the full terms in `LICENSES/`:

| Identifier | Covers |
|---|---|
| `LicenseRef-EUR-Lex-Reuse` | The EU acts. The Commission's reuse policy, Decision 2011/833/EU, quoted from the EUR-Lex legal notice |
| `CC-BY-4.0` | The consolidated EU texts, which are additionally the EU's own editorial content |
| `LicenseRef-EDPB-Reuse` | The EDPB guidelines, under the EDPB's own copyright page |
| `LicenseRef-Auteurswet-Art11-Public-Domain` | The Dutch acts, in which no copyright subsists |

The `PROVENANCE.md` and `SHA256SUMS` files, and this index, are the project's
own writing about the vendored material and stay under the project licence.

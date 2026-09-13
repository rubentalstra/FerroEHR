---
name: law-corpus-publisher-facts
description: Fetch facts for the DE and CH legal publishers (gesetze-im-internet builddate pin, recht.bund.de BGBl PDF, Fedlex filestore + SPARQL for consolidations) learned vendoring #3293/#3294 — the things that cost probing time
metadata:
  type: reference
---

- **gesetze-im-internet.de** has NO dated URL: `<slug>/xml.zip` is always the
  current state (one `BJNR….xml`, gii-norm DTD). The pin is the
  `<dokumente builddate="…">` stamp; `scripts/vendor/law-de.sh` refuses a
  fetch whose builddate differs from the table. The site calls its
  consolidations "nicht amtliche Texte" (the BGBl. is authentic). Amending
  acts (DigiG) have no consolidation there; the BGBl. issue is on
  recht.bund.de as a byte-stable PDF
  (`/bgbl/1/<year>/<nr>/regelungstext.pdf?__blob=publicationFile&v=1`).
  Nine-digit `gliederungskennzahl` values trip the BSN eleven-test, so
  `docs/law/` is in the privacy-boundary guard's skip list.
- **Fedlex**: `www.fedlex.admin.ch/eli/...` is a SPA (same 77 kB shell for
  every act). The text is the filestore:
  `fedlex.data.admin.ch/filestore/fedlex.data.admin.ch/eli/cc/<y>/<n>/<YYYYMMDD>/<lang>/html/fedlex-data-admin-ch-eli-cc-<y>-<n>-<date>-<lang>-html.html`,
  byte-stable. Current consolidations + languages come from the SPARQL
  endpoint (`fedlex.data.admin.ch/sparqlendpoint`, jolux:ConsolidationAbstract
  `jolux:isMemberOf` act, `jolux:dateApplicability`, `jolux:isRealizedBy/jolux:language`).
  English is non-binding ("has no legal force") and often lags the German
  consolidation; vendor it only at the same date. EPDV-EDI Anhänge 2/3/4/8/9
  are published by reference only (not in the corpus).
- Licence facts: § 5 Abs. 1 UrhG (DE), Art. 5 URG (CH; Abs. 2 covers official
  translations) — quoted in `LICENSES/LicenseRef-UrhG-Para5-Amtliche-Werke.txt`
  and `LicenseRef-URG-Art5-Nicht-Geschuetzte-Werke.txt`.

**How to apply:** re-pin by editing the script table on purpose, never by
following the publisher; read scope before substance (BDSG Part 3 = law
enforcement only; SGB V § 309 = TI controllers; EPDG binds communities).
See [[compliance-corpus-direction]].

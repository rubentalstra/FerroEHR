# Verordnung über die Alters- und Hinterlassenenversicherung (AHVV)

The ordinance under the AHVG, vendored for one identifier: **Art. 133** defines the thirteen-digit AHV number (the country code 756, nine digits that allow no inference about the person, a check digit), **Art. 133bis** puts its assignment with the ZAS, and **Art. 134quinquies Abs. 2** requires a check-digit control before a manual entry. The check-digit arithmetic itself is not in the ordinance; it is in the BSV's Wegleitung über Versicherungsausweis und individuelles Konto (WL VA/IK, 318.106.02 d, Anhang 7), which the identifier scanner's `ch-ahvn13` rule cites.

| | |
|---|---|
| Title (German, authentic) | Verordnung über die Alters- und Hinterlassenenversicherung (AHVV) |
| Enacted / state | vom 31. Oktober 1947 (Stand am 1. Januar 2026) |
| SR number | `831.101` |
| ELI | https://fedlex.data.admin.ch/eli/cc/63/1185_1183_1185 |
| Consolidation vendored | 20260101 (Stand am) |
| Human-readable at | https://www.fedlex.admin.ch/eli/cc/63/1185_1183_1185/20260101/de (this consolidation), https://www.fedlex.admin.ch/eli/cc/63/1185_1183_1185/de (whatever is current) |
| Fetched | 2026-09-13 (UTC) |
| Vendored by | `scripts/vendor/law-ch.sh` |

## Files

| file | bytes | SHA-256 | fetched from |
|---|---|---|---|
| `text-de.html` | 580095 | `0326786b15dff8816fc340b23d4f4f6a1ff92eb53fb452184e5d782da1c593ae` | https://fedlex.data.admin.ch/filestore/fedlex.data.admin.ch/eli/cc/63/1185_1183_1185/20260101/de/html/fedlex-data-admin-ch-eli-cc-63-1185_1183_1185-20260101-de-html.html |

The HTML the Fedlex filestore serves for this consolidation and language,
byte for byte. Nothing was converted or extracted: an extraction is an edit,
and an edited act is no longer the publisher's text. The www.fedlex.admin.ch
page above renders the same document but is a single-page application that
answers an identical shell for every act, which is why it is not the fetch
source.

**No English text exists** for this act on Fedlex; the German text is
the one vendored, and French and Italian are equally authentic at
https://www.fedlex.admin.ch/eli/cc/63/1185_1183_1185/20260101/fr and
https://www.fedlex.admin.ch/eli/cc/63/1185_1183_1185/20260101/it.

## Licence

Not protected by copyright. Art. 5 Abs. 1 lit. a URG: *"Durch das
Urheberrecht nicht geschützt sind: a. Gesetze, Verordnungen, völkerrechtliche
Verträge und andere amtliche Erlasse"*, and Abs. 2: *"Ebenfalls nicht
geschützt sind amtliche oder gesetzlich geforderte Sammlungen und
Übersetzungen der Werke nach Absatz 1."* The article is quoted in
`LICENSES/LicenseRef-URG-Art5-Nicht-Geschuetzte-Werke.txt` and declared for
this subtree in `REUSE.toml`.

Do not hand-edit anything in this directory. Re-run
`scripts/vendor/law-ch.sh` instead — a hand edit makes the SHA256SUMS line a
lie, which is the one thing a vendored legal text may never be.

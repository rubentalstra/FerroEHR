# FerroEHR brand assets

The visual identity approved on issue #1353 (owner decision 2026-07-31):
logo **"Fe element tile"** — iron's periodic-table tile with a heartbeat
tick in the atomic-number corner — with the **"Oxide & Iron"** palette.

## Files

| File | Use |
|---|---|
| `ferroehr-icon.svg` | Primary icon (rust tile). Carries its own background — works on light and dark. App icon, avatars, social. |
| `ferroehr-icon-iron.svg` | Iron-tile variant for surfaces where the rust tile is too loud. |
| `ferroehr-icon-mono.svg` | Single-colour variant (`currentColor`) for stamps/badges/no-colour contexts. |
| `favicon.svg` | Favicon master: tile + Fe only, no heartbeat tick (illegible below 32 px). |
| `ferroehr-lockup-light.svg` | Icon + wordmark for light backgrounds. |
| `ferroehr-lockup-dark.svg` | Icon + wordmark for dark backgrounds. |
| `ferroehr-lockup-auto.svg` | Theme-adaptive lockup (`prefers-color-scheme` media query inside the SVG) — for README/website contexts that serve one file to both themes. |
| `tokens.css` | The palette as CSS custom properties — the single source for brand colours. |

## Palette — "Oxide & Iron"

| Token | Hex | Role |
|---|---|---|
| Ferro | `#B7431B` | signature / accent — the one loud voice |
| Ember | `#D97742` | hover, gradients, accent on dark |
| Iron | `#21262B` | ink, dark ground |
| Steel | `#4E7382` | links, secondary UI |
| Porcelain | `#F2F1EF` | light ground |
| Graphite | `#6B7178` | muted text |

## Usage rules

- The tile always keeps its rounded corners and its own background — never
  place the bare "Fe" letters on an arbitrary ground.
- The heartbeat tick drops out below 32 px (use `favicon.svg`).
- Never recolour outside the palette; the monochrome variant exists for
  single-colour contexts.
- Wordmark: "Ferro" in Iron (Porcelain on dark), "EHR" in Ferro (Ember on
  dark), always set together, never stacked.
- Clear space around the icon: one tile-corner radius on all sides.
- The brand never contains "openEHR" (openEHR Foundation trademark); prose
  describes the product as "an openEHR® CDR" with the Foundation's required
  attribution line.

<!-- TODO: convert the SVG <text> elements (Fe + wordmark) to outlined paths so
     rendering stops depending on viewer-installed fonts (Avenir/Futura stack
     falls back to a generic sans elsewhere), and settle the final wordmark
     typeface at that point. -->
<!-- TODO: emit the raster favicon set (32/16 px PNG + .ico) and the README
     banner from these masters once outline conversion is done. -->

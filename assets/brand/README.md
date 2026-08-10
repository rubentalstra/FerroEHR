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
| `ferroehr-social.svg` / `.png` | The 1280×640 social-preview / banner master and its render (upload the PNG as the GitHub repository social preview). |
| `favicon-32.png` / `favicon-16.png` / `favicon.ico` | The raster favicon set, rendered from `favicon.svg` (48/32/16 in the `.ico`). |
| `tokens.css` | The palette as CSS custom properties — the single source for brand colours. |

## Intrinsic size

Every icon declares `width`/`height` of **512** beside its `viewBox="0 0 64 64"`.
The viewBox is what the artwork is drawn in; the attributes are what a consumer
that RASTERIZES the file uses as its natural size.

That distinction is not academic: with `width="64" height="64"` a package
registry stores a 64-pixel bitmap and a listing header wanting several hundred
renders it blurry or small. The artwork is vector and loses nothing at any size,
so the cap was purely those two attributes.

`favicon.svg` is deliberately excluded — a favicon genuinely wants a small
intrinsic size.

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

## Typeface

The wordmark and the tile "Fe" are set in **Inter** (SemiBold; "EHR" in
Bold) and committed as **outlined paths** — the SVGs carry no font
dependency and render identically everywhere. Inter is licensed under the
SIL Open Font License 1.1 (<https://github.com/rsms/inter>); the OFL
covers the font software, and outlined logo artwork is free to embed.
Regenerating the outlines requires the Inter TTFs + HarfBuzz shaping —
keep edits to geometry/colours in the SVGs themselves.

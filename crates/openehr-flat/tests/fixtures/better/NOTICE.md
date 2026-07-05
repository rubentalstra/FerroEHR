# Vendored Better `web-template-tests` fixtures

Every `*.opt` file in this directory is an OPT 1.4 operational template vendored
**verbatim** from Better's open-source **web-template-tests** repository — the
full `src/test/resources/res/*.opt` set (63 templates):

- Source: <https://github.com/better-care/web-template-tests> (`src/test/resources/res/`)
- Copyright: Better Ltd (<https://www.better.care>)
- License: **Apache License, Version 2.0** — see <http://www.apache.org/licenses/LICENSE-2.0>

They are used here **only as test inputs** for the `openehr-flat` WebTemplate
builder, which targets Better's `web-template` format as its interop oracle. No
modification has been made to the files. The full set (rather than a sample) is
vendored so the WebTemplate builder is gated against the complete Better corpus.

Per the Apache-2.0 terms, this NOTICE preserves the attribution and license of
the vendored material.

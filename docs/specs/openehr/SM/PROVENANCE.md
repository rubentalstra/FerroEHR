# Vendored openEHR spec docs: SM

- Source: https://github.com/openEHR/specifications-SM
- Ref: master
- Commit: `23ffc4711c10bae2ae43724b1948fe3b24a0964e`
- Vendored by: `scripts/vendor-spec-docs.sh` (text formats only: adoc md txt csv json yaml yml robot xml opt g4)
- Plus the 22 UML class-diagram SVG(s) under `docs/UML/diagrams/` that
  the vendored chapters reference as `image::{uml_diagrams_uri}/<name>.svg`,
  taken from the same pinned commit. Referenced files only — the upstream
  diagram directory is not mirrored wholesale.
- Other images, UML `.xmi`/`.mdzip`, XSDs and binaries excluded — fetch from
  the repo at the pinned commit.

Do not hand-edit files under this directory; re-run the script instead.

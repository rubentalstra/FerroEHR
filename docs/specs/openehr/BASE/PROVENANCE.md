# Vendored openEHR spec docs: BASE

- Source: https://github.com/openEHR/specifications-BASE
- Ref: master (BASE 1.3.0)
- Commit: `e48795762a0648cbe5701be58d42ec5df0c701a7`
- Vendored by: `scripts/vendor-spec-docs.sh` (text formats only: adoc md txt csv json yaml yml robot xml opt g4)
- Plus the 15 UML class-diagram SVG(s) under `docs/UML/diagrams/` that
  the vendored chapters reference as `image::{uml_diagrams_uri}/<name>.svg`,
  taken from the same pinned commit. Referenced files only — the upstream
  diagram directory is not mirrored wholesale.
- Other images, UML `.xmi`/`.mdzip`, XSDs and binaries excluded — fetch from
  the repo at the pinned commit.

Do not hand-edit files under this directory; re-run the script instead.

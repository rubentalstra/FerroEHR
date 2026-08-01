# Vendored openEHR spec docs: RM

- Source: https://github.com/openEHR/specifications-RM
- Ref: master (RM 1.2.0)
- Commit: `66d3ac45587e4532a94d5fd27ca24bcf049f5bf3`
- Vendored by: `scripts/vendor-spec-docs.sh` (text formats only: adoc md txt csv json yaml yml robot xml opt g4)
- Plus the 33 UML class-diagram SVG(s) under `docs/UML/diagrams/` that
  the vendored chapters reference as `image::{uml_diagrams_uri}/<name>.svg`,
  taken from the same pinned commit. Referenced files only — the upstream
  diagram directory is not mirrored wholesale.
- Other images, UML `.xmi`/`.mdzip`, XSDs and binaries excluded — fetch from
  the repo at the pinned commit.

Do not hand-edit files under this directory; re-run the script instead.

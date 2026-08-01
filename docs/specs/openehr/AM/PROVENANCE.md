# Vendored openEHR spec docs: AM

- Source: https://github.com/openEHR/specifications-AM
- Ref: master (AM 2.4.0 + ADL/AOM/OPT 1.4)
- Commit: `da06d63297e8549a351c854d8b1c45cd9f1d577c`
- Vendored by: `scripts/vendor-spec-docs.sh` (text formats only: adoc md txt csv json yaml yml robot xml opt g4)
- Plus the 27 UML class-diagram SVG(s) under `docs/UML/diagrams/` that
  the vendored chapters reference as `image::{uml_diagrams_uri}/<name>.svg`,
  taken from the same pinned commit. Referenced files only — the upstream
  diagram directory is not mirrored wholesale.
- Other images, UML `.xmi`/`.mdzip`, XSDs and binaries excluded — fetch from
  the repo at the pinned commit.

Do not hand-edit files under this directory; re-run the script instead.

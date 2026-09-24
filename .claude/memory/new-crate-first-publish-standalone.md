---
name: new-crate-first-publish-standalone
description: "How a NEW openehr-* crate gets its first crates.io version before the lockstep set is published (owner directive 2026-09-24, openehr-sdt)"
metadata:
  node_type: memory
  type: project
  originSessionId: 8a8b6023-3ec9-4b54-b380-837637f38238
  modified: 2026-09-24T18:05:01.642Z
---

A new `crates/*` member cannot be published at the lockstep version while
its siblings' new version is unpublished: `cargo publish` resolves the
`version =` requirements against the registry. On 2026-09-24 the owner wanted
`openehr-sdt` on crates.io alone (to configure its Trusted Publisher entries
before the PR merged). The way that worked: a LOCAL, uncommitted edit of the
new crate's manifest that drops `path =` on every sibling dependency (normal
AND dev) and points `version` at the published sibling version, then
`cargo publish -p <crate> --allow-dirty`, then `git checkout` the manifest.
The published version then depends on the OLD siblings, so the tree steps
the lockstep ONE more (0.0.68 → 0.0.69) in the same PR, leaving the bootstrap
version as a gap for the eight and the prose saying so.

**Why:** the release lane treats an existing version as done, so a coherent
set needs a fresh number once one crate was published out of band.

**How to apply:** if the owner asks to publish only a new crate, do the
dirty-manifest publish, revert, bump all nine again, refresh both
`Cargo.lock` files, and fix every version mention (CHANGELOG, READMEs, the
book's crates + licensing pages). Never commit the dirty manifest. Related:
[[merge-on-local-gates]], [[many-issues-per-pr]].

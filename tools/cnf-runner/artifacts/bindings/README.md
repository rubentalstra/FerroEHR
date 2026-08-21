# Operation bindings (`bindings/<its>/`)

One YAML per SM operation (plus `variant:` files for the alternate
realizations of the same operation): the request shape, the declared
`outcomes` map, the header/body expectations, and the `captures` a case reads
back. Only `*.yaml` is loaded — this file is documentation.

Bindings describe the WIRE, so every declaration in one is an EXECUTED
assertion: a violated header matcher is a red row, not a note. The two rules
below are the ones that are easy to get wrong from inside a single file, so
they are recorded once here rather than re-argued in each binding's comments.

## Header-matcher placeholders: structural tokens, then case variables

A `pattern:` matcher may carry `<name>` placeholders. They resolve in exactly
one order:

1. **Structural grammar tokens** — a closed vocabulary of names whose LEXICAL
   FORM a released spec defines. The matcher then asserts that grammar rather
   than an identity. Currently `<versioned_object_uid>` and `<system_id>`
   (BASE `base_types` master05 §Syntaxes: `object_id = uid`,
   `creating_system_id = uid`), `<n>` (the same section's
   `version_tree_id = trunk_version, [ '.', branch_number, '.',
   branch_version ]`), and `<template_hrid>` (AM Identification master03
   §Human-readable Identifier (HRID) + master04 §Artefact Versioning — the
   ADL2 identity; an ADL 1.4 OPT `template_id` is a free string, so that
   name stays case-variable space).
   A structural token **outranks** a same-named case variable.
2. **Case variables** — any other name resolves to the same-named capture or
   `with:` argument, regex-escaped, asserting an identity.
3. **Neither is a hard failure.** The row fails loud naming the placeholder.
   It does NOT degrade to a `.*` wildcard; that silent-wildcard behaviour was
   removed (#1852/#1865) because a matcher like
   `W/"<versioned_object_uid>::<system_id>::1"` collapsing to a near-tautology
   is the vacuous-assertion class of #1830 on the expectation side.

## Why the structural tokens exist: the vacuity rule

**A capture sourced from the very header a matcher judges can never
strengthen that matcher** — resolving the placeholder from that capture would
compare the header with itself, and the assertion passes for any value the
server sends. That is a vacuous assertion wearing the costume of a strict one.

The structural tokens exist precisely for that case. Each names a segment that
is *server-assigned or server-resolved*, that no request argument spells, and
whose only honest source would be the response under judgement: the container
id minted by a CREATE, the emitting deployment's system id, the version-tree
position, the stored template's resolved HRID. Since identity is unavailable
without circularity, the matcher asserts the segment's released **grammar**
instead — a real constraint that a wrong value can fail.

The converse is the test to apply when adding a matcher: if the value IS
available from a different response or a different channel, assert the
identity, not the grammar. `I_EHR_CONTRIBUTION.get_contribution`'s
`W/"<contribution_uid>"` is the worked example — `contribution_uid` is
captured on the COMMIT from the response body `uid.value`, so judging the
READ's ETag against it is a genuine cross-response identity assertion, and
`contribution_uid` is correctly absent from the structural vocabulary.

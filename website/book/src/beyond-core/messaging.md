# EHR Extract & messaging

Moving a patient's record between openEHR systems — migrating to another CDR,
replicating an EHR to a downstream repository, or importing an externally
produced document — is what openEHR's EHR Extract and messaging services are
for. FerroEHR implements whole-EHR export and import (including cross-system
cloning that preserves version identity) and Template Data Document (TDD)
import.

> [!NOTE]
> The openEHR **service model** defines a Message component, but the released
> ITS-REST 1.1.0 contract publishes no extract, message, or TDD endpoint at
> all. FerroEHR therefore serves these operations under a `/message` group of
> its own design — the routes are ours, not the standard's, and they gate no
> openEHR conformance claim. Unlike the admin extensions, they are **not**
> admin-gated: they carry the same ordinary authentication as the clinical API.
> The six routes, with their bodies, parameters, and status codes, are in
> [Operations → The messaging API](../operations.md#the-messaging-api-ehr-extract-and-tdd-import).

## Exporting an EHR

Export produces an openEHR EXTRACT: a self-contained package of an EHR's
versioned objects.

- **Whole-EHR export** takes every versioned object in an EHR at its latest
  version and assembles them into one extract — the simplest way to snapshot
  or hand off a complete record.
- **Spec-driven export** takes an extract specification (a manifest of which
  entities to include, and a version specification per entity) and produces one
  extract per manifest entity — for selective or policy-controlled export.

## Importing and cloning across systems

Import is the inverse, and it is where openEHR's distributed version identity
matters. When a record produced on one system is imported into another, the
imported versions must keep their original identity while being recorded as
having arrived from elsewhere.

- **Cloning a whole EHR** takes an extract and materializes it into an empty
  target EHR. You can let the server allocate the EHR id or reuse the source's
  id (a true clone). Each original version in the extract is committed wrapped
  in an `IMPORTED_VERSION`, so the record shows both the original authorship
  and the fact of import — version identity is preserved, not regenerated.
- **Importing into an existing EHR** merges an extract's versions into an EHR
  that already exists, following the openEHR change-control copying rules.

This is the mechanism behind cross-system EHR migration: export from the source,
import into the destination, and the destination's history faithfully reflects
where each version came from.

When ATNA auditing is enabled (see
[Security & multi-tenancy](../security.md#atna-audit-trail)), each export and
import emits a security-audit event under the ATNA `Extract` object class, so
records moving between systems are captured in the audit trail.

## Importing TDDs

A Template Data Document (TDD) is a template-shaped XML document carrying the
data for one composition. TDD import converts a TDD into a composition against
its operational template and commits it, returning the new version's object
version id. A batch variant imports several TDDs in one call, fail-fast and
all-or-nothing: if any document fails, none are committed.

TDD import commits through the same validated write path as any other
composition (see [Templates & validation](../templates-validation.md)), so a
malformed document, an unknown EHR, or an unknown template is rejected rather
than partially stored.

## Divergent copies and version branches

openEHR's version tree branches when a version that was created on another
system is modified locally, and FerroEHR implements that: the local write forks
a branch at the imported version's fork point, and the new version id ends in
a three-part tree id (`…::2.1.1`) rather than a plain trunk number. Branch
versions are served at the same URLs as trunk ones, appear in the history and in
`ALL_VERSIONS` queries, and the *latest* version of an object is always the
latest trunk version — see
[Content negotiation & errors](../using-the-api/content-negotiation.md).
So importing a modified copy of a record that has diverged on two systems is
supported, and the divergence is visible in the version tree rather than
flattened away.

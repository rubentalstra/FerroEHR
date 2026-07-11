# EHR Extract & messaging

Moving a patient's record between openEHR systems — migrating to another CDR,
replicating an EHR to a downstream repository, or importing an externally
produced document — is what openEHR's EHR Extract and messaging services are
for. EHRbase-rs implements whole-EHR export and import (including cross-system
cloning that preserves version identity) and Template Data Document (TDD)
import.

> [!NOTE]
> These capabilities are provided through the platform's **native service API**
> (the openEHR SM platform-service catalogue), not as HTTP endpoints. The
> ITS-REST 1.0.3 contract defines no extract, message, or TDD wire operations,
> so the server exposes none — the conformance suite records the messaging
> cases as _skipped with a reason_ (native-API-only) for exactly this reason.
> If you need these operations over HTTP, they are an integration you build on
> top of the native API, not a route the server serves today.

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

## Current limitations

Version branching is not enabled — the store is trunk-only — so importing a
_modified_ copy of a record that has diverged on two systems is out of scope
for this release; straight cloning and import of un-branched history are
supported.
The behaviour above is verified against the platform's native service traits and
the conformance messaging cases; because there is no REST binding, there are no
endpoints, headers, or status codes to document here.

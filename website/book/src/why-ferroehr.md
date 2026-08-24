# Why FerroEHR exists

This chapter is the project's position: what makes openEHR worth implementing,
what this implementation commits to, and what it asks of the organisations that
build on it. Read it if you are deciding whether to depend on FerroEHR, or
whether to contribute to it.

<!-- toc -->

## openEHR is worth building for

openEHR does something almost nothing else in health IT does: it separates
clinical knowledge from software, and then writes both down. What a blood
pressure, a medication order or a discharge summary *means* lives in
archetypes and templates authored by clinicians and modellers: published and
computable. The Reference Model underneath them is specified. So is
the query language, the REST interface, and the serialization, down to the
shape of the JSON on the wire.

The consequence is determinism. Given the same template and the same
composition, two conformant systems store the same record and answer the same
AQL query the same way. A clinical record stops being one application's
private state and becomes data that outlives the application, the vendor and
the procurement cycle. For a record that has to stay readable in twenty
years, by software nobody has written yet, that is the whole game.

## A specification is only as strong as the implementations you can run

A standard becomes real when there is something you can start with one
command, read the source of, check against the specification yourself, and
deploy without first negotiating a licence. Without that, an excellent
specification stays an idea that only well-funded organisations can act on.

That is the gap this project set out to close: one complete, openly
developed, permissively licensed openEHR CDR whose conformance is *measured
and published*.

## What we commit to

- **MIT for all of our own code, with no open-core tier.** Multi-tenancy,
  role- and attribute-based access control, IHE ATNA audit, per-version
  digital signatures, the FHIR R4 connectors, change events, the admin
  console — one repository, one licence. Nothing is held back to be sold back
  to you. (Vendored openEHR material keeps its own upstream terms, and the
  spec crates that embed it say so in their own metadata; see
  [Licensing & legal](licensing.md).)
- **Every claim checkable.** Conformance is executed by a runner against a
  live server, and the run records, per-case results, measured performance and
  the [comparison with another CDR](comparison.md) (in both directions) are
  committed to the repository. If a number appears on this site, the record it
  came from is in the tree, and a change that moves a verdict cannot land
  quietly.
- **The specification is the authority.** The normative openEHR text is
  vendored in the repository and cited decision by decision. Where we find it
  silent or self-contradictory, the finding is filed in public and reported
  upstream.
- **The specification layer as reusable libraries.** The generated openEHR
  model, the canonical codecs, the REST contract, the ADL engine and the AQL
  parser are published on crates.io as [standalone crates](crates.md), so the
  next Rust project does not have to re-model openEHR to get started.
- **Maintenance in the open.** Public roadmap, public issue tracker,
  changelog-driven releases, signed artifacts, and a security policy with a
  private reporting channel.

## Commercial use is the point

FerroEHR is MIT-licensed, and we mean it as an invitation:

- Integrate it into your product, embed it in your platform, or run it as a
  hosted service.
- Redistribute it, package it, white-label it, and build closed products on
  top of it. Charge for them.
- There is no contributor licence agreement, no copyright assignment, no dual
  licence and no "commercial licence" conversation to have. Contributors keep
  their copyright, and what you build on top stays yours.

Commercial adoption is the goal here. Standards reach patients through
products, and products are built by organisations that need to earn a living.
What matters is that they can do it on a shared, conformant foundation instead
of rebuilding one in private.

## What we ask in return

Contribute back.

The licence does not oblige you. The arithmetic does:

- **A private fork is the expensive option.** Fork it and you inherit the
  whole maintenance surface: specification releases, security advisories,
  database upgrades, re-running conformance, and re-merging your changes at
  every release, forever. Upstream the same change and it is maintained once,
  by everyone who runs it.
- **A defect found once should be fixed everywhere.** In clinical software
  the validation gap you patched privately is still live in every other
  deployment of the same code. Sharing the fix is the difference between one
  organisation being safe and all of them being safe.
- **Interoperability is a property of the population, not of any single
  implementation.** Every conformance case contributed, and every ambiguity
  resolved in the open, makes it likelier that your system and the next one
  actually agree about a record.

When the same fix is made privately in five places, the standard is no
stronger and five teams have paid for it. Nobody chose that outcome, and it is
an easy one to avoid.

## What contributing back can look like

You do not have to write Rust to make this project better:

- **A bug report** with the request that reproduces it, the most valuable
  thing most users will ever send.
- **A conformance case** for behaviour the catalogue does not cover yet, so
  the next release cannot regress it.
- **A specification finding**: the ambiguity you had to resolve in order to
  ship. We adjudicate those against the vendored text and report the genuine
  ones upstream to the openEHR Foundation.
- **Documentation**: a correction, deployment experience from your
  environment, or the paragraph that would have saved you two days.
- **Measurement** from your own hardware and workload. The instruments that
  produce our published numbers ship in the repository and run against any
  server, including one that is not FerroEHR.
- **Code**: fixes, features, connectors, packaging for your platform.
- **Sponsorship**, if your organisation depends on this and cannot spare
  engineering time.

[Contributing](contributing.md) is the practical starting point. Security
issues have their own private channel; please use it rather than the public
tracker.

## What this is not

This is not an argument against commercial or proprietary software. Closed
products built on open foundations are a perfectly good outcome, and we would
rather your product succeed with FerroEHR inside it than not exist at all.

It is also not a claim to be the only good openEHR CDR. FerroEHR began as a
fork of EHRbase and keeps that lineage in its git history; today it is an
independent Rust implementation with none of that code left in the tree, and
we measure ourselves against EHRbase with the same instrument, publishing both
directions of the result. FerroEHR is an independent implementation of the
openEHR® specifications and is not affiliated with or endorsed by the openEHR
Foundation.

And it is not a large team. The project is maintained by one person today,
with the machine gates (conformance, the fidelity suites, the CI guards)
standing in for review capacity a bigger group would have. The
[threat model](threat-model.md) and the repository's own governance and
maintainer documents say so plainly.

---

_Maintained by Ruben Talstra and the FerroEHR contributors. If your
organisation is building on FerroEHR, we would like to hear about it: open
an issue or say hello on the tracker._

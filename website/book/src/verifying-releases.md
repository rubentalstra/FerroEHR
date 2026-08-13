# Verifying releases

Before you run a FerroEHR binary, image, or chart in a clinical environment,
you should be able to answer two questions about the bytes you downloaded:
**did they arrive intact**, and **were they built by this project's own release
pipeline**. This page is the operator's procedure for both — one command per
artifact kind, plus the deliberately-failing runs that prove your verification
is actually checking something.

Every release publishes signed build provenance, a dependency SBOM, and a plain
checksum alongside each artifact. Provenance is signed through
[Sigstore](https://www.sigstore.dev/), so it is verifiable rather than merely
readable, and the signer identity is one you can pin to a single hardened
workflow.

<!-- toc -->

## What a release publishes

Substitute the release tag you downloaded for `<tag>` (for example
`v3.17.5`) and the architecture for `<arch>` (`x86_64` or `aarch64`) throughout
this page. Linux is the only published target.

| Asset | What it is |
|---|---|
| `ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz` | the stripped server binary |
| `…tar.gz.sha256sum` | a plain checksum of that tarball |
| `…tar.gz.sigstore.json` | the Sigstore bundle for its build-provenance attestation |
| `…tar.gz.sbom.sigstore.json` | the Sigstore bundle for its SBOM attestation |
| `…tar.gz.intoto.jsonl` | the same provenance as one DSSE-wrapped in-toto statement per line |
| `ferroehr-<tag>-<arch>-unknown-linux-gnu.cdx.json` | the CycloneDX dependency SBOM for that binary |
| `ferroehr-<tag>.spdx.json` | the SPDX SBOM of the source tree at the release commit |
| `docker-compose.yml` + the Keycloak and observability overlays | the quickstart stack, so a downloader never has to clone the repository |

Container images and the Helm chart are published to GHCR by their own lanes and
carry their own attestations — see [images](#a-container-image) and
[the chart](#the-helm-chart) below.

> [!NOTE]
> The release lane refuses to publish a release whose asset set is incomplete.
> It creates the release as a draft, attaches every asset, checks the full
> expected set is present, and only then publishes — because a published release
> is immutable on this repository, so a missing asset could never be added
> afterwards. The remedy for a bad cut is a new patch version, never a retag.

## A release binary

**The floor: the checksum.** Every tarball ships a `.sha256sum` beside it, and
this is the only verification available with neither `gh` nor `cosign` installed
— which is a realistic constraint in a locked-down clinical environment:

```bash
sha256sum -c ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz.sha256sum
```

The file records the bare filename, so this works in whatever directory you put
the two files in. On macOS, `shasum -a 256 -c` is the same check.

> [!WARNING]
> Be clear about what a checksum buys: it detects a **corrupt or truncated
> download**, not a substituted release, because anyone who could replace the
> tarball could replace the checksum beside it. Only the Sigstore bundle answers
> "who built this". The checksum is a floor, not a substitute.

**The real check: the provenance attestation.**

```bash
gh attestation verify ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz \
  -R rubentalstra/FerroEHR
```

**Without reaching GitHub.** Each release also carries its Sigstore bundles as
assets, so verification needs nothing but the artifact and the bundle — useful
on an air-gapped host, and the only form in which the signature travels with the
download:

```bash
gh attestation verify ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz \
  --bundle ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz.sigstore.json \
  --repo rubentalstra/FerroEHR
```

The `*.sbom.sigstore.json` asset beside it is the same thing for the SBOM
attestation, so "which dependency graph was this binary built from" is
verifiable offline too.

**Require the hardened signer.** Without a signer constraint you are trusting
that *some* workflow in this repository signed the artifact. Pin the one that
actually did:

```bash
gh attestation verify ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz \
  -R rubentalstra/FerroEHR \
  --signer-workflow rubentalstra/FerroEHR/.github/workflows/release-build.yml
```

That workflow is the reusable release-build lane described under
[SLSA levels](#what-slsa-level-each-artifact-reaches) below.

> [!TIP]
> `gh attestation verify` reports success by **exiting zero and printing
> nothing** in current `gh` versions. Check the exit status in scripts rather
> than grepping for a success message.

## Prove your verification can fail

A check that cannot fail proves nothing, so do both of these once, by hand,
before you trust a passing run:

```bash
# 1. Tamper with the artifact — verification must refuse it.
printf 'x' >> ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz
gh attestation verify ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz \
  --bundle ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz.sigstore.json \
  --repo rubentalstra/FerroEHR
# → Error: verifying with issuer "sigstore.dev"; exit status 1

# 2. Name a repository that did not build it — also non-zero.
gh attestation verify … --repo someone-else/something
```

Re-download the tarball afterwards. Only once you have seen both refusals does a
passing run mean something.

**Cross-check the digest.** The same digest is produced independently in three
places, so they are worth comparing against each other rather than trusting any
one of them:

```bash
# the published checksum file
cat ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz.sha256sum

# the digest the provenance statement was signed over
jq -r '.payload' ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz.intoto.jsonl \
  | base64 -d | jq -r '.subject[].digest.sha256'

# the bytes on your disk
sha256sum ferroehr-<tag>-<arch>-unknown-linux-gnu.tar.gz
```

## A container image

The three images (`ferroehr`, `ferroehr-admin-ui`, `ferroehr-postgres`) each
carry a Sigstore-signed SLSA provenance attestation, plus the SPDX SBOM and
provenance the builder writes onto the image index itself.

```bash
gh attestation verify oci://ghcr.io/rubentalstra/ferroehr:3.17.5 \
  -R rubentalstra/FerroEHR
```

> [!IMPORTANT]
> **Image tags carry no `v` prefix.** A release publishes `3.17.5`, `3.17` and
> `latest` (plus a `sha-…` tag per commit), while the release *assets* are named
> after the `v3.17.5` git tag. Using `v3.17.5` as an image reference will simply
> not resolve.

The development tags (`ghcr.io/rubentalstra/ferroehr:develop` and its two
siblings) are signed the same way, so you can rehearse the command against them
before a release.

Add `--signer-workflow rubentalstra/FerroEHR/.github/workflows/containers.yml`
to require the image lane specifically rather than any workflow in this
repository.

Enforcing this at admission time in a Kubernetes cluster is a separate job with
its own machinery — see
[Images: build, provenance, scanning](installation/hardening-supply-chain.md).

## The Helm chart

The chart carries a keyless **cosign signature** in addition to its provenance
attestation. The two are not redundant: the attestation says what the chart was
built from, the signature says who signed the artifact you pulled, and the
signature is what Helm-ecosystem tooling looks for.

```bash
cosign verify ghcr.io/rubentalstra/charts/ferroehr:<chart-version> \
  --certificate-identity-regexp '^https://github\.com/rubentalstra/FerroEHR/\.github/workflows/publish-chart\.yml@' \
  --certificate-oidc-issuer https://token.actions.githubusercontent.com
```

Both flags are the point: without an identity and an issuer, `cosign verify`
accepts a signature from anyone in the transparency log.

```bash
gh attestation verify oci://ghcr.io/rubentalstra/charts/ferroehr:<chart-version> \
  -R rubentalstra/FerroEHR
```

> [!NOTE]
> **The chart version is not the product version.** The chart runs its own
> SemVer line, so a chart version and the `appVersion` it deploys move
> independently. Read the chart's `appVersion` to learn which server release it
> defaults to.

The chart ships **no** PGP `.prov` file, so `helm install --verify` does not
apply. That is deliberate: a `.prov` needs a long-lived private key in CI, and
keyless Sigstore signing gives a consumer a stronger identity to pin without
one.

The publishing lane refuses to overwrite a chart version that already exists,
reads both the signature and the attestation back from the registry before
reporting success, and checks that the `appVersion` image accepts the chart's
rendered defaults.

## Three SBOMs, three questions

A release involves three SBOM documents. They are not redundant — they describe
different things for different readers, and both kinds of review genuinely
happen for a clinical deployment:

| Document | Where | Format | Answers |
|---|---|---|---|
| `ferroehr-<tag>-<arch>-unknown-linux-gnu.cdx.json` | a release asset, one per architecture | CycloneDX 1.5 | *what is inside the binary I am about to run?* Every cargo component with a `pkg:cargo/…` purl and licence, most with checksums, and the **dependency edges** — so "is this crate direct or four levels down" is answerable rather than guessed. This is what a vulnerability scanner consumes. |
| `ferroehr-<tag>.spdx.json` | a release asset, one per release | SPDX | *what am I redistributing, and under what terms?* The attribution and licence-obligation view of the source tree at the release commit — the document a legal or procurement reviewer of a multi-licence redistribution asks for. |
| the image SBOM | written onto each container image index by the builder | SPDX | *what is in the image's OS layer?* Which is what matters for `ferroehr-postgres`, built on the upstream `postgres` image. |

CycloneDX 1.5 is the highest version the generator emits (`cargo-cyclonedx`
accepts 1.3, 1.4 or 1.5 and defaults to 1.3, so the release lane sets it
explicitly); it carries everything 1.6 consumers read. The repository SPDX
document is checked at publish time for being real SPDX, naming itself
`ferroehr`, and listing at least one package — an SBOM that answers no question
is worse than none.

## What SLSA level each artifact reaches

The levels are [SLSA v1.0 Build levels](https://slsa.dev/spec/v1.0/levels), and
they differ per artifact, so a table is the honest form:

| Artifact | Level | Why |
|---|---|---|
| release binaries + their SBOM | **Build L3** | built and attested inside a *reusable* workflow, so the signing material is out of reach of any caller-defined step |
| container images | Build L2 | attested in the job that builds them |
| the Helm chart | Build L2 | same |

Build L3's distinguishing requirement is that the platform must "prevent secret
material used to sign the provenance from being accessible to the user-defined
build steps". Every step of a GitHub Actions job shares one runner VM, so
attesting inside the building job cannot satisfy it. The release lane therefore
builds and signs inside a reusable workflow: it runs on its own VM, and a caller
passes declared inputs — it cannot add steps. The calling job has no steps at
all, which is what makes the property hard to lose by accident, and it is why
`--signer-workflow` is worth passing.

**What is still not claimed**, in either lane: the isolation is GitHub's rather
than this project's, and nothing here asserts a reproducible or hermetic build —
those are separate SLSA tracks this project does not address. Provenance proves
*where* an artifact was built, not that the source was good. Naming the boundary
is worth more than rounding a level up.

## Findings a scanner will report

Run a scanner over a FerroEHR artifact and it will report findings. Every one
this project has assessed and accepted is published as an
[OpenVEX](https://openvex.dev) document under
[`security/vex/`](https://github.com/rubentalstra/FerroEHR/tree/develop/security/vex),
carrying a controlled-vocabulary justification and an impact statement you can
check — rather than an ignore entry that records only the verdict. Point your
tooling at them (`trivy --vex`, and most SCA platforms take an OpenVEX feed).

| Document | Covers |
|---|---|
| `rust-advisories.openvex.json` | the Rust dependency advisories: the ones the advisory gate accepts, plus one that only a `Cargo.lock`-reading scanner reports |
| `postgres-gosu.openvex.json` | Go standard-library findings in the `gosu` helper the upstream `postgres` image ships |

The Rust document is **generated** from `deny.toml` — the gate that actually
decides whether a build passes — joined with the published reasoning, and a CI
job fails if the two disagree in either direction. So an advisory cannot be
accepted without a justification reaching you, and a justification cannot claim
something the gate does not do.

One asymmetry worth knowing, because it produces findings that are real reports
of nothing. `Cargo.lock` records the union of every dependency any feature
combination *could* pull, so a scanner reading the lock file alone reports
crates this project's feature set never compiles. `cargo deny` resolves features
and does not. Where the two disagree in that direction the feature-resolving
tool is the more precise instrument — and the VEX document carries that argument
for the specific crate it applies to, so you do not have to take our word for it
in prose.

The published images are additionally re-scanned on a weekly schedule at the
tag an operator actually pulls, and a fixable high-or-critical finding both
fails that run and files a tracker issue — because a red scheduled run nobody
looks at is not a control.

## If verification fails

A failing verification is a security report, not a support question. Do not run
the artifact, and follow
[SECURITY.md](https://github.com/rubentalstra/FerroEHR/blob/develop/SECURITY.md):
report privately, never as a public issue. Note that only the newest release
receives fixes — there is no maintenance branch to backport to.

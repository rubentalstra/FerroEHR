# Deployment-conformance records

The two deployment harnesses write their machine-readable records here — the
deployment analogue of what `docs/conformance/<sut>/` holds for the wire.

| Harness | Record | Asks |
|---|---|---|
| `scripts/deploy-probe.sh` | `compose.json` | does the software work when it is deployed? |
| `scripts/deploy-probe-k8s.sh` | `kubernetes.json` | does the cluster apply what the chart asked for? |

Those are different questions, and only the second is what an operator running
the chart actually gets. `helm template` proves the YAML; the API server, the
kubelet and the container runtime each get a say afterwards, and every one of
them can decline. So the Kubernetes probes read their answers from the runtime
spec, from admission, and from the EndpointSlice — never from the manifest we
wrote.

A record is a measurement of **one SUT**, so it is only meaningful with the
image it was taken against. The harness names that image in its output, and
the record carries the per-probe outcome plus the layer each failure indicts.

## Reading a record

```json
{"platform":"compose","passed":9,"failed":1,"uncovered":7,
 "probes":[{"id":"P-MM-EXPAND","state":"working","outcome":"fail",
            "layer":"server","issue":"#2197","title":"…"}],
 "not_exercised":[{"what":"kubernetes platform","why":"…"}]}
```

- **`state`** is one of `off` / `working` / `broken`. All three matter: two of
  the defects that prompted this instrument lived in states nobody exercised —
  a disabled integration that stranded data, and a dependency outage nothing
  probed.
- **`layer`** is what a failure indicts — `server`, `image`, `chart`,
  `compose`, or `docs`. A red row that says only "it did not work" costs more
  than it saves, so the layer is part of the record rather than left to the
  reader.
- **`issue`** marks a probe that reproduces a defect this project shipped. It
  must fail on the unfixed code and pass on the fixed code; that is what turns
  a one-off sweep into a permanent net.
- **`not_exercised`** is not an appendix. Silence read as coverage is how ten
  defects reached a release while a 4447-test suite stayed green, so every gap
  is declared in the artifact itself.

## Why records are not committed by default

Unlike the CNF baseline, these are not a ratchet: the record depends on which
image, overlays and profiles the run used, so a committed file would invite
comparison between runs that measured different systems. Commit one only
alongside the SUT identification that makes it readable.

## Running the terminology probe against real SNOMED CT

The terminology family drives Snowstorm loaded with a real SNOMED CT release,
because a code system we seeded ourselves would only prove our client can talk
to our own fixture — `$subsumes` in particular means nothing without a
published hierarchy behind it.

**The RF2 package is licensed content and is never in this repository.** It
cannot be committed, fetched by a vendoring script, or baked into an image; the
operator supplies it, and without it the family declares itself not exercised
rather than substituting a fixture.

Locally, against a copy already downloaded from MLDS:

```bash
FERROEHR_SNOMED_RF2=~/Downloads/SnomedCT_InternationalRF2_PRODUCTION_20260801T120000Z.zip \
FERROEHR_SNOMED_RF2_MD5=878e480163d35bdf6ff3c1f5f9391d47 \
PROBE_ONLY=terminology \
  bash scripts/deploy-probe.sh
```

The MD5 is the one SNOMED International publishes beside the release. It is
optional and it is worth setting: a probe that silently ran against a truncated
or substituted archive would report conformance about content nobody chose. It
is pinned per release, so moving to a new edition is a deliberate edit.

In CI, the `Terminology probe (SNOMED)` workflow (manual dispatch) fetches the
release from wherever the affiliate keeps it:

| Secret / variable | Purpose |
|---|---|
| `SNOMED_RF2_URL` (secret) | where the archive is fetched from |
| `SNOMED_RF2_USER` / `SNOMED_RF2_PASSWORD` (secrets) | basic auth, if that source needs it |
| `SNOMED_RF2_MD5` (variable) | the published checksum for the pinned release |

The archive lives on the runner's disk for the life of the job and nothing
republishes it — no cache, no artifact upload. Only the probe RECORD is
uploaded.

**Two resource facts, stated because discovering them mid-import wastes an
hour.** Elasticsearch and Snowstorm want ~8 GB of memory between them, and a
full International Edition import needs well more disk than a standard
GitHub-hosted runner's 14 GB — which is why the workflow takes a runner label
as an input and defaults to a larger one. Elasticsearch also refuses to start
unless the HOST sets `vm.max_map_count=262144`, a sysctl no container can apply
to itself; the workflow sets it, and locally you may need
`sudo sysctl -w vm.max_map_count=262144` (on Docker Desktop,
`wsl -d docker-desktop sysctl -w vm.max_map_count=262144`).

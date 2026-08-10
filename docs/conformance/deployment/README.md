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

**Local only, and the package is never in this repository.** The RF2 archive is
licensed content under a SNOMED International Affiliate agreement: it is not
committed, not fetched by CI, and not baked into an image. There is deliberately
no CI lane — a workflow that downloaded it onto a shared runner would be moving
licensed content somewhere this project does not control. Without a package the
family declares itself not exercised rather than substituting a fixture.

**The setup is: drop the archive in the repository root.** `SnomedCT_*.zip` is
gitignored precisely so it can live there, and the probe finds it — no
configuration:

```bash
PROBE_ONLY=terminology bash scripts/deploy-probe.sh
```

With both editions present the NATIONAL one is used, because that is the
deployment reality being probed, and the International is loaded first so the
extension can resolve against it. `FERROEHR_SNOMED_RF2` still overrides if the
archive lives elsewhere.

Setting the published checksum is worth the extra line:

```bash
FERROEHR_SNOMED_RF2_MD5=876355868299a8c4d1534e53de6e75a5 \
PROBE_ONLY=terminology bash scripts/deploy-probe.sh
```

The MD5 is the one SNOMED International publishes beside the release, and it is
worth setting: a probe that silently ran against a truncated or substituted
archive would report conformance about content nobody chose. It is pinned per
release, so moving edition is a deliberate edit.

**Whether the national package needs the International Edition alongside it is
not something Snowstorm documents**, so the family does not assume — put both
zips in the root and the International is imported to `MAIN` first.

With only the national package, it is loaded on its own and the run reports what
was actually served — the subsumption probe checks that the concepts it names
are present before asserting anything about them, and declares a gap rather than
a defect if they are not.

**Two resource facts, stated because discovering them mid-import wastes an
hour.** Elasticsearch and Snowstorm want ~8 GB of memory between them. And
Elasticsearch refuses to start unless the HOST sets `vm.max_map_count=262144`, a
sysctl no container can apply to itself:

```bash
sudo sysctl -w vm.max_map_count=262144
# Docker Desktop:
wsl -d docker-desktop sysctl -w vm.max_map_count=262144
```

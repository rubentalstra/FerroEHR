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

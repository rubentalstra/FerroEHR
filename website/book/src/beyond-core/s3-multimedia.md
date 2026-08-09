# S3 multimedia

Clinical records sometimes carry large binary attachments — scanned documents,
images, waveforms — as `DV_MULTIMEDIA` values. Keeping big blobs inline in the
database bloats storage and slows queries. FerroEHR can transparently offload
large multimedia blobs to any S3-compatible object store, keeping only a small
content-addressed reference in the composition, and re-materialize them on
demand when a record is read back.

## How offload works

Offload is a commit-path transformation applied to `DV_MULTIMEDIA` nodes
(including a node's nested `thumbnail`, which is itself a multimedia value):

1. A node qualifies only when it is purely inline (it has `data` and no `uri`)
   and its **decoded** byte length is **strictly greater** than the configured
   threshold. A value at or below the threshold stays inline; a value that
   already references external media (has a `uri`) is stored verbatim, never
   touched.
2. The raw decoded bytes are written to the object store under a key that is
   the SHA-256 hash of those bytes (lowercase hex). Because the key is the
   content hash, identical blobs deduplicate automatically, and the upload is a
   no-op if the key already exists.
3. The node is rewritten in place: its inline `data` is removed and replaced
   with a `uri` of the form `s3://<bucket>/<hash>`, plus an `integrity_check`
   (the same SHA-256 digest — carried as RM `List<Byte>`, so canonical JSON
   renders it **base64**, while the `uri` spells it lowercase hex), an
   `integrity_check_algorithm` code phrase (`SHA-256`), and the original `size`.

Uploads happen before anything is persisted, so a failed upload aborts the
commit — a record is never half-stored.

> [!NOTE]
> What lives where after offload: the object store holds the blob bytes; the
> composition in PostgreSQL holds a compact, spec-legal `DV_MULTIMEDIA` that
> points at the blob by content hash. Everything remains canonical openEHR JSON
> — the `s3://` reference and integrity fields are standard RM attributes.

## Reading blobs back

By default a read returns the stored (offloaded) form — the compact reference.
To get the inline bytes back, request expansion on the read
(`?expand_multimedia=true`). The server fetches each of _its own_ externalized
blobs (only URIs of the exact form `s3://<configured-bucket>/<hash>` are
treated as its own; foreign `https://` or other-bucket references are left
alone), **verifies the SHA-256 hash of the fetched bytes against the key**, and
only then re-inlines the `data`. A hash mismatch is a hard error, so a
corrupted or tampered blob is never silently served.

If the object store is unreachable, both halves of the path fail loudly rather
than losing content: a commit that needs to offload is refused `500` and
nothing is written (the version count is unchanged), and an expanded read of an
already-offloaded record is refused `500`. A read **without** `expand_multimedia`
still answers `200` — it never touches the store.

## Turning it back off

`enabled` governs **new offloads**, not access to old ones. Switching it back
to `false`:

- New commits keep large `DV_MULTIMEDIA` **inline**, byte-identical, with no
  dependency on the object store. Nothing else about the request or the record
  changes.
- Records that were **already** offloaded keep their `s3://` reference and stay
  fully readable: `?expand_multimedia=true` still fetches, verifies and
  re-inlines them, as long as an `endpoint` is still configured. Content this
  server externalized does not become unreachable because a switch was flipped.

**Removing the `endpoint` as well is the decision that matters.** With no store
reachable at all, an expansion request against an already-offloaded record
**fails** — it does not quietly answer `200` with the compact reference. The
bytes are still in your bucket and still reachable with an S3 client; the API
refuses rather than pretending the request was honoured.

So decide what happens to the blobs already in the bucket before you remove the
endpoint, not before you flip `enabled`.

## Enabling it

Offload is off by default. These keys live in the `[multimedia]` section of
`ferroehr.toml`; each can be overridden with the shown `FERROEHR__MULTIMEDIA__*`
environment variable:

| Environment variable | Default | Meaning |
|---|---|---|
| `FERROEHR__MULTIMEDIA__ENABLED` | `false` | master switch |
| `FERROEHR__MULTIMEDIA__THRESHOLD_BYTES` | `262144` (256 KiB) | offload blobs larger than this; smaller stay inline |
| `FERROEHR__MULTIMEDIA__ENDPOINT` | unset | S3 endpoint URL (unset uses AWS default resolution) |
| `FERROEHR__MULTIMEDIA__BUCKET` | `openehr-multimedia` | target bucket |
| `FERROEHR__MULTIMEDIA__REGION` | `us-east-1` | S3 region |
| `FERROEHR__MULTIMEDIA__ACCESS_KEY_ID` | unset | access key (see note on credentials) |
| `FERROEHR__MULTIMEDIA__SECRET_ACCESS_KEY` | unset | secret key |
| `FERROEHR__MULTIMEDIA__ALLOW_HTTP` | `false` | permit plain-HTTP endpoints (development only) |

If both the access key and secret are unset, the client runs unsigned
(anonymous) — the mode a local development SeaweedFS accepts with no
credentials. Set both to use signed requests against a real store.

> [!WARNING]
> **The bucket must already exist.** The server never creates it, and a store
> that is reachable but has no such bucket is not a distinguishable condition on
> the wire: S3 answers a `PUT` into a missing bucket with `403 AccessDenied`, so
> every multimedia commit fails `500` and the log says `Access Denied` — which
> reads like a credentials problem and is not. Create the bucket before you
> enable the integration (the SeaweedFS recipe below shows the one-liner), and
> leave `endpoint` either unset or a full absolute URL — an **empty** endpoint
> string is accepted at startup and then fails the request.

> [!NOTE]
> Externalization is also a **cargo feature** (`multimedia`), on in the
> published images and any default build. A slim `--no-default-features` build
> contains none of the object-store code and refuses to boot with
> `multimedia.enabled = true` rather than silently storing every blob inline —
> see
> [From source → Build features](../installation/from-source.md#build-features).

> [!WARNING]
> Offloaded blobs are PHI. In production the bucket must be private, encrypted,
> and reached over HTTPS (`FERROEHR__MULTIMEDIA__ALLOW_HTTP=false`). Prefer
> instance or workload identity over static keys where your platform supports
> it. See [Operations](../operations.md) for the deployment-side security
> posture.

### On Kubernetes

Every key above is reachable as `config.multimedia.*`
([Any server setting is reachable](../installation/kubernetes.md#any-server-setting-is-reachable-not-only-the-ones-listed-here));
the S3 credentials go through `secrets.*`, which the chart mounts as files:

```yaml
# values.yaml
config:
  multimedia:
    enabled: true
    endpoint: https://s3.example.com
    bucket: openehr-multimedia
    threshold_bytes: 262144
secrets:
  multimediaAccessKeyId: "AKIA…"
  multimediaSecretAccessKey: "…"
```

**Before you enable it:** the bucket must exist — nothing in the chart creates
it, and an S3 write into a missing bucket answers `403 AccessDenied`, not
`404`. **To turn it off**, set `config.multimedia.enabled: false`; leave the
`endpoint` in place so already-externalized content stays readable (see
[Turning it back off](#turning-it-back-off)).

## Quick setup with SeaweedFS

Any S3-compatible store works (AWS S3, MinIO, SeaweedFS). SeaweedFS is a light
option for development and testing — its S3 gateway needs no credentials.

**Step 1 — create the bucket.** The gateway starts with no buckets at all, and
nothing in it creates one. This matters more than it sounds: an S3 write into a
bucket that does not exist answers `403 AccessDenied`, not `404 NoSuchBucket`,
so a missing bucket presents as a credentials problem. Against an
unauthenticated development gateway a bare `PUT` on the bucket path is enough:

```bash
curl -X PUT http://127.0.0.1:8333/openehr-multimedia
curl -s http://127.0.0.1:8333/            # the bucket now appears in ListAllMyBuckets
```

The [Compose stack](../installation/compose.md) does this for you — its
`seaweedfs-init` service performs exactly that `PUT` once the gateway is
healthy — so this step is only for a gateway you run yourself.

**Step 2 — point the server at the gateway** and allow plain HTTP for local
use:

```bash
export FERROEHR__MULTIMEDIA__ENABLED=true
export FERROEHR__MULTIMEDIA__ENDPOINT=http://127.0.0.1:8333
export FERROEHR__MULTIMEDIA__BUCKET=openehr-multimedia
export FERROEHR__MULTIMEDIA__ALLOW_HTTP=true
```

The same exports drive the Compose stack, which passes the whole
`FERROEHR__MULTIMEDIA__*` set through from your shell — only the endpoint
changes, to the in-network hostname `http://seaweedfs:8333`.

**Step 3 — check what the server actually took.** `/management/env` reports the
effective configuration, which is the quickest way to catch a variable that
never arrived:

```bash
curl -s -u ferroehr:ferroehr http://localhost:8080/management/env | jq .multimedia
```

With the feature enabled and the bucket present, large `DV_MULTIMEDIA` values
committed through the normal composition APIs (see
[Using the API](../using-the-api/index.md)) are offloaded automatically; nothing
about the request or the stored record changes except the size of what lives in
the database.

> [!NOTE]
> Base64 inflates a blob by about a third on the wire, and the whole request
> body is still subject to `[server.limits] body_bytes` (16 MiB by default) —
> a composition that exceeds it is refused `413` before offload is ever
> considered. See [`[server.limits]`](../installation/configuration.md).

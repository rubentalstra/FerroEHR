# S3 multimedia

Clinical records sometimes carry large binary attachments — scanned documents,
images, waveforms — as `DV_MULTIMEDIA` values. Keeping big blobs inline in the
database bloats storage and slows queries. EHRbase-rs can transparently offload
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
   (the SHA-256 digest), an `integrity_check_algorithm` code phrase (`SHA-256`),
   and the original `size`.

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

## Enabling it

Offload is off by default. These keys live in the `[multimedia]` section of
`ehrbase.toml`; each can be overridden with the shown `EHRBASE_MULTIMEDIA__*`
environment variable:

| Environment variable | Default | Meaning |
|---|---|---|
| `EHRBASE_MULTIMEDIA__ENABLED` | `false` | master switch |
| `EHRBASE_MULTIMEDIA__THRESHOLD_BYTES` | `262144` (256 KiB) | offload blobs larger than this; smaller stay inline |
| `EHRBASE_MULTIMEDIA__ENDPOINT` | unset | S3 endpoint URL (unset uses AWS default resolution) |
| `EHRBASE_MULTIMEDIA__BUCKET` | `openehr-multimedia` | target bucket |
| `EHRBASE_MULTIMEDIA__REGION` | `us-east-1` | S3 region |
| `EHRBASE_MULTIMEDIA__ACCESS_KEY_ID` | unset | access key (see note on credentials) |
| `EHRBASE_MULTIMEDIA__SECRET_ACCESS_KEY` | unset | secret key |
| `EHRBASE_MULTIMEDIA__ALLOW_HTTP` | `false` | permit plain-HTTP endpoints (development only) |

If both the access key and secret are unset, the client runs unsigned
(anonymous) — the mode a local development SeaweedFS accepts with no
credentials. Set both to use signed requests against a real store.

> [!WARNING]
> Offloaded blobs are PHI. In production the bucket must be private, encrypted,
> and reached over HTTPS (`EHRBASE_MULTIMEDIA__ALLOW_HTTP=false`). Prefer
> instance or workload identity over static keys where your platform supports
> it. See [Operations](../operations.md) for the deployment-side security
> posture.

## Quick setup with SeaweedFS

Any S3-compatible store works (AWS S3, MinIO, SeaweedFS). SeaweedFS is a light
option for development and testing — its S3 gateway needs no credentials.
Point the server at the gateway and allow plain HTTP for local use:

```bash
export EHRBASE_MULTIMEDIA__ENABLED=true
export EHRBASE_MULTIMEDIA__ENDPOINT=http://127.0.0.1:8333
export EHRBASE_MULTIMEDIA__BUCKET=openehr-multimedia
export EHRBASE_MULTIMEDIA__ALLOW_HTTP=true
```

With the feature enabled and the bucket reachable, large `DV_MULTIMEDIA` values
committed through the normal composition APIs (see
[Using the API](../using-the-api/index.md)) are offloaded automatically; nothing
about the request or the stored record changes except the size of what lives in
the database.

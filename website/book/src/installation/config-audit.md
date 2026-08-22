# Audit & subject proxy

The IHE ATNA audit trail (`[audit]` and its three sinks) and the FHIR systems a
subject-proxy frame may read from (`[subject_proxy]`). Precedence, the
environment-name grammar, and file discovery are on the
[Configuration reference](configuration.md) index.

<!-- toc -->

## `[audit]`

The IHE ATNA audit trail (see the [Audit trail chapter](../audit.md) for what a
record contains and how to search it). **On by default** with only the local
store active: every deployment gets a queryable audit trail with nothing leaving
the node, and forwarding to an external Audit Record Repository is opt-in per
sink.

```toml
[audit]
enabled = true
source_id = "ferroehr"
value_if_missing = "UNKNOWN"
suppress_login_events = true
fail_mode = "open"
resolve_subject = true
queue_capacity = 8192
```

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Master audit switch. |
| `enterprise_site_id` | string | unset | The `AuditEnterpriseSiteID` field. |
| `source_id` | string | `ferroehr` | The audit source id, also used for the destination participant. |
| `value_if_missing` | string | `UNKNOWN` | Fill value for empty mandatory fields. |
| `suppress_login_events` | bool | `true` | Skip successful-login records. Rejected accesses (`401`/`403`) are always recorded. |
| `fail_mode` | enum{open,closed} | `open` | What an undeliverable audit record does. `open` logs, meters and lets the request succeed; `closed` rejects auditable operations with `503` — including when the local store has stopped accepting writes — so no PHI access goes un-audited. |
| `resolve_subject` | bool | `true` | Enrich the patient participant with a background lookup of the EHR's subject. The lookup runs on the background drain, never on the request path; the IHE BALP patient patterns and the patient-centric audit search need the subject. |
| `queue_capacity` | int | `8192` | Bounded audit queue capacity. Sized for write-path bursts: the drain persists in multi-row batches, so the queue only needs to ride out sink latency spikes. |
| `server_host` | string | unset ⇒ the `value_if_missing` fill | This node's advertised network address, reported as the destination `NetworkAccessPointID`. |

> [!NOTE]
> The local store and the ATX:FHIR Feed both carry a FHIR R4 `AuditEvent`
> document, so both need the `fhir` build feature — on in the published binary
> and container images. A binary built with `--no-default-features` refuses at
> startup if `audit.store.enabled` or `audit.fhir_feed.enabled` is set; the
> DICOM/syslog feed needs no FHIR and stays available.

> [!NOTE]
> There is no `[atna]` section. Configuration is strict, so a file or
> environment variable still setting an `[atna]` key fails at boot with an
> unknown-key error — move the setting under `[audit]`.

### `[audit.store]`: the local Audit Record Repository

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Persist every record in the `audit` schema, served through the ITI-81 `GET /fhir/r4/AuditEvent` search. |
| `retention_days` | int | `0` | Days to keep records; `0` keeps them forever. Applied hourly by the retention reaper. |

The local store is the durability anchor of the whole subsystem: with it on, the
FHIR feed drains from it, so a down repository loses nothing.

### `[audit.syslog]`: the classic DICOM/syslog feed (ITI-20)

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Ship DICOM PS3.15 audit records to an external repository over syslog. |
| `host` | string | `localhost` | Repository host. |
| `port` | int | `514` | Repository port (514 for UDP, 6514 for TLS, conventionally). |
| `transport` | enum{udp,tls} | `udp` | Syslog transport: RFC 5426 UDP or RFC 5425 TLS. Use `tls` for PHI-adjacent audit. |
| `tls_ca_file` | path | unset | PEM file with the repository CA to trust for the TLS transport. |
| `tls_identity_cert_file` | path | unset | Client-certificate PEM for mutual TLS. |
| `tls_identity_key_file` | path | unset | Client-key PEM for mutual TLS. |

### `[audit.fhir_feed]`: the RESTful-ATNA feed (ITI-20 ATX:FHIR Feed)

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | `POST` each FHIR `AuditEvent` to an external FHIR Audit Record Repository. |
| `url` | secret URL | `http://localhost:8080/fhir` | The repository's FHIR base; records go to `{url}/AuditEvent`. Credentials embedded in the URL are redacted from every rendering. |
| `batch_size` | int | `64` | Outbox rows shipped per poll. |
| `poll_interval_ms` | int | `2000` | Outbox poll interval when idle. |
| `max_retries` | int | `3` | Per-record `POST` retries before the record is left pending (local store on) or dropped and metered (store off). |

With the local store on, the feed drains the store's outbox and is therefore
loss-free across a repository outage. With the store off it ships in-drain, and
a record that exhausts its retries is dropped and counted.

## `[subject_proxy]`

The named FHIR systems a subject-proxy `API_CALL`/`fhir_get` data frame may
retrieve from. **Empty by default and fail-closed**: no external FHIR system is
reachable until one is named here, and a frame whose `system_id` matches no
configured system is a typed rejection rather than an arbitrary outbound
request. The per-system key table and examples live on
[Subject Proxy — Connecting FHIR systems](../beyond-core/subject-proxy.md#connecting-fhir-systems).

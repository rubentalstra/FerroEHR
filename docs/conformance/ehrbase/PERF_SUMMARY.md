| Class | Corpus | Offered-load floor | p99 budget | Error budget | Measured sustained | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| POC | cnf.scale.10k | 2/s | ≤ 1000 ms | 0 | 2.0/s | not earned |
| S | cnf.scale.100k | 15/s | ≤ 1000 ms | 0 | — | not measured |
| L | cnf.scale.1m | 150/s | ≤ 1000 ms | 0 | — | not measured |
| R | cnf.scale.10m | 1500/s | ≤ 1000 ms | 0 | — | not measured |

Measured run `PERF-hospital_sim-class_POC` — class POC, offered load 2.03/s sustained over 3600 s (after 300 s warmup), environment: ci-runner (4 cores, 16 GB, ssd, single-node docker compose (docker/sut-ehrbase-java.yml, ehrbase/ehrbase:2.34.0 + ehrbase-v2-postgres:16.2; no readonly principal — EHRbase Basic auth carries one clinical user and one admin user)).

| Operation | Requests | Errors | p50 (ms) | p90 (ms) | p99 (ms) |
| --- | --- | --- | --- | --- | --- |
| adhoc_query | 953 | 0 | 44 | 90 | 145 |
| composition_commit | 315 | 0 | 73 | 154 | 272 |
| composition_delete | 4 | 0 | 36 | 56 | 56 |
| composition_read | 1906 | 0 | 19 | 29 | 57 |
| composition_read_current | 1072 | 0 | 24 | 41 | 80 |
| composition_revision_history | 1029 | 0 | 22 | 36 | 57 |
| composition_update | 77 | 77 | 52 | 93 | 158 |
| contribution_commit | 52 | 0 | 90 | 228 | 300 |
| contribution_read | 110 | 0 | 23 | 37 | 53 |
| directory_create | 13 | 0 | 25 | 47 | 62 |
| directory_read | 967 | 0 | 18 | 27 | 56 |
| directory_update | 13 | 0 | 43 | 85 | 117 |
| ehr_create | 13 | 0 | 36 | 50 | 51 |
| ehr_read | 98 | 0 | 20 | 29 | 41 |
| ehr_status_read | 26 | 0 | 27 | 40 | 51 |
| ehr_status_update | 26 | 26 | 36 | 63 | 74 |
| stored_query_execute | 204 | 0 | 38 | 56 | 76 |
| tags_put | 34 | 34 | 29 | 48 | 52 |
| tags_read | 34 | 34 | 28 | 42 | 62 |
| template_get | 85 | 3 | 60 | 93 | 201 |
| template_list | 85 | 0 | 18 | 24 | 34 |
| ward_query | 204 | 0 | 7963 | 9339 | 10887 |

Resources (measured context, never a verdict input) — sampled every 10 s; CPU/RSS derived over the measured phase:

| Container | CPU mean | CPU peak | RSS peak |
| --- | --- | --- | --- |
| sut `cnf-ehrbase-java-ehrbase-java-1` | 4.6% | 11.0% | 1.4 GB |
| db `cnf-ehrbase-java-ehrbase-java-db-1` | 103.5% | 289.5% | 2.5 GB |

Disk anchors: empty 77 MB → after scale seed 7.2 GB (≈ 7.1 KB / composition over 1,000,000 committed) → after ward seed 7.8 GB → after window 7.8 GB.

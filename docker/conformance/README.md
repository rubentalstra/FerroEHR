# Dockerised CNF conformance runs

Run the openEHR CNF suite against a **containerised** server (the deployable
artefact) and write a per-edition report set under `docs/conformance/<edition>/`.

```bash
docker/conformance/run.sh java   # EHRbase Java 2.33.0 (reference) → docs/conformance/java/
docker/conformance/run.sh rs     # ehrbase-rs (this project)       → docs/conformance/rs/
```

Each run:

1. `docker compose --profile <rs|java> up` — the app image + its PostgreSQL, on a
   fixed port (rs 8090, java 8091), ADMIN API activated, Basic auth
   `ehrbase`/`ehrbase`.
2. waits for the server to answer HTTP,
3. runs the CNF via the light CLI over `--base-url` (no `self-host` feature — no
   Cedar/testcontainers build), with `--auth` / `--admin-auth`,
4. tears the stack down.

The framework is **server-agnostic** — the same 322-case schedule runs against
both editions, which is what makes `CNF_COMPARISON.md` (repo root) a fair
head-to-head. Images are pinned by tag in `docker-compose.yml`; override with the
`EHRBASE_*_IMAGE` env vars (e.g. to pin a digest for a published result).

For the fast, no-Docker inner loop on the Rust edition (in-process app + a
PostgreSQL testcontainer):

```bash
cargo run -p conformance --features self-host --bin conformance -- \
  run --self-host --out docs/conformance
```

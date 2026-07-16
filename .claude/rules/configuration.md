---
paths:
  - "app/ehrbase/src/config/**"
  - "app/ehrbase/src/**/config.rs"
  - "app/ehrbase-rest/src/config.rs"
  - "app/ehrbase-rest/src/**/config.rs"
  - "app/ehrbase/assets/ehrbase.default.toml"
---

# Configuration discipline — one `ehrbase.toml`, one loader (W-13)

No openEHR spec governs configuration — this is entirely our own design.
The implementation contract is the loader itself
(`app/ehrbase/src/config/`) + the documented default file
(`app/ehrbase/assets/ehrbase.default.toml`) + the user book page; this rule
is the standing guard. **Cite the openEHR spec only where a config value is
spec-adjacent** (e.g. signing modes → RM common master06 §Digital Signature);
everywhere else flag "no openEHR spec governs configuration — our own design".

## The invariants (do not regress)

- **One serde root.** `ehrbase::config::EhrbaseConfig` is the ONLY configuration
  root. Each section struct lives in the crate that consumes it and is a field
  of the root. **Never add a second config root, a `figment`/`config` chain, or
  a per-subsystem `EHRBASE_*_CONFIG` file pointer.** `figment` is banned from the
  workspace — `grep -rn figment` over the manifests must stay empty.
- **No per-struct `load()`.** Section structs are plain
  `#[serde(default, deny_unknown_fields)]` data with a `Default` that is the
  single source of defaults. Loading happens once, in `ehrbase::config::assemble`
  (a pure `fn(file, env_map, overrides)`), behind the process-env shim
  `ehrbase::config::load`. Subsystem constructors take the typed section by
  value/ref — never read the environment themselves (no `std::env::var`, no
  `LazyLock` env reads).
- **One env grammar (P-4).** `EHRBASE` + the TOML path, `__` between *every*
  segment boundary — including after the `EHRBASE` prefix
  (`EHRBASE__DB__MAX_CONNECTIONS`, `EHRBASE__AUTH__OIDC__ISSUER`) — with single
  `_` only inside a key word. This is mechanical and reversible; never add a
  bespoke env mapping. List-typed keys are comma-separated and registered
  in `alias::LIST_KEYS`. `DATABASE_URL`/`RUST_LOG` are the only non-`EHRBASE_`
  names, and they sit *below* their `EHRBASE_` forms.
- **Strict by default (P-5).** Unknown keys (file and the `EHRBASE_` env
  namespace) are boot errors with did-you-mean; type errors carry provenance;
  semantic checks are aggregated in `EhrbaseConfig::validate` (all at once).
  Never make a bad/misspelled value a silent default.
- **Secrets never render (P-6).** Every secret field is `ehrbase_sm::Secret`
  (or `SecretUrl` for credential-bearing URLs) with a `*_file` sibling resolved
  by the loader. Redaction is a property of the type — never a per-endpoint
  redactor list. A new secret field MUST use these types + add a `*_file` sibling.
- **Zero-config boot (P-2).** An empty file and empty env must boot with
  dev-appropriate defaults; the dev-default `db.url` is announced with a boot
  `warn` (never a silent production trap).

## When you change the schema

1. Edit the section struct in its owning crate (`#[serde(default,
   deny_unknown_fields)]`, `Default` is the source of truth).
2. If you add/rename a key, update the annotated template
   `app/ehrbase/assets/ehrbase.default.toml` **in the same change** — the
   template-sync tests (`config::tests`) fail otherwise. There is NO legacy
   alias layer (greenfield, owner ruling 2026-07-15): renamed/removed keys
   simply fail at boot via the strict sweep's did-you-mean; never add a
   remapping table.
3. If the change is user-visible, update
   `website/book/src/installation/configuration.md`, the Helm `values.yaml`
   `config:` block + golden renders (`deploy/helm/validate.sh --update`), and
   `CHANGELOG.md` — same PR.
4. The default TOML + the book page are the schema of record; reconcile
   the deployment/compose spellings against them.

## Tests

Config tests drive the pure `assemble(file, env_map, overrides)` seam with
injected env maps + `assert_fs` temp files — **never** the process environment.
Every section gets an env-mapping test (the missing test class is what let a
documented-but-dead env form ship historically). Never weaken these to pass.

# v3.17.4 — the multi-generation spec-version program (#1936)

Working plan for issue #1936: user-selectable BASE/RM specification version —
stable + development generations side by side, selected at the application
level. The issue body is the contract; the settled design + owner rulings live
in the #1936 comments (2026-08-05). This file is the deep working plan and is
DELETED in the PR that closes the last child issue.

## Settled decisions (do not re-litigate)

- **One coupled profile**: `spec_profile = "stable" | "development"` selects
  the generation SET together — owner HARD RULE (2026-08-05, extended the
  same day to LANG): `development` = RM 1.2.0 + BASE 1.3.0 + LANG 1.1.0
  (current pins); `stable` = RM 1.1.0 + BASE 1.2.0 + LANG 1.0.0 (the
  released generations). Per-component free choice rejected — incoherent
  combinations stay unrepresentable.
- **One uniform structure for every generated crate** (no package is
  special): version-named generation modules, the codegen composition table
  as the single authority for which generations exist, an emitted per-crate
  `Generation` enum (derived `Default` marking the current generation,
  per-variant `spec_version()`/`as_str()`, `FromStr`/`Display`), prelude re-exporting the current generation only,
  runtime dispatch in the APPLICATION (generated crates stay dispatch-free).
- **Migration order**: (a) uniform-structure rename, zero behaviour change →
  (b) emit released RM 1.1.0 / BASE 1.2.0 side by side → (c) the application
  profile seam. Each step is a child issue; (b) blocked-by (a), (c)
  blocked-by (b). All milestoned v3.17.4.

## The Rust best-practice bar (owner directive 2026-08-05 — applies to every
## remaining child and to ALL emitted API surface)

The program ships PUBLISHED crate API; every new or emitted item is held to
the official conventions — the Rust API Guidelines checklist
(<https://rust-lang.github.io/api-guidelines/checklist.html>), the Rust Book,
and RFC 505/1574 (already binding via `.claude/rules/comments.md`) — with the
emitter producing that quality BY CONSTRUCTION, never via suppressions:

- **Standard names, never bespoke ones (C-CASE, C-GETTER, C-CONV):** the
  canonical string form of an enum is `as_str()` (the std spelling — `str`,
  `ParseError` families), never an invented accessor (`module()` was removed
  for exactly this, 2026-08-05); conversions follow `as_`/`to_`/`into_` cost
  semantics; no `get_` prefixes.
- **`Display`/`FromStr` are a round-tripping pair (C-STR):** `Display`
  forwards to `as_str()`; `FromStr::Err` is a dedicated `…ParseError` type
  shaped like std's `ParseIntError` — a struct with private fields, `Display`
  naming the valid tokens, implementing `std::error::Error` (C-GOOD-ERR:
  errors are types, not strings).
- **Common traits derived eagerly (C-COMMON-TRAITS):** small closed enums
  (`Generation`, the profile enum) derive
  `Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord`; ordering =
  declaration order = oldest generation first, which is the meaningful order.
- **Built-in std mechanisms over bespoke items, and no speculative API:**
  the current generation is the derived-`Default` `#[default]` variant (std
  1.62 mechanism), never a bespoke `CURRENT` const (removed 2026-08-05); the
  unconsumed `ALL` variant-list const was removed the same day (the official
  guidelines prescribe NO variant-enumeration shape — verified first-hand
  2026-08-05 — so shipping one before a consumer exists is speculative
  surface; when one is needed it arrives as the ecosystem-standard shape,
  a deliberate minor-version addition); pure accessors are `const fn` +
  `#[must_use]`.
- **Deliberately exhaustive enums:** the `Generation` and profile enums are
  NOT `#[non_exhaustive]` — the set comes from the composition table and a
  new generation is a deliberate, semver-visible API event; consumers should
  be forced to handle it (recorded decision, not an omission).
- **Doc comments per RFC 1574:** method summaries in third person
  ("Returns…"), one sentence, blank line, then detail — the same bar
  `comments.md` enforces.
- **Emitted code passes the full workspace clippy bar by construction**
  (pedantic included): a lint finding in generated output is an emitter bug,
  fixed structurally (e.g. an 8-argument emitter fn becomes a context
  struct; identical match arms fold into or-patterns), never `#[allow]`ed
  away beyond the adjudicated verbatim-spec-prose exceptions in the
  generated `lib.rs` header.
- **No fixed crate-level pins that contradict selection:** removed
  (owner ruling 2026-08-05) — a multi-generation crate exposes its pins ONLY
  through the `Generation` enum and per-generation-module `SPEC_VERSION`s;
  any future "one value for the whole crate" convenience is the same legacy
  class and gets refused.
- **Legacy residue discovered en route is never carried** (owner directive
  2026-08-05): remove it properly in-scope, or file it as a sub-issue of
  #1936 so it queues inside the program.

## Ground facts (verified 2026-08-05)

- The composition table is `tools/openehr-codegen/src/plan/composition.rs`
  (`COMPOSITIONS`): today one entry per crate-or-variant (`base`, `rm`,
  `lang`, `am14`, `am24`, `term`), `variant: Option<&str>` only for AM, LANG
  as ONE entry with two `own` files and last-wins prelude ownership
  (`Generation::owned`).
- Three emit shapes exist in `render/emit.rs` and `cli.rs::cmd_emit`:
  `emit_crate` (single), `emit_generations` (LANG: disjoint-path generations
  + one collision-filtered prelude), `emit_multi_crate` (AM: `am14`/`am24`
  prefix modules, `include_prelude: false`). The uniform design collapses
  these into ONE path.
- `SPEC_VERSION` is a crate-level const today (openehr-am carries "2.4.0",
  the am24 pin — a single-const shape that cannot describe two generations).
- `json_serde.rs` is one file per crate covering all generations (AM's
  mentions am14+am24 paths ~2000 times) — it survives the rename as a path
  rewrite inside the emitter.
- **RM 1.1.0 and BASE 1.2.0 BMM JSONs are ALREADY VENDORED** — the whole
  ITS-BMM `components/` tree is mirrored verbatim under
  `tools/openehr-codegen/vendor/bmm/` (PROVENANCE.md, pinned 2026-07-04
  @ `37b31739`). Child (b) is composition-table + emission work, not
  fetching. Caveat: BASE 1.3.0 currently comes from the component-repo
  exception; BASE 1.2.0 / RM 1.1.0 come from the ITS-BMM mirror — provenance
  is already recorded, but the RELEASED files were never codegen inputs
  before ("pins = the versions available as clean `*.bmm.json`",
  `docs/VERSIONS.md`), so (b) opens with `openehr-codegen -- check` triage
  over both files; any upstream defect is adjudicated (override with
  citation, or an `upstream-report` issue) — never a trimmed closure.
- Spec crates are at 0.0.4; any packaged-content change bumps all eight to
  0.0.5 lockstep in the same PR (`crate-version-guard`).

## Child (a) — refactor(codegen): the uniform multi-generation structure

Zero behaviour change (wire formats byte-identical); one big mechanical PR.

Design (in-session, critical path):

1. **Composition table redesign**: one entry per CRATE; each entry carries a
   `generations: &[GenerationSpec]` list — vendored file, generation module
   name, per-generation spec version, `current` marker. Module names derive
   from the spec version (`1.2.0` → `v1_2`); LANG's two generations are the
   BMM meta-model majors `v2`/`v3` (both files are LANG 1.1.0 — the version
   rule cannot name them), so the module name is explicit table data with a
   derivation convention, reviewed not computed.
2. **One emit path**: `emit_crate`/`emit_generations`/`emit_multi_crate`
   collapse into a single generation-list renderer. Every generated crate —
   including single-generation `base`/`rm`/`term` — gains its version module
   now (`openehr_base::v1_3`, `openehr_rm::v1_2`, `openehr_term::v3_1`,
   `openehr_am::v1_4`/`v2_4`, `openehr_lang::v1_1`), so child (b) is
   purely additive (no second workspace sweep).
3. **Emitted `Generation` enum** per crate (from the same table): variants
   per generation (`V1_2`, …), `Generation::CURRENT`, `spec_version()`,
   `FromStr`/`Display`. Crate-level `SPEC_VERSION` stays and equals
   `CURRENT.spec_version()` (the published-crates pin datum in
   `docs/VERSIONS.md` keeps meaning).
4. **Preludes**: re-export the CURRENT generation only. LANG's
   collision-driven one-type-per-name prelude is REMOVED (v3 is current;
   the v2 twin is reachable by full path only). AM's per-variant preludes
   become the uniform shape (crate prelude = current = v2_4; wait — AM's
   current is am24/ADL2 while ADL 1.4 remains fully in use: `CURRENT` is a
   table datum per crate — AM's current = v2_4, consumers of v1_4 import by
   full path, exactly as today's `am14::` paths do).
5. **Hand-written modules stay outside generation modules**: LANG's
   `odin`/`lexer`/`el`/`bel`/`escape`/`position`, BASE's `serde_support` and
   `containers`, and the `*_impl.rs` siblings move WITH their generation
   (they implement generation-specific types) — the emitter's
   `sibling_impls` preservation must track the new paths.
6. **Consumer sweep**: workspace-wide import rewrite (`openehr_rm::X` →
   `openehr_rm::v1_2::X`, `openehr_am::am14::` → `openehr_am::v1_4::`,
   `openehr_lang::{prelude,bmm3,…}` → `openehr_lang::v3::…`, etc.), incl.
   the generated cross-crate references (prelude index in codegen) and the
   openehr-its generated codecs/dispatch. Mechanical; fanned to ≤2
   implementer subagents with file fences. Blast radius: see the survey
   section below.
7. Lockstep 0.0.5 bump of the eight `crates/*`; CHANGELOG entry (module
   paths of published crates change — user-visible); no `docs/VERSIONS.md`
   policy change yet (that is (c)); codegen-drift green.

## Child (b) — feat(codegen): emit released RM 1.1.0 + BASE 1.2.0 side by side

1. `openehr-codegen -- check` over `openehr_rm_1.1.0.bmm.json` and
   `openehr_base_1.2.0.bmm.json`; triage every defect first (adjudicated
   override with citation / upstream-report issue). The released files were
   never inputs before — this is the risk gate of the whole program.
2. Table: `rm` gains `v1_1` (spec 1.1.0), `base` gains `v1_2` (spec 1.2.0);
   `current` stays `v1_2`/`v1_3`. RM v1_1 composes against BASE v1_2 (the
   coherent pairing — the table records it; the resolution model of one
   generation must resolve against its PAIRED dependency generation, not
   the crate-level current view: `compose()` grows per-generation
   `model_deps` generation selection).
3. Complete emission of both generations (never trimmed); `json_serde`,
   `emit-validate`, `emit-rm-model` become generation-covering (each
   generation's model/validate subtree under its module). The `_type`
   dispatch + declared-key tables in `openehr-its` and the XML/REST emits
   stay CURRENT-generation-only in (b) (codecs for the stable generation are
   wired in (c) where the seam exists — until then nothing consumes v1_1).
   If (c)'s design instead needs the codecs emitted here, that lands in (b)
   as a follow-up commit — decide at (c) pickup, recorded on the issue.
4. Hand-written `*_impl.rs` siblings for the v1_1/v1_2 generations: audit
   which impls the older generation needs (invariants, paths); write them
   (they are hand-written spec behaviour, per-generation by design).
5. `docs/VERSIONS.md` matrix rows RM/BASE list both pins (released +
   development); zero behaviour change for current-generation consumers;
   lockstep bump; CHANGELOG.

## Child (c) — feat(app): the `spec_profile` seam

1. Config: `spec_profile` in the `ferroehr` config tree (+ `FERROEHR_*`
   env), a two-variant profile enum mapping to the per-crate `Generation`
   selectors — coherent pairs only, unrepresentable otherwise.
2. Thread the profile through the generation-sensitive seams: canonical
   JSON/XML codec selection, the RM attribute model the AQL planner reads,
   validation/invariant passes, WebTemplate/FLAT conversion. Dispatch lives
   in `ferroehr`/`ferroehr-rest`; generated crates stay dispatch-free; no
   shadow models.
3. Wire honesty: active generation visible on the served surface (native
   OpenAPI metadata + conformance statement). Mixed-generation storage
   position adjudicated and recorded (within-major supersets: newer reader
   accepts older instances; the reverse direction is the design question —
   spec-cited adjudication on the issue).
4. `docs/VERSIONS.md` §Spec version policy REWRITTEN to the two-generation
   policy with the owner ruling (2026-08-05) recorded; website
   (`website/book/src`) configuration page for `spec_profile` in the same
   PR; CHANGELOG.
5. Acceptance instrument: `bash scripts/conformance.sh` green on BOTH
   profile selections; zero drift; full gate battery. This PR deletes this
   plan file.

## Blast-radius survey (measured 2026-08-05, grounds the (a) fan-out)

- **Prelude consumers are the majority and cost ZERO churn** — the prelude
  path survives (current generation only): `openehr_rm::prelude` in 44
  ferroehr + 15 ferroehr-rest + 25 openehr-its + 2 ext + 6 codegen files;
  `openehr_base::prelude` in 30 ferroehr + 5 rest + 29 adl + 10 its files.
- **Direct-module consumers get the `v*_*` prefix** (hand-written only —
  generated cross-refs are the emitter's job): rm-direct ≈ 17 ferroehr + 2
  rest + 21 its + 2 adl files; base-direct ≈ 8 ferroehr + 5 adl + 9 its +
  21 lang-hand-written + 3 codegen files.
- **`openehr_am::am14/am24` outside the crate**: 53 refs in openehr-adl,
  5 ferroehr, 4 its, 2 codegen — small sweep.
- **`openehr_lang`**: `prelude` 278 refs (36 openehr-am files — generated,
  emitter-rewritten — + 19 adl files hand-written). v2-only names drop out
  of the prelude (v3 = current) → those consumers move to
  `openehr_lang::v2::…` full paths. Hand-written top modules
  (`odin` 43, `lexer` 16, `escape` 10, `bel` 7, `el`, `position` 2 refs)
  stay top-level — zero churn. `bmm` 42 + `beom` 2 refs → `v2::…`.
- **Generated cross-crate references switch from dependency-PRELUDE paths
  to explicit generation-module paths** (`External` index change): RM v1_1
  must bind BASE v1_2 — the current-gen prelude cannot express that pairing,
  so generated code references `openehr_base::v1_3::…` (resp. `v1_2`) full
  paths. This is the load-bearing emitter change of (a).
- **Hand-written `*_impl.rs`/spec-behaviour files inside generated crates**
  (move with their generation module; (b)'s older generations need their
  own): base 28, rm 66 (incl. `paths.rs`, the `validate/` subtree), lang 39
  (incl. `bmm_persistence` loader machinery — v2-generation code), term 2,
  am 0. `openehr_base::serde_support`/`containers` are cross-generation
  runtime and stay top-level (adjudicate at (a) pickup).
- **`openehr_rm::model` consumers** (the AQL planner's oracle): 8 app files
  (aql/, storage, validation/opt), 2 adl, ~10 its; `openehr_rm::validate`:
  3 app + ~7 its. These become generation-scoped paths in (a) and the (c)
  dispatch seam later.

## Gates (every child)

fmt · the three CI clippy lanes + slim lanes · comment-guard `--all` ·
rustdoc (`-D warnings`) · deny · `nextest --workspace` · codegen-drift ·
cnf-validate + coverage drift; (c) additionally the dual-profile CNF runs.
Merge on local gates green; CI is the post-merge backstop.

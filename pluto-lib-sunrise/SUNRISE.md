# PlutoLib — Sunrise variant

This tree is the Sunrise-adjusted variant of PlutoLib. `pluto-lib/` (in the
repository root) is the horizon-rust master; this tree was seeded from it by
`~/workspace/sunrise-convert/convert.py` (mechanical regex conversion) and is
then **hand-adjusted and maintained in place** — never re-seed over it.
Consumers (Tinkers / Pipes / Stellarsky) stay single-source on the public
API; only PlutoLib has two trees.

## Pinned target

- Sunrise revision: **ffb8601b02db279261c4e9bbfd977b4b2e3892ab**
  (latest Mod SDK release `sdk-ffb8601b…`, 2026-11-25; game "26.3").
  The tree was seeded against the previous SDK release
  `3d34bbe63d71bb24c098c1e4021ef866be12fe2d` (2026-11-24) and retargeted to
  ffb8601b by the checkpoint below; per owner decision (2026-11-25) only the
  latest SDK is supported — older generations are dropped. Per the
  Sunrise-latest research, the porting unit is an SDK release, not raw main.
- SDK evidence (`sdk.json` of ffb8601b): every pin field is identical to
  3d34bbe — `mixins.rev = e59da9094f0c2fdda6c627fe9eff22dec6a906f0` (the
  same horizon-mixins pin as the Horizon variant) + mixins patch
  `c4a46bf9…`; `toolchain.rust = 1.99.1` (this repo's Horizon toolchain is
  1.95.0 — do not confuse them); `content_api = "0.2"`;
  `package_format = 6`. The delta is API surface only (~2,128
  `surface.changes` entries: Attachments rework, mass hook-version bumps,
  capability-provider rework, some moved mixin targets).
- Release naming (owner rule, 2026-11-25): the Sunrise variant's release
  name carries "sunrise":
  **`pluto-lib-sunrise-v0.1.0-sunrise.ffb8601b-mixins.e59da909`**.
  Signing on Sunrise is GitHub artifact attestation + `.sigstore.json`
  produced by the registry's shared workflow — authors hold no keys;
  the Horizon variant's minisign plan does NOT carry over.

## What the conversion did (2026-11-25)

1. **Mechanical seed** (convert.py, rules as drafted + 6 rules added at
   conversion time): 856 files, 325 changed, 1,989 replacements
   (crate renames 1,533; target paths 388; manifest 59; repo URLs 9) +
   150 further renames from the added rules (`horizon_client_core`,
   `horizon_core`, `horizon_protocol` and kebab forms — real code paths the
   draft had missed; the target crates sunrise-client-core / sunrise-core /
   sunrise-protocol all exist at the pinned revision). 19 manifests renamed
   `horizon-mixin.toml` → `sunrise-mod.toml`. The mixins pin line and the
   horizon-mixins URL were deliberately left untouched.
2. **Manifest restructure** (all 19 manifests, TOML-validated): the Horizon
   header (`[mod]` / `[compatibility] horizon_revision` / `[api]`) became
   the Sunrise form — top-level `id` / `name` / `version`, `api = "0.2"`,
   `side = "both"`, `[native] game = "26.3"`, `sunrise = "3d34bbe…"`,
   `mixins = "e59da909…"`. `[[injection]]` → `[[native.mixin]]` (main
   manifest: 170 rows), `[[module]]` → `[[native.module]]` (141 rows).
3. **Quarantine**: `[[state]]` / `[[state.constructor]]` rows (32 across the
   tree) and `[[crate]]` rows (11) were moved verbatim into
   `sunrise-quarantine.toml` files beside their manifests (6 files).
   `[[native.state]]` is **refused** by the Sunrise package reader — state
   must migrate to mod_api Attachments. `[[crate]]` has no manifest
   equivalent — those API crates become Cargo dependencies of the native
   build. The `[api]` package section likewise has no Sunrise manifest
   equivalent; the `mod/api` tree itself is unchanged.

## Verification so far

- Master `pluto-lib/` untouched by the seed (git status clean).
- All 19 restructured manifests parse as valid TOML.
- Target-anchor census against the pinned Sunrise tree: all **39**
  `sunrise_client_lib::` targets' anchor modules exist in
  `crates/sunrise-client/src/client_lib.rs` (the crate is not hollow — its
  `[lib] path` points at that file, as on Horizon). Of the module anchor
  files, 118/141 exist at the seeded path; 23 moved with the `game/` area
  reorganisation (see Remaining, item R1).
- Repo lints on the combined tree: check_api_docs 0 problems,
  check_injection_ownership 0 problems, check_line_endings 0 problems
  (after normalising the seeded tree to LF — the repo convention).
  check_docs_budget (46) and lint_forbidden_api (26) flag the new tree
  through path-keyed baselines and allowlists written for `pluto-lib/`
  only — tooling adaptation is item R6.

## ffb8601b checkpoint (2026-11-25, branch `pluto-lib-sunrise/sunrise-ffb8601b-mixins-e59da909`)

Worklists and logs for everything below live in the lane scratch
`~/workspace/pluto-lib-sunrise-latest/` (census-report.txt, r1-worklist.csv,
replaceable-list.txt, sdk-diff.txt, STATUS.md).

- **R1 — census done against the ffb8601b clone**
  (`A:\Project\sunrise-src-latest`, rev-parse verified; the 3d34bbe clone
  stays at `A:\Project\sunrise-src` for `sdk diff`). Modules: 216 rows —
  187 anchor files exist, 29 moved/gone with the `game/` area
  reorganisation (basename candidates found for most; `game/creatures.rs`,
  `game/remote.rs`, `game/maps.rs` and sunrise-world `attachments.rs` have
  none — their rows re-derive). **40 module rows anchor outside
  `crates/sunrise-client/src/`** and are refused as woven
  (E_MIXIN_MODULE_DESTINATION): fold the support code into the dependent
  helpers or re-anchor where the consumer lives. Mixins: 236 rows — 187
  resolve in-crate, 28 fn-not-found (several are census parser artifacts on
  `<Type as Trait>` targets; the real signal is ~12 `native_*` hook entry
  points to re-derive or substitute via mod_api). The lower-crate
  `#[replaceable]` surface is tiny (88 markers, 71 fns extracted);
  **21 rows are refused as woven** (e.g. `Items::mining_speed`,
  `ChunkMap::clear`, worldgen `Decoration::decorate`) and must retarget to
  a replaceable caller/callee or a mod_api primitive. Retargets whose new
  anchor is already verified are applied in this checkpoint (e.g.
  `pluto-living-load-access` moved `entry.rs` → `client_lib.rs`, with its
  consumers in hooks.rs and pluto-living-attributes-join.rs repointed to
  `sunrise_client_lib::pluto_living_load_access`).
- **R2 — merged cargo-sunrise-mod project + weave plan.** All shipped
  bundles are merged into ONE project rooted at `mod/`: 317 woven files
  under `mod/src/`, `mod/api` folded into the root crate (package
  `mod-pluto_lib`, `[lib] name = "pluto_lib"`; the sunrise-client git dep
  is refused by the tool and is gone), internal API crates stay path deps
  (pluto-inventory-api, pluto-movement-api, pluto-item-api). Colliding
  mixin/module ids and file basenames were bundle-prefix renamed.
  `cargo sunrise-mod check` parses the merged manifest.
  **Weave plan verdict: 296 of 421 rows plan-clean; the remaining 125 rows
  are the redesign list** (the R1 refusals above: outside-crate anchors,
  non-replaceable targets, missing `native_*` entry points).
- **R3 — OVERWRITE fingerprints re-derived** from the plan runs'
  `E_FINGERPRINT` `got` values and applied (scratch harvest_fp*.py /
  apply_fp.py) — never computed from source text.
- **R4 — state rows recounted for the shipped set: 10 `[[state]]` rows**
  (mod/ 5, pluto_native_creative 3, pluto_creative 1, guide-lifetime 1) +
  5 `[[crate]]` rows (→ path deps; 3 of 5 wired in the scaffold). The 32
  figure in the seed section counted examples/experiments too. The state
  rows are woven-side scratch on internal game objects (ChunkMap ×3,
  Navigation, ModBlockEntity, KeptEntry, CombatTracker, Creative) — they
  are NOT mod_api attachment-shaped (no Holder), so the migration target
  is **support-module-owned statics keyed by the identity the field hung
  on**, not Attachments. None blocks the weave plan (already quarantined).
- **R5 — bridge inversion is the main follow-up.** Measured on hooks.rs:
  of 124 mod_api names the Horizon bridge imports, 80 exist at ffb8601b and
  **44 are missing** (list in STATUS.md). The structural finding: on
  Horizon, hooks.rs registers staged handlers INTO the `pluto_*` support
  modules; on Sunrise the mod crate cannot name the game crate, so the
  direction inverts — **the support modules must call INTO `pluto_lib`,
  and game-type adaptation moves from hooks.rs into the modules/helpers**.
  The pure registries already live in the API, so the inversion is
  systematic; it is the next lane after this checkpoint.
- **R6 — still open** (path-keyed tooling baselines, as above).
- **R7 — partially scoped**: `cargo sunrise-mod check` release notes flag
  icon.png, license and page.toml as missing; the prose pass is still open.
- **R8 — not started** (registry v2 registration + release; only the
  signed v1.0.0 ships, per owner rule).
- **Toolchain (PC)**: the SDK targets **x86_64-pc-windows-gnu** (the
  1.99.1 msvc install does not satisfy it) and **rustup.exe crashes in SSH
  sessions** (0xC0000005, any subcommand). Working setup: the standalone
  tarball `rust-1.99.1-x86_64-pc-windows-gnu` merged into
  `A:\Project\sunrise-rustup-home\toolchains\1.99.1-x86_64-pc-windows-gnu`
  — no rustup involved; its `bin\` on PATH satisfies the tool's
  rustc probe. C: filled during the first install attempt (0.58 GiB free),
  so RUSTUP_HOME / TEMP / TMP live on A:.
- **Pure suite: green** with the gnu toolchain (private CARGO_TARGET_DIR):
  **427/427 unit tests + 1307/1307 doctests (1 ignored)**. Getting there
  took two path corrections of the same conversion class:
  `mod/src/pure/resource_loader_v0.rs`'s `#[path]` for
  pluto-reload-registry-access.rs (`../../` → `../` after the merge moved
  the file under `mod/src/`), and the living-load facade in
  `tests/pure/src/living_load_host_tests.rs` gaining the
  `sunrise_client_lib` shape the R1 retarget introduced.
- **Sync note**: this branch's master copy is pre-fix-wave (base
  a5375a643). When PlutoLib v1.0.0 lands, the manager runs
  `convert.py --sync-check` + a delta pass; nothing in this checkpoint
  pre-empts it.

## Remaining (precise; in dependency order)

- **R1 — Anchor re-derivation (host drift).** Census against ffb8601b is
  DONE and verified retargets are applied (see the checkpoint above). What
  remains is the 125-row redesign list: 40 module rows anchored outside
  `sunrise_client` (fold into helpers or re-anchor), 21 non-replaceable
  lower-crate targets (retarget to a replaceable caller/callee or a
  mod_api primitive), ~12 `native_*` entry points (re-derive or
  substitute), and the anchorless rows (creatures/remote/maps/attachments).
- **R2 — Weave plan / build.** The merged `cargo sunrise-mod` project
  exists and the plan verdict stands at 296/421 rows (checkpoint above).
  What remains is burning down the 125-row redesign list in clusters with
  the pinned Sunrise source as canon, then the full build.
- **R3 — OVERWRITE fingerprints.** Done for the rows the plan reaches
  (checkpoint above); re-derive any remainder from `E_FINGERPRINT`'s `got`
  during the build — never compute from source text.
- **R4 — State migration.** The shipped set is **10 quarantined state
  rows** (recount in the checkpoint above): they are woven-side scratch on
  internal game objects, not attachment-shaped, so redesign each as a
  support-module-owned static keyed by identity — not as a mod_api
  Attachment. (Where a future row IS holder-shaped, the Attachments route
  with `AttachmentDescriptor` / `AttachmentKey` applies; mind gap G6 —
  attachment codecs get no Host access at save time.)
- **R5 — Bridge inversion + API-substitution pass.** First the inversion
  (checkpoint above): support modules call INTO `pluto_lib`; hooks.rs's
  game-type adaptation moves into the modules/helpers; the 44 missing
  mod_api names are the worklist. Then the substitutions (Sunrise team
  guidance: check covered gaps first; public API signatures must not
  change — only helper bodies):
  menu reaches (13 `pluto-menu*` ids in the main manifest) → MenuScreen /
  register_menu_screen / MenuPart; access support modules (42 `*access`
  ids) → `[[native.bridge]]` GET/GET_MUT/SET/INVOKE declarations;
  block-entity tick reaches → the published loop
  `[[native.loop]] block_entities.tick`; per-frame item models →
  SpecialModelFactory; particles → ParticleBatch/register_particle_type;
  fluid interactions → Registrar::fluid_interaction; projectile ownership
  → EntitySpawn.owner. Gaps with no cover stay as mixins; the primitive
  that would retire each is listed in the Sunrise-latest report §4
  (G1–G16, N1) — join the existing upstream issues rather than filing new
  ones (N1 excepted).
- **R6 — Tooling adaptation.** Line-ending/docs-budget/forbidden-API
  baselines are keyed to `pluto-lib/` paths; add the Sunrise tree's entries
  or give the tree its own baseline set. The plan/boundary tooling under
  `pluto-lib-sunrise/tools/` still speaks Horizon (`horizon_source`
  arguments, `.local/upstream` layout) and needs its Sunrise mode.
- **R7 — Prose pass.** Capital-H "Horizon" naming the game in docs/comments
  (hundreds of mentions) — editorial, human-reviewed; historical references
  stay. The `horizon_cit` fixture namespace and `__horizon_mixins_*`
  generated symbol names are NOT prose — the weaver is still
  horizon-mixins, so generated names stay as they are.
- **R8 — Registry v2.** Registration (numeric repo ID + 8-char project ID)
  in sunrise-org/sunrise-mods, then `cargo sunrise-mod release`; native
  mods ship source + Cargo.lock and are built on the player's machine.

## Maintenance

When the master changes: `convert.py --source pluto-lib --sync-check
pluto-lib-sunrise --set SUNRISE_REVISION=<rev>` reports drift; port by hand,
preserving this tree's adjustments. On an SDK bump, also run
`cargo sunrise-mod sdk diff <old> <new>`.

# PlutoLib — Sunrise variant

This tree is the Sunrise-adjusted variant of PlutoLib. `pluto-lib/` (in the
repository root) is the horizon-rust master; this tree was seeded from it by
`~/workspace/sunrise-convert/convert.py` (mechanical regex conversion) and is
then **hand-adjusted and maintained in place** — never re-seed over it.
Consumers (Tinkers / Pipes / Stellarsky) stay single-source on the public
API; only PlutoLib has two trees.

## Pinned target

- Sunrise revision: **3d34bbe63d71bb24c098c1e4021ef866be12fe2d**
  (latest Mod SDK release `sdk-3d34bbe6…`, 2026-11-24; game "26.3").
  Per the Sunrise-latest research, the porting unit is an SDK release, not
  raw main (main HEAD e250d0545… at conversion time).
- SDK evidence (`sdk.json` of that release): `mixins.rev =
  e59da9094f0c2fdda6c627fe9eff22dec6a906f0` — the same horizon-mixins pin as
  the Horizon variant; `toolchain.rust = 1.99.1` (the SDK build installs it
  via rustup; this repo's Horizon toolchain is 1.95.0 — do not confuse them);
  `content_api = "0.2"`; `package_format = 6`.
- Release naming (owner rule, 2026-11-25): the Sunrise variant's release
  name carries "sunrise":
  **`pluto-lib-sunrise-v0.1.0-sunrise.3d34bbe6-mixins.e59da909`**.
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

## Remaining (precise; in dependency order)

- **R1 — Anchor re-derivation (host drift).** 23 `[[native.module]]` anchor
  files moved (e.g. `game/native_hooks.rs` → `crafting/native_hooks.rs`,
  `game/chat.rs` → `game/interface/chat.rs`, `game/creatures.rs` → the
  `game/creatures/` area folder) and 7 `sunrise_client::game::` targets need
  per-file path re-derivation (game/ subfolders are areas, not modules).
  16 targets sit in lower crates (sunrise_world 5, sunrise_client_render 8,
  sunrise_items 2, sunrise_assets 1): each needs a `#[replaceable]` marker
  check, else retarget to a replaceable caller in `sunrise_client`.
- **R2 — Weave plan / build.** The Horizon plan tooling does not read the
  Sunrise manifest grammar; the Sunrise-native path is a `cargo
  sunrise-mod` project (root Cargo.toml + src/) built against the SDK.
  PlutoLib's layout (mod/ + api/ + mixins/ bundles) must be packaged into
  that shape first; then `cargo sunrise-mod check`/`build` gives the plan.
  Expect a large first-failure count (every target string changed at once);
  fix in clusters with the pinned Sunrise source as canon.
- **R3 — OVERWRITE fingerprints.** 4 OVERWRITE rows; their Horizon
  fingerprints are void on Sunrise. Re-derive each from `E_FINGERPRINT`'s
  `got` during the R2 build — never compute from source text.
- **R4 — State → Attachments migration.** The 32 quarantined state rows:
  redesign each as a mod_api Attachment (`AttachmentDescriptor` /
  `AttachmentKey`) on the matching holder; mind gap G6 (attachment codecs
  get no Host access at save time).
- **R5 — API-substitution pass** (Sunrise team guidance: check covered gaps
  first; public API signatures must not change — only helper bodies):
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

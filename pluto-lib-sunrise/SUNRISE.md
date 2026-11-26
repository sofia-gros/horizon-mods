# PlutoLib — Sunrise variant

This tree is the Sunrise-adjusted variant of PlutoLib. `pluto-lib/` (in the
repository root) is the horizon-rust master; this tree was seeded from it by
`~/workspace/sunrise-convert/convert.py` (mechanical regex conversion) and is
then **hand-adjusted and maintained in place** — never re-seed over it.
Consumers (Tinkers / Pipes / Stellarsky) stay single-source on the public
API; only PlutoLib has two trees.

## Pinned target

- Sunrise revision: **ba1f24e4ad41246b558230aa665539dc768b1287**
  (latest Mod SDK release `sdk-ba1f24e4…`, 2026-11-26; game "26.3").
  The tree was seeded against SDK release
  `3d34bbe63d71bb24c098c1e4021ef866be12fe2d` (2026-11-24), retargeted to
  ffb8601b by the checkpoint below, re-based onto 6a02076f by the L1
  foundation section below (2026-11-26), and onto ba1f24e4 by the
  re-pin 2 section below (2026-11-26); per owner decision (2026-11-25)
  only the latest SDK is supported — older generations are dropped. Per
  the Sunrise-latest research, the porting unit is an SDK release, not
  raw main.
- SDK evidence (`sdk.json` of 6a02076f, read 2026-11-26): every pin field
  is identical to 3d34bbe and ffb8601b — `mixins.rev =
  e59da9094f0c2fdda6c627fe9eff22dec6a906f0` (the same horizon-mixins pin
  as the Horizon variant) + mixins patch `c4a46bf9…`;
  `toolchain.rust = 1.99.1` (this repo's Horizon toolchain is 1.95.0 — do
  not confuse them); `content_api = "0.2"`; `package_format = 6`. The
  deltas between SDK releases are API surface only (3d34bbe → ffb8601b:
  ~2,128 `surface.changes` entries — Attachments rework, mass
  hook-version bumps, capability-provider rework, some moved mixin
  targets; ffb8601b → 6a02076f: 7 breaking / 6 conditional / 34 additive,
  itemised in the L1 section below; 6a02076f → ba1f24e4: 51 breaking /
  40 conditional / 586 additive, itemised in the re-pin 2 section
  below — and `sdk.json` of ba1f24e4, read 2026-11-26, shows every pin
  field identical again, so only the surface moved).
- Release naming (owner rule, 2026-11-25): the Sunrise variant's release
  name carries "sunrise":
  **`pluto-lib-sunrise-v0.1.0-sunrise.ba1f24e4-mixins.e59da909`**.
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
  check_docs_budget (47) and lint_forbidden_api (25) flagged the new tree
  through path-keyed baselines and allowlists written for `pluto-lib/`
  only — adapted by R6 in the L1 section below (both now 0).

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
- **R6 — baselines done by L1** (2026-11-26, see the L1 section below);
  the tools/ Sunrise mode remains open there.
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

## L1 foundation (2026-11-26): SDK 6a02076f + frozen worklist

This section freezes the worklist for the implementation lanes (L2–L7).
Sources: the design lane's `DESIGN.md` / `GAPS.md`
(`~/workspace/sunrise-design/`), re-confirmed against SDK 6a02076f here.
Lane record: `~/workspace/sunrise-l1/STATUS.md`. The classification below
is the version of record; lanes update it in place as verification moves
rows between classes.

### SDK re-confirmation

- Releases API re-crawled 2026-11-26 (the design lane's same-day attempt
  hit a GitHub 403; the L1 retry succeeded). Newest SDK release is
  **`sdk-6a02076f89cd189d74d63492dda5b023fa85a38f`** (2026-11-25T14:22:58Z),
  two releases after ffb8601b (intermediate: `bdad86d2d398…`,
  2026-11-25T04:16:45Z).
- Pins are identical across 3d34bbe / ffb8601b / bdad86d2 / 6a02076f
  (sdk.json read directly for each): game "26.3", mixins `e59da909` +
  patch `c4a46bf9`, rust 1.99.1, content_api "0.2", package_format 6.
  No structural re-evaluation is needed; the delta is digested here.
- `cargo sunrise-mod sdk diff ffb8601b… 6a02076f…` (run on the PC,
  exit 0): surface **breaking — 7 breaking / 6 conditional / 34
  additive, 0 breaking without a note**.
- Delta against this tree (git grep over `pluto-lib-sunrise/`,
  2026-11-26):
  - No references to `render_name_tag` / `RenderNameTag` / `HookId::` /
    `player_destroyed_block` / `register_render_pipelines` /
    `PlayerRef::place_block` — the name-tag rename, the destroyed-block
    v2, the place_block signature change and the WGSL→GLSL pipeline
    change do not touch the seed code.
  - No `hook_version` pins anywhere in the tree — the conditional
    version bumps (entity_place v2, player_destroyed_block v2) stay
    inert until L2 records `uses`.
  - **New hook `fluid_passage`** (server-side, close-only, asked exactly
    where `FlowingFluid.canMaybePassThrough` is checked) is the
    primitive upstream gap **G9** asked for → **G9 is resolved at
    6a02076f** (L5 verifies in-tree and closes the loop on #5087).
  - New `BlockPart::on_place` / hook `on_place`, hook
    `play_break_sound`, hook `is_valid_book_shelf`; **`Registrar` now
    also requires `EnchantingTableOffsetsRegistrar`** (L2: scaffold
    Registrar impls gain the super-trait); `BlockEntityRef::remove`
    added; `ClientViewClientHost::tick_rate` + `ClientTickRate` added.
  - Host content wire changed: `type:Block` wire layout and
    `BlockProperties.spawn_rule` (a code-declared BlockProperties
    literal without `..Default::default()` needs `spawn_rule: None`).
    PlutoLib's own public `BlockProperties` (`src/lib.rs`) is
    unaffected; L2/L3 watch the content-pipeline (a) rows.
  - `player_destroyed_block` v2 is heard on both sides (client: the
    local player's predicted break, ClientHost, oldState,
    block_entity always none). GAPS A4's destroy surface grew, but it
    is still a notification, not a cancellable destroy event with full
    context — the A4 disposition stands.
- No frozen-table row changes class on this delta alone. The table
  below is the design lane's Appendix A verbatim, with the two
  upstream-gap closures (G7 at ffb8601b, G9 at 6a02076f) recorded in
  the gap digest.

### Frozen classification of the 125 plan-failing rows

Method and yardstick are the design lane's (DESIGN §2.1): (a) native
mod-API substitution removes the row, (b) it still weaves after a
`#[replaceable]` retarget / module re-anchor / helper fold-in,
(c) primitive decomposition + Sunrise-side issue (the largest subgroup
is expected to fall to (a) under verification), (d) unreachable.
Counts: **(a) 37 / (b) 14 / (c) 74 / (d) 0**. Plan error codes:
NR = E_MIXIN_NOT_REPLACEABLE, TM = E_MIXIN_TARGET_MISSING,
MD = E_MIXIN_MODULE_DESTINATION.

| id | plan | 処遇 | 根拠 |
|---|---|---|---|
| api | MD | (b) | 畳み込み: 消費ヘルパーのいる sunrise_client 側へ（Items access） |
| pluto-attachment-transient | MD | (a) | Attachments 再設計（Descriptor/Key）が公式の移行先 |
| pluto-block-entity-client-level | MD | (c) | game/remote.rs 消滅。remote/session 系へ再導出 |
| pluto-living-drop-state-module | MD | (b) | 再アンカー: items/combat.rs か mobs/combat.rs |
| pluto-living-drops-game | MD | (c) | game/creatures.rs 消滅。分割先（death_weapon 等）へ再導出 |
| pluto-map-tick | MD | (c) | game/maps.rs 消滅。LevelRef のマップ面で (a) 化の可能性 |
| pluto-projectile-flight-game | MD | (c) | 同上（projectile 系の分割先へ再導出） |
| pluto-projectile-impact-game | MD | (c) | 同上 |
| pluto-reload-registry-access | MD | (b) | 再アンカー: クライアント側レジストリ面の消費者位置へ |
| pluto-reload-registry-source | MD | (b) | 再アンカー: game/interface/chat.rs（census で特定） |
| pluto-skull-model-access | MD | (a) | **L3a done**: module deleted; `Hooks::create_skull_models` registration supersedes it (and L4's fold-in of this helper) |
| pluto-structure-command | MD | (b) | 再アンカー: game/administration/commands/place.rs |
| pluto-throw-sound-game | MD | (c) | 同上 |
| recipe-book-invalidation | MD | (b) | 再アンカー: recipe_book 移転先（census 候補あり） |
| recipe-book-preview | MD | (b) | 同上 |
| recipe-placement-attempt | MD | (b) | 同上 |
| wire-read | MD | (b) | 畳み込み: 同上（item_data access） |
| advancement-item-predicate | NR | (c) | 進捗述語の公開面の粒度を確認 |
| block-color-world-tint | NR | (a) | **L3a done**: `register_block_tint_sources` / `BlockTintSource` registration (PlutoTintSource, 8 layers) |
| crafting-match-veto | NR | (c) | Recipes 内部の置換は crafting hook の粒度確認から |
| crafting-special | NR | (c) | 同上 |
| drops | NR | (c) | ItemPart の drops 面の網羅を確認（GAPS B3-1） |
| max-damage | NR | (c) | スタック同期（LAN guest 変換）の形を再導出 |
| movement-table | NR | (b) | RETARGET: game::api_values::block_state::bind（索引に実在） |
| pluto-block-after-fall | NR | (c) | G12/G4（既存 issue）合流＋呼び出し側付け替え検討 |
| pluto-block-entity-factory | NR | (a) | **L3a done**: Registrar type declaration + `BlockEntityPart::block_entity` (factory weave deleted) |
| pluto-block-entity-revision | NR | (c) | BE 同期リビジョンの公開面を確認 |
| pluto-block-fall-damage | NR | (c) | 同上 |
| pluto-block-inside-motion | NR | (c) | 同上 |
| pluto-block-settings-gameplay | NR | (a) | content 宣言（mod_content の上書きは宣言化） |
| pluto-crafting-match-veto | NR | (c) | 同上 |
| pluto-creative-rebuild | NR | (c) | built_creative_tabs 面で代替可否を確認 |
| pluto-entity-particle-kind | NR | (c) | 着地イベント＋ParticleBatch の組合せを検証 |
| pluto-entity-spawn-add | NR | (a) | finalize_spawn / mob_spawned イベントへ |
| pluto-entity-spawn-conversion | NR | (c) | 変換イベント面を確認 |
| pluto-hanging-stand-tick-head | NR | (a) | EntityTickPre/PostEvent（種別フィルタ） |
| pluto-head-loot-gate | NR | (c) | loot の roll 面を確認（LootDropsEvent の実在確認と対） |
| pluto-item-after-fall | NR | (c) | 同上 |
| pluto-item-color-layers | NR | (c) | ItemRenderer の色レイヤー面を確認（SpecialModel 系で代替の可能性） |
| pluto-item-enchantability | NR | (c) | ItemPart の enchantable 面を確認 |
| pluto-item-ender-mask | NR | (c) | エンダーマン視線面を確認 |
| pluto-item-inside-motion | NR | (c) | 同上 |
| pluto-item-placement-export-write | NR | (a) | content 宣言＋パッケージ構造の領分 |
| pluto-item-placement-root-gate | NR | (a) | 同上 |
| pluto-item-placement-root-path | NR | (a) | 同上 |
| pluto-item-predicate | NR | (c) | 同上 |
| pluto-item-prototypes | NR | (c) | Items::load_with の呼び出し側／登録面を確認（GAPS B3-1） |
| pluto-living-attributes-amount | NR | (c) | 属性確定パイプラインの hook 面を検証（(c) 最大の不確定塊） |
| pluto-living-attributes-base | NR | (c) | 同上 |
| pluto-living-attributes-compute | NR | (c) | 同上 |
| pluto-living-attributes-has | NR | (c) | 同上 |
| pluto-living-attributes-remove | NR | (c) | 同上 |
| pluto-living-cause-fall | NR | (c) | living hook の版上げ形への追随可否を検証 |
| pluto-living-entity-attack | NR | (c) | 同上 |
| pluto-living-entity-fall | NR | (c) | 同上 |
| pluto-living-entity-jump | NR | (c) | 同上 |
| pluto-menu-actions | NR | (a) | **L3a done**: `MenuPart::menu_clicked` forwards the NativeMenuEvents feed (the staged slot-click handler itself is a (c) residual — see the L3a section) |
| pluto-menu-actions-local | NR | (a) | **L3a done**: `register_menu_screen` factory (MenuScreen); the OVERWRITE is deleted |
| pluto-menu-equipment-set-slot | NR | (a) | **L3a done**: deleted — equipment slots are declared in the menu type's content (`data/<ns>/sunrise/menu/<name>.json`: `slots`/`results`), the native rule; no weave joins them |
| pluto-menu-equipment-slot-count | NR | (a) | **L3a done**: deleted — same content-declaration premise as set-slot |
| pluto-menu-pick-all | NR | (c) | **L3a reclassified**: no native surface (no pick-all exclusion query on MenuPart) — deleted; decomposition ask joins the L5 issue list |
| pluto-mob-after-fall | NR | (c) | 同上 |
| pluto-mob-effect-apply-tick | NR | (c) | プレイヤー属性更新の面を確認 |
| pluto-mob-fall-damage | NR | (c) | 同上 |
| pluto-mob-inside-motion | NR | (c) | G12/G4 と隣接 |
| pluto-mob-split-children | NR | (a) | mob_split hook v4（event.child/children） |
| pluto-path-node-type | NR | (c) | パス探索の型面を確認 |
| pluto-player-visibility | NR | (c) | 視認率の公開面を確認 |
| pluto-retexture-layer | NR | (b) | 同上 |
| pluto-skull-document-parts-return | NR | (a) | **L3a done**: `Hooks::create_skull_models` registration (not SpecialModelFactory — the game's own decode/render covers registered kinds) |
| pluto-skull-extents | NR | (a) | **L3a done**: `Hooks::create_skull_models` registration (not SpecialModelFactory — the game's own decode/render covers registered kinds) |
| pluto-skull-parts-return | NR | (a) | **L3a done**: `Hooks::create_skull_models` registration (not SpecialModelFactory — the game's own decode/render covers registered kinds) |
| pluto-skull-special-decode | NR | (a) | **L3a done**: `Hooks::create_skull_models` registration (not SpecialModelFactory — the game's own decode/render covers registered kinds) |
| pluto-skull-submit | NR | (a) | **L3a done**: `Hooks::create_skull_models` registration (not SpecialModelFactory — the game's own decode/render covers registered kinds) |
| pluto-structure-generation-start | NR | (c) | worldgen フックが最弱。プリミティブ分解（GAPS B3-1） |
| pluto-structure-natural-start | NR | (c) | 同上 |
| pluto-structure-saved-piece | NR | (c) | 同上 |
| pluto-structure-sources | NR | (c) | 同上 |
| pluto-structure-start | NR | (c) | 同上 |
| protection-arm | NR | (c) | 装甲文脈からの level 取得はイベント文脈で代替可否を検証 |
| protection-kernel | NR | (c) | ダメージイベント（V2/V3）確認で (a) 見込み |
| protection-player-scope | NR | (c) | ダメージイベント確認で (a) 見込み |
| protection-project | NR | (c) | 状態効果の保護面を確認 |
| recipe-placement-definition | NR | (c) | 同上 |
| speed | NR | (c) | ItemPart の mining-speed 面の網羅を確認（GAPS B3-1） |
| station-ingredient-parser | NR | (c) | IngredientEvent の粒度を確認（IngredientTestEvent 欠落と対） |
| tool-decode | NR | (c) | 同上（from_network 側） |
| tool-encode | NR | (c) | 同上 |
| pluto-block-bonemeal-perform | TM | (a) | BlockPart の bonemeal 面（native_hooks::base の trait が内部形） |
| pluto-block-bonemeal-success | TM | (a) | 同上 |
| pluto-block-bonemeal-valid | TM | (a) | 同上 |
| pluto-block-entity-client-tick | TM | (a) | **L3a done**: `BlockEntityPart::block_entity_client_tick` (transient data keyed by BlockEntityRef) |
| pluto-block-entity-container-reach | TM | (a) | **L3a done**: `block_container` / `block_fluids` declarations; the game's default capability providers answer neighbours |
| pluto-block-facts-head | TM | (c) | break-speed 面の新名を確認（G7 隣接） |
| pluto-block-facts-publish | TM | (c) | 本体が他クレートを名指しし複写不可。呼び出し側か reload 面へ付け替え |
| pluto-block-fire-odds | TM | (a) | LevelHost 駆動の fire-odds hook 面（mod_api_host 実装が索引に実在） |
| pluto-block-may-place-on | TM | (a) | BlockPart の配置面 |
| pluto-block-placement | TM | (a) | BlockPart の配置面 |
| pluto-block-random-tick | TM | (a) | BlockPart の tick 面 |
| pluto-block-scheduled-tick | TM | (a) | BlockPart の tick 面 |
| pluto-block-survive | TM | (a) | BlockPart の survive 面 |
| pluto-block-update-shape | TM | (a) | BlockPart の shape 面 |
| pluto-break-context-head | TM | (c) | GAPS A4（破壊イベント）と対で処理 |
| pluto-damage-dragon | TM | (a) | LivingIncomingDamage イベント群（対象種別で分岐） |
| pluto-damage-victim | TM | (c) | LivingIncomingDamageV3Event の実在確認で (a) 見込み |
| pluto-effect-icon-head | TM | (c) | GAPS A2（HUD コールバック）が受け皿 |
| pluto-entity-spawn-converted | TM | (c) | 変換完了のイベント面を確認 |
| pluto-entity-spawn-natural | TM | (b) | RETARGET 候補が索引に実在（natural_spawning::GameLevel 側） |
| pluto-entity-spawn-runtime | TM | (c) | スポーン内部の再導出（creatures 分割先） |
| pluto-hanging-frame-from-nbt | TM | (c) | 額縁 NBT 構築の移転先を再導出（同名関数は索引に無い） |
| pluto-hanging-frame-hit | TM | (c) | 額縁の対話面を再導出 |
| pluto-item-indestructible | TM | (c) | アイテムエンティティの環境 tick。G15 と隣接 |
| pluto-item-menu-open | TM | (a) | **L3a done**: `HostOps::open_item_menu` via `open_mod_menu`; seed items move in `player_container_open` (`MenuRef::set_items`) |
| pluto-item-particle-factory | TM | (a) | register_particle_type / ParticleBatch（book 確認） |
| pluto-living-preview-flush | TM | (c) | ScreenRenderPostEvent で足りるか検証 |
| pluto-map-face-flush | TM | (c) | 同上 |
| pluto-mob-effect-apply | TM | (c) | 同上（V2/V3 確認） |
| pluto-open-item-screen | TM | (a) | **L3a done**: `register_menu_screen` factory builds the MenuScreen (layout/draw/click/tooltip) |
| pluto-player-animate-tick | TM | (c) | PlayerTickEvent の実在確認で (a) 見込み |
| pluto-player-hurt | TM | (c) | ノックバック面。G1（既存 issue）へ合流 |
| pluto-reload-before-tables-reload | TM | (c) | GAPS A6（リロード実体型）と対 |
| pluto-reload-registry | TM | (c) | 同上 |
| send-custom-data | TM | (c) | NetFromClient/ServerEvent の実在確認 |
| world-fluid-neighbor | TM | (a) | 同上（update-shape と共有ターゲット） |
| world-fluid-tick | TM | (a) | 同上（scheduled-tick と共有ターゲット） |

Drift watch on the 296 passing rows (GAPS §B1): 19 rows sit in
hook-version-bump territory (mob-split, attributes-join,
projectile-impact, menu, BE tick) and stay "passing, re-verify" until
L2 re-checks their meaning; the 2 passing OVERWRITE rows
(`pluto-menu-actions-guest`, `pluto-entity-spawn-spawner-add`) need a
fingerprint re-derivation on every SDK move. Weather / tick-rate /
Renderer / ParticleEngine moves touch 0 passing rows.

The ~40 unverified inline mod_api references (GAPS §A12) are L2's
first compile pass: each is confirmed against 6a02076f or folded into
a §A unit. `LivingIncomingDamageV3Event`, `LivingKnockbackV2Event`
and `PlayerTickEvent` are the named candidates that would drop the
damage-victim / mob-effect-apply / player-hurt / player-animate-tick
(c) rows to (a).

### Gap digest (from GAPS.md; dispositions frozen)

mod_api surface (§A — 43 missing names in 9 units, 7 present-but-changed,
~40 unverified inline):

- **A1 fluids (8)** — no fill/drain observe/change/cancel events for BE
  tanks. New upstream issue (title in GAPS §A1). Until it lands, the
  fluid face keeps its woven reach; public API shape unchanged.
- **A2 menus/screens (11)** — mostly a mapping table onto present
  Menu/Screen events. True gaps: HUD render callback and DisplayContext
  — new upstream issue(s).
- **A3 interaction V2 (4)** — mapping-table work in L2 (V1 names exist);
  an issue only if a counterpart proves absent.
- **A4 block lifecycle (5)** — destroy_progress covers the query side;
  a cancellable destroy event with context is the true gap (check
  #3938's current state first; join rather than duplicate).
- **A5 commands/config entities (5)** — registration exists, execution
  and file handles do not. New upstream issue (A6 may merge into it).
- **A6 reload (3)** — `AddServerReloadListenersEvent` exists; listener
  objects + apply-completion notification are the gap.
- **A7 client/input (3)** — join existing issues (#5072 / #4746 / #5076).
- **A8 loot/ingredient (2)** — granularity check first (L5); at most one
  small hook request.
- **A9 other (2)** — `NativeItemEntity`, `WorldPrimitivesEvent`: L2
  verifies EntityRef / render-primitive coverage.
- **A10 non-gaps** — `RegisterCmds` is a rename residue
  (`RegisterCommandsEvent as RegisterCmds`, L2 busywork);
  `AttachmentType` is the Attachments redesign (Descriptor/Key), not a
  gap.
- **A11 present-but-changed (7)** — open_menu/close_menu removed,
  mob_split v4, entity_join_level v5, projectile_impact v3,
  chunk_data v4, typed capability providers, Part `state_key()`.
  L2 follows all seven in the bridge (own work, no issues).

Weave side (§B3): the only row groups that become upstream issues are
the lower-crate non-replaceable cluster (items table functions /
worldgen structure-start — two issues, or joins if hook requests
already exist) — everything else is own redesign. Vanished-home
modules (creatures/remote/maps/attachments) are re-derivation, not
upstream defects.

Existing upstream gaps (§E, from the Create port): G1–G16 + N1.
**G7 is resolved** (`PlayerRef::destroy_progress`, #5030, already at
ffb8601b) and **G9 is resolved at 6a02076f** (`fluid_passage`, above).
The rest are joins with a PlutoLib example, not new issues — except
**N1** (a published entity-tick / render-prep loop), which is a new
loop request. Filing list (frozen): 5 new issues (A1 fluids, A2 HUD,
A4 destroy, A5 commands/config, N1 loop) + the §E joins + the §B3 pair
if no hook request exists yet.

Non-code deltas (§D, 12 items — package format, manifest generation,
attestation signing, registry v2, tag shape, …) carry no issues; they
are folded into the release procedure (DESIGN §5).

### R6 — tooling adaptation (outcome)

- **Baselines: done** (commit `0315d9e5b`). Measured on this tree
  before: docs budget 47 problems, forbidden-API 25 violations.
  Adaptations: `tools/check_docs_budget.py` ALLOWED gained the Sunrise
  tree's entries (README, this ledger, the docs/ mirrors, docs/api/*);
  `tools/forbidden_api_baseline.json` gained the 23
  `pluto-lib-sunrise/` python-write-text-newline entries at the exact
  counts of their `pluto-lib/` conversion twins (measured identical);
  `tools/lint_forbidden_api.py` gained `RULE_PATH_EXEMPTIONS` scoping
  the zero-tolerance cargo-git rule off inside `pluto-lib-sunrise/`
  only (that tree's host is sunrise-org; its two git deps point at the
  official Sunrise repository — the rule still applies everywhere
  else, and the lint's unit tests cover both sides). Verified after:
  **docs budget 0, forbidden-API 0, api-docs 0, injection-ownership 0,
  line endings 0, tests/test_lint_forbidden_api.py 6/6**.
  (Line endings needed no baseline: the index was already LF; this
  worktree's 66 CRLF working copies were renormalised in place.)
- **Tools Sunrise mode: still open, by design.** The fixture tooling
  under `pluto-lib-sunrise/tools/` (~20 `prepare_*.py` +
  `check_cloud_host_boundary.py`) takes `--horizon-source` and emits
  `horizon_source` / `horizon_revision` evidence keys, and its fixtures
  are derived from Horizon host facts. A Sunrise mode is therefore not
  a rename: each fixture family must be re-derived from the Sunrise
  source (SDK api-source or the sunrise-src clones) and its boundary
  checks re-validated. Owner of the work: the first lane that runs a
  given boundary family against Sunrise (expected L2 for the
  direct-host family, L5 for the plan families), one family at a time,
  keeping the Horizon mode working (the master tree uses the same
  scripts from `pluto-lib/tools/`).

### Lane handover

- **L2 bridge inversion** (the critical path): registration-direction
  reversal, game-type adaptation into modules/helpers,
  `[[native.bridge]]` conversion of the 42 `*access` modules, the
  44-missing-name mapping (DESIGN §3.2 table), the A11 follow-ups, the
  A12 inline verification, the `RegisterCmds` alias, the Attachments
  Descriptor/Key rewrite, and the new `EnchantingTableOffsetsRegistrar`
  super-trait on Registrar impls. State rows (10) go to module-owned
  statics (L6), not Attachments.
- **L3 (a) substitution**: the 37 (a) rows by cluster — menu group →
  MenuPart / MenuScreen / register_menu_screen; BE group → Registrar /
  Parts + the published `block_entities.tick` loop; skull group →
  SpecialModelFactory; spawn/damage events; content-pipeline rows →
  content declarations; native_hooks entry group → Parts / LevelHost.
- **L4 (b) retargets**: the 14 (b) rows — Mesher::face → Mesher::mesh,
  movement-table → `game::api_values::block_state::bind`, the 6 module
  re-anchors (recipe-book ×3, chat, commands/place, combat), the 4
  helper fold-ins (api, skull access, wire-read, living-drop module).
- **L5 (c) verification**: the 74 (c) rows, split L5a living/motion,
  L5b items/crafting, L5c worldgen/structures; verify G9's
  `fluid_passage` in-tree; draft the primitive decompositions and the
  join comments for the frozen filing list.
- **L6 finishers**: R4 statics (10 state rows), R7 prose pass +
  package files (icon.png, license, page.toml) for the release.
- **S1 sync** (manager's lane, prerequisite for L7): the seed base is
  pre-fix-wave (`a5375a643`); PlutoLib v1.0.0's master delta comes in
  via `convert.py --sync-check` + a hand pass.
- **L7 integration + release**: plan all-green → build → scoped tests
  (pure / domain / doctests) → DESIGN §5 release procedure. Registry
  v2's live status is re-checked first (GAPS D6).
- This freeze also exists as branch
  `pluto-lib-sunrise/sunrise-6a02076f-mixins-e59da909`; base follow-up
  work on it (the ffb8601b-named branch is kept for history).

## Re-pin 2 (2026-11-26): SDK ba1f24e4

A newer SDK shipped the same day (`sdk-ba1f24e4ad41246b558230aa665539dc768b1287`,
published 2026-11-26T02:04:02Z) and was caught by the L7 lane's opening
latest-check, which stopped before merging. This section digests the
re-pin L1-style before any L7 integration work; the L1 worklist and the
L2–L6 lane sections carry over unchanged onto this foundation.

- **Diff** (`cargo sunrise-mod sdk diff 6a02076f ba1f24e4`, run on the
  PC, exit 0): **51 breaking / 40 conditional / 586 additive** (677
  surface changes; hash `30247197…` → `bfc42991…`; `surface.previous`
  is 6a02076f, so this is the direct successor). Main drivers:
  convention tags (#1244) — `TagKey` becomes `TagKey<K = ()>` and the
  `Tags` / `LoopContext` / `*_key::is_in_tag` queries take typed keys;
  frame-graph setup + compute pipelines (#963); the Collider /
  ColliderPose rework (constants and `set_collider_transform` removed);
  datapack vanilla value kinds (#5368 part 2, `DatapackCandidate::build`
  gains a decoder argument); dynamic textures reworked (budget /
  free-slot surface removed); `BlockProperties` loses
  `skip_same_block_faces` to data-driven face rules; the item-use hook
  family goes to version 2 (the event now carries the using player's
  entity id); `collision_shape` goes to version 3 (no per-tick answer
  caching). Hooks removed: `dragon_egg_form`, `shader_params`. Hooks
  added (15): `client_break_progress`, `client_particle_add`,
  `client_take_item_entity`, `dragon_fight_tick`, `dragon_fight_won`,
  `dragon_respawn_stage`, `end_crystal_destroyed`, `frame_graph_setup`,
  `living_death_protection`, `register_pipeline_modifiers`,
  `screen_files_dropped`, `screen_mouse_moved_pre`,
  `screen_pointer_motion`, `sleep_status_announce`, `sleepers_needed`.
- **Pins**: only the SDK revision moved. `sdk.json` of ba1f24e4 (read
  2026-11-26) shows game `26.3`, `mixins.rev =
  e59da9094f0c2fdda6c627fe9eff22dec6a906f0` + patch `c4a46bf9…`,
  `toolchain.rust = 1.99.1`, `content_api = "0.2"`, `package_format =
  6` — all identical to 6a02076f. The manifest pin
  (`mod/sunrise-mod.toml`, `[native] sunrise =`) is settled to
  `ba1f24e4ad41246b558230aa665539dc768b1287` uniformly with this
  foundation (this is the same field L2 advanced to 6a02076f on its
  branch, which settles the notation question L7 had open).
- **125-row classification impact: none.** No frozen row changes class
  from this diff alone. The two removed hooks are referenced by no row
  and no landed code; the conditional items are hook-version bumps and
  the tree still carries no `hook_version` pins anywhere (grep-verified
  on the foundation and the landed branches), so they do not fire —
  the same finding as L1. The one new surface that touches a drafted
  (c) cluster is `client_particle_add` (below).
- **Landed-disposition impact — L7 repair list.** The landed branches
  (L2 / L3a / L3b / L4 / L5a / L5b / L5c / L6 / S1) were grep-checked
  against every breaking/conditional symbol:
  - **R1 (certain, compile)**: `Tags::block_in_tag` now takes
    `TagKey<BlockKey>`; three landed call sites pass the untyped
    `TagKey<()>` returned by `mod_api::__private::tag_key` —
    `hooks.rs` (1 site, the L2/S1 line) and `block_part.rs` (2 sites,
    L3b). Repair at L7: bind the key once with `.typed::<BlockKey>()`
    (`TagKey::typed` is added by this SDK) at those sites.
  - **R2 (probable, verify at first compile)**: mod_api `HurtSource`
    gains `direct_type: Option<&str>`. Woven helpers construct the
    game-side `health::HurtSource` with full 3-field literals at 5
    sites in 3 files (`explosion-equipment-head.rs`,
    `ordinary-equipment-head.rs`, `pluto-entity-push-access.rs`); if
    the game-side struct is the same type, those literals need
    `direct_type: None` added.
  - Everything else landed is untouched by the diff: L2's 41 wired
    hooks, L3a's Parts / `create_skull_models` / tint sources, L3b's
    BlockPart / `mob_split` / reload adapter, L5a's
    `living_jump_power` / `get_equipment_drop_chance`, L5b's item tint
    registration, L5c's `fluid_passage` wiring, L6's statics
    (`ModBlockEntity::revision().identity()`) — none of their surfaces
    appear in the change lists. The command-argument match already has
    a wildcard arm, so the new `CommandArgumentValue` variants are
    inert. `Collider`, `DatapackCandidate`, `EnchantmentInfo`,
    `submit_geometry`, `skip_same_block_faces`, dynamic textures and
    `WorldRenderView` have no references in landed `mod/src`.
  - The woven re-anchors (L4's nine, L5a's tick-post / creatures
    retargets, L5c's maps / remote / break-speed retargets) target
    game-source files, which the surface diff does not cover; L7
    re-measures them with its plan set-diff against ba1f24e4's index,
    using the foundation baseline below as the reference set.
- **New surfaces vs drafted gaps: no draft is fully resolved.**
  `client_particle_add` (client-side, may change or cancel each
  particle add) partially answers L5a's draft 06 (particle
  interception) for `pluto-item-particle-factory` /
  `pluto-entity-particle-kind`; whether it covers the pure factory
  contract is an L7-era verification, so the rows stay (c).
  `living_death_protection` and the `BlockStates::path_type` reader
  are recorded as new surfaces adjacent to the protection and
  path-type drafts respectively; neither matches a drafted ask.
- **Foundation verification** (this tree, scoped): pure --lib
  427 passed / 0 failed (checkpoint baseline reproduced exactly);
  doctests 1307 passed / 0 failed / 1 ignored (checkpoint-identical);
  lints 5/5 exit 0; plan failing-id set re-measured with L5c's
  consistent extraction (unique `pluto_lib:<id>` from error lines) =
  **102 unique ids** (118 `E_MIXIN` error lines), set-identical to a
  same-session measurement of this same tree at 6a02076f — the bump
  itself adds and removes no foundation plan failures.
- This freeze exists as branch
  `pluto-lib-sunrise/sunrise-ba1f24e4-mixins-e59da909`; the 6a02076f
  branch is kept for history. L7 merges the lane branches onto this
  foundation in its recorded order (L5a / L5b / L5c, then L4, then L6,
  then S1 reconciled), applying R1 (and R2 if confirmed) as part of
  the integration.

## L2 bridge inversion (2026-11-26, branch `sunrise/l2-bridge`)

- The mod crate (`mod/src/hooks.rs` + `lib.rs`) now names only `mod_api`
  and `crate::pure`: the Horizon `hooks()` bundle is deleted, and each
  game-side support module registers into the pure registries itself
  (the 48 `set_*` calls already passed pure functions; only the game-type
  adapter closures move module-side). The manifest pin moved to the L1
  target `6a02076f89cd…`. Verified by MSVC `cargo check` (exit 0); the
  shipped build stays the tool's gnu build at L7.
- 41 Hooks-trait methods are wired (45 of the old 111 names; merges:
  living_incoming_damage v1+v2+v3, ingredient+ingredient_test,
  perform_command → register_commands executors). Host adapters
  (PlayerOps/WorldOps) run over the generated ref types under the same
  scope nesting as Horizon. Attachments are Descriptor/Key: two
  multiplexed JSON attachments (`pluto_lib:json`, `pluto_lib:json_synced`)
  preserve the per-id public API; the 10 quarantined state rows migrate
  to module-owned statics in L6.
- 22 old names are Part-scoped hooks, not Hooks-trait methods
  (menu_clicked, block_entity_*, the fluid/capability family, use_item…)
  — L3 registers Parts forwarding to the same pure entries. 44 are
  residuals with per-item reasons in the lane record: the largest class
  is HostSide hooks (player/entity ticks, break speed, use-item trio),
  whose pure bodies need host reads the hook does not carry; then event
  shapes that lost facts (arrow_loose, projectile_impact,
  compute_fov_modifier, critical_hit) and true gaps (hud = A2,
  world_primitives = A9, destroy cancellation = A4, reload apply = A6 —
  the add_server_reload_listeners adapter is designed, L3/L5).
- Bridge audit of the 40 `*access` modules: 29 cannot become
  `[[native.bridge]]` (23 woven into `sunrise_client_lib`, 5 into
  sunrise-world, 1 into sunrise-assets; bridges target `sunrise_client`
  only). 11 are crate-eligible (block_exp, block_settings, entity_climb,
  entity_ignite, entity_push, entity_ride, entity_teleport,
  living_entity, mob_offhand, player_visibility, trade); their
  per-function conversion travels with L4's weave work, so no
  declarations were added here.
- A11 verdicts: open_menu/close_menu have no bridge call sites (no-op);
  entity_join_level v5 wired; chunk_data had no old body (chunk state
  rides attachments — closed); mob_split's trait hook exists but its
  adapter is an L3 rebuild; typed capability providers are the
  Part-scoped family (L3 + A1). `Registrar` has no PlutoLib-side impl;
  `register` compiles against the new supertrait
  (EnchantingTableOffsetsRegistrar) unchanged.

## L3a native-API clusters (2026-11-26, branch `sunrise/l3a-native-api`)

- Menu / block-entity / skull clusters are native now. New
  `mod/src/parts.rs`: `register_parts(registrar)` (public; consumers call
  it with their own registrar after their pure registrations, PlutoLib's
  own `register` calls it too — duplicate declarations/Parts are refused
  harmlessly by the game). Block entity types are declared per pure type
  id with one `PlutoBlockEntityPart` whose subscription list is computed
  from the pure types' shapes (manual `block_entity_subscribed`, per-type
  — the `#[sunrise_part]` macro cannot express per-instance slices).
  Forwards: factory, server tick (snapshot + write-back diff, menu-data
  mirror into the view's data slots), client tick (transient map keyed
  by `BlockEntityRef`), submit (a `BlockEntityCanvas` over the native
  draw context), update tag, menu quick move, can-place / can-take
  through face. Containers and fluid tanks are `block_container` /
  `block_fluids` declarations answered by the game's default capability
  providers.
- Menus: one `PlutoMenuPart` per menu (clicked feed, button press with
  menu switching through `PlayerRef::open_menu`, slot rules, close
  bookkeeping with owned-slot return) and one `MenuScreen` factory per
  pure screen handler (layout/draw/click/tooltip through a `ScreenCanvas`
  over the native draw context). Item menus open through
  `HostOps::open_item_menu` (`open_mod_menu`); their seed items are
  transferred by the `player_container_open` hook (`MenuRef::set_items`),
  the surface's documented pattern.
- Skulls: the whole woven machinery (8 manifest entries, incl. the
  passing-but-inert worn-kind HEAD) is deleted. `Hooks::create_skull_models`
  registers each pure definition; the game's own head decode accepts
  registered kinds, so no decode/parts/submit reach is needed. L4's
  fold-in of `pluto-skull-model-access` is superseded by this deletion at
  integration (land L3a after L4, or take L3a's deletion over the fold).
- Tints: `register_block_tint_sources` registers one `BlockTintSource`
  per (block, layer index 0..8) forwarding to the pure colour providers.
- Verification: MSVC `cargo check` exit 0; plan errors 117 → 102 with
  all 17 cluster rows gone and zero new errors; pure lib 425/425
  (checkpoint 427 minus the 2 tests of the deleted woven-skull test
  harness, removed with its subject), doctests 1307/1307 + 1 ignored
  (checkpoint-identical); lints 5×0.
- Residuals handed to L5/L7: (1) pick-all exclusion — no native query
  (upstream ask: a MenuPart pick-all exclusion); (2) pure slot-click
  handlers need a staged `MenuAccess` — `MenuClickedEvent` carries only
  scalars (upstream ask: extend it with a subject like
  `MenuButtonEvent`'s); (3) `armor_stands::skull_kind` does not consult
  registered skull kinds (upstream gap; the deleted worn-kind weave was
  inert — its provider had no reachable installer); (4) custom fluid
  capability providers (types with their own filler/drainer/reader) —
  L3a declares tanks only, neighbour access uses the declared-tanks
  default; (5) canvas `model(name)` — a native `ModelRef` is issued at
  resource load, never resolved from a name at draw time (needs a
  name→ref cache filled at client resource load); (6) tint providers are
  asked with an empty property set — `BlockMetadata` cannot enumerate a
  state's properties (upstream ask); (7) skull animation is not forwarded
  (the native surface animates only the vanilla dragon/piglin kinds) and
  a skull definition's model layer is registered by the consumer's own
  `register_layer_definitions`; (8) `GuiCanvas` serves GUI-atlas sprites
  only — the block/item atlas sprite call and the 3D scene calls stay
  the pure defaults.
## L3b native-API clusters (2026-11-26, branch `sunrise/l3b-native-api`)

Base: L2 tip `142291ae3`. Scope: the (a) rows of the spawn/damage, content
and native_hooks clusters, plus the L2 hand-over adapters in that range
(reload A6 wiring, mob_split, the entity-by-id reader, the HostEntities
scope). Verified by MSVC `cargo check` (exit 0); plan re-run and the
pure/lint battery are recorded in the lane record
(`~/workspace/sunrise-l3b/STATUS.md`). 19 manifest rows removed:

- **BlockPart (`mod/src/block_part.rs`, new).** One stateless
  `PlutoBlockPart` is attached per pure-registered behaviour block
  (`register_parts` from `Hooks::register`; consumers attaching later use
  the pub `attach` helper). It replaces the ten `pluto-block-*` rows
  (survive, may-place-on, placement, update-shape, random-tick,
  scheduled-tick, bonemeal valid/success/perform, fire-odds) by running
  the same pure `block_api_v1` dispatch over LevelHost/LevelWriteHost/
  LevelRandomHost adapters. `placement_facts` answers None: the class
  chain was a woven-bridge artifact — in the Part model the game owns
  vanilla survival and behaviour answers come first. `break_block` is
  None and `place_feature` false: the writable level host has neither
  call. The bonemeal-success roll runs over a read-only editor (writes
  fail, draws work), because that part call lends only a random host.
- **Fluid rows ride the same part.** `world-fluid-tick`: `tick_block`
  claims a fluid cell first and runs `pluto_api::fluid::tick` over a
  FluidWorld adapter (cell rule mirrors the game-side adapter: block id
  -> `fluid::for_block`, `level` property 0..=8). `world-fluid-neighbor`:
  `update_shape` schedules the cell's block tick after the definition's
  delay. Difference kept from the game side: its `replace` also gated
  spread through the protection surface (`allow_spread`); that gate
  belongs to the protection rows, not to this engine port.
- **`pluto-block-settings-gameplay` + the item-placement trio**
  (`-export-write`, `-root-gate`, `-root-path`): manifest entries only.
  Sunrise content is static declaration and the vocabulary matches the
  pure mapping 1:1 — `BlockProperties.push_reaction` is the settings
  row's declaration half, and its runtime half is the part's
  `piston_push_reaction` (same registry, same 0..4 mapping). The overlay
  gate/path/write machinery (`mod_content::pluto_item_placement_export`)
  has no runtime counterpart; the support modules stay for other rows.
  Caveat: declarations computed at runtime from code-registered pure
  registries have no dynamic channel in Sunrise — consumers author them
  as static content.
- **`pluto-mob-split-children`** -> the `mob_split` Hooks adapter.
  Child writes are recorded as Dress entries during `inherit` and
  replayed one pending-child editor at a time, as the woven bridge did.
  Partial, with evidence: (1) no equipment drop-chance reader exists in
  mod_api (only the drop-chance hook), so `head_drop_chance` is
  vanilla's 0.085; (2) the parent's random stream is lent only inside a
  hit context, so the adapter seeds a mod-owned `LegacyRandomSource`
  from one level-stream draw — same distribution, not the same stream;
  (3) `PendingChildEditor` has no no-ai / invulnerability write, so
  those two dress entries are recorded but not applied (pure defaults
  are false; only a true request differs); (4) pending children have no
  entity id, so the pure child `info().id` is 0. Candidates for L5's
  upstream filing list.
- **`pluto-damage-dragon`** -> `living_incoming_damage` now takes the
  hit host and, for an ender-dragon victim, notes `VictimFacts { health,
  attack_damage: None }` from `HitHost::victim()` before the hurt/damage
  stages; the note's 100 ms thread-local lifetime covers the same hit's
  damage-pre stage. The victim's attack damage has no reader in the hit
  context.
- **`pluto-hanging-stand-tick-head`** -> `entity_tick_pre/post` with a
  `minecraft:armor_stand` kind filter: the stand's kept `Variant` NBT
  resolves through the pure `HangingRegistry`; fire is cleared pre and
  post, and post also sets invisibility from the variant's
  hidden-when-equipped rule. The sibling woven after-pass helper
  (`pluto-hanging-stand-tick-after.rs`, riding the tick-cushions head)
  duplicates the post behaviour idempotently; its row may be dropped at
  integration.
- **Reload adapter (A6, mod-crate wiring).** `add_server_reload_listeners`
  registers one native listener per pure JSON listener
  (`json_reload_listeners(ServerData)`), each with its
  `FileToIdConverter::json(directory)`. Prepare decodes its folder's
  winning layers into a capture shared by the whole pass, so a handler
  reading another folder via `reload_json_stacks` does not depend on
  listener order; the apply closure installs the capture with
  `with_reload_folders` and calls `apply_json_reload_listener` with the
  effective (top-layer) files. Two documented differences from the
  woven bridge: handlers run at apply time, so a handler failure cannot
  preserve that listener's previous data the way a prepare failure
  preserves every listener's; and registry enumeration has no source in
  the reload context, so the context's registry half is empty (a lookup
  answers "unavailable"). No server reload-finished hook exists, so the
  simple listeners fire (`server_data_reloaded`) from the last
  registered listener's apply closure, which runs after the other
  applies in registration order; if no JSON listener is registered the
  simple listeners do not fire. The woven reload rows
  (`pluto-reload-registry`, `pluto-reload-before-tables-reload`) are (c)
  and stay for L5.
- **Entity-by-id reader + HostEntities scope.** The reader is
  `Host::entity(id)` plus the handles' own methods
  (`entity.position(host)`, `entity.nbt(host)`, `LivingRef::visit_effects`,
  ... — most entity reads/writes are inherent handle methods, not host
  trait methods). `living_experience_drop` now reads the dying entity's
  effects through it (was `Vec::new()`), and `projectile_impact` reads
  motion, NBT and the target's effect ids through it. `HostEntities`
  implements the pure `EntityOps` in full; `scoped_entities` opens it
  under the player/world/book scopes for `entity_tick_pre/post`,
  `entity_interact_specific` (pure `entity_interact`) and
  `projectile_impact`. Impact residuals: the arrow's pierced list has
  no reader in mod_api (only its pierce level), so `piercing_ignored`
  is empty and the ignore-list write-back is not applied; impact actions
  apply their effect grants directly, the rest stay module-side as in
  L2's player actions.

Reclassified (a) -> (c), rows stay in the manifest:

- **`pluto-entity-spawn-add`.** The completion half consumes a Pending
  state that the woven prepare points produce (natural prepare is (b),
  runtime prepare is (c) — both outside this cluster) and that lives in
  a game-side mob nav-state field the mod crate cannot reach under the
  inversion. `finalize_spawn`'s `UnspawnedMobEditor` cannot consume it,
  there is no post-join `mob_spawned` hook (the 748-method Hooks
  surface has only the pre-join `finalize_spawn`, with `&dyn Host`), and
  a mod-side prepare at finalize would double-prepare while the woven
  prepare still runs, without the entity id or a random loan. Unblocks
  when the prepare points migrate (L5 spawn work).
- **`pluto-item-particle-factory`.** No particle-add interception hook
  exists in the Hooks trait (only `effect_particle_modification`,
  `register_particle_groups`, `register_particle_providers`); the
  game-side QUERY provider (`pluto_item_particles::set_provider`) is
  never installed anywhere in this tree and no pure item-particle
  registry exists. Existing surfaces for L5's filing:
  `register_particle_providers` (typed client factories),
  `ItemParticlesClientHost::item_particle_options(stack)`,
  `ClientHost::add_particle(options, at, velocity)`,
  `ParticlesRegistrar::register_particle_type`.

## L3 merge (2026-11-26, branch `sunrise/l3-merged`)

- L3a (`64023f7b5`) and L3b (`062fc7759`) are merged onto the L2 tip
  `142291ae3` (merge commits `1955d2499` and `97fab99da`); this branch
  is the base for the L5 lanes. The only conflicts were this file
  (both lanes' sections kept, above) and the `Hooks::register` body in
  `hooks.rs`, which now calls both `crate::parts::register_parts`
  (menu / block-entity / skull Parts) and
  `crate::block_part::register_parts` (BlockParts) — distinct Part
  sets, no duplicate registrations. The manifest holds the disjoint
  union of both lanes' deletions: 422 -> 384 entry ids, and none of
  the 38 removed ids is present.
- Merged verification (scoped): MSVC `cargo check` exit 0; pure lib
  425/425 (the L2 checkpoint's 427 minus the 2 tests of the deleted
  woven-skull harness; L3b added no pure tests — the merged test list
  is name-for-name L3a's), doctests 1307 pass / 1 ignored
  (checkpoint-identical, list-verified); plan failing-id set = 93,
  exactly the L2 baseline set (129) minus the union of both lanes'
  removed rows (17 + 19, disjoint), zero new errors; lints 5x0.

## L5a life・motion (2026-11-26, branch `sunrise/l5a-life-motion`)

Base `4ae287fab` (L3-merged). Cluster: the (c) rows for living entities,
damage, spawn, movement, drops and projectiles — 43 of the 93 failing ids
at the base. Step 0 re-confirmation of the latest SDK was attempted per
the always-track-latest policy; the Releases API fetch was unreachable
from this lane, so the lane proceeded on the confirmed pin `6a02076f89cd`
(the daily SDK watch and the L7 gate are the backstops). Verification:
MSVC cargo check exit 0; pure --lib 425/425; doctests
1307 pass / 1 ignored; plan failing-id set 93 → 86 (exactly the
7 rows resolved below, zero new); lints 5/5 zero.

### Implemented (7 rows leave the manifest)

- `pluto-player-animate-tick` — retargeted. `native_player_tick` was
  split into `native_hooks::base::Game::native_player_tick_pre/post`
  (both weavable in the SDK index); the arm-swing/break flush the helper
  runs belongs at the tick's end, so the HEAD now sits on `_post`. The
  flush module itself was already in place.
- `pluto-projectile-flight-game`, `pluto-projectile-impact-game`,
  `pluto-throw-sound-game` — module re-anchors. The split of
  `game/creatures.rs` left its successor at
  `game/mobs/creatures/mod.rs` (game.rs:392 `#[path]`); the three modules'
  `file` fields point there now, logical paths
  (`crate::game::creatures::<name>`) and sources unchanged, as with L4's
  re-anchors. Their caller reaches already passed. Shapes verified
  against the pinned source: `Creatures.projectiles` is the same
  `JoinList<Projectile, false>` the flight module's take/into pattern was
  written for; `ArrowState` still carries `pierce`, `in_ground`,
  `in_ground_time`, `pickup`, `pickup_item`, `dealt_damage` and —
  closing L3b's impact residual — `pierced: Vec<Who>`, so the impact
  module's ignore-list stash works as written; `Pickup::from_byte`,
  `projectiles::thrown_by_item`, `drops::die/drop_chance/loot_entity`
  all exist. Leaf type-checking of module bodies lands with L7's guarded
  build, as for every re-anchor.
- `pluto-living-entity-jump` — decomposed. The `living_jump_power` hook
  (loop context; entity, kind, mutable power) is wired in the bridge to
  `LivingEntityBehaviors::jump_power`. Two context fields degrade: the
  hook carries no host, so `slime_size` is None and `block_jump_factor`
  reads 1.0 — the inherited power the event carries already folds the
  factor in, and a class answer replaces the whole power, as the woven
  reach's did.
- `pluto-damage-victim` — decomposed. `living_incoming_damage` now notes
  `VictimFacts` for every victim before the hit's stages run, read from
  the victim view the hit context lends (`HitHost::victim` →
  `LivingView::health` / `attribute_value`) — the reader L3b recorded as
  missing in the hit context exists in this view shape (the by-id
  reader does not: `HitHost` has no `entity()`). The ender dragon keeps
  its shape (health noted, attack not named, per the pure contract).
- `pluto-player-hurt` — decomposed. The woven reach did two jobs at the
  game's knockback scope (a target that no longer exists):
  (1) AttackerFacts for the hit's damage events — now noted at the head
  of `living_incoming_damage` from the causing entity, read through the
  entity-by-id reader (kind, both hands, chest, attack attribute,
  state); a player's note carries its hands only, as the pure contract
  shapes it. The woven 0-id player opening does not arise here: the
  causing id is the real entity.
  (2) The wearer-hit report — now noted from the hit itself when the
  victim is a player (`note_wearer_hit` with the attacker as the hit
  context lends it and `armor_of_the_only_wearer()`), resolved by the
  existing `resolve_wearer_hit` call. Its effect grants for the attacker
  are queued in the bridge and land at the next entity tick's drain
  through the server host: the hit context lends reads only (its
  `LivingView`s write nothing), so the grants cannot land where they
  are computed; the woven drain was a game-side module queue, and the
  queue now lives in this bridge with the same next-tick timing.
  Noting from the hit itself drops the woven version's known wrong
  case (a mob fighting another mob beside the player was heard as
  hurting the player) and its seven-block gate with it. The report's
  own tick-side queues (wearer self-effects, counter damage, actions)
  still wait for a flush point — draft 11 below. `damage_type`/`amount`
  are resolved by the damage event, as the pure contract shapes them
  (the woven report never knew them either). Two woven reads have no
  hit-context counterpart and degrade: the attacker's chest piece (the
  causing view lends hands, not equipment) and the positional half of
  its state (on ground / in water / burning — the view names kind,
  effects and health only), which also leaves the wearer report's
  attacker position empty.

Also wired, as a partial decomposition of `pluto-living-drops-game`: the
`get_equipment_drop_chance` hook now answers from
`LivingEntityBehaviors::equipment_drop_chance`. The class answer replaces
the recorded chance (`original_chance`); the enchantment share the
event's `chance` already carries over the recorded one (the killer's
Looting included) rides on top — the fold the woven death-loot module
applied by hand when it swapped chances into the mob. Looting is read
live from the causing entity's main hand through the reader. A cube's
size is not lent by the death roll, so `slime_size` is None (the woven
module read it off the mob).

### Joined to existing upstream issues (no code)

- `pluto-item-indestructible` → G15 (upstream #5082, non-living entity
  fire immunity / environment handling). Its target
  (`ItemEntity::environment_tick`) also no longer exists at this pin.
- The eight motion/fall interior rows (`pluto-block-after-fall`,
  `pluto-block-fall-damage`, `pluto-block-inside-motion`,
  `pluto-item-after-fall`, `pluto-item-inside-motion`,
  `pluto-mob-after-fall`, `pluto-mob-fall-damage`,
  `pluto-mob-inside-motion`) → G12/G4 (upstream #5074 / #5076), the
  design's prescription for the generic movement-function interiors
  (`check_inside_blocks`, `restitute`, `mod_block_fall`,
  `tick_item_with_block_contacts`, `effects_from_blocks`, `do_move` —
  all in `sunrise_client_lib` / `sunrise_world`, no `#[replaceable]`).
  Recorded for the follow-up: the Part hooks `fall_on` (writable
  distance/multiplier + landing writes), `bounce_on` (writable
  movement) and `bounce_restitution` (factor query), and the
  `entity_inside` hook, decompose the mod-block subset of these rows;
  they do not cover the vanilla-block side (the woven reaches replace
  the inside-effects pass wholesale and serve classes, not states).
- `pluto-living-preview-flush` → the A2 unit (HUD/screen callback
  surface). Its target (`native_screen_draw`) is gone; the nearest
  surfaces (`screen_render_post`, `gui_layer_post`) carry no HudBatch
  equivalent a queued living preview can flush into.

### Drafted upstream issues (in the lane record, not filed)

Drafts live in `~/workspace/sunrise-l5a/issues/` (13 files):

1. **Identity tokens** — per-entity/per-level instance identity (the
   block-entity `revision().identity()` pattern extended). Unblocks
   L6's five held state rows and the spawn/head-claim state below.
2. **Spawn completion** — the spawn prepare-point migration verdict:
   NOT possible with current primitives. `finalize_spawn` listeners run
   before finalization (edits overwritten), get no RNG loan, and its
   editor lacks can-pick-up-loot / drop-chance / enchant / attachment
   writes; coverage also misses non-finalizeSpawn paths the woven
   runtime head serves. Ask: a post-finalization pre-join completion
   hook + RNG loan + the editor writes + a non-generic
   `GameLevel::finalize_spawned` entry (L4's ask). Rows:
   `pluto-entity-spawn-{add,runtime,natural,conversion,converted}` stay
   (c); the prepare points do not migrate.
3. **`MovementProperties::bind` reach** — `movement-table` (L4's ask,
   with the index name-collision evidence).
4. **LivingRef equipment drop-chance reader** — for `mob_split`'s
   Dress (the adapter's hard-coded 0.085 head chance).
5. **mob_split random loan + PendingChildEditor writes** —
   `set_no_ai` / `set_invulnerable`; parent-stream loan.
6. **Particle-add interception / item-particle registry** —
   `pluto-item-particle-factory`, and the landing-splash kind
   substitution of `pluto-entity-particle-kind`.
7. **Attribute computation participation** — the five
   `pluto-living-attributes-*` reaches serve the game's live attribute
   engine in `mobs::finalize`; `LivingRef`'s modifier surface is the
   permanent group (wrong order, wrong lifetime — saved). Ask: a
   transient class-modifier channel (after permanent, before the game's
   own transient ops) + transient reads/removal + the armor base mirror.
8. **Class query overrides** — `pluto-living-entity-attack` (the damage
   events carry the finished hit, not the attribute the query is
   defined over), `pluto-living-entity-fall` (an absolute calculated
   damage is not expressible as a `fall_on` multiplier, and the query
   is per type, not per landed state), `pluto-living-cause-fall`
   (wholesale landing application with a velocity write, on any block).
9. **Protection pipeline** — `protection-{kernel,player-scope,arm,
   project}`: the kernel needs the post-magic stage mid-pipeline
   (`sunrise_world::combat_rules::after_magic`); the damage events
   bracket the calculation but never see the protection value.
10. **Death-drop context** — `pluto-living-drops-game` is NOT
    re-anchored: the native it replaced was restructured (server_loot,
    roll_death/finish_death, `is_direct`, `this_nbt`, the explicit
    `source` argument, listened enchanted-loot rolls), so a verbatim
    re-anchor would silently lose behaviour. `living_drops` cannot
    substitute: `ItemDrop` is non-exhaustive with no constructor (the
    list cannot grow), and head eligibility/claim is game bookkeeping
    with no reader. Ask: an ItemDrop constructor + head
    eligibility/claim verbs. `pluto-head-loot-gate` rides the same
    eligibility read.
11. **Tick flush point** — `pluto-mob-effect-apply` /
    `pluto-mob-effect-apply-tick` are queue drains; under the inversion
    their fill points land at hooks (the wearer grants above are the
    first), and the remaining queues (contact, damage drops, the
    wearer tick queues) need a per-player/per-entity tick hook with a
    writable host, or application rights at the fill hooks.
12. **Path-type query** — `pluto-path-node-type`: the path hooks are
    Part-scoped, but classification is asked for every state a mob
    crosses, including vanilla states adjacent to mod blocks.
13. **Visibility player identity** — `pluto-player-visibility` stays
    (c): `living_visibility` is the woven RETURN's exact counterpart
    (last in `getVisibilityPercent`, vanilla percent in), but its host
    is read-only and under it the target player cannot be named —
    `EntityRef::uuid` wants `&mut dyn ServerHost`, and `LivingRef`
    offers no player/holder read — while the pure factor is keyed by
    player UUID. Ask: the UUID (or a player handle) at the hook.

### Notes for L7

- `pluto-living-entity-jump.rs` stays in `mod/src` although its weave
  entry is gone: the pure suite's living-query host harness
  (`tests/pure/src/living_query_host_tests.rs`) `include!`s it as its
  test host, the same shape as the still-(c) fall helper beside it.
- The access module whose reach retired here
  (`pluto-player-hurt-access`) remains in the manifest, as do the
  spawn/protection/attribute support modules whose rows stay (c) and
  `pluto-player-visibility-access` (its row stays (c), draft 13).
- The re-anchored projectile modules' leaf type-check is L7's guarded
  build, as with L4's re-anchors. One leaf was verified by shape only:
  the flight module's `game.creatures` field access and `Game` scope
  imports come from `creatures/mod.rs`'s `super::*` (its scope imports
  nearly everything the modules name).

## Remaining (precise; in dependency order)

- **R1 — Anchor re-derivation (host drift).** Census against ffb8601b is
  DONE and verified retargets are applied (see the checkpoint above). What
  remains is the 125-row redesign list (frozen and classified in
  the L1 section above): 40 module rows anchored outside
  `sunrise_client` (fold into helpers or re-anchor), 21 non-replaceable
  lower-crate targets (retarget to a replaceable caller/callee or a
  mod_api primitive), ~12 `native_*` entry points (re-derive or
  substitute), and the anchorless rows (creatures/remote/maps/attachments).
- **R2 — Weave plan / build.** The merged `cargo sunrise-mod` project
  exists and the plan verdict stands at 296/421 rows (checkpoint above).
  What remains is burning down the 125-row redesign list (frozen in the L1
  section) in clusters with
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
- **R6 — Tooling adaptation.** Baselines done by L1 (docs budget /
  forbidden-API / line endings all 0; see the L1 section above). The
  plan/boundary tooling under `pluto-lib-sunrise/tools/` still speaks
  Horizon (`--horizon-source` arguments) — its Sunrise mode is scoped
  in the L1 section and stays open.
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

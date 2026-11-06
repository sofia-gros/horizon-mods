# World-fluid and rendering boundaries for feature-group reorganization

Read-only inventory, 2026-11-06. This is classification input, not a PR reorganization or implementation change.
Baseline: [published v15](https://github.com/sofia-gros/horizon-mods/commit/89744814b8d536a117e82f8ea3c8cdc9798faa95), local `8715b1232c0fca133fe012f35830e2108e653d59`, runtime `f5a83539525c26fd2142e26e43789a3dd0107033`. Official Horizon `624c4f6265c02296fbe8a80f029dd0d987fb94c0` and Mixins `e8cd0aa38cffef49cb2e088ef716baa107148c02`. Main remains `08afa992d5091e03360f3473ea93fd17aa793987`.

**23 original PRs remain open drafts with unchanged heads; all 23 retain the original needs-fix classification.** Their cumulative 70 family declarations are content, not 70 playable fluids. The newer ordinary Pluto package has plan/apply/type evidence; that does not retroactively verify those PRs.

## Recommended grouping

Keep one coherent world-fluid/render feature group with separate layers: consumer definitions/assets; generic registry/flow/source operations; read-only world/native facts; rendering mechanisms; and explicit native effect ownership. Cross-link generic services to group 01 and tank/machine mutation to group 06. Research proposals stay separate and uninstalled. Do not create a PR per helper.

## Original PR ledger

| PR | Incremental scope | Exact head | Parent PR |
|---|---|---|---|
| [#716](https://github.com/sofia-gros/horizon-mods/pull/716) | 4 base metals | `889fb19ff610984b5a69029ea978892ac090d7fd` | main |
| [#720](https://github.com/sofia-gros/horizon-mods/pull/720) | 2 debris/netherite families | `43adf1443ee126ba6f4e5096a09f49d1c374b974` | 716 |
| [#722](https://github.com/sofia-gros/horizon-mods/pull/722) | 4 overworld alloys | `80217bc300fa20a6e517b6dea174eb1dab734e33` | 720 |
| [#726](https://github.com/sofia-gros/horizon-mods/pull/726) | 4 advanced alloys | `7f5c97b1e2dd50ba7ca234560f58367f456c429a` | 722 |
| [#731](https://github.com/sofia-gros/horizon-mods/pull/731) | 5 stone families and corrected _fluid block names | `a13797f0882034248ac60f8542cf278391796f95` | 726 |
| [#733](https://github.com/sofia-gros/horizon-mods/pull/733) | 4 gem families | `e07225fbf72fd012b4454b4ef6b711f3ee1ea268` | 731 |
| [#738](https://github.com/sofia-gros/horizon-mods/pull/738) | Blazing Blood and Fiery Liquid | `3acf4fc1fd1b146ab96246d7ebf92034f33d0fba` | 733 |
| [#743](https://github.com/sofia-gros/horizon-mods/pull/743) | 30 conventional molten metals/alloys | `15aa913b3e6e7da964f2ca73104de8f5005c6dec` | 738 |
| [#750](https://github.com/sofia-gros/horizon-mods/pull/750) | 4 soups/stews | `6e46fd8c3dcdb9ae94221bde2576808f358e7d5a` | 743 |
| [#757](https://github.com/sofia-gros/horizon-mods/pull/757) | Molten Glass, slope distance 8 | `d0227d252ee10d35b25b4f27e62d82bd5a105aa8` | 750 |
| [#765](https://github.com/sofia-gros/horizon-mods/pull/765) | Venom/Honey/Liquid Soul contact effects | `4798a0cb184adaf85a943b27681d54f7e824f260` | 757 |
| [#768](https://github.com/sofia-gros/horizon-mods/pull/768) | Magma slime/burn policy | `e5bbbbb1b9a89d1ad1dcc27514fc405daacb468a` | 765 |
| [#784](https://github.com/sofia-gros/horizon-mods/pull/784) | Ichor/Cinderslime upward flow | `9da6c61c74940eeb3b2430c17696f0acc54d63b3` | 768 |
| [#791](https://github.com/sofia-gros/horizon-mods/pull/791) | 3 contact effects marked no_curatives | `b1bec77708152d0fb49cbcbf239a890332426b1c` | 784 |
| [#798](https://github.com/sofia-gros/horizon-mods/pull/798) | Earth Slime and Bouncy status | `0ab9bfd347fac67524dcd6f1e7a034692d015e66` | 791 |
| [#801](https://github.com/sofia-gros/horizon-mods/pull/801) | Sky Slime and Ricochet status | `7c6e4626a9995b86180cfd92eca20a8eb5ddab8a` | 798 |
| [#804](https://github.com/sofia-gros/horizon-mods/pull/804) | Ender Slime/Molten Ender and Enderference player guards | `74b523d4b30bc2cd031e9d40998abb7a85217736` | 801 |
| [#811](https://github.com/sofia-gros/horizon-mods/pull/811) | Bouncy landing/turn audio identity | `bfb7efc9e379e2e0e3f16ed6da8462c8887480d1` | 804 |
| [#824](https://github.com/sofia-gros/horizon-mods/pull/824) | Fog policy for all 70 families | `986f682af56ef71cd3c88f6c289cb865abd6bf19` | 811 |
| [#830](https://github.com/sofia-gros/horizon-mods/pull/830) | Automatic fog colour for 57 families | `37247d5737ace65b89bf4bb27c510e67a13b2777` | 824 |
| [#835](https://github.com/sofia-gros/horizon-mods/pull/835) | Camera overlays for all 70 families | `c9238186fbc3c869c38b5ec08e36cbd9509a9984` | 830 |
| [#866](https://github.com/sofia-gros/horizon-mods/pull/866) | Lava path classification for 59 hot families | `9a4faf43b900d12de5e7f0af39d3a27d55ad940c` | 835 |
| [#932](https://github.com/sofia-gros/horizon-mods/pull/932) | Drowning immunity for 5 slime fluids | `d82b7664cfda55a8a8e850e97808c77dcdbcb732` | 866 |

The JSON companion retains exact base SHAs, dependency paths and source evidence. Head/base metadata is current as of this audit; the original source classification is not changed. The immutable earlier report remains a historical snapshot, including its then-unproved kernel state.

## Existing service, consumer and proof boundaries

### 70 cumulative family declarations, original assets and source policy

**Existing caller:** Original tconstruct/content/data/tconstruct/registered_fluid/*.json; not a proven current Pluto runtime caller.

**Pluto:** Reusable Definition/registry exists; old declaration loader and family startup conversion are not migrated.

**Keep in consumer:** Names/IDs, exact sprites/tints, acquisition/compatibility recipes, source per-fluid policy.

**Evidence scope:** Original heads remain needs_fix. Main is unchanged; library proof is not retroactive.

**Still unproved:** No recovered 70-family consumer activation. No source-equivalent world behavior or full consumer bundle proof.

**Retire old code only if:** Retire old declaration/runtime paths only after each family is mapped, startup/assets installed and consumer compatibility/behavior proved. Preserve assets/provenance.

Evidence: [original_iron](https://github.com/sofia-gros/horizon-mods/blob/d82b7664cfda55a8a8e850e97808c77dcdbcb732/tconstruct/content/data/tconstruct/registered_fluid/molten_iron.json), [original_ichor](https://github.com/sofia-gros/horizon-mods/blob/d82b7664cfda55a8a8e850e97808c77dcdbcb732/tconstruct/content/data/tconstruct/registered_fluid/ichor.json), [world_api](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/world-fluids.md), [official_content](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-mods/src/content.rs), [responsibilities](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-api-boundary.md).

### One process-owned definition registry, atomic batches and revision-bearing snapshots

**Existing caller:** world-fluid adapters and fluid-render preparation consume pluto_api::fluid; no TConstruct registration call found at exact v15 runtime cutoff.

**Pluto:** Implemented: register_all, conflict rejection, deterministic snapshots and late-add invalidation. One registry owner.

**Keep in consumer:** Which families register and when; definition values and fixed-world lifecycle policy.

**Evidence scope:** Real API suite43 passes in grouped composition; whole-library plan/apply/type gate passed. Not live registration/reload parity.

**Still unproved:** No dynamic data-pack replacement/removal. Snapshot validity is not a world/actor permission or long-lived render lease.

**Retire old code only if:** Retire consumer duplicate registries only after every caller uses the single owner and declared startup/late-registration behavior is validated.

Evidence: [fluid_api_source](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/api/src/fluid.rs), [world_api](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/world-fluids.md), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Finite downward world flow and loaded-cell scheduling

**Existing caller:** world-fluid-tick.rs and world-fluid-neighbor.rs; generic World interface and native scheduled BLOCK tick adapter.

**Pluto:** Implemented bounded downward kernel, levels0..8, same-family/air writes and unloaded retry.

**Keep in consumer:** Source family flow choice, slopes/upward policy, mixing/solidification, replaceable blocks and interaction semantics.

**Evidence scope:** Kernel/API tests and actual packaged adapters type-check. Existing native queue/storage reuse is architectural evidence only.

**Still unproved:** No upward flow, slope search, source conversion, mixing, waterlogging or source movement parity. Nine-level prototype is not a drop-in for the original sixteen-level declarations. No live save/reload/LAN scheduling proof.

**Retire old code only if:** Do not discard original source settings. Retire duplicate flow only after the common service covers required behavior and no competing tick hooks remain.

Evidence: [world_api](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/world-fluids.md), [world_adapter](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/world-fluid.rs), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md), [original_ichor](https://github.com/sofia-gros/horizon-mods/blob/d82b7664cfda55a8a8e850e97808c77dcdbcb732/tconstruct/content/data/tconstruct/registered_fluid/ichor.json).

### Authoritative custom source placement and pickup

**Existing caller:** world-fluid-use.rs via native bucket use boundary; place_source/pickup_source through shared World.

**Pluto:** Implemented permission/cancel/hand snapshot checks, source-only fill, world-write-first inventory commit.

**Keep in consumer:** Bucket content/ID, acquisition and desired replacement/container/dispenser behavior.

**Evidence scope:** Actual selected-entry and complete package type proof; not linked gameplay or rollback of post-write side effects.

**Still unproved:** Air-only placement, no vanilla liquid-container fallback; no actual playable placement/flow/pickup/full consumer/save/LAN test.

**Retire old code only if:** Retire consumer bucket operations only after exact source behavior, cancellation and shared permission/hand order are tested in combined consumer.

Evidence: [world_use](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/world-fluid-use.rs), [world_api](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/world-fluids.md), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Built-in water/lava facts, player snapshots and climbability

**Existing caller:** source-native-hook-bodies.rs; source-ender-propagule.rs; source-mining-context-shared.rs; source-projectile-airborne.rs; source-splash-conditional.rs.

**Pluto:** Mechanical native enum/field decoding moved to fluid-facts.rs; motion-query constructs actual MotionWorld with required health.

**Keep in consumer:** Geode/grass/propagule/mining/projectile/splash eligibility and formulas.

**Evidence scope:** Actual v15 library composition type-check includes the extracted adapters; original published missing-health constructors remain original-head failures.

**Still unproved:** Custom ordinary block cells are not native FluidKind; no direct equivalence. Native fact unit tests not all executed; full TConstruct behavior not verified.

**Retire old code only if:** Retire direct host reads only after equivalent caller migration is reconciled; do not restore old constructor from an unrelated PR.

Evidence: [fluid_facts](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/native-fluid-facts.md), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Read-only host-registered tank projection

**Existing caller:** source-gauge.rs uses horizon_client::pluto_fluid::first_snapshot.

**Pluto:** Native ID/amount/capacity projection and validation in fluid-read.rs; no storage mutation.

**Keep in consumer:** Machine selection, capacity policy, display names/layout and invalid-state handling at owned boundary.

**Evidence scope:** Type-checked in actual shared composition. Three private snapshot tests remain separately unexecuted; read-only conversion is not live tank proof.

**Still unproved:** Not an arbitrary capability system or opaque-NBT validator; no fill/drain/sync authority.

**Retire old code only if:** Retire duplicate read decoding only after each gauge/provider preserves missing/default/invalid semantics; write adapters require group06 review.

Evidence: [fluid_facts](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/native-fluid-facts.md), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Reusable tinted crop/tile/cuboid/light mechanics

**Existing caller:** Existing source smeltery/heat/fire/gauge crops; part-draw station blocks; source-side-draw and builder-info-ui crop closures.

**Pluto:** rendering.rs owns reusable arithmetic; real official vertex equivalence checks cover bounded old/new snippets.

**Keep in consumer:** Sprites, source regions, luminosity, hover/page/selection/layout, missing-sprite choices and original hook placement.

**Evidence scope:** Native renderer-type fixture progressed8/10/12/14/16 tests; these are cumulative, not60 distinct tests. Complete shared package type-checked.

**Still unproved:** Not custom terrain fluid tessellation or a full consumer/GPU/visual proof.

**Retire old code only if:** Retire duplicate arithmetic only where guarded equivalence and consumer-specific layout are retained. Do not remove machine/guide policy with it.

Evidence: [rendering](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/rendering.md), [rendering_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/rendering-proof.json), [render_side_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/render-side-proof.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Read-only native potion/effect identities and facts

**Existing caller:** Six mechanical users: source-smeltery-fuel, potion naming, fluid-effect selection, Slurping, Spilling and Splashing.

**Pluto:** One potion-facts owner preserves lookup None, raw/canonical IDs, tuple order, instant and color facts.

**Keep in consumer:** Potion allowlist, NBT schema, custom-effect cap, weighted color, duration scaling, cure/immunity, actor selection and application.

**Evidence scope:** Five actual client-core catalog tests PASS at7b4d6f42; grouped v15 type proof; composed consumer-source guard with named codec delta.

**Still unproved:** No application or network/save proof; syntax-valid ID does not mean registered effect.

**Retire old code only if:** Retire direct catalog access only after raw/canonical/missing behavior and source policy are preserved; do not move source allowlists into library.

Evidence: [catalog](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/potion-facts.md), [catalog_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/potion-facts-proof.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Immutable registered-cell point/box sampling

**Existing caller:** world-fluid-query.rs View and game-private wrappers; no live entity-effect caller installed.

**Pluto:** Shared registered-state decoder, loaded-before-block sampling, exact shape/air behavior and bounded grouped contact kernel.

**Keep in consumer:** When and whose bounds to query, effects, timing, thresholds, owner and permissions.

**Evidence scope:** Actual native-loader fixture8 tests PASS atb9949dcd, then9 with query_heights. These use explicit minimal resources, not production assets. Shared package type-checked.

**Still unproved:** Production gameplay assets and Game-wrapper runtime untested; no with_level flush or world write is introduced.

**Retire old code only if:** Retire duplicated sampling only after same-source decoder/budgets and exact caller timing are established, not by replacing native contact with an unrelated tick query.

Evidence: [contact](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/fluid-contact-query.md), [view_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/fluid-contact-world-view-proof.json), [immersion_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/immersion-curatives-proof.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Caller-explicit adjusted bounds and surface-height immersion calculations

**Existing caller:** Reusable immersion kernel and View.query_heights only; no native contact/physics timing activation.

**Pluto:** Explicit generic height aggregation reuses bounded View sampling and budgets.

**Keep in consumer:** Deflation, boat/eye/body sampling, source contact thresholds and invocation timing.

**Evidence scope:** 12 new pure cases within36-test source suite; real View fixture9 PASS, combined actual API43 with later lease. No effect runtime.

**Still unproved:** Not source getHeight or mixed-fluid travel parity; no swimming/current/buoyancy/drowning/burn hook.

**Retire old code only if:** Retire native sampling duplicates only after an actual synchronous owner caller and source-compatible shape/timing are proved.

Evidence: [immersion_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/immersion-curatives-proof.json), [effect_timing](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/contact-effect-attachment.md), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Immutable per-instance curative policy value versus native attachment

**Existing caller:** Generic effects.rs value/clone/update helpers only; no native EffectInstance attachment or milk activation.

**Pluto:** Generic bounded curative value building block exists; caller chooses item IDs/defaults.

**Keep in consumer:** TConstruct no-curative choices, effect durations/amplifiers, source merge/cure/death/expiry semantics.

**Evidence scope:** Pure API cases pass. Exact E_STATE negative planner observed, not an applied-field success. Unapplied extension/record/legacy proposals have separate scoped tests only.

**Still unproved:** Actual native instance payload is absent; planner E_STATE rejects additive state on public externally constructed EffectInstance. Mob tuple representation differs. Save/hidden-chain/copy/LAN attachment contract unresolved in production.

**Retire old code only if:** Do not retire native/source cure paths until actual lifetime/copy/merge/persistence behavior and consumer activation are established. A policy value alone is not migration completion.

Evidence: [immersion_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/immersion-curatives-proof.json), [effect_attachment](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/contact-effect-attachment-proof.json), [record](https://github.com/sofia-gros/horizon-mods/blob/61f556b159592c59404dc7f11c202e69620daf9c/pluto-lib/docs/legacy-save-contact-migration.md).

### Explicit generic cloud descriptor and native staged factory

**Existing caller:** Reusable descriptor validator and private construct; no TConstruct call enabled.

**Pluto:** Generic timing/radius/effect/color/label/attribution validation and opaque StagedCloud construction.

**Keep in consumer:** All duration/scaling/label/color/effect selection and requested attribution. No implicit current player.

**Evidence scope:** Six actual client-core descriptor cases PASS; actual private factory type-check in complete v15 package.

**Still unproved:** Six descriptor tests do not execute private factory; two factory tests remain unrun. Not a spawn/actor grant.

**Retire old code only if:** Retire consumer native constructor only after an owned caller supplies every field and uses a separately verified insertion path.

Evidence: [cloud](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/cloud-construction.md), [cloud_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/cloud-construction-proof.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Explicit requested attribution bound to actual native Ready/level/actor lookup

**Existing caller:** game-private cloud-insertion.rs capability, unactivated by consumer.

**Pluto:** Validates Ready server role, LevelRef/EntityRef through official resolution, matching descriptor owner, capacity before ID allocation, native publication order.

**Keep in consumer:** Who requested action, allowed policy/phase and exact desired attribution. Explicit Unowned is never inferred.

**Evidence scope:** Complete exact package plan/preflight/apply/lib+bin PASS fromv11b throughv15. Source review is not live insert proof.

**Still unproved:** Detached active guest without resolvable roster is rejected. No resources-backed Game spawn, actor, save/LAN or contact-effect proof. Four private tests remain unrun.

**Retire old code only if:** Retire direct cloud publication only after exact caller authority, phase and behavior are tested. Do not derive authority from equipment DTOs or matching numbers.

Evidence: [insertion](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/cloud-insertion.md), [insertion_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/cloud-insertion-proof.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Resource-owned derived rendering bindings and stale registry rejection

**Existing caller:** PackResources::assemble HEAD prepares derived lookup from one fluid registry; terrain adapter may inspect it.

**Pluto:** Weak exact Assets/RenderAssets/MeshTables owners, immutable bindings, explicit unavailable and registry revision checks.

**Keep in consumer:** Fluid family textures/state selectors and visual choices.

**Evidence scope:** 12 bounded cache/resolver tests include modeled renderer shells; actual library composition type-checked. Live resource-reload proof absent.

**Still unproved:** Prepared binding is not actual custom-fluid tessellation, dynamic reload support or long-lived lease.

**Retire old code only if:** Retire duplicated preparation only after startup registry/owner/reload contracts and every consumer state/sprite are reconciled.

Evidence: [terrain_catalog](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/terrain-binding-catalog.md), [fluid_api_source](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/mixins/pluto_native_creative/api/src/fluid.rs), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Explicit cell geometry and bounded custom-cell mesher dispatch

**Existing caller:** Mesher::mesh_reusing guarded adapter; startup provider registry has no TConstruct provider activated.

**Pluto:** Fixed-size geometry plus version-pinned native caller copy retaining no-claim/fallback behavior; source guards packaged.

**Keep in consumer:** Corner heights, flow direction/UVs, fluid family geometry/light/tint choices and provider registration.

**Evidence scope:** 17 bounded std tests/19 source guards and actual plan/type package proof. Geometry12288 comparisons are pure coordinate evidence, not a mesher runtime result.

**Still unproved:** Actual CPU equivalence/timing and GPU visual proof pending at this snapshot. CI work is separate and no result is inferred. No full custom fluid smoothing, inverted visuals/fog or source rendering parity.

**Retire old code only if:** Retire old terrain drawing only after native no-claim/claimed/stale CPU tests, provider activation and visual/source behavior checks. Do not delete nine-model fallback now.

Evidence: [terrain_adapter](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/terrain-cell-adapter.md), [terrain_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/terrain-cell-adapter-validation.json), [terrain_geometry](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/terrain-cell-geometry-validation.json), [v15_proof](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/shared-foundation-validation-20261105.md).

### Burning, Bouncy/Ricochet/Enderference, fog, path nodes and drowning exceptions

**Existing caller:** Original registered-fluid policy data and Bouncy/Enderference helper hooks exist on PR heads; no migrated world contact grants the effects.

**Pluto:** Only generic facts/query/render/effect-value foundations exist; these source-specific behaviors are not shared-library rules.

**Keep in consumer:** Every source formula, sound/tint/overlay, potion identity, immunity, path choice, momentum and per-family behavior.

**Evidence scope:** Original PR source inventory only; no new tests/CI/state changes. Shared type-proof does not resolve consumer conflicts.

**Still unproved:** Old Bouncy StatusFlags fields absent from official. Fog/overlay/path/slime tag declarations have no completed common runtime consumer. Mixed-fluid motion, items/projectiles/mobs, inversion, mixing, dispensers and save/LAN remain distinct.

**Retire old code only if:** Retire old hooks only after replacement preserves callback order/source outcomes and reconciles equipment feature ownership; keep content data/provenance.

Evidence: [original_bouncy](https://github.com/sofia-gros/horizon-mods/blob/d82b7664cfda55a8a8e850e97808c77dcdbcb732/tconstruct/mixins/tconstruct/source-bouncy-status.rs), [official_status](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client-render/src/effects.rs), [official_fluid](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-world/src/fluid.rs), [effect_timing](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/contact-effect-attachment.md), [world_api](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/world-fluids.md).

### Actual same-invocation Game borrow and post-move continuation

**Existing caller:** Existing official static block_contact callback has no mutable Game; proposed private guest Game probes are uncalled and unapplied.

**Pluto:** No completed synchronous effectful-contact owner adapter. This is research, not production migration.

**Keep in consumer:** TConstruct contact eligibility/effects; native owner must determine legal continuation after body/level/resource changes.

**Evidence scope:** Prepared positive/expected-E0499 Game probes and10 source checks each; no actual native result. Prototype10/19/7 tests are excerpt/record/legacy inspection, not installed save/effect/contact proof.

**Still unproved:** Borrow probes have source checks only, compiler/CI outcome pending. Guest observation helper uncompiled/unrun. Same ID/epoch/position is not a generation/lifetime proof; leave_guest notification can reenter, panic restoration not guaranteed. Mob detached roster/navigation loans remain separate.

**Retire old code only if:** No current runtime boundary is eligible for retirement based on these experiments. Require actual owner/frame implementation, native same-pass order and persistence/copy validation.

Evidence: [official_contact](https://github.com/horizon-rust/horizon/blob/624c4f6265c02296fbe8a80f029dd0d987fb94c0/crates/horizon-client/src/block_contact.rs), [effect_attachment](https://github.com/sofia-gros/horizon-mods/blob/89744814b8d536a117e82f8ea3c8cdc9798faa95/pluto-lib/docs/contact-effect-attachment-proof.json), [probe](https://github.com/sofia-gros/horizon-mods/blob/61f556b159592c59404dc7f11c202e69620daf9c/pluto-lib/docs/guest-game-borrow-v15.md), [continuation](https://github.com/sofia-gros/horizon-mods/blob/61f556b159592c59404dc7f11c202e69620daf9c/pluto-lib/docs/guest-continuation-observation.md), [record](https://github.com/sofia-gros/horizon-mods/blob/61f556b159592c59404dc7f11c202e69620daf9c/pluto-lib/docs/legacy-save-contact-migration.md).

## Immediate dependency order

1. Keep the proven library composition and mandatory guards intact. The ordinary launcher still does not execute the required full-plan guards.
2. Finish bounded real terrain CPU and actual Game borrow evidence through the coordinated CI owner. No result is assumed in this inventory.
3. Resolve same-invocation actor/frame continuation and native instance copy/merge/save/curative attachment. Do not replace them with numeric IDs, a pointer table, a later tick, or an unapplied proposal.
4. Migrate one coherent TConstruct consumer group, preserving source content and policy and removing conflicting old hooks only under the conditions above. The generic downward nine-level prototype does not cover all original flow settings.
5. Validate the combined consuming bundle, then affected gameplay, production resources, save/reload and LAN. Library-only and minimal synthetic-resource fixtures remain narrower evidence.

No PR state, branch base or runtime source changed during this inventory. No new native tests, CI inspection, official-host modifications or prohibited-fork access occurred.

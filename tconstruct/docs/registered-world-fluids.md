# Registered world-fluid draft

This slice declares fourteen molten world-liquid families: iron, gold, copper,
cobalt, debris, netherite, Slimesteel, Amethyst Bronze, Rose Gold, Pig Iron,
Manyullyn, Hepatizon, Queen's Slime and Soulsteel.
They use actual sixteen-level world liquids. Existing 1000 mB tank/bucket paths
supply the items. The host's registered-fluid system handles placement, flow,
source pickup, scheduling and saving, rendering, player current/travel and burn
contact using each family's identity.

**Requires the unmerged Horizon registered-fluid stack through
[Horizon fork PR #24](https://github.com/sofia-gros/horizon/pull/24).** Keep this
mod PR draft until the host support is integrated and the final compatibility
build is checked. This does not change the released compatibility pin or claim
that the current release can run these definitions.

These declarations ship through the native `.hmod` content overlay. The legacy
manual resource-preparation path does not install registered world liquids.

Pinned TConstruct source at `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:

- `FluidDeferredRegisterExtension.registerMetal`: level drop 3, slope distance 3,
  tick rate 30 for all declared families except debris.
- Molten debris uses `registerGem`: level drop 4, slope distance 4, tick rate 45.
  None of these fourteen families generates new sources.
- `TinkerFluids.hot`: current scale 0.0023333333333333335, no swimming or drowning.
- `TinkerFluids` declarations and `BurningLiquidBlock`: 10 seconds burning;
  iron/gold/copper deal 5, cobalt 6, debris 8 and netherite 10;
  light 12/12/12/8/14/14.
- Slimesteel, Amethyst Bronze, Rose Gold and Pig Iron each deal 6 damage with
  the same 10-second burning duration; their light levels are 10/12/12/10.
- Manyullyn, Hepatizon, Queen's Slime and Soulsteel deal 8/7/6/7 damage with
  10 seconds burning; their light levels are 11/8/9/6.
- Iron, gold, copper and cobalt retain their exact still/flow sprites and
  animation metadata from `textures/fluid/molten/ore/<metal>/`.
  `content/reference/registered-world-fluid-assets.json` records source paths,
  revision and SHA-256 checksums. Source artwork remains under the existing
  SlimeKnights MIT license in `third_party/tconstruct/LICENSE`.
- `FluidTextureProvider` gives debris the shared `molten/stone` still/flow
  sprites with RGB tint `0x411E15`; netherite uses its dedicated
  `molten/alloy/netherite` sprites with white tint. Both retain the source
  animation metadata.
- The four overworld alloys each use their dedicated `molten/alloy/<name>`
  still/flow sprites with white tint and source animation metadata. Pig Iron
  retains its pink artwork instead of reusing another metal's sprite.
- Manyullyn, Hepatizon, Queen's Slime and Soulsteel also retain their dedicated
  alloy sprite pairs and source animation metadata. The provenance manifest
  covers all fourteen families. Queen's Slime's source camera fog color remains
  outside the current host schema and is not applied as a world-fluid tint.

Cinderslime is intentionally undeclared: the source uses an inverted fluid,
which cannot be represented by the host's current downward-only flow policy.
Soulsteel's existing machine/bucket paths remain unchanged; declaring its world
liquid does not introduce a new survival acquisition recipe.

The explicit travel coefficients use Horizon's water-style drag/falling
integrator without assigning native water tags. Their equivalence to Forge's
generic fluid travel, fall scaling and mixed-fluid precedence remains unverified.
They are provisional host integration parameters, not source-parity claims.

Remaining: every other fluid family, source-specific potion contact effects and
curatives, mob/item/projectile interaction, burn audio, camera fog, mixing and
solidification, inverted flow, dispenser handling, final build and actual
placement→flow→pickup→save/reload/LAN acceptance. The `#379` high-priority fluid
item remains incomplete. The new focused resource regressions are included for
the final test batch; full tests, CI checks and merges are intentionally deferred.

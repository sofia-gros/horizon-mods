# Registered world-fluid draft

This slice declares molten iron, gold, copper and cobalt as actual sixteen-level
world liquids, not solid placeholder blocks. Existing 1000 mB tank/bucket paths
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
  tick rate 30. These families do not generate new sources.
- `TinkerFluids.hot`: current scale 0.0023333333333333335, no swimming or drowning.
- `TinkerFluids` declarations and `BurningLiquidBlock`: 10 seconds burning;
  iron/gold/copper deal 5, cobalt 6; light 12/12/12/8.
- Exact still/flow sprites and animation metadata are retained from each
  `textures/fluid/molten/ore/<metal>/` path. No white fallback or lava texture.
  `content/reference/registered-world-fluid-assets.json` records source paths,
  revision and SHA-256 checksums. Source artwork remains under the existing
  SlimeKnights MIT license in `third_party/tconstruct/LICENSE`.

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

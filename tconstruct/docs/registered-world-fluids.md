# Registered world-fluid draft

This slice declares sixty world-liquid families. The initial families are iron, gold, copper,
cobalt, debris, netherite, Slimesteel, Amethyst Bronze, Rose Gold, Pig Iron,
Manyullyn, Hepatizon, Queen's Slime, Soulsteel, Seared Stone, Scorched Stone,
molten Clay, Glass, Porcelain, Obsidian, Emerald, Quartz, Amethyst, Diamond,
Blazing Blood and Fiery Liquid. Four cool food liquids are also included:
Beetroot Soup, Mushroom Stew, Rabbit Stew and Meat Soup.
The remaining thirty conventional metals and compatibility alloys are listed
below. Their existing availability and acquisition conditions remain unchanged.
They use actual sixteen-level world liquids. Existing 1000 mB tank/bucket paths
supply the items. The host's registered-fluid system handles placement, flow,
source pickup, scheduling and saving, rendering, player current/travel and
declared contact policies using each family's identity.

**Requires the unmerged Horizon registered-fluid stack through
[Horizon fork PR #25](https://github.com/sofia-gros/horizon/pull/25).** Keep this
mod PR draft until the host support is integrated and the final compatibility
build is checked. This does not change the released compatibility pin or claim
that the current release can run these definitions.

These declarations ship through the native `.hmod` content overlay. The legacy
manual resource-preparation path does not install registered world liquids.

Liquid blocks use Mantle's source `<fluid>_fluid` naming. Source and flowing
fluid IDs and bucket IDs are unchanged. This keeps `seared_stone_fluid` and
`scorched_stone_fluid` separate from the existing solid masonry blocks. The
fourteen earlier draft liquid blocks are renamed consistently; this is a
pre-release draft correction, not a migration of already saved draft worlds.

Pinned TConstruct source at `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:

- `FluidDeferredRegisterExtension.registerMetal`: level drop 3, slope distance 3,
  tick rate 30 for the forty-three metal/alloy families.
- Molten Debris, Emerald, Quartz, Amethyst and Diamond use `registerGem`:
  level drop 4, slope distance 4, tick rate 45.
- Seared Stone, Scorched Stone, Clay, Porcelain and Obsidian use `registerStone`:
  level drop 2, slope distance 2, tick rate 40.
- Blazing Blood uses Mantle `FluidBuilder` defaults: level drop 1, slope
  distance 4, tick rate 5. Fiery Liquid keeps drop 1/slope 4 but explicitly
  uses tick rate 30. The four soups also use the defaults: drop 1, slope 4,
  tick rate 5. None of the sixty families generates new sources.
- Molten Glass uses `registerGlass`: level drop 8, slope distance 8, tick rate 30.
  It needs the extended host slope bound; its policy is not approximated as
  stone or clamped to four. It burns for 5 seconds with damage 2 and light 1.
- `TinkerFluids.hot`: current scale 0.0023333333333333335, no swimming or drowning
  for the fifty-six hot families.
- The four soups use `TinkerFluids.cool`: the same current scale, extinguishing
  enabled, and Forge's default swimming/drowning and 0.5 fall scaling. They
  have no burn contact and emit no light. Their 400 K source temperature does
  not make them burning liquids. Defaults are read from `FluidType.Properties`
  at the existing bucket-reference Forge source revision
  `243490d78001917c7e6ee6e94b45ef98776aa36f`.
- `TinkerFluids` declarations and `BurningLiquidBlock`: 10 seconds burning;
  iron/gold/copper deal 5, cobalt 6, debris 8 and netherite 10;
  light 12/12/12/8/14/14.
- Slimesteel, Amethyst Bronze, Rose Gold and Pig Iron each deal 6 damage with
  the same 10-second burning duration; their light levels are 10/12/12/10.
- Manyullyn, Hepatizon, Queen's Slime and Soulsteel deal 8/7/6/7 damage with
  10 seconds burning; their light levels are 11/8/9/6.
- Seared Stone, Scorched Stone, Clay, Porcelain and Obsidian deal 2/2/2/2/4 damage,
  burn for 8/7/5/5/12 seconds, and emit light 6/4/3/2/3 respectively.
- Emerald, Quartz, Amethyst and Diamond deal 6/5/5/7 damage with 10 seconds
  burning and light 9/6/11/13. Each uses its exact `molten/ore/<gem>` still/flow
  sprites and animation metadata with white surface tint. Source camera fog
  differences are not represented by the current host schema.
- Blazing Blood deals 5 damage and burns for 15 seconds; Fiery Liquid deals 6
  and burns for 20 seconds. Both emit light 15. They retain their source
  `molten/blaze` and `slime/fiery` sprite pairs and animation metadata.
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
  covers all sixty families. Queen's Slime's source camera fog color remains
  outside the current host schema and is not applied as a world-fluid tint.
- Seared Stone, Scorched Stone and Clay share the source stone sprites with
  tints `0x4F4A47`, `0x3E3029` and `0x9B6045`; Porcelain and Obsidian use their
  own `molten/stone/<name>` sprites. Porcelain availability still depends on
  existing recipes and compatibility content; no new acquisition is added.

Additional conventional metal families:

- Steel, Knightmetal and Knightslime.
- Tin, Aluminum, Lead, Silver, Nickel, Zinc, Platinum, Tungsten, Osmium,
  Uranium, Chromium and Cadmium.
- Bronze, Brass, Electrum, Invar, Constantan and Pewter.
- Enderium, Lumium, Signalum, Refined Glowstone, Refined Obsidian, Nicrosil,
  Duralumin, Bendalloy and Steeleaf.

All thirty use metal flow and ten seconds burning. The source-specific damage,
light, tint and texture expectations are recorded in
`tests/test_remaining_world_metals.py`. Nicrosil intentionally retains the
source's different fluid emission (14) and block light (12). Chromium, Cadmium
and Bendalloy share the source molten sprite with tints `0xC4B180`, `0x8AB8C8`
and `0xD6D2C1`; the others retain their own ore/alloy/compatibility sprite pairs.
This does not install external mods or add their ores, ingredients or recipes.

The four soup liquids share the exact `food/stew` source still/flow sprites and
animation metadata, with RGB tints `0x84160D`, `0xCD8C6F`, `0x984A2C` and
`0xE03E35` respectively. Their existing bucket IDs are reused, and their liquid
blocks use the `_fluid` suffix without replacing bowl food items. This adds no
food consumption, nutrition, bowl transfer, Suspicious Stew or potion effects.
Ordinary bowl supply is separate work in [PR #680](https://github.com/sofia-gros/horizon-mods/pull/680).
The source camera fog and sound settings remain outside this declaration schema.

Cinderslime is intentionally undeclared: the source uses an inverted fluid,
which cannot be represented by the host's current downward-only flow policy.
Soulsteel's existing machine/bucket paths remain unchanged; declaring its world
liquid does not introduce a new survival acquisition recipe.
Fiery Liquid also retains its existing compatibility/acquisition conditions;
this declaration does not add external materials, recipes or mobs.
Molten Glass retains the exact `molten/glass` still/flow sprites and animation
metadata with white surface tint. Its source camera fog range remains outside
the current schema. The required host extension accepts slope distances through
eight and bounds repeated search work with a per-spread cache, discarded before
the next tick. Previous host drafts capped this distance at four and reject the
new Glass declaration.

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

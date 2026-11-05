# Registered world-fluid draft

This slice declares sixty-seven world-liquid families. The initial families are iron, gold, copper,
cobalt, debris, netherite, Slimesteel, Amethyst Bronze, Rose Gold, Pig Iron,
Manyullyn, Hepatizon, Queen's Slime, Soulsteel, Seared Stone, Scorched Stone,
molten Clay, Glass, Porcelain, Obsidian, Emerald, Quartz, Amethyst, Diamond,
Blazing Blood and Fiery Liquid. Four cool food liquids are also included:
Beetroot Soup, Mushroom Stew, Rabbit Stew and Meat Soup.
Venom, Honey and Liquid Soul add the first standard player contact effects.
Magma adds a burning slime-family liquid. Ichor and Molten Cinderslime add
the first upward-flowing liquids. Earth Slime adds the Bouncy player effect.
The remaining thirty conventional metals and compatibility alloys are listed
below. Their existing availability and acquisition conditions remain unchanged.
They use actual sixteen-level world liquids. Existing 1000 mB tank/bucket paths
supply the items. The host's registered-fluid system handles placement, flow,
source pickup, scheduling and saving, rendering, player current/travel and
declared contact policies using each family's identity.

**Requires the unmerged Horizon registered-fluid stack through
[Horizon fork PR #31](https://github.com/sofia-gros/horizon/pull/31).** Keep this
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
  tick rate 30 for the forty-four metal/alloy families.
- Molten Debris, Emerald, Quartz, Amethyst and Diamond use `registerGem`:
  level drop 4, slope distance 4, tick rate 45.
- Seared Stone, Scorched Stone, Clay, Porcelain and Obsidian use `registerStone`:
  level drop 2, slope distance 2, tick rate 40.
- Blazing Blood uses Mantle `FluidBuilder` defaults: level drop 1, slope
  distance 4, tick rate 5. Fiery Liquid keeps drop 1/slope 4 but explicitly
  uses tick rate 30. The four soups also use the defaults: drop 1, slope 4,
  tick rate 5. None of the sixty-seven families generates new sources.
- Molten Glass uses `registerGlass`: level drop 8, slope distance 8, tick rate 30.
  It needs the extended host slope bound; its policy is not approximated as
  stone or clamped to four. It burns for 5 seconds with damage 2 and light 1.
- `TinkerFluids.hot`: current scale 0.0023333333333333335, no swimming or drowning
  for the fifty-eight hot families, including non-burning Liquid Soul.
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
  covers all sixty-seven families. Queen's Slime's source camera fog color remains
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

The standard player contact-effect slice adds:

- Venom: Poison for 100 ticks, amplifier 0; default drop 1, slope 4, tick rate 5.
- Honey: Slowness for 100 ticks, amplifier 0; drop 1, slope 4, tick rate 50.
- Liquid Soul: Blindness for 100 ticks, amplifier 0; glass drop/slope 8 and tick
  rate 30. It emits light 2 but does not burn. Its hot-fluid properties disable
  swimming, drowning and extinguishing; Venom and Honey enable those flags.

The three retain exact `slime/venom`, `food/honey` and `slime/soul` source artwork
and animation metadata with white surface tint. Existing bucket and acquisition
paths remain unchanged. Only the authoritative player path adds effects, using
positive body height for the matching family, with ordinary effect merging,
refresh, expiry and network synchronization. All three effects explicitly set
`no_curatives: true`, following `MobEffectLiquidBlock`'s empty curative-item list.
The required host curative extension keeps those instances through milk-style
generic cures, including after leaving the liquid, and persists the active
instance policy in owner/guest playerdata. Direct targeted removal, commands,
death and expiry still remove effects. Remote clients wait for authoritative
cure results rather than clearing protected effects speculatively.

This is not arbitrary item-specific curative support or full effect parity.
The host attaches the policy to accepted duration/amplifier updates and restores
hidden-instance policy on downgrade; mixed-source equivalence with Forge is
unverified. Its existing save model does not persist hidden chains. Mobs,
custom slime/Enderference effects, fog and gameplay/LAN validation remain separate.

Earth Slime retains source slime flow (drop 1, slope 4, tick rate 50), light 0,
no burn, and enabled swimming/drowning/extinguishing. Matching body contact
applies `tconstruct:bouncy` for 100 ticks, amplifier 0, with no generic curatives.
The effect is registered as beneficial with source color `0x71AC63` and the
original status icon. Its current presence enables the host's explicit
`bounce_on_fall` flag after each normal effect-status update; expiry/removal
clears it and synchronized remote/guest effects use the same binding.

The required host landing extension handles source threshold/step-height guards,
0.5 crouching damage multiplier, gravity-based authoritative rebound, client
reverse-velocity factors, post-collision application and velocity sync. It does
not enable the archived armor-bounce injections. Source repeated air-momentum
preservation, bounce sound and mob behavior remain incomplete. Native injection
planning/build and actual landing/expiry/LAN behavior are unverified, so this is
not full Bouncy parity. The source still/flow art and animation metadata use
`slime/earth`; the separate `slime-effect-assets.json` records the original
Bouncy icon provenance. Existing Earth Slime bucket/bottle/cake paths and solid
slime blocks remain unchanged.

Magma retains `registerSlime` flow: drop 1, slope 4 and tick rate 50, with light 3
and 8 seconds/3 damage of burning contact. Despite burning, its source uses the
`slime()`/`cool()` properties: player swimming, drowning and extinguishing stay
enabled. Do not replace these with the molten-metal flags. The host declares
both extinguishing and burn behavior; exact same-tick ordering needs the final
source-parity check. It uses exact `slime/magma` still/flow sprites and metadata.
`SlimeFluid` adds no active custom flow beyond source/flowing levels; its dirt
conversion idea is commented out in the source and is not implemented here.
`SlimeFluidType` exempts slime-tagged mobs from drowning. The current host path
is player-only, so that mob exception remains unimplemented. Existing Magma
bucket/bottle/acquisition paths and solid Magma Slime blocks are unchanged.

Ichor and Molten Cinderslime explicitly set `policy.vertical_direction` to `up`.
Ichor retains slime flow (drop 1, slope 4, tick rate 50), light 0, no burning,
and enabled player swimming/drowning/extinguishing. Cinderslime retains metal
flow (drop 3, slope 3, tick rate 30), light 10 and 10 seconds/7 damage of burning,
with swimming, drowning and extinguishing disabled. Neither creates new sources or adds a
contact status effect. Existing bucket and acquisition paths are reused.

Their exact `slime/ichor` and `molten/alloy/cinderslime` source artwork includes
still, flowing and `flowing_inverted` sprites and all animation metadata. The
normal flowing path stays in the declaration; the host chooses its `_inverted`
variant in the world. This matches Mantle's `ClientInvertedFluidType`, including
reversed source animation frame lists, rather than flipping a different sprite.
The provenance manifest now records 240 original fluid files. Source inspection of
Mantle `InvertedFluid` and its type shows unchanged liquid surface/contact
geometry: only simulation, current direction and the flowing sprite change.
The host therefore does not invent ceiling-anchored geometry. The new upward
simulation and current/render paths require Horizon fork PRs #27 and #28 in
addition to the previous stack. Ichor's slime-mob drowning exemption, sound and
fog differences remain outside the player-focused host schema.

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

Remaining: Sky Slime, Ender Slime and Molten Ender world fluids,
source-specific contact effects, item-specific curatives and mixed-source merge parity, mob/item/projectile interaction, burn audio, camera fog, mixing and
solidification, dispenser handling, final build and actual
placement→flow→pickup→save/reload/LAN acceptance. The `#379` high-priority fluid
item remains incomplete. The new focused resource regressions are included for
the final test batch; full tests, CI checks and merges are intentionally deferred.

# Mining, durability and repair trait follow-up

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

Material mining, durability and repair traits use original Tinkers' Construct
revision `f4e77d43` with shared mining, wear and repair paths.

This unit covers the mining modules of Lightweight, Heavy, Vintage, Entangled,
Jagged, Stonebound, Maintained and Crumbling, plus Dense repair and durability prevention.
Overlord requires functional Overslime and remains unimplemented. Other modules
of these traits are not implied to be complete.

## Implementation and limits

Station and Creative regeneration store the original harvest-tagged tool's
Lightweight/Heavy/Vintage/Entangled stat and Jagged/Stonebound/Maintained levels on the item. A shared item boundary recognizes effective per-stack tool
rules for both the client and host using the active world item tags; separate context hooks apply conditional
speed after mining efficiency and potion effects, before underwater/airborne
penalties. Materials and part swaps rebuild the stored values. Previously saved
tools need regeneration before these new fields are present.

Dense repair uses the original level ranges. Dense damage prevention rolls once
per durability point through the existing shared `durability_loss` path.
Original conditional wear formulas use the `multiply_all` durability multiplier;
the earlier melee calculation mistakenly included material percent and
`multiply_base` bonuses. The implementation corrects that multiplier for the existing
Jagged/Stonebound melee path too. Representative melee and repair regressions passed.

Crumbling adds one mining speed per level (scaled by the tool and player mining multipliers) on blocks that do not require a tool, even when the held tool is not effective. It skips broken tools. An isolated Linux test client with eight unrelated injections excluded measured dirt at 15 versus 13 ticks and stone at 18 in both cases; a broken tool took 56 ticks on dirt. Crumbling combined with Stonebound remains unresolved: a piston comparison gave 10 versus 14 ticks despite a higher calculated conditional speed.

Dwarven is connected on the development branch between Maintained and the percentage-based Jagged and Stonebound effects. An isolated Linux test client with eight injections excluded measured 13 ticks at Y=40 and 43 at Y=240 using supplied parts and a trait fixture; existing tools need regeneration to receive the trait data.

Original modifier priority runs Maintained before Dwarven and the percentage-based Jagged
and Stonebound effects; equal-priority effects retain source insertion order.
The development branch uses PlutoLib transport for prepared per-stack `minecraft:tool` data; the complete TConstruct LAN path remains unverified. Legacy custom tool data is retained for existing boundaries. Existing saved tools need a supported regeneration or part replacement
before the new fields are added. See [Harvest tiers and effective blocks](harvest-rules.md)
for representative tier, drop and LAN validation; not all tools and materials
have been individually tested.

## Representative Linux verification

The official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` transform
(173.42 seconds) and client/ctl build (134 seconds) passed. A temporary test
datapack placed the original trait IDs at selected levels on a flint head while
keeping the other tool materials identical. In the controlled stone test,
measured times included the same 5-tick post-break delay: baseline 18 ticks,
Lightweight level 2 at 17, Heavy level 2 at 15, Vintage level 2 at 22, and
Entangled at 10. Jagged level 3 changed from 18 fresh to 21 worn and 23 at damage 84/85; the broken tool was retained. Stonebound
changed from 18 to 17, and Maintained from 10 to 18. On dirt, a block ineffective
for the pickaxe, the baseline and Entangled both took 20 ticks.

Reversing material positions gave equal times for Maintained with Jagged (22
and 22 ticks) and Stonebound (17 and 17). Efficiency II was applied using a
test item modifier because the current vanilla enchant command rejected the
registered TConstruct item; with Haste and wear, baseline versus Jagged took
10 versus 11 ticks. A deterministic unit test also covers the operation order.
One Dense level 3 test prevented 10 of 12 wear points; after replacing that
head, four of four blocks caused ordinary wear. Dense reduced representative
repair from 51 to 42 durability restored. Jagged mining stayed at 21 ticks
before and after save/reload. A LAN guest mined stone in 7 ticks using a
command-supplied cobalt-head tool regenerated through the Station; ordinary
vanilla iron-pickaxe mining and the existing Jagged/Stonebound melee and
Cultivated/Preserved repair paths were also checked.

These are representative checks, not exhaustive tests of every material,
tool, effect combination or Windows. The selected mining traits' other modules,
applied mining modifiers, and Overlord/Overslime remain incomplete. Full
Mixins CI and independent review are still pending.

![Dense repair in the Tinker Station](material-mining-repair.png)

![LAN guest's cobalt tool](material-mining-lan.png)

## Airborne mining

The Skyslime Vine Airborne trait multiplies mining speed by five while the
holder is off the ground, including on blocks where the tool is ineffective.
It applies after flat conditional mining bonuses and does not apply to broken
tools. The host and client selectors target the final speed read after airborne and
water penalties, so flat bonuses can be applied in the source order. The
transformed combined build and runtime ordering still require verification. Rebuilding or swapping material updates the stored trait. This implements
the mining portion of the [original Airborne definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/airborne.json);
the conditional projectile velocity portion remains unimplemented.

Focused local Rust tests checked trait presence, grounded and broken cases,
and the Maintained combination after airborne and water penalties, using the
saved older host NBT library. An isolated official Mixins plan resolved both
final speed reads and their local captures; the full combined plan remains pending. Current-target compilation, gameplay,
material acquisition, save/reload and LAN remain unverified. In the combined
client, make a supported harvest tool with Skyslime Vine, compare the same
block on the ground and while airborne, including one ineffective block, then
repeat after breaking the tool and after replacing that material. Capture both
local and LAN guest behavior; use a labelled fixture if the material is not yet
naturally obtainable.

## Overwield mining

For a non-broad harvest tool with Knightslime's Overwield trait, an effective
block receives the source multiplier `1 + 0.2 * min(overslime, level)`. It uses
the current stored Overslime amount, not capacity, and applies in the same
ordered percentage stage as Jagged and Stonebound. The tool is checked against
the loaded `tconstruct:modifiable/broad` item tag. Zero Overslime gives no
bonus; broad tools and ineffective blocks do not receive this mining effect.
The broad variant is described below. Both correspond to the mining-speed modules in the [source
Overwield definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/overwield.json).
Overslime consumption, broad-tool mining-capacity behavior and material
acquisition remain separate work.

Rust syntax was checked; focused tests, current-target build and gameplay are
pending. Verify zero, one and multiple Overslime units and trait levels, a broad
and a non-broad tool, effective and ineffective blocks, live refill and
consumption, broken tools, and combinations with other mining modifiers. Check
material replacement and save/reload. Use labelled fixtures where necessary.

### Broad tools

The broad-tool speed module instead uses `1 + 0.2 * min(overslime, 5 * level) / 5`
on effective blocks. This reads the same live Overslime value and uses the loaded
broad tag; it does not use AOE size as the amount. The source additionally
spends up to five Overslime per level at the mining capacity hook. That spend
is not connected by this speed change and must be implemented before this can
be considered complete.

Rust syntax was checked. Tests, current-target build and gameplay are pending.
Check levels 1 and 2 at amounts 0, 1, 4, 5, 6 and 10, ineffective blocks,
AOE center and extra blocks, material swaps, save/reload and multiplayer.

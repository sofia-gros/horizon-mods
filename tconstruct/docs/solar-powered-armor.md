The subsequent [Overslime armor extension](overslime-armor-wear.md) adds shield
absorption after Solar and before Unbreakable, without changing Solar draws.

# Solar Powered ordinary armor wear

This issue #379 slice connects Solar Powered to the retained ordinary armor-wear
path, stacked on Reinforced/Unbreakable #931 (after Dense #929, Tanned #925 and
retained writeback #923). It does not enable every armor modifier or claim whole-row completion.

## Runtime

The exact pinned `solar_powered.json` definition is required. Resolved material
and saved upgrade levels are combined with the existing overflow/malformed-data
checks. Eligible intact stock armor still requires the correct worn slot,
live armor/durability tags, no native fixed-max-damage ownership, and a supported
complete modifier set from the underlying armor-wear path.

The effective light is the acting player's floored feet skylight minus the
current world sky darkening, clamped at zero. Solar Powered's prevention chance
is `(0.01f * level + 0.04f) * effectiveLight`, evaluated in the source float
operation order. Each remaining wear point rolls once; a draw equal to the
chance keeps the point. Zero amount, zero chance and certain prevention do not
advance the random stream.

The effect runs after Tanned (priority 200) and before Unbreakable (125) and the
Dense/Reinforced equal-priority group (100). Solar itself is priority 185.
Native durability/item-damage callbacks then receive the reduced amount once.
The existing final-loss collector writes back the matched armor stack, retains
broken gear and refreshes Maintained when its durability changes. Recording and
commit validation do not roll a second time.

The local holder and LAN guest both use the active `Game.player` and world,
never the host camera's position. Darkening is sampled from the world's time
and weather through the native sky-darkening evaluator, not the render frame's
cached `darken`. A missing/unlit column or invalid position does not substitute
full daylight. The pinned vanilla Nether and End have no skylight, including
above the host column's height where its generic light accessor returns 15.
No extra persistent counters or item fields are added.

## Deliberate limits

Only the ordinary `wear_armor`/`hurt_armor` scope is connected. Held/shield,
projectile/detached, modifier-caused wear, arbitrary custom dimension sky rules,
unknown/changed definitions and other unsupported armor combinations remain
outside this slice. Overslime absorption and Overshield's protection consumption
are separate unfinished work; pieces carrying those unsupported modifiers
remain excluded by the underlying writeback gate.

## Source and review

Pinned Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`:

- `library/json/variable/entity/EntityLightVariable.java`
- `library/modifiers/modules/behavior/ReduceToolDamageModule.java`
- generated `data/tconstruct/tinkering/modifiers/solar_powered.json`

The earlier Solar Powered local-tool candidate is not copied into this branch;
its old Tanned signature and inactive armor-writeback route are not reintroduced.

Brief static review covers source ordering, actor/dimension selection, loaded
light guards, native API/privacy and single application before the existing
final writeback. Four pure rule tests were added but not run. No build, planner,
tests, CI inspection, version checks or merge were performed.

This slice adds no native extension. Its defense-chain base still requires the
separate native signed-protection `09f0179db` and upper-bound `5e47547e2` changes;
combined gameplay, save/reload and LAN behavior remain unverified.

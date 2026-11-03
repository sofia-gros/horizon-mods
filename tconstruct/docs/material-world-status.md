# Material, equipment and world status for v0.5

This is a focused working list for F02, F05, F06, F12, F14 and F15. A draft
is a source candidate, not proof of a working feature. These rows do not claim
that the rest of the original material, modifier or loot inventory is complete.
The complete functional plan and the individual source-parity documents remain
the scope of the audit.

## Submitted candidates

| Area | Candidate and current evidence | Remaining acceptance |
| --- | --- | --- |
| Fixed tool traits | [#152](https://github.com/sofia-gros/horizon-mods/pull/152) resolves fixed tool-definition traits. | Verify source levels and slot conditions through assembly, modification and save/reload on the combined target. |
| Raging shots | [#91](https://github.com/sofia-gros/horizon-mods/pull/91) adds projectile damage; [#140](https://github.com/sofia-gros/horizon-mods/pull/140) corrects acting-player state. | On the same shot, compare local health 20 and 6 and confirm the correct guest's health is used; check resulting arrow damage and vanilla regression. |
| Spiny ranged speed | [#150](https://github.com/sofia-gros/horizon-mods/pull/150) supplies the velocity component. | Check levels, draw/release and broken tools on the integrated target; the separate source self-damage is not included. |
| Snow Boots | [#156](https://github.com/sofia-gros/horizon-mods/pull/156) and [#158](https://github.com/sofia-gros/horizon-mods/pull/158) address walking and placement collision. | Check equip, unequip, broken state and vanilla boots locally and for a guest. |
| Bouncy | [#243](https://github.com/sofia-gros/horizon-mods/pull/243), [#245](https://github.com/sofia-gros/horizon-mods/pull/245), and the aligned [#247](https://github.com/sofia-gros/horizon-mods/pull/247) are unverified local/guest candidates. Package-only checks for #243 and the aligned #247 head cover archive structure, including state-present and state-absent fixtures, not host compatibility. | Run official combined package, apply and type checks; compare crouched and ordinary fall damage, rebound and sent guest velocity, repeated landings, broken armor and player isolation. |
| Earth/Sky islands | The b53/ddd integration plus [#244](https://github.com/sofia-gros/horizon-mods/pull/244) passed official resource preparation. Natural dirt, leaves and logs were observed; representative Earth leaf, dirt and log collection and persistence were checked using supplied tools and teleportation. | Validate all foliage placements, actual sounds, other island types, LAN and source placement conditions. Source fluids and foliage terrain tint remain excluded. |
| Ender tree | [#240](https://github.com/sofia-gros/horizon-mods/pull/240) supplies root and vine components; [#242](https://github.com/sofia-gros/horizon-mods/pull/242) is an unverified planted-propagule candidate. | Apply and type check; compare both tree variants, root replacement, waterlogging, obstruction, soil, decorators, RNG order, chunk boundaries and save/reload. Island tree markers remain separate. |
| Blood/Ichor fungi | [#231](https://github.com/sofia-gros/horizon-mods/pull/231) covers planted fungi and [#236](https://github.com/sofia-gros/horizon-mods/pull/236) covers Blood island fungus markers. | Confirm normal growth, source blocks and placement constraints, marker generation and persistence on the combined target. |

## Known missing or narrowly deferred operations

- **Spiny self-damage:** retain the source-specific damage type and amount. The
  confirmed damage registration boundary is missing; velocity alone is not
  completion. Revisit when the dedicated damage API exists.
- **Ender island tree markers:** `PieceWorld` does not forward block-tag queries
  or shape-edge updates from `PlacementContext`. Forward both operations through
  a supported shared boundary before enabling the marker; do not skip updates.
- **World slime fluids and custom slime mobs:** their respective host fluid and
  entity registration boundaries are unavailable. Keep those operations
  deferred without disabling independent terrain, vegetation or loot.
- **Position-dependent foliage tint:** the foliage face injection was isolated
  in [#241](https://github.com/sofia-gros/horizon-mods/pull/241) after the
  official binding failure. It is not established by the island build.
- **Entangled equipment removal:** source self-damage on actual removal remains
  unconnected across hotbar, inventory and guest paths; releasing an arrow is
  not an equipment removal. See [ranged material traits](ranged-material-traits.md).
- **Crystalbound other actions:** sling and knockback angle restrictions and
  armor/movement effects remain open beyond the representative bow checks in
  [ranged material traits](ranged-material-traits.md).

F02 material conditions and statistics, F05 equipment actions, F06 armor
behavior, F12 modifier modules, and F14/F15 generation and loot still contain
additional source entries requiring individual audit. Do not mark a whole area
complete from the selected rows above or from registration alone.

## Overlord durability and Overslime candidate

The Queen's Slime source trait includes Overlord and Overslime. The Overlord
candidate applies the source durability `multiply_base` reduction (15% per
level through six, then 99.999%) and copies 10% of durability per level into Overslime before its own
durability reduction, using the source float stat and global multiplier order. These
changes are not yet applied, type-checked or verified in game. Acceptance must
cover levels 0, 1, 6 and 7, material multipliers, Diamond, Emerald, Netherite,
Overcast, replacement/removal, and shield-capacity clamping without duplicated
bonuses. Compare against the original stat builder before claiming parity.

The Overlord follow-up keeps Ductile attack and toughness updates when Ductile
and Overlord durability changes cancel. The added combined-trait regression
still needs the dedicated test run.

The durability follow-up also applies the source base-stat order without
Overlord: Diamond adds to the base, Emerald and Netherite add 0.5 and 0.2 to
its percentage, and the global multiplier applies last. For an unmodified
100-durability baseline with Emerald and Netherite, the source reference is
170, rather than the previous multiplicative result of 180. Validation must
include both source float rounding and material replacement.

## Durability modifier order candidate

The candidate records new upgrade insertion order separately from the existing
`tic_upgrades` compound, retains it on part replacement, and removes an entry
when its upgrade is removed. It rebuilds material-trait order from the selected
parts. Durability calculation visits upgrades before material traits at the
same source priority and applies Overlord's lower-priority reduction afterward.
Creative generation uses the same recorded trait order. For older stacks without
an order list, existing upgrade keys are read in deterministic key order; their
original historical order cannot be recovered. The source also has tool traits
and modifier-provided traits, whose complete ordering coverage is not claimed.

Reference case: with Netherite then Emerald upgrades and Ductile 1 / Preserved 4
material traits, adding Ductile before Preserved produces the float32 percentage
`1.4000000953674316`; the reverse order produces `1.399999976158142`. The added
focused checks, Station/Creative/part-swap and extraction paths, legacy saves,
priority coverage, official apply/type and local/LAN behavior are unverified.

## Non-durability base multiplier candidate

The supported melee and mining stat rebuild uses the saved modifier order and
source add / multiply-base / multiply-all phases. Ductile and Solid add to the
same attack-damage percentage; Lightweight and Swiftstrike add to the same
attack-speed percentage; Lightweight, Heavy and Netherite add to the same
mining-speed percentage. Vintage remains a multiply-all operation. The source
reference for base attack 10 with Ductile 1 and Solid 1 is 12; base attack
speed 1 with Lightweight 1 and Swiftstrike 1 is 1.13; mining speed 4 with
Lightweight 1 and Heavy 1 is 4.92. These candidate paths and focused tests are
not yet applied, type-checked or verified in game. Other modifier modules and
unsupported stat operations are not claimed complete.

The stat integration candidate combines the non-durability stat branch with
splash candidate #266 and the world-scoped Bouncy follow-up #264. It retains the
workstation upgrade-order changes and the fluid/crafting declarations from both
branches. This merge has not passed official package, plan/apply, type, runtime
or conflict-regression checks; use the exact integration head for those checks.

The follow-up integration excludes the Bouncy landing injections: the formal
plan could not resolve `MotionWorld::player_fall_damage` through either tested
module path. The unrelated stat, crafting and fluid candidates remain present.
The generic Pluto landing state remains included; neither this isolation nor its
formal apply, type or gameplay validation is established by the earlier tests.

A follow-up corrects two direct NBT compound lookups to use the host's
`JavaString` key type. The old-host diagnostic fixture passed its focused tests
with this correction; the exact follow-up head still needs the dedicated tests
and official target type check.

## Ranged stat-order candidate

A source candidate rebuilds bow draw speed, velocity, accuracy and projectile
damage through the same ordered add, multiply-base and multiply-all phases.
It also connects the Solid trait's source `+0.75` projectile damage per level
for the supported launchers. The nonzero pre-existing draw/velocity percentage
in the focused check is a synthetic builder input; natural material coverage is
not established by it. The candidate has not been applied, type-checked or
checked in game. Compare source float ordering, material/part swaps, Quick
Charge, Netherite, Vintage, Solid and conditional arrow velocity on the exact
integrated target before claiming parity.

### Source-backed crossbow reference cases

The original `crossbow` definition uses a limb, grip and bowstring, with a
2x durability multiplier. The following are source-data calculation inputs,
not completed natural-survival playthroughs:

| Limb / grip / bowstring | Source traits | Quick Charge 1 expected draw / velocity / accuracy |
| --- | --- | --- |
| Cobalt / Cobalt / String | Lightweight 2, Stringy 1 | 1.3875 / 1.11 / 0.85 |
| Amethyst Bronze / Wood / String | Crystalbound 1, Cultivated 1, Stringy 1 | 1.0625 / 1.25 / 0.70 |

The values use the original generated material stats and traits at
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`. Cobalt ore generation and ore-to-part
steps have representative tests with supplied tools; the Amethyst Bronze
smeltery route has a representative test. Neither proves fixture-free acquisition
of these complete crossbows. The added product-helper cases require a dedicated
run and original builder comparison.

The Solid projectile-damage candidate in #86 and the #279 stat rebuild edit the
same calculation. #86 is not an ancestor of #279: on #279 the source module
adds Solid once when rebuilding `tic_projectile_damage`, and the projectile
path reads that stored result. Preserve that single addition when integrating
#86; do not apply its old formula a second time. Invar's Solid source material
is compatibility-conditional, so its ordinary acquisition is not asserted.

The combined stat/splash candidate's formal apply stopped at `E_MODULE` for
support sources with inner documentation headers. A follow-up replaces those
headers with ordinary comments; a manifest-source scan found no remaining
leading inner-doc headers. The follow-up's formal apply and subsequent type or
game checks remain pending.


The ranged/fluids integration candidate combines the source-backed crossbow
references, the fluid support-module header correction and the potion-cloud
candidate. All support-module source files referenced by its checked-in
TConstruct and Pluto manifests were statically scanned for inner documentation
attributes and shebangs. No such source lines remained at that scan. The merged
candidate has not passed formal package, apply, type or runtime checks.

### Longbow Power candidate

The original Power upgrade accepts Ichor slime crystals on the longbow: 72
items per level, at most five levels, with one upgrade slot per level. Its
projectile-damage addition is `0.5 + 0.5 * effective_level` while present,
including partial levels. This candidate connects that recipe to the existing
upgrade and ordered ranged-stat paths. The two supported bows have different
source tags: the crossbow does not accept Power. These changes are source-level
only and still need official apply, type and game verification. A full natural
crystal-to-upgrade survival route has not been demonstrated.

For a base projectile stat of 2, check zero, one, 72 and 360 crystal points:
expected values are 2, approximately 2.506944, 3 and 5 respectively. Also check
slot limits, extraction and reapplication, save/reload, and rejection on a
crossbow. Values before downstream arrow effects are the comparison point.

### Ranged upgrade integration candidate

The newer ranged-stat candidate also carries the earlier Power (#109) and
Blindshot (#133/#134) transactions and progress labels. Blindshot uses one Dirt
per point, ten points per level, and does not occupy upgrade slots or have a
source salvage recipe. Its accuracy decrease is 0.1 per effective level; the
current connected launcher scope is longbow and crossbow. The original default
maximum applies when its recipe omits a level range. This integration is a
source-level candidate, not an apply, type or gameplay result. Check partial
and full levels, zero free slots, unsupported tools, accuracy minimum, crystal
and salvage behavior, saved data and progress labels in the integrated build.

The source-backed crossbow test binds both input part arrays before borrowing
them. An earlier exact-head test compile failed with E0716; a disposable
old-host diagnostic passed after the same lifetime correction. This new
candidate has not yet passed the dedicated exact-head test or official build.

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

Source stat builder `MULTIPLY_BASE` factors compound in modifier order for durability, mining and the supported melee/ranged stats. The focused older-host helper fixtures cover these phases; the current-host build and game behavior remain unverified for this candidate.

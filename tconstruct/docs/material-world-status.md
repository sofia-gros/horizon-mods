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
| Bouncy | [#243](https://github.com/sofia-gros/horizon-mods/pull/243), [#245](https://github.com/sofia-gros/horizon-mods/pull/245), and the aligned [#247](https://github.com/sofia-gros/horizon-mods/pull/247) are unverified local/guest candidates. The earlier #243 package-only check covers archive structure, not host compatibility. | Run official combined package, apply and type checks; compare crouched and ordinary fall damage, rebound and sent guest velocity, repeated landings, broken armor and player isolation. |
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

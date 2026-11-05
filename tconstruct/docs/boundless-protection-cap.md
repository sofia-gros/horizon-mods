# Boundless protection cap (#379)

This local slice connects the canonical Boundless attribute on supported held
shields and worn armor. It raises the cap on existing protection; it grants no
protection points and does not change shield blocking amount or angle.

## Source behavior

The attribute starts at 0.8. Each equipped Boundless modifier contributes
`float(0.05 * level + 0.05)`, widened to the double attribute. The attribute cap
is `double(0.95f)`, then multiplied by 25 and capped/cast as the source protection
hook does. One level-I piece permits 22.5 EPF (90% reduction). Level II, or two
level-I pieces, reaches 23.75 EPF (95%). A zero/absent level adds no flat bonus.

The actual selected main hand, offhand and four armor slots are checked. Stock
Travelers Shield, Plate Shield and Battlesign are candidates in hands, but both
live `modifiable/held` and `modifiable/armor` tags must admit them. Worn items
must match their supported stock armor slot and the live armor tag. Current
material traits and separately saved levels combine once. Invalid snapshots or
unsupported modifier definitions reset this slice to the native cap.

The canonical percent-tooltip AttributeModule uses equipment-change hooks, not
the broken-tool-filtered protection iterator. Those hooks and its tag condition
do not skip broken equipment, so this cap remains while an eligible broken item
is equipped. Unequipping, selection changes and source reload remove it.

Status/equipment refresh and the per-player server tick update the owned cap.
Native per-hit health environments clone that actor's StatusFlags, so the cap
is applied at the real protection stage after armor/Resistance. Native bypass
rules and the separate negative Depth Protection path remain unchanged. Remote
clients retain neutral status; authoritative damage remains server-owned.

## Dependencies and limits

- Mod base: `feat/turtle-shell-acquisition-379` (`12a53a00`).
- Native dependency: `feat/player-protection-upper-bound-379` (`5e47547e2`), based
  only on signed protection `09f0179db`. No fluid-propulsion draft is included.
- Boundless acquisition, arbitrary equipment, arbitrary external protection-cap
  attributes, legacy capability bonuses and smaller custom caps are not added.
- Sources: TConstruct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`, including
  `boundless.json`, `AttributeModule`, `EquipmentChangeWatcher`, `ModifierUtil`,
  `TinkerAttributes`, `ProtectionModifierHook`, `ToolEvents` and `ArmorUtil`.
  The license is MIT (Copyright 2022 SlimeKnights); no upstream implementation
  was copied. The adapter and native hook are original implementations.

Brief static review and whitespace checks only. Pure arithmetic/definition
regression cases were added but not run. No tests, builds, planner transforms,
CI checks, version checks, merges, live combat, persistence or LAN verification.
Both branches remain unpublished while publication is paused.

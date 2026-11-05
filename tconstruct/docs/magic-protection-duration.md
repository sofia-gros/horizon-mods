# Magic Protection harmful-effect duration

Related to #379. Stacked on held Crystalstrike (#788), preserving its Balm
and worn/held Crystalstrike operation ordering. This implements the secondary
Magic Protection attribute, not magic damage protection or acquisition.

## Coverage

- Resolve the highest effective Magic Protection level among 13 correct-slot
  stock worn armor classes and Travelers Shield, Plate Shield and Battlesign in
  either selected mainhand or offhand. Shields need not be raised. Worn pieces
  require the current worn tag; shields require `tconstruct:modifiable/held`.
- Combine live fixed/material traits with saved `tic_upgrades` five-unit progress.
  Partial levels preserve the source started-level plus negative-fraction float
  arithmetic. Duplicate equipment levels do not add together.
- Compute `-0.05f * maximum level` as MULTIPLY_BASE alongside Balm, then apply
  every worn and held Crystalstrike MULTIPLY_TOTAL, clamp the completed attribute
  to 0–100 and convert the incoming duration once. A level-one Magic Protection
  piece nominally reduces duration by 5%; source float rounding is retained.
- Read equipment and the complete current source definition at effect application.
  Broken pieces, invalid count/damage/level records, opaque components, unsupported
  item classes and changed definitions do not contribute. No saved derived state
  or transient host attribute is needed for the two existing effect entry points.
- Reuse `give_player_effect` and `add_player_effect` for known milk-curable
  nonbeneficial effects. Infinite duration stays infinite; zero multipliers still
  leave one tick. Existing active/hidden effects are not retroactively shortened.

## Boundaries

Magic damage reduction remains unimplemented. Acquisition is in the separate
protection-acquisition work and must be integrated for the normal station route.
Other held tools, custom classes/effects/curatives, external attribute commands,
direct effect-list writes, other max-attribute modules sharing the source key,
source UUID iteration ordering, remote prediction and LAN parity are not covered.
Remote clients retain the native path. Actual equipment changes, saves/reloads,
pack reloads and multiplayer behavior have not been exercised.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`magic_protection.json`, `MaxArmorAttributeModule`, `MaxArmorLevelModule`,
`IncrementalModifierEntry` and `ModifierEvents.onPotionStart`.

Brief static review only. Four regression cases were added, not run. No tests,
builds, planner runs, CI inspection, version checks or merges were performed.

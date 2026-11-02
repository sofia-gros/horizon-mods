# Material, equipment and world validation handoff

This handoff describes the fixed #325 candidate
`6c744c9001945258f106aefaeea08a3ca7624196` on Horizon
`0c7b5b61a5e746d621afffba919b12aeccf9c0fa` and Mixins
`ddd9700b21997c95d95ac6ee5ecafe3830cf622a`. It is not a completeness claim.
Original rules below refer to TinkersConstruct commit
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2`.

Package generation and three focused old-host helper tests were reported on
the exact candidate. The registry in those tests was reconstructed. An offline
Mixins CLI build could not start because `cargo_metadata` was absent; official
plan, apply, type, build, game and LAN are not established by that attempt.
Use fixtures where necessary and do not label them as natural acquisition.

| Plan | Submitted source and bounded verification input | Still incomplete or deferred |
| --- | --- | --- |
| F02 | `source-station-stats.rs`, `source-material-traits.rs`, `source-upgrade-rules.rs` and `resource-tables.rs`: compare equivalent Station and Creative armor, including Forming 2 on base armor 4 → 6 and base resistance 0.1 → 0.2, without gems. Check Heavy/Wool/Ductile, source multipliers, material replacement, skull stats and slimy chestplate alias. | All 104 material conditions and combinations are not validated. Original `MaterialTraitsDataProvider.java` assigns Necrotic to necrotic bone and Tasty to pig iron; the material selector resolves trait names, but a name registration or wear-cause exemption is not an implementation of life steal or food use. See the concrete open entries below. |
| F05 | `source-projectile-stats.rs` and `source-mining-*`: for Raging level 2, projectile bonus is 0 at health 20, 1 at health 6 and 2 at health 1 before the stat multiplier. Check the same firing actor and arrow for local and distinct guest health. Compare Overwield level 2 with 1/5/10 Overslime on small and broad tools, and record pre-break, cancellation, permission and area spend. | Spiny's source damage type is not registered; velocity is only a partial effect. Overwield pre-break uses the host native destroy boundary after host permission checks; source `ToolHarvestLogic.runBlockBreak` invokes `startHarvest` before `breakBlock`, so denied/creative cases are unresolved. Other source combat and item actions remain incomplete. |
| F06 | `material-armor-attributes.rs`, `source-snow-boots.rs`, and `golden-armor.rs`: check equip/remove/broken and multiple pieces; vanilla leather boots and gold armor provide regressions. Snow Boots walking and equipment sync are candidate hooks. | Powder-snow placement hook is omitted after official `E_UNSUPPORTED_SIGNATURE` for generic `Game::with_level`. Bouncy landing hooks remain deferred after plan failures; neither is a game pass. Featherweight protection and item-use speed and Entangled removal damage are separately incomplete. |
| F12 | `source-upgrade-rules.rs` and Station transactions: test Power on Longbow with 0/1/72/360 points → projectile stat 2/~2.506944/3/5 from base 2; reject Crossbow. Test Blindshot with 10 Dirt → -0.1 accuracy without an upgrade slot, and no salvage. Check Gilded slot exchange/removal, Step Up and Speedy alongside Spl​ashing, including rejection without consumption. | The bounded fluid-effect and bottle operations in `workstation-fluid-integration-handoff.md` are partial; source predicates/effects outside their supported subset remain unimplemented. Other modifier modules still need individual audit; six fluid abilities remain under the workstation/fluid owner. |
| F14 | `source-slime-island-*`, `source-slime-leaves.rs` and vegetation candidates: use disposable newly generated worlds; inspect Earth/Sky and other dimension placements, leaf distance/decay, planted growth, source shapes, save/reload and LAN. Distinguish natural generation from commands, teleports and supplied tools. | Slime fluids and custom slime mobs need host registration boundaries. Island Ender tree markers still need PieceWorld block-tag lookup and shape-edge update forwarding. Position-dependent terrain tint is isolated after a binding failure. |
| F15 | Cobalt source loot sequence, crystal harvest item tag, original ingot/nugget registrations and Pig Iron/Knightmetal block candidates: compare source drop conditions, Silk Touch/Fortune, mining tool tags, registry and saved blocks. | Registration is not natural availability. Source Pewter requires external Tin or Lead and must not be given an invented core recipe. Full natural loot and acquisition have not been demonstrated. |

## Concrete source gaps for continued implementation

- **F05/F12 Necrotic:** `tools/data/material/MaterialTraitsDataProvider.java`
  assigns `necrotic` to necrotic bone; `tools/data/ModifierProvider.java` sets
  life steal to 5% of damage per level. `tools/modules/combat/LifestealModule.java`
  requires a fully charged critical melee hit and excludes the source entity
  blacklist; projectile healing has separate target-health and power rules.
  The current melee hit context is in `source-spilling.rs`, but it is scoped to
  Spilling and does not implement Necrotic. Check source blacklist resolution,
  actual landed damage, player health mutation, cooldown/critical state,
  attacker identity, projectile behavior and source sound before implementing;
  these have not been established as unavailable APIs.
- **F05/F12 Tasty:** the same material provider assigns `tasty` to Pig Iron;
  `ModifierProvider.java` registers its edible trait, hunger, saturation and
  durability behavior. No `tconstruct:tasty` effect dispatcher was found in
  the candidate's TConstruct Rust sources. Check use duration, hunger/saturation,
  durability and held-item/armor source conditions as one coherent operation.
- **F05/F06 Entangled:** source `ModifierProvider.java` registers removal damage
  separately from the mining and draw stat additions already present in
  `source-mining-stack.rs` and `source-upgrade-rules.rs`. Verify real equipment
  removal across hotbar, armor and guest paths; a shot release is not removal.

These are named examples, not an exhaustive count of the remaining trait and
modifier modules. Keep unreviewed source entries open rather than deriving a
completion percentage from registry or PR counts.

# Parrying acquisition

This #379 slice enables the canonical Parrying recipe through the existing
Station/Anvil upgrade transaction. It stacks on #865's Blockade acquisition.
The pinned source's `modifiable/melee/parry` tag contains only Dagger; this
bounded implementation also requires that exact tool class and the live tag.

- Consume one steel ingot and two planks in three occupied ingredient slots.
  Both ingredient tags use the current resolved item tags; steel is not
  substituted with iron or an invented supply route.
- Add one saved Parrying level, consuming one ability slot, not an upgrade slot.
  Existing purchased or material Parrying prevents another application.
- Permit a matching modifier crystal instead of the ordinary ingredients.
- Extract the saved level with a Sky Slime Crystal and Wet Sponge, returning
  its modifier crystal and Dry Sponge through the existing transaction.
- Refuse extraction when Parrying is the last Blocking/Parrying prerequisite
  for any remaining saved or material Blockade, including partial pane progress.
- Preserve existing extraction selector IDs by appending the new entry. The
  item tooltip and Station panel use the existing translated modifier name.

The complete canonical recipe is required. Changed costs, inputs, target
predicates and level rules fail closed. Zero-count inputs, stacked tools,
malformed saved values and unavailable trait snapshots are rejected. Slot
revalidation after part swaps now counts the saved ability. A static source
`ParryingModifier` registration is not mistaken for a missing dynamic JSON.

## Separate runtime

Acquisition is not combat completion. Source Parrying inherits Offhand Attack,
starts a 20-tick block action after the accepted offhand use and has its own
finish cooldown. The offhand attack, block amount/angle, use lifecycle, movement
and cooldown integration are not added by this acquisition slice. Main-hand
shield-style blocking must not be inferred from the source comment alone.

Existing native item/NBT and Station transaction paths retain the saved level.
Gameplay, steel availability, save/reload, client preview and LAN have not been
run. Three regression tests were added but not executed. Brief static review
covered recipe equality, trait cap, ability accounting, crystal/extraction,
Blockade dependencies and append-only selector IDs. No tests, build, planner,
CI inspection, version checks or merge were performed.

Source: Tinkers' Construct f4e77d43, `recipes/tools/modifiers/ability/parrying.json`,
its salvage recipe, `tags/items/modifiable/melee/parry.json`,
`TinkerModifiers.parrying`, `ParryingModifier` and `OffhandAttackModifier`.

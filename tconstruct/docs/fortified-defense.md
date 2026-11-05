# Fortified defense slots

This #379 slice builds on the local Knockback Resistance acquisition change.
It implements Ancient Hide's Fortified material effect: one additional defense
slot per resolved level, added to the current tool definition's base defense
slots. Upgrade and ability capacity are unchanged.

The existing material resolver determines the level from actual material
identities and stat-type selection. Ancient Hide's default is Fortified, while
its melee/harvest fallback is Fortune; the latter does not grant defense slots.
The highest current modifier definition must match the original Fortified
module, including its defense slot target, per-level amount and priority.
Unsupported modifier overrides, command-added Fortified levels, malformed
values and arithmetic overflow are refused rather than granting free slots.

The new capacity is used by the bounded Travelers/Plate Knockback Resistance
recipe and by material replacement validation. Replacing or removing Ancient
Hide recomputes capacity before an output can be taken. Removal is rejected
when the remaining slots cannot support the installed defense upgrade. Casting
material replacement can now change Fortified when the stock modifier contract
is recognized and the same slot validation succeeds.

Workstation information displays free/total defense slots. Remote menus receive
the server's complete result in an appended field of the existing information
frame. Existing trait/error field positions are preserved. Information changes
are compared independently of the item preview, so a changed definition can
refresh the display even when the item is unchanged. No capacity is persisted
into item NBT or accumulated between rebuilds.

## Bounds

- The currently implemented purchased defense consumer remains one level of
  Knockback Resistance on the eight worn Travelers/Plate armor items. This does
  not implement the other Protection recipes or their damage effects.
- Unknown or mixed purchased defense modifiers are refused. Arbitrary module
  overrides and custom hook assignments are not evaluated as stock capacity.
- This does not add Ancient Hide acquisition, material loot, or a general
  migration of previously saved stat values. Existing material identities are
  resolved through the current world's snapshots.
- No claim is made that the grouped Fortified/Revitalizing/Turtle Shell issue
  row is complete. Turtle Shell effects are separate work.

## Review and deferred validation

Brief static review covered per-stat material selection, checked arithmetic,
shared host/server module ownership, defense spending, material replacement,
and appended menu framing. Focused regression code was added for capacity,
trait selection, recipe spending/removal, modifier overrides and framing.
No tests, builds, Mixins transformation, CI inspection, version checks or merges
were performed. Real save/reopen and local/LAN gameplay remain unverified.
Publication is paused.

Sources, pinned Tinkers' Construct f4e77d43:
- `tinkering/modifiers/fortified.json`: modifier_slot, defense, each_level 1,
  priority 60.
- `tinkering/materials/traits/ancient_hide.json`: default Fortified I,
  melee_harvest Fortune I.
- `ModifierSlotModule.addVolatileData`: adds the computed level-dependent slot
  amount, rather than writing a persistent capacity increment.

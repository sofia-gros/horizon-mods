# Knockback Resistance defense upgrade

This bounded #379 slice admits the original Knockback Resistance recipe on
worn Travelers and Plate armor. One Anvil, Chipped Anvil or Damaged Anvil item,
or one matching modifier crystal, grants the single upgrade level at a Tinker
Station or Anvil. The recipe consumes one defense slot, not an upgrade or
ability slot. The world-resolved armor tag and original recipe still decide
ingredient eligibility.

The source modifier definition must match the supported stat-boost contract.
The source's +0.1 resistance per level is rebuilt with direct material trait
levels, the existing stat multiplier and the normal 0–1 clamp. The existing
equipment attribute bridge then projects that saved stat while armor is worn in
its correct slot and unbroken. No second live attribute bonus is added.

The new upgrade participates in item/workstation text and extraction. Removing
it returns the normal modifier crystal and rebuilds the original material
resistance. Current definition defense capacity plus supported Fortified material
traits is checked on acquisition and
when rebuilding a material swap that retains this purchased upgrade. Saved
Knockback Resistance levels other than zero or one cannot be upgraded or
extracted as though they were valid normal progress.

## Deliberate bounds

- Only the eight Travelers and Plate worn armor items are admitted. Shields,
  staffs and Slime armor are not enabled by this slice.
- Fortified adds one defense slot per resolved material-trait level. General
  defense-modifier support remains separate work; see `fortified-defense.md`.
- Unknown saved modifiers and other purchased defense modifiers refuse this
  acquisition route rather than being counted as zero occupied slots. Material
  traits are separate and do not consume defense slots.
- Source modifier overrides outside the recognized definition refuse new
  acquisition. Existing saved-stat migration after arbitrary datapack changes
  is not added.
- Existing equipment refresh, durability and guest synchronization paths are
  reused. Their complete gameplay behavior is not established here.

## Review

A brief static review covered recipe ingredients, one-level limits, defense
accounting, extraction, shared module ownership and resistance rebuilding. A
focused planner/stat regression was added but not run. No tests, Mixins
transformation, build, CI, version checks, merge, save/reopen or live gameplay/LAN
validation were performed, as requested. Publication is paused.

Source: Tinkers' Construct f4e77d43, retained
`tools/modifiers/defense/knockback_resistance.json` and
`tinkering/modifiers/knockback_resistance.json`.

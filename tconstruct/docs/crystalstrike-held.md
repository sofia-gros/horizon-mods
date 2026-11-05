# Held Crystalstrike on stock shields

Related to #379. Stacked on #761 and its #724 prerequisite, reusing their
worn Crystalstrike/Balm attribute and effect-duration pipeline.

## Added coverage

- Selected mainhand and offhand Travelers Shield, Plate Shield and Battlesign
  contribute Crystalstrike independently of worn armor. The selected inventory
  slot is read directly, not an interaction's temporary acting hand. Merely
  holding an eligible shield is enough; it need not be raised or blocking.
- Each hand adds a separate source `0.05f * level` MULTIPLY_TOTAL amount to
  attack speed and supported harmful-effect duration. Two level-one shields
  produce two multipliers, not one level-two multiplier. Existing worn Balm
  base reductions still run before every Crystalstrike total multiplier.
- Intact held shields in the current `modifiable/armor/held` tag contribute
  their levels to the same received-knockback angle restriction as worn armor.
  Tag removal disables only that held knockback contribution. The two percent
  AttributeModules have explicit hand slots and no such tag requirement.
- Broken shields keep both percent attribute effects, following the source
  EquipmentChange AttributeModules. They cease contributing to knockback,
  following ArmorLevelModule's `allow_broken: false`.
- The combat, harmful-effect and knockback consumers resolve live source
  traits/upgrades at use time. Owned mainhand/offhand attribute IDs are removed
  when the hand no longer qualifies. Refresh also runs after the native status
  update, catching same-item material edits, selection/offhand changes and
  retained broken tools even when the host's selected item ID does not change.
- Existing exact source-definition, malformed-level, stack-count and opaque
  component guards remain. No new persistent state or actor cache is added.

## Boundaries and static review

This does not implement shield block amount, angle, use/charge, or durability
mechanics. Staffs, other held tools, held armor items, custom item classes and
held Balm remain outside this change. Balm's new two hand entries stay zero.
Only the previously supported known nonbeneficial milk-curable effects are
scaled; external bad-effect attributes/custom curatives remain unsupported.

Native status/equipment/reload refresh is reused, but actual hand switching,
same-item edits, broken-state changes, mixed armor/hand multipliers, save/reload,
remote prediction and LAN behavior have not been exercised. Full source UUID
attribute iteration and cross-mod ordering are not claimed.

Source reference: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`crystalstrike.json`, `AttributeModule`, `ArmorLevelModule`, and
`EquipmentChangeWatcher`.

Two pure held-scope/multiplier tests were added but not run. Brief static review
and formatting only. No tests, builds, planner runs, CI inspection, version
checks or merges were performed.

# Turtle Shell Station acquisition

Related to high-priority #379. Stacked on Turtle Shell underwater protection.

## Added coverage

- Accept the exact stock incremental modifier recipe: five scutes per level,
  one defense slot per started level, and the live `modifiable/armor` tool tag.
  Do not broaden it to the other Protection recipes' armor-or-held predicate.
  The armor tag itself can include held armor; held membership alone is not
  sufficient. The existing supported worn/shield item-ID boundary is retained.
- Fill a partial level with only the missing scutes and no additional slot.
  Starting another level or applying a crystal requires a free defense slot.
  Crystal application adds five saved units without erasing the partial remainder.
- Include Turtle Shell in the shared defense budget with the other five
  Protections and Knockback Resistance, including material-swap validation.
  It does not consume upgrade/ability slots or charge for material trait levels.
- Extract a complete five-unit level through wet sponge plus sky slime crystal,
  keeping partial remainder and returning the modifier crystal/dry sponge through
  the existing Station transaction. Require the armor tag for extraction too.
- Reuse the exact Turtle Shell definition guard from its underwater runtime.
  Append the UI selection ID without changing prior IDs, show five-unit progress,
  and identify swim speed as still unavailable.
- Extend the existing scute compatibility preparation only for the canonical
  `tools/modifiers/defense/turtle_shell` recipe: `minecraft:scute` becomes
  `minecraft:turtle_scute` when only the native renamed item exists. Preserve
  customized recipes and legacy-host registries. Both manual and bundled
  preparation paths use it. Invalidate the bundled content cache so previously
  prepared recipes receive the migration. No saved item or global alias rewrite.

## Boundaries

Swim speed and Turtle's Grace remain separate. The existing defense planner's
finite saved range (up to 5*32767 units), supported stock item IDs, opaque saved
modifier rejection and full Station/reload/save/LAN validation limits remain.
This branch inherits the shared underwater runtime and Depth Protection native
signed-protection dependency `09f0179db`.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`recipes/tools/modifiers/defense/turtle_shell.json`, its salvage recipe and
`tinkering/modifiers/turtle_shell.json`.

Extended the shared Protection recipe/crystal/extraction/budget regression,
added native-scute/partial-level/armor-tag and rename-guard cases, and updated
cache migration expectations. None were run. Brief static review and
`git diff --check` only. No tests, builds, planner runs, CI inspection, version
checks or merge. Publication remains paused.

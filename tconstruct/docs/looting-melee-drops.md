# Looting on direct player melee kills

Issue #379. This is a bounded bridge into native death loot, separate from the
Fortune block-harvest work. It does not change attack damage or drop counts after
they have been rolled.

## Source behavior

The pinned source [LootingModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/combat/LootingModule.java)
adds weapon and armor module levels to the current Looting value.
[ModifierLootingHandler](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/helper/ModifierLootingHandler.java)
selects the actual responsible slot for non-projectile damage and then includes
armor. Projectiles use their captured modifier capabilities instead of the held
weapon's modifiers.

The source weapon hook skips broken tools. Armor hooks iterate non-broken valid
equipment. The supported source definitions contribute:

- `looting`: its full modifier level from a melee tool, and its full level from
  each valid worn piece.
- `looter`: one per worn piece with any positive modifier level.
- `fortunate`: one per worn piece only at modifier level two or higher.

Material traits and stored upgrades are combined once by the existing live
world resolver. Active definitions must match the pinned source JSON exactly.
Looter experience bonuses and Fortunate Luck attributes are not included.

## Host connection

A non-cancelling HEAD hook on `Game::drop_death_loot` modifies only the native
`LootEntity` killer snapshot. It requires a direct `PlayerAttack` by `Who::Player`,
player kill credit, a loaded matching player UUID, and a nonempty mainhand item.
The current stack must still match the mainhand snapshot already made by the
native code. Guests are resolved using the host's existing actor-switch helper;
an unresolved or departed guest never falls back to the host's modifiers.

The summed bonus is represented as a temporary mainhand Looting enchantment in
that snapshot. Native loot-table conditions, count increases and equipment-drop
chance then consume it through their ordinary path. No item in inventory is
enchanted, copied into a drop, or saved with the synthetic level. Native random
sequences, mob random, callbacks, drop ordering, and experience handling remain
in the original death method.

Mainhand source bonuses require live modifiable and melee tags. Worn bonuses
are limited to the correct slots of the stock Travelers, Plate and Slime armor
families, with the live modifiable/worn tags. Malformed or opaque participating
stacks, unknown snapshots, duplicate native enchantment keys, arithmetic
overflow, and totals above 255 decline projection. Broken pieces add no bonus.
Native enchantments remain in place even when the source projection declines.

## Deferred and validation

- Projectile capability Looting, offhand attacks, barehand armor Looting,
  mob-held tools, other damage kinds, custom tools/definitions, and cross-actor
  deaths during another guest's active turn are outside this bounded path.
- No new Station acquisition, extraction, Fortune harvest, Luck/fishing,
  experience multiplier, or Lacerating amplifier connection is added.
- The native Looting enchantment definition controls how the projected level
  affects native tables and equipment drops. Full source-versus-native table
  probabilities and arbitrary modified enchantment definitions are not audited.
- Brief static review covered source gates and level rules, actor identity,
  private-module visibility, native snapshot ownership, and untouched RNG/drop
  control flow. Three pure regression tests were written but not run.
- No tests, builds, planner runs, CI inspection, version checks or merges were
  performed. Actual loot distributions, worn combinations, save/reload, native
  hook composition and LAN still require the final integrated validation.

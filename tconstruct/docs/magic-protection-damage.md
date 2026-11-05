# Magic Protection damage reduction

Related to #379. Stacked on the shared Fire/Blast post-armor protection stage.

## Added coverage

- Intact supported worn armor and either-hand stock shields contribute 3 EPF
  per effective Magic Protection level, including fractional five-unit progress.
- Read the live `tconstruct:protection/magic` tag. The stock source tag expands
  `minecraft:witch_resistant_to`, Wither, Wither Skull and Dragon Breath, with
  optional Twilight Forest damage IDs. Only types actually in the host catalog
  can match. Missing required nested tags disable this modifier independently.
- Magic Protection has no virtual vanilla protection enchantment. Broken worn
  armor and broken held shields therefore contribute zero, unlike Fire/Blast's
  broken-worn integer enchantment remainder.
- Fire, Blast and Magic points add together on overlapping damage tags in the
  same transient table. Native enchantment points remain separate until the one
  post-armor/Resistance protection stage applies its default cap.
- Keep the existing equipment/status/tick/reload refresh, invalidation and
  malformed-record guards. No additional injection, item mutation, saved bonus
  or early damage multiplier is introduced.

## Boundaries

The harmful-effect-duration secondary effect is implemented on a separate
Magic Protection duration branch and is not part of this stacked damage branch.
It must be integrated separately. Melee/Projectile damage protection,
Boundless/extended caps, negative protection, other custom tools and mobs remain
outside this change. Native bypass rules, wear/stat rounding, mixed-modifier
ordering and multiplayer limits remain as documented in the Fire damage bridge.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`magic_protection.json`, `tags/damage_type/protection/magic.json`,
`ProtectionModule` and `EquipmentContext`.

Three regression cases were added for fractional/broken Magic Protection,
combined points and source magic-tag projection. They were not run. Brief
static review only; no tests, builds, planner runs, CI inspection, version
checks or merges. Runtime damage, reloads, hook matching and LAN are unverified.

# Projectile Protection damage reduction

Related to #379. Stacked on the shared Fire/Blast/Magic post-armor stage and
the earlier Projectile Protection knockback-resistance implementation.

## Added coverage

- Intact supported worn armor and either-hand stock shields contribute **2 EPF**
  per effective Projectile Protection level, retaining fractional five-unit
  progress. One level gives 8% reduction at the protection stage below the cap.
- Match the live `tconstruct:protection/projectile` tag. Its stock source content
  includes `minecraft:is_projectile` plus Falling Anvil, Falling Block and
  Falling Stalactite, with optional Twilight Forest Falling Ice. It is not an
  arrows-only check. Only damage types present in the host catalog can match.
- Broken armor/shields contribute zero because this modifier has no virtual
  vanilla protection enchantment. The Fire/Blast broken-worn exception is not
  incorrectly applied to Projectile Protection.
- Damage points sum across eligible pieces. This differs from its independent
  knockback-resistance secondary effect, which uses the highest equipped level.
- Combine Projectile with Fire, Blast, Magic and native enchantment points at
  one post-armor/Resistance stage, with one default 20-point cap. An overlapping
  source damage tag does not cause another percentage multiplier or early cap.
- Reuse independent definition/tag guards and equipment/status/tick/reload
  refresh. No new injection, persistent bonus or enchantment mutation is added.

## Boundaries

Broader Melee Protection direct-source coverage, Boundless/extended caps, Overshield, negative protection,
other custom held tools, mobs, exact mixed-modifier ordering and full multiplayer
parity remain outside this patch. Native bypass, wear/stat rounding and source
unregistered-ID projection limits are unchanged. The separate Magic duration
and Melee use-speed branches still require their own integration.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`projectile_protection.json`, `tags/damage_type/protection/projectile.json`,
`ProtectionModule` and `EquipmentContext`.

Three regression cases cover the two-point fractional coefficient, broken
pieces, shared cap input and falling-block tag membership. They were not run.
Brief static review only; no tests, builds, planner runs, CI inspection, version
checks or merges. Actual projectile/falling-block damage, reloads and LAN remain
unverified. Publication remains paused.

The subsequent [Melee Protection patch](melee-protection-damage.md) adds bounded
verified native direct-hit coverage at this shared protection stage.

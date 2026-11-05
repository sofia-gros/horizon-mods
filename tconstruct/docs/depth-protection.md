# Depth Protection

Related to high-priority #379. This mod patch requires the separate native
`feat/signed-protection-bound-379` dependency. Integrate both together: the new
`StatusFlags::minimum_protection` injection target does not exist without it.

## Added coverage

- Resolve the exact stock Depth Protection definition, live material traits and
  saved integer modifier levels on the supported stock worn armor and either-
  hand shields. Broken pieces contribute zero. No enchantment or save mutation.
- Reproduce source float height arithmetic, casting the wearer's actual Y to
  f32 before calculations. Below Y=64 the multiplier increases linearly from
  zero, reaching 1 at Y=0 and its cap of 2 at Y=-64. Y=64 through 96 is neutral.
  Above Y=96 it becomes negative, reaching -1 at Y=160 and staying there.
- Each piece contributes `height_multiplier * (1.25 * level)` EPF. One level at
  Y=0 gives 5% protection, at/below Y=-64 gives 10%, and at/above Y=160 gives a
  5% damage penalty when no other protection offsets it.
- Sum with the existing Fire/Blast/Magic/Projectile and scoped Melee protection,
  plus native enchantments, at the shared post-armor/Resistance stage. Negative
  points first offset positive protection. Only a negative final total causes
  extra damage, bounded at -20 EPF (80% additional damage). Positive cap stays 20.
- Refresh equipment-derived levels on equipment/status/tick/reload and recompute
  height immediately at `PlayerMotion::hurt_caused_by`, before the environment
  clones status. Falling or moving between ticks cannot use the old Y.
- Opt the actor into the native negative lower bound only while Depth
  Protection's own height contribution is negative. Unmodified native actors
  retain their zero lower bound. Native protection/effects bypasses still apply.

## Boundaries

Supports the same 13 stock worn item IDs and three held shield IDs as the shared
defense bridge. Other custom held tools, mob equipment, dynamic modifier
formula replacements, Boundless/extended positive caps and Overshield consumption
remain separate. The mixed-modifier addition order, original Forge inverse-armor
rounding, wear/resistance statistics and third-party hook ordering retain the
shared bridge's limitations. No custom tooltip formula or acquisition recipe
is added. Remote clients do not calculate authoritative bonuses; final guest
refresh, save/reload and multiplayer behavior remain unverified.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`depth_protection.json`, `DepthProtectionModule` and `ArmorUtil` (MIT license).

Three pure mod regression cases and two native arithmetic cases were added,
not run. Brief static review and `git diff --check` only. No tests, builds,
planner runs, CI inspection, version checks or merge. Both publications remain
paused.

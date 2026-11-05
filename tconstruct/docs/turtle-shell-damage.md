# Turtle Shell underwater protection

Related to high-priority #379. This patch connects Turtle Shell's two protection
modules, independently of its swim-speed attribute and Station acquisition.

## Added coverage

- Resolve live material traits, saved five-unit incremental levels and the exact
  stock Turtle Shell definition. An intact qualifying piece contributes 2.5 EPF
  per effective level (10% reduction per level below the shared cap). Broken
  pieces contribute zero. No new enchantment or derived tool-save fields.
- The upper-body tool condition uses the live helmets/chestplate tags and
  Mantle's `eyes_in_water`: the actor's `wasEyeInWater` sample. It does not use
  a fresh eye-fluid query or require the feet to still be submerged.
- The lower-body tool condition uses the live leggings/boots tags and Mantle's
  `feet_in_water`: `Entity.isInWater`, mapped to native `was_touching_water`.
  It does not require the eyes to be submerged.
- Conditions are independent. Tag overrides can make both modules match a
  supported item, in which case both contributions apply when their water
  predicates match. A held shield ordinarily has neither armor-subtype tag and
  therefore adds no protection; merely having Turtle Shell is insufficient.
- Read those native water samples again at the actual hit before health clones
  the actor's status. Refresh equipment-derived levels on equipment changes,
  status replacement, native player tick and resource reload.
- Sum with existing source/native protection at the shared post-armor/Resistance
  stage and one cap. Native bypass tags are preserved. Depth Protection's signed
  penalty can offset this positive contribution rather than becoming a separate
  damage multiplier.

## Boundaries

The swim-speed attribute, normal scute-based acquisition/crystal/extraction path
and Turtle's Grace's separate shell trait remain outside this patch. Existing
Turtle material trait resolution supplies this runtime; saved modifier entries
must use the source incremental units. Supports the shared 13 worn item IDs and
three stock held shield IDs; other custom tools and mobs remain unsupported.
Remote clients do not calculate authoritative bonuses. Guest equipment refresh,
water transition timing and save/reload/LAN remain unverified. Native fluid
sampling is reused, not a reimplementation of every source fluid interaction.

The shared bridge's mixed-modifier ordering and Forge inverse-armor/stat/wear
rounding limits remain. This branch inherits native signed-protection dependency
`09f0179db` from Depth Protection and must be integrated with it.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`turtle_shell.json` and Turtle material traits. Water predicate semantics were
checked against Mantle `3cf1c1628717163fef50ef44ec01847c723b1618`,
`LivingEntityPredicate.EYES_IN_WATER/FEET_IN_WATER`.

Three regression cases were added for independent water conditions, fractional
levels/overlapping tags and invalid data. They were not run. Brief static review
and `git diff --check` only. No tests, builds, planner runs, CI inspection,
version checks or merge. Publication remains paused.

The subsequent [Station acquisition patch](turtle-shell-acquisition.md) adds the
scute, crystal and extraction paths described as outside scope above.

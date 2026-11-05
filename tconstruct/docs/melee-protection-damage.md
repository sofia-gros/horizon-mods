# Melee Protection native direct-hit damage

Related to #379. Stacked on the shared Fire/Blast/Magic/Projectile protection
runtime. This is bounded direct-hit coverage, not completion of every source
Melee Protection case.

## Added coverage

- Intact supported worn armor and either-hand stock shields add 2 EPF per
  effective level, including fractional five-unit progress. Broken pieces add
  nothing. Below the default cap, one level reduces damage by 8% at this stage.
- Resolve the live `tconstruct:protection/melee` tag and the exact stock modifier
  definition. Keep the resulting points separate from unconditional protection.
- Wrap the native `mob_hits_player` call into `hurt_player_from`. Native creature
  attacks (including the native bee/effect-bearing attack path) and host PvP
  primary/sweep hits use this entry point. Recognize only `mob_attack`,
  `mob_attack_no_aggro`, `sting` and `player_attack`; also reject any active
  projectile/rocket context or native projectile-tag membership.
- A short-lived, per-player status context identifies that one damage type for
  the synchronous hit. Native health clones it into its damage environment.
  Restore the prior context after every normal return, including canceled hits,
  and preserve nested contexts. No process-global state or saved-item mutation.
- Sum the conditional points with other source modifiers and native protection
  before the existing post-armor/Resistance stage applies one 20-point cap.
  Native invulnerability, shields, bypass tags and damage handling stay intact.

## Deliberate boundaries

Native health constructs `DamageSource<()>` with both entities absent. Calling
its `is_direct()` would therefore wrongly accept every indirect hit. A tag is
also insufficient evidence of directness. This patch does not do either.

Cramming, `/damage` (even when direct), CubeHurtPlayer's separate slime/cube path,
other callers that bypass `mob_hits_player`, arbitrary custom damage types,
source-only fluid melee attacks and third-party entity identity are not covered.
These need a richer source-identity bridge. Data-pack tag additions alone cannot
turn these unsupported paths into protected direct hits. No blanket classification
of all non-projectile damage is used.

Remote clients do not calculate local bonuses. Host PvP uses the native wrapper,
but guest equipment refresh and full multiplayer behavior have not been verified.
The separate Melee Protection use-speed branch is not included here. Boundless,
negative/extended caps, Overshield consumption, other held tools and mob equipment
remain outside this patch. Mixed-modifier float ordering and armor-wear/stat
rounding retain the shared bridge's limitations.

Source: Tinkers' Construct `f4e77d43f39ba1047bf93ae816b7e1beaab514c2`,
`melee_protection.json`, `protection/melee.json`, `is_melee.json`,
`ProtectionModule` and `EquipmentContext`.

Regression cases were added for fractional/broken pieces, verified direct kinds,
projectile rejection and live melee tag projection. They were not run. Brief
static review and `git diff --check` only. No tests, builds, planner runs, CI,
version checks or merges. Publication remains paused.

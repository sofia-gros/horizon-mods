# Enderference on ordinary arrows

This #379 slice extends the published primary-melee and mob-teleport work in
#881. Chorus's default material trait now follows arrows fired by an intact,
single, recognized TConstruct Longbow or Crossbow through the existing
material-shot context. The current world's ranged tag, resolved material
traits and exact pinned Enderference definition are required. Armor's
Enderclearance trait is not substituted. Opaque saved purchased Enderference
entries, broken launchers and wire-patched stacks fail closed.

The native arrow's copied `fired_from_weapon` records the launcher before
launch wear. A boolean marker is added to that copy after other launch hooks;
its eligibility still uses the intact pre-wear snapshot, not the live held
item. The live inventory is not changed.
Changing equipment after release cannot add/remove the effect from that arrow.
The native weapon NBT serialization carries the marker; no new save format,
world cache, ammo loop, charge threshold, RNG draw or durability cost is added.
Glowing preserves that native snapshot instead of overwriting it. Multishot
arrows each pass through the existing shot context.

On an ordinary living-mob impact, Enderference I is applied for 100 ticks before
damage, independent of positive material level and projectile charge. This is
the source projectile hook rather than the successful-damage callback:
invulnerable or otherwise refused damage can still leave the effect. Native
impact cancellation runs first. The bounded ordinary-mob set is shared with
the melee fluid path, plus Shulkers; player, vehicle, armor-stand and special
boss entity paths are excluded. Existing effect ticking/removal and #881's
Enderman, Shulker and fox teleport guards are reused.

## Remaining scope

The special source arrow-damage override for an affected Enderman is **not**
implemented. Such a mob gains the effect and cannot complete its teleport, but
still retains the host's arrow-damage immunity. Native `DamageKind::Arrow` also
represents tridents, so disabling that dodge globally would incorrectly change
blacklisted projectiles. A separate projectile-aware damage boundary is needed.

This does not add thrown tools, tridents, modifiable ammo traits, other ranged
tool classes, mob-fired weapons, player/PvP effects, new recipes or potion/fluid
sources. Remote clients and unmatched/disconnected guest contexts do not apply
the effect. Unknown/replaced Enderference definitions disable application on
impact rather than continuing hard-coded behavior. The preceding melee layer's
weaker-effect restoration limitation remains.

Batch integration must retain the other projectile-launch calls (including
Spectral/Insatiable/Looting) once, and keep Glowing's snapshot-preserving change.
Enderference stamping belongs last, after any launcher identity comparisons
(Spectral compares the copied weapon against the live held item).
This branch stacks on #881; it does not copy those independent PRs.

Brief static review covered launch snapshot ownership, source hook timing,
authoritative guest routing, native cancellation and NBT paths, and the exact
flat duration/amplifier. Two marker regression tests were added but not run.
No tests, build, Mixins planner, CI inspection, version checks or merge were
performed. Gameplay, save/reload and LAN remain unverified.

Sources: Tinkers' Construct f4e77d43, `enderference.json`, Chorus traits,
`MobEffectModule.WeaponCommon`, `ProjectileHitModifierHook`, and
`ModifierEvents.projectileImpact`; host `release_bow`, `fire_crossbow`,
`projectile_hit`, native impact settlement and arrow entity NBT handling.

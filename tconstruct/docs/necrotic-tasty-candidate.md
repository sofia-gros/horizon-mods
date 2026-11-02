# Necrotic and Tasty candidate

This candidate follows TinkersConstruct source
`f4e77d43f39ba1047bf93ae816b7e1beaab514c2` and remains subject to official
package, apply, type and game checks. These are bounded paths, not complete
modifier parity.

## Necrotic

The source `LifestealModule` restores 5% of actual dealt damage per Necrotic
level on a fully charged (`cooldown > 0.9`), critical melee hit and rejects
the `tconstruct:necrotic_blacklist` entity tag. The candidate packages the
source optional dummy and Forge blacklist tag entries and reads the host's
prepared entity tags. Its mob-attack context is per actor, invalidates parent
contexts on nested attacks, checks the actual landed attack record and held
stack, and applies the host player healing event before setting health.

This covers the supported mob melee callback only. Projectile healing, PvP,
armor-stand behavior, world datapack tag overlays and other source hooks are
not established. A follow-up adds the original `necrotic_heal` event mapping
and local audio call; guest sound delivery remains unverified. Verify real package tag loading and
missing/optional tags, blacklisted and allowed mobs, killed targets, actual
post-armor damage, critical/noncritical and 0.9/above-0.9 charge, maximum
health and canceled healing, nested attacks, broken/changed tools, creative,
local and two guests. The source entity-tag files are not evidence that the
active host registry has loaded them.

## Tasty

For a supported held TConstruct tool with Tasty, the
candidate requests the source 16-tick eating duration and animation, restores
one hunger per level with saturation modifier 0.6, and applies direct tool wear
of `5 + 5 * level` after eating outside creative. It does not start while full
outside creative, on a broken tool, or when an active Slurping use takes
precedence. A follow-up supports a material-derived Scrumptious combination: its hunger,
saturation and direct durability costs are combined and poison is removed.
Savory and stored Scrumptious/Savory upgrades remain refused.

Eating while another use is active, counter-attacks, representative bacon,
food particles, durability-changed callbacks and other mixed edible modifiers
remain open. The source eating and burp sound events are played locally using
the source pitch formula; delivery to other players is not established. Runtime validation must cover offhand and LAN actor
routing, immediate release/cancellation, full versus hungry and creative,
levels 1/2, near-break and unbreakable flags, current max health/food,
Slurping, Tasty + Scrumptious poison removal and unsupported edible combinations, item identity changes,
repeated use and save/reload. Focused source arithmetic tests exist but were
not run by the development session.

## Entangled

Source `DamageOnUnequipModule` applies `tconstruct:entangled` damage on a real
equipment removal. `HealthResources::load` on the declared host reads its
`damage-catalog.json` and `damage-tags.json` into the damage catalog; this
candidate does not have an established supported path for registering the
source type and passing the previous stack for all local/guest removal slots.
Do not substitute another damage type or treat a shot release as removal. The
removal operation remains deferred while the required common API is resolved;
its existing mining and ranged stat effects remain separate.

## Projectile and player targets

The host `projectile_hit` sees the firing weapon and target, but it executes
before the source-equivalent blocked/hurt outcome is resolved. The later
`finish_arrow_hit` receives an outcome but not the target's pre-hit health;
LAN projectile routing can also swap the victim into `game.player` while the
shooter is a different guest. The candidate therefore does not infer shooter
identity or heal from the wrong health value. A scoped handoff of projectile,
target pre-hit health, outcome and shooter identity is still required; this is
a concrete remaining implementation task, not a claim that shooting is
impossible. PvP melee requires the analogous attacker/victim separation.

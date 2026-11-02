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

This covers the supported mob melee callback. PvP,
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

A subsequent candidate bites at tick 16 during an active Longbow, supported
shield, or Slurping use without ending it. It excludes crossbows, breaking bites,
and other active interactions pending safe stack-continuation integration.
Counter-attacks, representative bacon,
food particles, durability-changed callbacks and other mixed edible modifiers
remain open. The source eating and burp sound events are played locally using
the source pitch formula; delivery to other players is not established. Runtime validation must cover offhand and LAN actor
routing, immediate release/cancellation, full versus hungry and creative,
levels 1/2, near-break and unbreakable flags, current max health/food,
Slurping before/at/after tick 16, Longbow release and shield use, crossbow and near-break exclusions, two-guest sound routing and duplicates, Tasty + Scrumptious poison removal and unsupported edible combinations, item identity changes,
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

For tracked player arrows hitting mobs, the candidate records the pre-hit
health, firing weapon and source arrow power at `projectile_hit` entry, keyed
by acting player and projectile ID, and consumes that record only for the same
arrow at `finish_arrow_hit` with `Hurt`. A later impact invalidates a stale
record; an offline guest cannot use the host's health. Source power is
`ceil(clamp(base_damage * speed, 0, i32::MAX))`; healing is 5% per level of
the smaller of that power and pre-hit health. Firing-weapon persistence after
reload is not established.

This remains a partial path: source runs before damage if not blocked, whereas
the host combines shield and other refusals and the candidate requires Hurt.
PvP and mob hit refusals are not implemented. Test local and guest mob hits,
blacklist, health lower than arrow power, failed and nested hits, projectile
identity, disconnected guest, canceled healing and shot save/reload. The
state and injection have not passed official plan, type or game checks.

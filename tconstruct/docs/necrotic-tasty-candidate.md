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

This covers the supported mob melee callback.
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

A subsequent candidate bites at tick 16 during an active Longbow, Crossbow, supported
shield, or Slurping use without ending it. Launcher continuations re-read the updated stack after the bite; a broken
crossbow is not loaded. Breaking bites during shields or Slurping and other
active interactions remain deferred. The host crossbow loads during its use
tick; source crossbow loading occurs on release, an existing integration
difference requiring game verification.
Counter-attacks, representative bacon,
food particles, durability-changed callbacks and other mixed edible modifiers
remain open. The source eating and burp sound events are played locally using
the source pitch formula; delivery to other players is not established. Runtime validation must cover offhand and LAN actor
routing, immediate release/cancellation, full versus hungry and creative,
levels 1/2, near-break and unbreakable flags, current max health/food,
Slurping before/at/after tick 16, Longbow release and shield use, crossbow charging and near-break behavior, shield/Slurping near-break exclusions, two-guest sound routing and duplicates, Tasty + Scrumptious poison removal and unsupported edible combinations, item identity changes,
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
the smaller of that power and pre-hit health. The formal host `entity_nbt.rs` writes and reads the arrow `weapon` stack;
`claim_owner` resolves connected guests by UUID but does not resolve the host
player. A follow-up compares the retained UUID to the acting player when the
owner is unresolved. The complete saved-arrow path still needs a game test.

This remains a partial path: source runs before damage if not blocked, whereas
the host combines shield and other refusals and the candidate requires Hurt.
PvP and mob-hit refusals are not implemented. Test local and guest mob hits,
blacklist, health lower than arrow power, failed and nested hits, projectile
identity, disconnected guest, canceled healing and shot save/reload. The
state and injection have not passed official plan, type or game checks.

For projectile PvP, the formal host first calls `projectile_pvp_victim` and
`projectile_hits_player`. The latter takes the LAN host out of `game.lan`,
checks PvP and teams, then enters `guest_hit_by`: the target's player state
is swapped into `game.player` while `projectile_hit` determines the outcome.
Only after that returns does it restore `game.lan`. A correct implementation
must retain projectile identity, shooter UUID and victim pre-hit health across
that scope, record the actual outcome, then resolve and heal the shooter after
restoration. A subsequent candidate implements this for a guest victim and a
connected shooter, using an invalidating stack of scopes and exact projectile
and shooter UUID matching. It accepts only the host Hurt outcome. Nested hits,
disconnection, refused hits and mismatched owners must not heal. The return
injection and complete game behavior are unverified; a follow-up handles a guest arrow hitting the host with a separate
projectile-ID/UUID handoff. The host's
`projectile_pvp_victim` also requires a resolved player owner; test saved
arrows separately rather than assuming the UUID fallback extends that route.

For a Tasty bite that breaks a Slurping tool, source `EdibleModule` performs
the bite during `onUsingTick`; source `SlurpingModule` can still complete its
independent use. The candidate's `source-slurping::plan_for` rejects broken
tools and `tick_used_item` keeps the pre-bite stack for the current tick.
Supporting this case needs an active-use continuation that preserves the
original use, re-reads the updated stack and allows source-valid completion;
simply dropping the breaking-bite guard would lose or misapply that use.
The same stale-stack continuation requires explicit handling for shields.

The host-victim follow-up records pre-hit health and arrow power only when a
connected guest's UUID matches the arrow owner, then rechecks that connection
after Hurt before switching to the shooter and healing. Verify disconnection,
reconnection under a different UUID, nested impacts, shield/PvP refusal,
health caps and owner restoration; this has not passed plan, apply or game.

The #341/#343 melee PvP candidate is disabled in this integration. In source,
`ToolAttackUtil.attackEntity` records health before the modifier
`beforeMeleeHit` hooks, calls `target.hurt`, invokes the tool-level
`afterMeleeHit`, then calculates the health delta before subsequent
knockback and modifier `afterMeleeHit` hooks. The host `pvp_land` captures
health after strike construction; `mob_hits_player` calls `hurt_player_from`,
which runs player damage, blocking and native hooks before returning. The
source tool-level after-hit boundary is not connected at that point. Changes
to health in those unaligned hooks can change the source delta, including
healing and absorption-related cases. Neither equivalence nor safe guards for
all modifiers are established; no melee PvP healing runs in this candidate.

Verify the source and host call sequences with healing in pre-hit and tool
after-hit hooks, absorption, armor, canceled/blocked hits, nested attacks,
totems and changing victim health, and establish an exact boundary before
reenabling. #341 and #343 remain preserved as unverified historical candidates.

## Candidate dependency map

The combined branch builds on #329 (bounded mob melee and Tasty), #330
(mixed food and sounds), #332 (mob projectile), #334 (concurrent food),
#335 (launcher wear), #336 (saved arrow UUID), #338 (guest projectile victim),
and #339 (host projectile victim). Each fixed head remains independently
reviewable. This branch defers melee PvP; #325 remains the independent fixed
integration baseline. Validate the combined head and its declared base, not
the earlier candidates as if they included later changes.

| Scope | Candidate condition or deferral | Required verification |
| --- | --- | --- |
| Necrotic mob melee | Landed, critical, cooldown above 0.9; blacklist and actual damage | Armor, canceled hits, nested attacks, tool identity and guests |
| Necrotic arrows vs mobs | Tracked matching arrow and actor; host Hurt only | Pre-hit health cap, shield/refusal, reload, disconnected shooter |
| Necrotic arrows vs players | Guest victim or host victim; UUID and projectile match; host Hurt only | Host/guest pairs, two guests, PvP/team denial, nested hits, reload and reconnect |
| Necrotic melee PvP | Deferred; health-delta hooks differ | Source hook ordering, healing/absorption, totem and nested damage before enabling |
| Tasty standalone/mixed | Tasty and supported Scrumptious values and wear | Hunger, saturation, poison, creative, near-break and offhand |
| Tasty concurrent | Longbow, Crossbow, supported shields and Slurping; shield/Slurping breaking bite deferred | Tick 15/16/17, release, charging, near-break, Slurping completion and guest audio |
| Entangled removal | Deferred; exact damage type and prior equipment boundary absent | Exact source damage type and slot/previous-stack integration |

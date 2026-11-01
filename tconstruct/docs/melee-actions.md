# Source melee area actions

This draft implements source circle attacks against mobs for Scythe and Kama,
source mob sweeps for Sword and Cleaver, and innate Knockback for Excavator
and Battlesign. The tested runtime is `6a2fc260c6109a6709da26bdd2e8c89ed30d6be2`,
transformed with official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`.

Linux gameplay confirmed circle primary and nearby secondary hits, distance
exclusion, identical low-charge scaling and per-hit wear; a nearly broken
Scythe stopped after its first secondary. Sword and Cleaver hit their primary
and nearby secondary once, with the source range difference; low charge and
sprinting suppressed Sword sweep. Hosted LAN confirmed a guest Scythe against
mobs. Battlesign and Excavator produced observed knockback velocities 0.7 and
1.2 respectively, with one and two durability consumed. A vanilla iron sword
regression passed. These are representative tests, not all weapons or targets.
Allied exclusions, critical hits, broader LAN combinations and Windows have not
been individually verified in game. Expanded and Sweeping remain unfinished.

PvP primary and secondary targets are not connected. The current player sweep
queue stores the attacker position and hardcodes a three-block distance;
source Scythe needs distance from the primary target, and Cleaver needs a
four-block attacker distance. Guest sweeps are deferred until the acting player
is restored, so per-target circle attacks also need the original attacker's
item and durability to be handled in source order. The existing player-victim path has no `weapon_hit` / `weapon_wear` call. A
landed-hit wear hook now routes the source tools through the acting player, but
is only a prototype and does not establish PvP area or wear-order correctness. The tested official Mixins
`a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` planner rejected a correctly
selected `MODIFY_CONSTANT` of the existing `sweep_players` distance with
`E_UNSUPPORTED_CONTROL_FLOW` due to a closure in that function. A separate
state-addition attempt on `Sweep` at `player_sweep` also failed planning because
that constructor contains a closure. This describes
one attempted injection, not all possible hooks. The draft does not replace
that limit with a misleading vanilla approximation.

Remaining acceptance includes player targets and PvP permissions, allied target
exclusions, additional critical and LAN cases, and other modifier behavior. Off-hand attack, other modifier abilities, armor and ranged
weapons are future work. Crop shears loot remains unimplemented.

## Planner boundary evidence

Against the pinned Horizon workspace and Mixins CLI `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab`:

* `MODIFY_CONSTANT` targeting
  `horizon_client::game::lan_pvp::Game<'_>::sweep_players`, selector `9.0`,
  `ordinal = 0`, `require = 2` returned `E_UNSUPPORTED_CONTROL_FLOW`:
  `closures, async, try/?, unsafe and const blocks are unsupported injection contexts`.
* Adding state to `horizon_client::game::lan_pvp::Sweep` with constructor
  `Game<'_>::player_sweep`, selector `Sweep`, `require = 1` returned the same
  code and message; its constructor contains a `map_or` closure.
* `BEFORE_CALL self.weapon_hit` in `Game::attack_mob`, `ordinal = 0`,
  `require = 1` returned the same code and message. Independent `weapon_hit`
  HEAD, `attack_mob` HEAD/RETURN, `extra_knockback` RETURN and `pvp_land`
  RETURN probes were accepted.

In the present prototype a mob primary does **not** damage any player in its
area: the provisional mob selection only visits mobs. The player-primary path
likewise does not perform source area attacks. The separate landed-PvP wear
hook is provisional and does not fix either missing area path.

Further bounded probes: `Game<'_>::guest_attack` RETURN and
`Game<'_>::lan_guest_packet` RETURN in `lan_actions` both returned the same
closure `E_UNSUPPORTED_CONTROL_FLOW`. `Game<'_>::flush_sweeps` HEAD and
`Game<'_>::attack_guest` RETURN were accepted. The flush boundary occurs after
`self.lan` is restored and could host a separate player-action queue, but that
queue still needs a proven way to preserve the attacking guest's original tool,
per-target ordering and primary durability loss if the guest disconnects or
several hits are pending. Planner acceptance alone does not verify those
semantics.

A separate official planner probe accepted both HEAD and RETURN at
`lan_actions::Game<'_>::leave_guest`. HEAD runs while the guest is the acting
player; RETURN runs after the host state is restored. This may permit a safe
handoff at a narrower boundary, but applying damage to the host while the
guest is still installed would target the wrong player. Source wear ordering,
subsequent target eligibility and actor disconnect behavior have not been
implemented or validated through this candidate.

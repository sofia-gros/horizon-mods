# Source melee area actions

This draft targets source circle attacks for Scythe and Kama, source sweep
attacks for Sword and Cleaver, and the built-in Knockback levels of Excavator
and Battlesign. The current code is a prototype and has not been formally
transformed, type-checked or tested in game. Equipment registration and
material stats alone do not establish these actions.

The prototype connects source mob-target circle and sweep ranges and the
built-in knockback calculation. Circle reuses the normal mob attack path,
keeps the initiating cooldown and suppresses its secondary critical bonus;
sweep retains its separate charge and reduced-damage rules. Broken tools,
teams, other targets, durability, cooldown and reentrancy still require
representative gameplay verification. Native modifier features, including
Sweeping damage and Expanded ranges, are not claimed complete.

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

Acceptance remains source ranges and charge/ground/sprint conditions,
no duplicate primary hit, allied and ineligible target exclusions, damage,
knockback, wear and breakage, hosted LAN and PvP permissions, plus a vanilla
sword regression. Off-hand attack, other modifier abilities, armor and ranged
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

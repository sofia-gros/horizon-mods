# Double Jump acquisition

This #379 slice adds the missing normal Double Jump acquisition path. It does
not implement extra-jump movement and must not be counted as functional Double
Jump completion. The workstation description states that limit explicitly.

The source five-input recipe consumes two Pistons, one Sky Slimeball and two
Phantom Membranes in separate input slots, in any order. The normal recipe
therefore needs the five-slot Anvil. A matching Modifier Crystal can add one
level through the existing lower-capacity workstation path.

Travelers, Plate and Slime Boots must belong to the current world's boots tag.
The upgrade supports two acquired levels, costing one ability slot each and no
upgrade slots. Existing abilities and slotless bonuses still affect capacity.
The exact supported source recipe and current highest modifier definition are
required; changed costs, caps, modules and unsupported overrides fail closed.
Source material traits do not consume purchased ability slots or count against
the recipe cap unless the source recipe opts into trait checking, which this
stock recipe does not.

The saved integer uses the existing `tic_upgrades` schema. The existing station
transaction rebuilds the assembled item and preserves other modifier entries.
Negative, noninteger, noncompound and above-cap saved progress is not silently
replaced. Material swaps validate occupied ability slots. Modifier Crystal
extraction removes one purchased level and releases its ability cost. Existing
selection indices are preserved by appending the new extraction entry. Item
and workstation lists include the acquired level.

## Deferred work and review

No movement attribute projection, jump input, charge/reset state, fall reset,
network prediction or synchronization is added. Wings, Dreambound extra jumps,
other armor modifiers and non-boots trait behavior remain separate work.
This acquisition branch stacks on the earlier Depth Strider branch, rather
than the later melee-effect chain.

Brief static review covered the source five-slot recipe, two-level cap,
ability-versus-upgrade accounting, live boots tag, saved progress, extraction
and changed-contract rejection. Regression cases were added but not executed.
No tests, builds, Mixins transforms, CI inspection, version checks or merge were
performed. Actual workstation behavior, saving/reloading and LAN remain
unverified. Publication is paused.

Sources: Tinkers' Construct f4e77d43,
`recipes/tools/modifiers/ability/double_jump.json` and
`tinkering/modifiers/double_jump.json`.

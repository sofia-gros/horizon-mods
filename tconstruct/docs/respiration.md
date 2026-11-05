# Respiration helmet upgrade

This bounded #379 slice enables the source Respiration upgrade for Travelers,
Plate and Slimeskull helmets. The current world's helmet tag must also admit
the item. Ordinary acquisition consumes two fish, one colorless glass and two
kelp, using the five separate source ingredient slots at an Anvil. A modifier
crystal can supply one level at either supported workstation. Each of levels
I–III consumes one upgrade slot. Existing extraction returns one complete level
in a crystal, with the source sponge and Sky Slime Crystal inputs.

The original recipe remains in the source archive; no replacement fixed recipe
is introduced. The planner reads world-resolved ingredients and conditions,
retains the three-level and one-slot contract, rejects malformed saved progress,
and rebuilds outputs through the existing assembly pipeline. Unsupported
Respiration modifier definitions block acquisition rather than consume inputs
for an unimplemented effect. Item and workstation text now includes the upgrade.

## Breathing

The pinned source `EnchantmentModule.Constant` adds one Respiration enchantment
level per modifier level. The host consumes its native `oxygen_bonus` attribute
when choosing whether to spend air each submerged tick. A transient, separately
named attribute adds the TiC level there, preserving native enchantment bonuses
without writing a second enchantment onto the item.

Only the actual head slot, a supported helmet, complete world context, an intact
material tool, and the stock Respiration modifier definition qualify. Direct
Respiration material traits are added to independently saved upgrade levels.
Unknown definitions, broken equipment and missing/invalid snapshots remove the
owned bonus. Other native attributes are not changed.

Equipment updates, source reload and each server player's end-of-tick hook
refresh the projection. The host's guest actor swap selects that guest's armor
and attributes. A remote client does not add another prediction modifier to the
authoritative attribute. Same-item damage or tag changes are therefore re-read
at the next server tick even without a new item ID. These are static wiring
claims, not observed LAN or drowning results.

## Limits and review

Nested `respiration_skull` traits and Turtle's separate underwater effects are
not implemented here. Other armor upgrades, altered modifier definitions, and
general enchantment projection remain outside this slice. Existing material
armor wear limitations are unchanged.

A brief static review covered source recipe costs and level limits, shared
host/library module ownership, native air consumption, per-actor equipment
selection, reload and removal. A focused planner regression was added but not
run. No tests, Mixins transformation, host build, CI, version checks, save/reopen
or live gameplay/LAN validation were performed, as requested. Keep the PR draft
pending the final validation batch.

Specification: [Tinkers' Construct f4e77d43, EnchantmentModule.Constant](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/build/EnchantmentModule.java),
plus `tools/modifiers/upgrade/respiration.json` in the retained recipes archive
and `tinkering/modifiers/respiration.json` in the retained modifiers archive.

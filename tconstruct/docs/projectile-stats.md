# Material projectile speed and accuracy

This draft targets the original Longbow and Crossbow projectile speed and
accuracy using their saved material stats. The source bow multiplies charge by
velocity; crossbows multiply their projectile speed by velocity. Both use
`3 * (1 / accuracy - 1)` for inaccuracy. Existing Horizon random generation is
retained. A registered stat or a successful planner probe does not demonstrate
that the emitted arrow uses it.

Representative Linux gameplay using runtime `f898d875dad20b4a3d3752a75b910258197a2fab` confirmed actual arrow speed and spread for both weapons, source fractional low/partial/full charge, main/offhand, source three-arrow fan, ammunition, mob hits, survival wear and broken retention, repair and save/reload, hosted LAN guest shots, and vanilla bow/crossbow firing. A Tinker Station swap from wood to cobalt limb changed Crossbow `tic_velocity` from 1.0 to 1.05 and measured arrow Z velocity from approximately 2.838 to 2.984; the replacement persisted after reload. Several comparisons used synthetic stat data to isolate the shared formula, and are not a test of every material. The speed samples include normal flight drag.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` applied in 198.89 seconds; the client/ctl build succeeded in 1m24s. Four scoped Rust tests, 44 TConstruct Python tests and 7 root Python tests passed. Windows and material combinations beyond the representative cases are unverified. Firework rockets, other conditional stat modifiers, armor actions, PvP area attacks, crop shears loot and trim remain unfinished.

Official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` rejected
`release_bow` MODIFY_ARG at `arrow.shoot_from_rotation` with
`E_UNSUPPORTED_CONTROL_FLOW` (closure context), and `shoot_crossbow`
MODIFY_ARG at `arrow.shoot` with the same code (reference temporary lifetime).
A separate `add_projectile` HEAD probe was accepted, but runs after random
spread has already been applied. It must not merely scale the noisy vector:
that would not reproduce source accuracy or multi-shot direction. A scoped shot context carries the shooter and source direction to that boundary. Firework rockets use a separate launch path.
A draft scoped shot context now reruns arrow launch with source speed, spread
and fan angles at that boundary using Horizon's RNG; it consumes additional
random draws after the original vanilla launch. The representative arrow scenarios above have been built and tested. Firework rockets remain unimplemented. The draft permits source-valid low-charge shots through the existing vanilla
dispatch threshold; the representative low-charge case passed; remaining conditional stat modifiers are incomplete. See the Maintained implementation and its verification limits below.

## Maintained velocity

Bronze's Maintained trait now contributes to Longbow and Crossbow arrow velocity
at firing time. For remaining durability above half the maximum, it adds
`((remaining - maximum / 2) / (maximum / 2)) * level * 0.08 * velocityMultiplier`.
The bonus is zero at or below half durability. Repair restores the bonus on the
next shot; it is not baked into the saved base velocity. Rebuilding after a
material swap replaces the stored trait level rather than adding another bonus.

This follows the pinned [Maintained definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/maintained.json)
and [ConditionalStatModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/behavior/ConditionalStatModule.java).
Its flat adjustment has priority 100, before Dwarven's priority 85 altitude
adjustment. Consequently Dwarven's high-altitude penalty also scales the
Maintained bonus. The conditional velocity multiplier includes the tool definition, but not
Netherite’s base-only multiplier. Existing bows need a material rebuild or
regeneration to refresh the saved multiplier. Both existing firing hooks read the changed calculation;
no new hook registration is required.

Focused Rust tests cover both bows, full/three-quarter/half/broken durability,
repair, repeated rebuild, trait removal, definition/Netherite multipliers and
Dwarven's altitude penalty. The local 41-test run used the saved older host's
`ItemStack`/NBT libraries (the former client items module aliased as
`horizon_items`), not a newly built fd6 client. **Official current-target build,
actual arrow measurements, save/reload and LAN checks for this change remain
pending.** The existing broken-weapon firing guards are unchanged.

For the combined-client check, rebuild a Bronze-limb bow through the Station or
Creative material path so it acquires the new fields, then compare full, 25%,
and 50% damage at the same charge and altitude. At level 1 and multiplier 1, the
velocity bonuses are 0.08, 0.04 and 0.0 respectively. Repair and repeat, swap out
the Bronze part and confirm removal, save/reopen, then repeat a representative
LAN guest shot. Add a Dwarven part and check below sea level and at the full
altitude penalty. Record actual arrow velocity with the same observation delay;
ordinary flight drag changes later samples.

Bronze is a compatibility material: this change does not add a native tin or
Bronze acquisition path. Use a clearly labelled diagnostic fixture if the
material is unavailable in the test world; this is not a survival progression
claim. Keen remains outside this batch: its original Quartz assignment is to
ammo, and adding a bow-only number would not implement the material ammunition
path. Other conditional projectile modifiers and armor effects remain incomplete.

## Airborne projectile velocity

A bow with the Airborne trait adds `0.1 * velocityMultiplier` to arrow velocity
when its holder is off the ground, out of water, not riding and not climbing.
The source gives the flat bonus once regardless of trait level. It runs before
Dwarven's altitude adjustment. The single-player path uses the existing host
climbable query, including its ladder and trapdoor rules; if the query fails,
the bonus is not applied. The acting player's motion is used during a LAN turn;
guest water, climbing and riding state still require runtime verification.
The conditional velocity multiplier includes the tool definition, not
Netherite’s base-only multiplier or a base-stat percent. Existing items need a
supported material rebuild or regeneration to refresh the saved multiplier.
The mining portion is tracked separately in [mining traits](mining-traits.md).

Focused Rust checks use older host item and NBT libraries. A combined Linux
client on Horizon `02c8607` tested supplied Longbows in local and guest play:
airborne speed increased, while underwater did not. Ladder exclusion was
observed in diagnostics, and a mounted case did not isolate riding from the
grounded state. Crossbow Airborne and guest water, ladder and riding remain
unverified. A supplied Crossbow material swap removed the trait and persisted
after save/reload. Natural material acquisition and Windows remain unverified.

## Solid projectile damage

Invar's Solid trait adds 0.75 projectile damage per level, scaled by the
tool definition's projectile-damage additive multiplier. It combines with
Diamond and Ductile during stat rebuilding and is removed on material swap;
it is not added again when the same tool is rebuilt. This implements only the
projectile-damage portion of the [original Solid definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/solid.json).

A combined Linux client on Horizon `02c8607` tested both bows with supplied
parts and a temporary Solid II datapack. Crossbow material replacement removed
the bonus and persisted after restart in local and LAN guest play. Invar is a
compatibility material; its natural acquisition, all level combinations and
Windows remain unverified. Protection, movement and gravity are separate work.

## Raging projectile damage

Pewter's Raging trait adds conditional projectile damage at firing time:
`clamp((11 - health) / 10, 0, 1) * level * projectileDamageMultiplier`.
At 11 health or higher there is no bonus; at 1 health or lower it reaches its
maximum. The tool's material traits are rebuilt on a part swap. This implements
the projectile portion of [the original Raging definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/raging.json).

The firing path uses the current acting player's health, including during a LAN
turn when the host has swapped that player's body into the game. On the combined
Linux `02c8607` client, supplied Raging II bows showed different damage at
health 20 and 6; guest shots followed guest health rather than host health. A
supplied Crossbow material swap removed the trait and persisted after reload.
Pewter's natural acquisition, all levels and Windows remain unverified; melee
is separate work. See [the test evidence](https://github.com/sofia-gros/horizon-mods/pull/140#issuecomment-6585341785).

## Spiny and Blindshot

Spiny adds `0.1 * level` to projectile velocity before the tool's velocity
multipliers, including Netherite. Rebuilding after a material swap removes the
trait's contribution. This covers the velocity stat only; Spiny's source
shooting wear is not implemented here.

Blindshot uses dirt in ten increments per level, reducing accuracy by 0.1 per
effective level before the tool's accuracy multipliers and the 0.1 lower clamp.
The implementation supports Longbows and Crossbows, consumes no upgrade slot,
and excludes Blindshot from modifier extraction, including when Gilded is
present. Focused tests cover partial progress, repeated rebuild and removal;
current-host build and gameplay checks remain pending.

## Overwield ranged stats

Overwield adds `0.06 * min(currentOverslime, level) * statMultiplier` to both
projectile velocity and draw speed. The value is read from the tool at use time,
so refill, depletion and material replacement change the next calculation. The
velocity adjustment follows Dwarven's altitude adjustment, matching their source
priorities. Definition `multiply_stats` applies to both conditional bonuses.
Vintage’s multiply-all also affects the draw bonus; Quick Charge and Netherite
modify base draw and velocity only. Rebuild existing items to refresh these
stored conditional multipliers.

This implements the two ranged-stat modules in the [original Overwield
definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/overwield.json).
The source Overwield firing-time shield consumption is not connected, so the
full ranged behavior is incomplete. The projectile hook can spend shield for
Glowing wear, which is separate from the Overwield firing cost. Focused helper
tests cover draw and velocity for zero, partial and capped Overslime; current-host
build and gameplay checks remain pending. Verify zero/partial/full Overslime at multiple levels,
charge times for both bows, velocity at low and high altitude with Dwarven,
Quick Charge, Netherite, material replacement, reload and LAN. Record a fixture
when the material is not naturally available.

## Power upgrade

The original Power recipe consumes Ichor Slime Crystals incrementally: 72 per
level, up to five levels, on tools in `tconstruct:modifiable/ranged/power`.
Once Power is present, its source stat module adds `0.5 + 0.5 * effectiveLevel`
to projectile damage, scaled by the tool definition's additive multiplier.
The Station selector, tooltip progress and modifier extraction include Power.
The implementation currently supports the Longbow, not the Crossbow; the
existing recipe and tag also govern eligibility and consumption.

Current-target build and gameplay verification remain pending. Verify zero, 1/72, 72/72 and max progress; upgrade slots, wrong inputs,
crystal extraction/reapplication, tag rejection, the eligible Longbow, projectile
damage, material replacement, save/reload and LAN. Refer to the pinned original
[recipe](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/tools/modifiers/upgrade/power.json)
and [modifier](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/power.json).

## Raging melee integration pending

The same material's Raging melee module adds up to `2.5 * level *
attackDamageMultiplier` based on the attacker's current health, using the
same health factor as its projectile effect. The added damage precedes the
Jagged and Stonebound percentage traits, receives attack-cooldown scaling, and
is not itself multiplied by the critical bonus. The source module requires a
living target.

The shared Mixins manifest passes the target method’s `living: bool` to the
helper. The combined plan, build and runtime ordering still require verification
before this behavior is considered complete. The acting-player health path is
used for local and guest turns; melee gameplay and LAN remain unverified. Pewter remains a compatibility
material; the source alloy requires Tin or Lead, neither supplied by the base
TConstruct content.

Test both living and nonliving targets, health 20/11/6/1, broken tools,
multiple levels and non-unit tool multipliers, partial/full cooldown, critical
and secondary hits, Jagged/Stonebound combinations, material replacement and
LAN. Unit, current-target build and gameplay verification remain pending.

## Ambush shooter-health projectile damage

Manyullyn's Ambush adds 0.5 projectile damage per level, scaled by the
projectile-damage multiplier, when the firing player's health rounds up to at
least their maximum health. The Longbow and Crossbow read the current acting
player at firing, including the player body used during a LAN guest turn.
The maximum is read from that player's current max-health attribute, not assumed
to be 20. If health or maximum is unavailable or invalid, the Ambush bonus is
omitted. Supported material rebuilding stores the trait level; an existing
item must be regenerated or have its material rebuilt to acquire that data.
Changing its material rebuilds the value rather than accumulating it.

This covers only the shooter-health projectile-stat module from the
[pinned Ambush definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/ambush.json).
The projectile's target-health power bonus and Ambush melee effects remain
unimplemented. Focused Rust cases cover both launchers, fractional health,
non-default max health, stacked Raging, bad/missing values and rebuilding;
official integrated build, real firing, actual material acquisition, save/reload,
and local/LAN gameplay still require verification.

### Raging melee conditional multiplier correction

Raging melee reads the attack-damage conditional multiplier: the tool
`multiply_stats` factor and Emerald's 1.25 conditional multiplier apply.
Handle percentages and Solid/Netherite base-stat multipliers do not multiply
this conditional bonus. Rebuilding recalculates the saved value, so removing
Emerald removes its contribution. Existing items need a supported material
rebuild or regeneration to refresh the stored multiplier. This corrects the
existing Raging melee path only; it does not implement Ambush melee, other
conditional modifiers, or the separate Duel Wielding all-damage multiplier.
The source distinction is in [StatBoostModule](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/build/StatBoostModule.java),
[FloatToolStat](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/stat/FloatToolStat.java), and
[MeleeFormula](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/json/variable/melee/MeleeFormula.java).
Focused tests cover multiplier selection and rebuilding; integrated host,
actual combat, material replacement, and local/LAN behavior remain unverified.

### Raging and Ambush projectile multiplier correction

The firing-time Raging and Ambush bonuses use the source global conditional
projectile-damage multiplier, including the tool definition's `multiply_stats`
factor. A base-stat percent affects base projectile damage, but not these flat
conditional additions. Other modifiers that change the global multiplier, such
as Soft, are not implemented by this change. Rebuild or regenerate existing
bows to refresh their saved multiplier. Focused tests cover both launchers, a nonzero base percent,
both effects separately and stacked, repeated rebuild and trait removal.
Integrated host build, actual shots, save/reload and LAN remain unverified.

### Dwarven ranged multiplier correction

Dwarven's below-sea-level velocity addition uses the global conditional
velocity multiplier, including the tool definition's `multiply_stats` but
excluding base-stat percent and Netherite's base-only multiplier. The high
altitude penalty remains a multiplier on the already accumulated velocity.
Existing bows need a supported material rebuild or regeneration to refresh the
stored conditional multiplier. Focused tests cover both launchers, depth,
high altitude, repeated rebuilding and trait removal. Integrated host build,
actual firing, save/reload and LAN remain unverified. This does not change
Dwarven mining behavior.

## Preserve fractional conditional projectile damage

The release-time conditional damage helper now starts from the original saved
`tic_projectile_damage` float rather than narrowing its native-arrow damage
delta and adding two back. It applies the existing Raging/Ambush float additions
in their existing order, then widens the final float to double before subtracting
two for the native arrow adapter. This prevents inactive conditions from changing
ordinary fractional stats such as 0.1 or 0.3. Missing/nonfinite saved-stat fallback,
missing health handling, condition predicates and final clamps are unchanged.

Pinned TConstruct's `ModifiableBowItem` and `ModifiableCrossbowItem` calculate
conditional projectile damage as a float before passing that result to the
arrow's double base damage. The conditional modules remain those in the pinned
[`Raging`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/raging.json)
and [`Ambush`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/ambush.json)
definitions. This change fixes those conversion boundaries, not other modifier
semantics or arbitrary ammunition behavior.

The current adapter still adds a double delta to the arrow's native base damage.
Exact reconstruction is verified only for tested representable values on base-2
arrows. Extremely tiny values such as `1e-30f32` and the smallest positive float
subnormal still become zero through the `2 + delta` interface. Custom arrows
with a different native base also retain their existing ordering difference.
Neither limitation is resolved by this slice.

The focused harness runs the actual projectile helpers against cached Horizon
item/world libraries: 17 tests pass, including inactive fractional stats,
conditional float additions, native base-2 reconstruction, Diamond fallback,
invalid context, clamps and an explicit regression documenting the tiny-value
limit. This is not a generated-host build or gameplay test. In-game bow/crossbow
shots, save/reload, LAN and Windows remain unverified.

## Longbow release threshold order

The Longbow release hook now tests `charge * velocity < 0.1` before computing
the host's three-times launch power, matching pinned
[`ModifiableBowItem.releaseUsing`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/item/ranged/ModifiableBowItem.java).
The previous `(charge * 3) * velocity < 0.3` comparison can round differently
on either side of the threshold. Exact equality still permits release.
The dispatched launch power remains `charge * 3`; this is only a firing-gate
correction, not a launch-vector or projectile-damage change.

The isolated cached-library harness executes the real release hook, draw,
charge, and stat helpers with actor/dispatch boundary doubles. It checks both
rounding-error directions, threshold equality and adjacent floats, ordinary
charge times, broken/changed-hand/non-Longbow gates, missing-stat fallback,
and actor-context cleanup. Source ammo/Forge-event ordering, full actor
behavior, native projectile damage, gameplay, LAN, and Windows remain outside
this validation. Run the harness through `test_longbow_release_threshold.py`
with `TC_LONGBOW_HARNESS_DEPS` pointing to compatible cached Horizon rlibs;
no dependency build is performed.

## Crossbow primary arrow position

Homogeneous volleys of supported arrows now mark index `count / 2` as primary,
matching pinned
[`ModifiableCrossbowItem.fireCrossbow`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/item/ranged/ModifiableCrossbowItem.java).
That is the center of an odd volley and the upper middle index of an even one.
Shot order, angles, power, and all projectile damage calculations are unchanged.

The correction applies only when every loaded slot has the same item and
components and that item is a normal, tipped, or spectral arrow. Source ammo
is one repeated ItemStack, not the host's arbitrary charged-item list. Mixed
items/components, unknown ammunition, and any volley containing rockets keep
the old original-index-zero primary behavior. Rockets remain excluded from
the arrow context queue. In particular, this does not newly remove the primary
from a mixed volley with a center rocket or promote an arrow when the first
loaded slot is a rocket. Those payloads remain outside source parity.

Primary currently controls the extra Glowing wear. That cost moves to the
middle arrow for eligible volleys. The host applies ordinary wear after each
projectile; source applies ordinary wear after the entire volley. A retained
crossbow can therefore break before its middle arrow, and the existing broken
wear guard can suppress the Glowing cost then. This ordering mismatch is not
fixed here. The firing hook snapshots the full volley before dispatch and
does not recalculate or cancel queued arrows on intermediate breakage.

The isolated cached-library harness executes the actual crossbow-use helper
with actor/dispatch boundary doubles. It checks counts one through five,
matching and differing component payloads, mixed arrow types, center/first
rockets, all rockets, unknown ammo, and broken/using/empty/missing-stat gates.
A one-durability case checks the snapshotted queue, not live wear execution.
Run `test_crossbow_primary.py` with `TC_CROSSBOW_HARNESS_DEPS` pointing to
compatible cached Horizon rlibs. No dependency build, full generated-host
build, live mid-volley wear, gameplay, LAN, or Windows validation was run.

## Draw-time signed saturation

For Longbow release and Crossbow charging, the existing draw-speed helper now
narrows the rounded-up duration to signed i32 before returning the host's u32.
Pinned [`GeneralInteractionModifierHook.getDrawtime`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/hook/interaction/GeneralInteractionModifierHook.java)
uses `(int)Math.ceil(20f * speedFactor / drawSpeed)`. At the supported speed factor
1, zero or sufficiently small positive speed therefore saturates at 2147483647,
not the previous unsigned 4294967295. The upper bound is enforced by integer
conversion, not an f32 clamp: representing i32::MAX as f32 rounds up to 2147483648.

The existing host minimum of one tick remains deliberate. In particular NaN
still yields one tick, although Java's NaN-to-int result is zero. Saved-speed
clamping, malformed/missing-stat fallback, Overwield addition, ordinary rounding,
and unsigned elapsed/vanilla-charge conversion remain unchanged. This does not
claim parity for every malformed input or implement additional conditional draw
modifiers. No projectile damage or projectile-consumer code is changed.

The cached `longbow_release_threshold.rs` harness checks zero, subnormal/minimum
normal speeds, the signed-limit boundary and its next f32 neighbor, ordinary
speeds, NaN/infinities, saved-data fallback and Overwield. It executes both actual
caller helpers: saturated Crossbow charge duration, Longbow rejection after only
20 ticks and full-charge dispatch after the saturated duration. Host actor/world
and dispatch boundaries are doubles; this is not a real multi-year hold or a
full generated-host/gameplay/LAN validation. Run with compatible cached Horizon
rlibs via `TC_LONGBOW_HARNESS_DEPS=/path/to/deps python3 tconstruct/tests/test_longbow_release_threshold.py`.

## Trueshot incremental-level precision

The bow stat rebuild now reconstructs Trueshot's whole logical level from fixed
10-unit progress, merges material trait levels, then applies the f32 partial
correction before adding 0.1 per level to accuracy. This follows pinned
[`IncrementalModifierEntry.merge` and `scaleLevel`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/IncrementalModifierEntry.java),
the [Trueshot stat module](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/trueshot.json)
and its [10-unit recipe](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/recipes/tools/modifiers/upgrade/trueshot.json).
For example, base accuracy 0.1 and progress 1 now produce 0.11000000685453415
instead of 0.10999999940395355. The final accuracy clamp does not hide that case.
An unrepresentable merged logical level supplies no Trueshot bonus; this is a
fail-closed host boundary rather than Java integer-overflow emulation.

Only the Trueshot branch changes. Fixed progress storage, trait lookup, modifier
ordering, bow eligibility, surrounding stat algebra, accuracy clamp and other
upgrades remain unchanged. This does not implement arbitrary datapack denominators
or change projectile-damage, firing-consumer or projectile-motion processing code.
The corrected saved accuracy can still propagate to projectile spread.

The actual `apply_stats` cached-library harness exercises both bow IDs, four
bitwise counterexamples, full/partial 10-unit boundaries, negative/large/overflow
inputs, repeated rebuilding, clearing progress/traits, non-bow gating, preserved
foreign/progress data and unchanged draw speed/velocity. Existing tests cover
combination with other ranged modifiers. No full generated-host build, gameplay,
LAN or Windows validation was run.

# Overslime ordinary armor absorption

Partial high-priority #379 durability implementation, stacked on Solar Powered
armor #933 and the retained writeback/prevention chain #923/#925/#929/#931.
This is ordinary armor durability absorption, not Overshield's health-damage
protection-point spending.

## Calculation and commit

The source statically registers Overslime at priority150. It is admitted without
requiring a nonexistent dynamic modifier JSON definition; a same-ID JSON file
does not replace the static source registration. Live material/saved levels must
show it active before positive stored shield can be used.

The pre-native sequence is Tanned200, Solar185, Overslime150, Unbreakable125,
then Dense/Reinforced100 in their validated insertion order. Overslime absorbs
one point of durability per point of shield, with no RNG. Its live
`durability/bypass_overslime` tag is resolved for ordinary EMPTY cause, including
the nested `bypass_extra_bar` tag and source Glowing member. Unknown tag leaves
or invalid resources fail closed. The static Overslime ID is also recognized as
registered when resolving live modifier tags.

Calculated shield consumption has a dedicated per-actor, per-ArmorHurt-event
collector. It never uses or drains generic modifier-wear `overslime_pending`.
A zero-spend record is included so every final native wear result must pair with
exactly one calculation. Duplicate, missing, replaced or ambiguous records do
not mutate a slot. Both collectors are removed before equipment refresh and
outer nested scopes restored afterward.

At the existing end-of-event writeback boundary, shield and final durability are
committed together to the unchanged equipped item. Shield is consumed even if
later Unbreakable/Reinforced/native cancellation leaves zero durability damage.
Solar/Tanned prevention happens before absorption and therefore preserves shield.
The next ArmorHurt event reads the committed remaining amount. A changed stack
refreshes Maintained; a newly broken stack refreshes equipment and is retained.

## Bounds

The current port's positive-capacity storage contract is preserved: active shield
records require an integer saved capacity in 0..32767 and a nonnegative stored
amount no greater than it. Missing amount means zero. Positive orphan shield,
missing capacity, malformed or over-capacity state is rejected. This does not
recompute capacity, add capacity/refill modifiers or repair stale saved data.
Other active modifiers must satisfy the existing bounded armor guards. The
follow-up [friend eligibility slice](overslime-armor-friends.md) admits six exact
capacity/recovery definitions and two proven no-wear static registrations;
other friend/capacity combinations remain outside the supported set. Native enchantments and source/native unbreakable flags,
creative/remote actors, wrong slots, opaque data and broken gear remain excluded.

Shield persistence is intentionally deferred with durability to the per-event
commit, rather than mutating immutable native wear snapshots. Arbitrary nested
mod callbacks that inspect intermediate shield state are not claimed source-
identical; ambiguous callbacks fail closed. Source break animation, generic held
wear, Overshield health protection, native batched health ordering and earlier
integration limits remain. Gameplay, persistence and dedicated/guest/LAN are
unverified. No native extension or provenance-blocked fluid code is added; the
base defense chain still requires native 09f0179db and 5e47547e2.

## Sources and review

TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2 (MIT): static registration in
TinkerModifiers, OverslimeModifier, DurabilityShieldModifier,
DurabilityShieldModule.onDamageTool, OverslimeModule and the two bypass tags.

Brief static source/API and whitespace review. Five pure regression cases added
and one previous guard case updated, not run. Tests/builds/planner belong to the
separate batch integration pass. No CI inspection or merge was performed.

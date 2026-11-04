# Mining, durability and repair trait follow-up

> **Provisional build:** Some effects and operations described below are unavailable or under verification. Earlier test results refer to their stated revisions; see [current limitations](provisional.md).

Material mining, durability and repair traits use original Tinkers' Construct
revision `f4e77d43` with shared mining, wear and repair paths.

This unit covers the mining modules of Lightweight, Heavy, Vintage, Entangled,
Jagged, Stonebound, Maintained and Crumbling, plus Dense repair and durability prevention.
Overlord requires functional Overslime and remains unimplemented. Other modules
of these traits are not implied to be complete.

## Implementation and limits

Station and Creative regeneration store the original harvest-tagged tool's
Lightweight/Heavy/Vintage/Entangled stat and Jagged/Stonebound/Maintained levels on the item. A shared item boundary recognizes effective per-stack tool
rules for both the client and host using the active world item tags; separate context hooks apply conditional
speed after mining efficiency and potion effects, before underwater/airborne
penalties. Materials and part swaps rebuild the stored values. Previously saved
tools need regeneration before these new fields are present.

Dense repair uses the original level ranges. Dense damage prevention rolls once
per durability point through the existing shared `durability_loss` path.
Original conditional wear formulas use the `multiply_all` durability multiplier;
the earlier melee calculation mistakenly included material percent and
`multiply_base` bonuses. The implementation corrects that multiplier for the existing
Jagged/Stonebound melee path too. Representative melee and repair regressions passed.

Crumbling adds one mining speed per level (scaled by the tool and player mining multipliers) on blocks that do not require a tool, even when the held tool is not effective. It skips broken tools. An isolated Linux test client with eight unrelated injections excluded measured dirt at 15 versus 13 ticks and stone at 18 in both cases; a broken tool took 56 ticks on dirt. Crumbling combined with Stonebound remains unresolved: a piston comparison gave 10 versus 14 ticks despite a higher calculated conditional speed.

Dwarven is connected on the development branch between Maintained and the percentage-based Jagged and Stonebound effects. An isolated Linux test client with eight injections excluded measured 13 ticks at Y=40 and 43 at Y=240 using supplied parts and a trait fixture; existing tools need regeneration to receive the trait data.

Original modifier priority runs Maintained before Dwarven and the percentage-based Jagged
and Stonebound effects; equal-priority effects retain source insertion order.
The development branch uses PlutoLib transport for prepared per-stack `minecraft:tool` data; the complete TConstruct LAN path remains unverified. Legacy custom tool data is retained for existing boundaries. Existing saved tools need a supported regeneration or part replacement
before the new fields are added. See [Harvest tiers and effective blocks](harvest-rules.md)
for representative tier, drop and LAN validation; not all tools and materials
have been individually tested.

## Representative Linux verification

The official Mixins `a2af5c0dd4117d51ecd70066ba7a00b4a2c10dab` transform
(173.42 seconds) and client/ctl build (134 seconds) passed. A temporary test
datapack placed the original trait IDs at selected levels on a flint head while
keeping the other tool materials identical. In the controlled stone test,
measured times included the same 5-tick post-break delay: baseline 18 ticks,
Lightweight level 2 at 17, Heavy level 2 at 15, Vintage level 2 at 22, and
Entangled at 10. Jagged level 3 changed from 18 fresh to 21 worn and 23 at damage 84/85; the broken tool was retained. Stonebound
changed from 18 to 17, and Maintained from 10 to 18. On dirt, a block ineffective
for the pickaxe, the baseline and Entangled both took 20 ticks.

Reversing material positions gave equal times for Maintained with Jagged (22
and 22 ticks) and Stonebound (17 and 17). Efficiency II was applied using a
test item modifier because the current vanilla enchant command rejected the
registered TConstruct item; with Haste and wear, baseline versus Jagged took
10 versus 11 ticks. A deterministic unit test also covers the operation order.
One Dense level 3 test prevented 10 of 12 wear points; after replacing that
head, four of four blocks caused ordinary wear. Dense reduced representative
repair from 51 to 42 durability restored. Jagged mining stayed at 21 ticks
before and after save/reload. A LAN guest mined stone in 7 ticks using a
command-supplied cobalt-head tool regenerated through the Station; ordinary
vanilla iron-pickaxe mining and the existing Jagged/Stonebound melee and
Cultivated/Preserved repair paths were also checked.

These are representative checks, not exhaustive tests of every material,
tool, effect combination or Windows. The selected mining traits' other modules,
applied mining modifiers, and Overlord/Overslime remain incomplete. Full
Mixins CI and independent review are still pending.

![Dense repair in the Tinker Station](material-mining-repair.png)

![LAN guest's cobalt tool](material-mining-lan.png)

## Airborne mining

The Skyslime Vine Airborne trait multiplies mining speed by five while the
holder is off the ground, including on blocks where the tool is ineffective.
It applies after flat conditional mining bonuses and does not apply to broken
tools. The host and client selectors target the final speed read after airborne and
water penalties, so flat bonuses can be applied in the source order. An official combined Linux build on Horizon `02c8607e` verified representative airborne and grounded mining; material swapping remains unverified. Rebuilding material updates the stored trait. This implements
the mining portion of the [original Airborne definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/airborne.json);
the conditional projectile velocity portion remains unimplemented.

Focused local Rust tests checked trait presence, grounded and broken cases,
and the Maintained combination after airborne and water penalties, using the
saved older host NBT library. An isolated official Mixins plan resolved both
final speed reads and their local captures. The combined Linux transform and build passed on Horizon `02c8607e`; the main branch's earlier pinned host was not retested in that run. The combined Linux client confirmed a supplied tool retains its Airborne data over save/reload. Broken-tool gameplay timed out; material swapping, natural acquisition, LAN and Windows remain unverified. In the combined
client, make a supported harvest tool with Skyslime Vine, compare the same
block on the ground and while airborne, including one ineffective block, then
repeat after breaking the tool and after replacing that material. Capture both
local and LAN guest behavior; use a labelled fixture if the material is not yet
naturally obtainable.

## Overwield mining

For a non-broad harvest tool with Knightslime's Overwield trait, an effective
block receives the source multiplier `1 + 0.2 * min(overslime, level)`. It uses
the current stored Overslime amount, not capacity, and applies in the same
ordered percentage stage as Jagged and Stonebound. The tool is checked against
the resolved source `tconstruct:modifiable/broad` item tag. If the tag is
unavailable, both Overwield speed variants are skipped until it resolves. In hosted LAN, the host sends the resolved broad-tool membership to each guest and replaces it when the tag changes; reconnect, a world reset, and tag reload start unknown until a fresh snapshot arrives. Unchanged snapshots refresh within 20 active host ticks (about one second at 20 ticks per second; longer if paused or slow). The frame is capped at 16 KiB, so periodic payload is at most 16 KiB per guest per 20 ticks, plus protocol overhead; oversized membership is unknown. Invalid or unavailable snapshots remain unknown rather than treating every tool as narrow. Overslime spending remains host-authoritative. Reload and datapack changes invalidate parked levels’ item-tag snapshots; the next level update rebuilds them before a fresh snapshot can be sent. This synchronization still needs gameplay verification, including datapack reload across dimensions and reconnect. Zero
Overslime or an ineffective block gives no bonus. The formulas correspond to
the [source Overwield definition](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/overwield.json).
Material acquisition remains separate work.

Focused helper tests passed against Horizon 9a6 item/world libraries; host build and gameplay remain unverified. Verify zero, one and multiple Overslime units and trait levels, a broad
and a non-broad tool, effective and ineffective blocks, live refill and
consumption, broken tools, and combinations with other mining modifiers. Check
material replacement and save/reload. Use labelled fixtures where necessary.

### Broad tools

The broad-tool speed module instead uses `1 + 0.2 * min(overslime, 5 * level) / 5`
on effective blocks. This reads the same live Overslime value and uses the loaded
broad tag; it does not use AOE size as the amount. A pre-destroy hook spends
up to five Overslime per level for broad tools or one for other harvest tools.
A scoped actor flag prevents repeated spends for the center and additional AOE
blocks; broken tools and unknown item-tag snapshots do not spend. This host
boundary runs after permission checks and mod cancellation handlers, but before
block removal. Creative, guest behavior and full source parity remain unverified.

### Incremental modifiers supplied as material traits

When a datapack supplies Haste, Sharpness, Swiftstrike, Quick Charge or
Trueshot as material traits, the existing stat rebuild now combines those
integer trait levels with the saved fractional incremental upgrade progress.
The implemented scope is harvest-tool mining speed, melee attack damage and
speed, and longbow/crossbow draw speed and accuracy. Rebuilding after a material
change recalculates from the baseline. Chest-slot Haste traits are resolved on
the authoritative server from the current material snapshot, the non-harvest
tag and the byte-exact pinned Haste definition. Each client receives only its
own chest item and trait level and uses the result only while the item's decoded
components, count and damage match. The host comparison ignores opaque wire
patches; those components are not used by this trait lookup. A client can
briefly predict an old level after a world-only change until invalidation or a
fresh frame arrives; the server remains authoritative. Unknown snapshots and mismatched equipment omit the trait contribution; source
overrides, other armor slots and held non-harvest tools remain unsupported. The
existing chest upgrade contribution remains available. This does not provide a
complete modifier registry or prove behavior when an external native mod
overrides the same modifier.
The source-data and host-type fixtures cover
trait-only, partial-upgrade and combined inputs; actual Station, Creative,
reload and LAN behavior on the integrated host remains unverified.

## Blasting candidate

Blasting's incremental gunpowder recipe is connected for the source harvest/stone
tag, and its effective level scales a flat mining-speed bonus by the target
block's explosion resistance. The source formula caps the per-level resistance
term at 10; the bonus applies only to effective, unbroken tools and precedes
percentage effects. Chest Haste is applied after those tool effects, matching
the source hook order. Existing tools need a supported regeneration or station
operation to populate the mining-effect snapshot. Focused formula and station-plan tests passed, including partial consumption and
rejection checks. The full transformed client, actual station
transactions, save/reload, and local/LAN mining remain unverified. This does
not resolve the separate Crumbling plus Stonebound observation.

The source data declares resistance 3.0 as a fallback. In the live hook Horizon
always supplies its block `StateFacts` resistance; for an absent or non-finite
value the helper conservatively skips Blasting rather than assuming a block
value. This defensive case is a deliberate difference from the source fallback.
The pinned source `PostFixFormula.processLevel` uses the modifier's effective
level, including incremental progress.

Flat conditional mining bonuses use the pinned source's Haste/Conduit maximum,
Mining Fatigue table, binary Aqua Affinity check, and airborne penalty before
chest Haste. Deterministic tests cover these combinations for Blasting,
Crumbling and Maintained; local/LAN gameplay remains unverified. Horizon's base
speed itself differs at Mining Fatigue III and above and uses a configurable
submerged-mining attribute; those host base-speed differences remain open. Its
separate `block_break_speed` attribute adjusts the incoming host speed, but
is not included in the source flat-bonus multiplier. The fixed source applies
flat tool hooks to the event speed it receives before the final armor multiplier;
focused tests cover incoming speeds including zero. This source-order check does
not establish behavior with other mods or in a running client.

The already-adjusted host speed cannot safely be inverted here: a zero factor
loses the original value, and contributions from other mods cannot be isolated
at this hook. The high-Fatigue and custom-water base-speed differences therefore
remain open pending a safe earlier boundary.

## Hydraulic: underwater portion

Hydraulic's harvest-tool upgrade accepts the source prismarine inputs and
combines partial progress with supported material trait levels. When the
acting player's last eye-water sample is set, it adds the source underwater
bonus, including Aqua Affinity, effective-tool and broken-tool conditions.
This is deliberately partial: the source also grants a bonus when rain reaches
the player, but that branch is not connected. Tool and Station information warn
that the rain bonus is unavailable. Existing items need a supported regeneration
to receive the new conditional snapshot. Recipes changing its 36-item level or
five-level maximum and malformed stored amounts are rejected rather than
consuming ingredients or rewriting the data. Text warnings are currently English.

Focused tests cover the formula, partial progress, actor data wiring, salvage,
and ingredient/remainder planning. They do not verify an actual Station
transaction, local/LAN mining, weather, save/reload or the full host build.

## Conditional mining multiplier order

The saved conditional mining multiplier includes Emerald whether supplied by
an upgrade or a resolved material/fixed trait. Emerald's conditional factor
and Vintage's multiply-all factor are applied in saved modifier order after the
tool definition's global factor. This preserves source floating-point rounding
when both are present. Material regeneration replaces the saved value; existing
items need a supported rebuild to acquire the correction. This is limited to
the existing conditional mining path, not complete Emerald behavior for every
possible datapack trait. Focused tests cover both trait orders and an installed
Emerald upgrade. Integrated host build, real mining, save/reload and LAN remain
unverified.

## Diamond trait stat additions

A resolved Diamond material or fixed trait now contributes the same flat
mining (+2), melee (+0.5) and projectile (+0.5) base-stat additions as an
installed Diamond upgrade on supported tools. The flat source modules run once
when trait and upgrade are both present, regardless of trait level; rebuilding
or removing the trait recalculates rather than stacking. This change covers only
these three stat paths. Other Diamond modules, full datapack support and
existing-item migration remain separate;
existing items need a supported rebuild or regeneration for the stat change.
Focused tests cover the three paths, both bows, trait plus upgrade, repeated
rebuild and removal. Integrated host build, actual mining/combat, save/reload
and LAN remain unverified.

## Diamond and Emerald trait durability and armor

The existing supported assembly and Creative rebuild paths now recognize
Diamond and Emerald from resolved material or fixed traits as well as installed
upgrades for gem durability and armor stats. Diamond adds 500 base durability
(or 250 for armor), one armor point and the worn-armor flag; Emerald applies
its 1.5 base durability multiplier and adds one armor toughness. A trait plus
an upgrade contributes each flat module only once. The assembly pipeline
rebuilds from the base before applying modifiers; previously saved items need
a supported rebuild or regeneration. Repair and full datapack behavior remain separate. Focused tests cover non-armor and worn armor,
non-unit trait levels, both sources together, repeated application and a
simulated base reset followed by trait removal. Integrated host build, actual equipment, save/reload and LAN remain unverified.

## Gem trait harvest tiers

For supported harvest-tagged tools, Station and Creative rebuilds now include
resolved Diamond, Emerald and Netherite traits when choosing the existing
material-derived harvest-tier floor. The highest supported floor wins, and a
trait plus the same upgrade does not lower or duplicate it. Removing a trait
on an ordinary head-based tool rebuilds its rules from the remaining materials
and modifiers. If traits cannot be resolved, the existing upgrade-only fallback
remains; no-head Netherite and full datapack tier ordering remain separate.
Previously saved tools need a supported rebuild or regeneration. Focused Rust
tests cover tier selection and tag gating; a static test checks both rebuild
entry points. The host integration build, real mining, save/reload and LAN are
unverified.

## Emerald trait Station material repair

For resolved traits, Station material repair applies known Emerald, Cultivated,
Preserved and Dense hooks to each per-item repair amount in modifier order.
Emerald's 1.5 factor applies once even if it is both a trait and an upgrade.
Ambiguous upgrade order and invalid or overflowing supported levels are rejected
before consumption, including older non-Emerald items that previously used a
guessed order. If traits cannot be resolved, the prior fallback remains; unknown
or custom repair hooks are not implemented. Casting part-swap
remains separately limited as described below.
Focused tests cover both factor orders and trait/upgrade deduplication; a static
test checks the Station wiring. Actual repair plans and ingredient counts,
host integration, save/reload and LAN remain unverified.

## Emerald trait modifier repair

Station modifier-repair recipes use the ordered known repair-factor
calculation starting at 1.0, as the original Station recipe does, whether or
not Emerald is present. Emerald trait plus upgrade applies the factor once;
non-optional fixed traits are resolved by the existing trait reader. Ambiguous
multiple upgrade ordering or invalid levels reject the operation before
consumption, including non-Emerald tools that previously used a guessed order.
Unknown or custom repair hooks are not implemented. Casting part-swap remains separately limited as described below. Focused
arithmetic and static wiring tests cover this path; an actual Station plan,
ingredient consumption, integrated host build, save/reload and LAN are unverified.

## Emerald trait casting part-swap repair

The existing ordinary casting part-swap path now permits an unchanged resolved
Emerald trait on a damaged tool and applies its flat 1.5 repair factor once,
including when Emerald is also an upgrade. Composite casts do not repair. The
existing checks still reject changed unsupported traits, fixed-definition traits,
unknown traits and broken tools. When Emerald is a resolved trait, supported
Cultivated, Preserved and Dense repair hooks can also apply in source order to
the rebuilt tool's per-item amount; ambiguous upgrade order fails closed.
The same ordered path also supports combinations of those known repair traits
without an Emerald trait, including an installed Emerald upgrade. Unknown or
ambiguous modifier order still fails closed. The tool is rebuilt and validated
before the existing transaction proceeds. Focused factor
and static wiring tests cover this narrow path; actual casting, fluid/input
consumption, host integration, save/reload and LAN remain unverified.

## Dense repair rounding at high levels

Dense repair levels 6 and above now use the source float evaluation order,
`1 + (-0.5 + -0.05 * level)`, before applying the factor. This affects the
existing Station and limited casting repair calculations. At level 7 and an
initial repair amount of 20, source truncation yields 2 rather than 3; level 8
and amount 10 yields 1 rather than 0. Mixed-factor Station ordering and casting
restrictions are unchanged. Focused tests cover the factor and arithmetic;
actual repair transactions, host integration, save/reload and LAN are unverified.

### Casting repair upgrade ordering

Ordinary material part-swap casting now uses the same ordered known repair
hooks as Station repair, including Cultivated, Preserved, Dense and Emerald
when supplied as upgrades as well as supported material traits. Multiple
active upgrades require an unambiguous saved upgrade order; malformed levels,
missing order, and unsupported traits reject the candidate before consumption.
The source applies repair hooks to the rebuilt tool in modifier order (pinned
`PartSwapCastingRecipe.java`, lines 159–178). This is a focused code-level
candidate; integrated build, gameplay, datapack-supplied upgrades, persistence,
and LAN remain unverified. Composite swaps do not repair and are unchanged.

## Jagged and Stonebound mining factor precision

The shared active mining helper now follows the pinned per-factor float
operation order: `sqrt(lost / durability_multiplier) * level * coefficient + 1`.
Jagged's mining coefficient is `-0.01`, and Stonebound's is `0.01`. Previously
the coefficient was multiplied before the level, which changes f32 rounding.
The factors still multiply the current speed sequentially in the supplied
modifier order. See pinned
[Jagged](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/jagged.json)
and
[Stonebound](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/stonebound.json).

This slice preserves the existing effective-block gate, nonnegative levels,
invalid-durability floor, final nonnegative speed clamp and modifier resolver.
It does not add support for custom modifier formulas, change upgrade/trait
resolution, or resolve the separately documented Crumbling/Stonebound gameplay
difference. It changes no melee arithmetic.

A dependency-free harness executes the actual shared mining and Dwarven helpers:
13 Rust tests pass, including bitwise counterexamples for Jagged (lost 1,
multiplier 0.1, level 5) and Stonebound (lost 2, multiplier 3, level 5), supplied
order, effectiveness, zero wear and invalid-durability behavior. Two Python tests
run the harness and check the shared module/rule wiring. Full generated-host
compilation, in-game mining timing, save/reload, LAN and Windows remain unverified.

### Broad-tool Overwield factor precision

For broad tools, the active mining helper now divides the capped overslime
amount by five before multiplying by `0.2`, matching the pinned
[Overwield mining formula](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/overwield.json).
Previously these operations were reversed. At level 2 with eight overslime,
the float factors differ by one bit before multiplying the current speed.

The existing broad-tag lookup, missing-tag refusal, integer level/cost caps,
effective-block gate, modifier order and final speed clamp are unchanged.
Narrow tools still have unit cost and the same factor. This does not expand
supported modifier definitions, change overslime consumption, or verify
gameplay timing. The actual-helper harness now passes 15 Rust tests, including
a bitwise counterexample and broad/narrow, depleted, capped and ineffective
cases; its two Python checks pass. No full host build or gameplay test was run.

### Hydraulic and Blasting incremental precision

The shared mining helper now reconstructs source incremental effective levels
from the existing merged snapshot amount. Hydraulic uses 36 units per level;
Blasting uses 20. Whole levels convert directly to float. Partial levels use
`(whole + 1) as f32 + (remainder - needed) as f32 / needed as f32`, matching
pinned
[IncrementalModifierEntry.scaleLevel](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/IncrementalModifierEntry.java).
The existing producer already combines integer trait levels and upgrade
progress before the snapshot is evaluated; no trait level is added again.
The corrected level enters the existing
[Hydraulic](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/hydraulic.json)
and
[Blasting](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/blasting.json)
formula operations without reordering their multipliers.

Fixed denominators, the producer's saturating integer snapshot encoding,
negative-amount normalization, effective-block checks, supported water/Aqua
Affinity and resistance contexts, modifier ordering and final speed clamp
remain unchanged. Saturation can lose original combined levels at extreme
values; this slice does not claim source parity beyond that existing encoding.
Custom required amounts/formulas and Hydraulic rain behavior remain outside
this change.

The real-helper harness passes eighteen Rust tests and two Python checks,
including one-unit bitwise counterexamples, whole and partial boundaries,
combined trait-equivalent amounts, both water variants, extreme stored amounts
and missing/ineffective contexts. No host build, mining timing, gameplay or LAN
validation is claimed.

## Blasting power precision

Blasting now widens the already-computed float exponent to double for the
power operation and narrows its result to float before the cap, incremental
level, mining multiplier, and speed addition. This matches the numeric types
in pinned
[`PostFixOperator.POWER`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/json/math/PostFixOperator.java)
and the
[Blasting formula](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/blasting.json).
The subtraction and division forming the exponent remain float operations.
The previous float-only power operation can round differently, including at
resistance `7.024` (float bits `40e0c49c`): the old Linux result was `3ef1dae4`,
while Java and the new path produce `3ef1dae3`.

The lightweight harness executes the active helper, checks three fixed
OpenJDK 25 power fixtures, and covers narrowing before later arithmetic,
partial levels, cap, absent/nonfinite resistance, and ineffective tools.
This aligns the operation types; it does not promise universal bit identity
between Rust platform math libraries and Java Math.pow. The existing missing
resistance fallback, context gating, and modifier ordering are unchanged.
No full generated-host build, gameplay, LAN, or Windows validation was run.

## Haste tool-stat incremental precision

The harvest-tool regeneration path now reconstructs Haste's integer logical
upgrade level from the existing fixed 45-unit progress, merges the material
trait's whole level, then applies the float partial-level correction. It no
longer divides progress by 45 directly or adds the trait after rounding the
partial upgrade. This follows pinned
[`IncrementalModifierEntry.merge` and `scaleLevel`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/IncrementalModifierEntry.java)
and Haste's
[4-per-level additive mining stat module](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/haste.json).
The source-ordered additive, percent, and overall multiplier stages and the
final speed clamp remain unchanged. Negative or malformed progress retains
the existing zero-progress fallback. Unrepresentable merged integer levels
suppress only Haste's bonus rather than overflowing.

The cached-library mining integration harness executes the actual regeneration
helper and effective-block return hook. Bitwise counterexamples cover one
progress unit alone, three units plus one trait level, and six units plus two
trait levels. It also covers full and partial level boundaries, repeated
regeneration, trait/progress removal, non-harvest gating, unmatched blocks,
large/negative inputs, and preservation of progress, foreign data, and the
legacy Tool mirror. Existing integration cases cover other mining modifiers.
This remains limited to the supported fixed-45 progress representation; it
does not add arbitrary incremental denominators, datapack interpretation, or
change the separate equipment Haste attribute path. No full generated-host
build, gameplay, LAN, or Windows validation was run.

## Chest Haste incremental-level precision

The existing chest Haste mining multiplier now reconstructs fixed-45 upgrade
progress and merges whole material trait levels before applying the float
partial-level correction. This follows pinned
[`IncrementalModifierEntry.merge` and `scaleLevel`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/IncrementalModifierEntry.java),
whose effective level feeds the
[Haste attribute formula](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/generated/resources/data/tconstruct/tinkering/modifiers/haste.json).
Unrepresentable merged integer levels now provide no Haste bonus. Existing
negative/malformed progress handling and durability-limit gating are retained.

This slice corrects only effective-level rounding. The existing single-chest
float multiplier and final float speed multiplication remain unchanged; they
do not emulate the source double attribute evaluation or its 0-to-100 clamp.
Other equipment slots and source attribute stacking remain outside this
change. End-to-end bit-exact source speed is not claimed.

The cached-library integration harness executes the actual chest helper with
four old-versus-new bitwise counterexamples, complete/partial level boundaries,
negative/malformed/large inputs, overflow, and durability gates. A real wire
encode/decode/matching test supplies the trait level to the same helper and
checks local/remote arithmetic, stale-frame fallback, and repair. This is a
boundary integration test, not a LAN or full actor-context simulation. No
full generated-host build, gameplay, LAN, or Windows validation was run.

## Chest Haste broken-flag guard

Chest Haste now contributes no multiplier when saved `tic_broken` is a nonzero
Byte, even if damage is still below the saved maximum. This matches the
`!tool.isBroken()` guard around armor modifier attributes in pinned
[`ModifiableArmorItem.getAttributeModifiers`](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/tools/item/armor/ModifiableArmorItem.java)
and uses the same flag representation as the existing retained-tool broken
predicate. Clearing or removing that flag restores the prior calculation only
when the existing maximum/damage checks also permit it. Wrong-typed flag values
retain the existing unflagged behavior. Neither the stack nor snapshots change.

The actual-helper integration tests cover positive/negative nonzero Bytes,
zero/absent/wrong-typed flags, partial upgrade plus trait and upgrade-only
cases, maximum/damage gates, and non-mutation. A real wire roundtrip verifies
that flag-only breakage suppresses both trait and upgrade bonuses and that
repair invalidates the old matching frame before a fresh frame restores the
bonus. Source/tag/actor lookup, incremental arithmetic, float output pipeline,
and chest-only scope are unchanged. No full generated-host build, gameplay,
LAN, or Windows validation was run.

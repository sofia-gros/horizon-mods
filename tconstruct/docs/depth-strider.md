# Depth Strider boot upgrade

This #379 slice enables the original three-input recipe: two fish and one
Prismarine Bricks block in three occupied input slots, in any order. It works at
a Tinker Station or Anvil. The current world's source fish tag supplies accepted fish;
the original source recipe is retained rather than replaced by a fixed copy.

Travelers, Plate and Slime Boots admitted by the live boots tag can acquire
Depth Strider I–III, spending one upgrade slot per level. Modifier crystals add
one level and the existing extraction flow removes one complete level. The
planner rejects malformed saved progress and unsupported source level/slot
contracts. Item and workstation text includes the upgrade.

## Native underwater movement

The source modifier adds one native Depth Strider enchantment level per modifier
level. The bridge reads direct material traits and independently saved upgrades,
then uses the native enchantment's own attribute formula to compute the required
`water_movement_efficiency` contribution. It subtracts an already-present native
Depth Strider contribution, avoiding double counting, and caps the combined
enchantment level at three.

The actual host water movement code consumes this attribute, halves its effect
off the ground and retains the native Dolphin's Grace behavior. No enchantment
is saved into the tool and no custom movement or packet algorithm is added.
This delegates movement arithmetic to the host; it is not a claim of bit-exact
Java floating-point arithmetic.

Only an intact supported tool in the actual feet slot qualifies. A complete
world snapshot, the live boots tag and the stock source modifier definition are
required. The native enchantment must expose a single finite additive water
movement attribute in the feet slot; unsupported definitions produce no bonus.
Existing native enchantments above level three are outside this bounded bridge.

The transient modifier is refreshed on equipment changes, source reload and
the end of each server player's tick. Guest actor swaps select the guest's own
boots and attributes. Remote clients do not add a second projection to the
server-synchronized attribute. Missing context, removal and broken equipment
remove the owned bonus at the next refresh, leaving unrelated attributes alone.

## Review and limits

A brief static review covered the source recipe, native attribute consumer,
level/slot costs, extraction, double-count prevention, actor selection and
refresh/removal. A focused planner regression was added but not run. No tests,
Mixins transformation, build, CI, version checks, save/reopen or gameplay/LAN
validation were performed, as requested. Keep this draft for the final batch.

Other armor upgrades, general enchantment projection, altered source definitions
and existing material-armor wear limitations are outside this change. This is
independent of the separate Respiration work.

Specification: [Tinkers' Construct f4e77d43, EnchantmentModule.Constant](https://github.com/SlimeKnights/TinkersConstruct/blob/f4e77d43f39ba1047bf93ae816b7e1beaab514c2/src/main/java/slimeknights/tconstruct/library/modifiers/modules/build/EnchantmentModule.java),
plus the retained `tools/modifiers/upgrade/depth_strider.json` and
`tinkering/modifiers/depth_strider.json` source resources.

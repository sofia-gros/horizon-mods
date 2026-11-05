# Overslime friend combinations in ordinary armor wear

Partial high-priority #379 armor/Overslime integration, stacked on #934. The
retained armor path formerly rejected every positive modifier not in its bounded
allowlist. Consequently, even an otherwise valid Overslime armor stack carrying
Overforced or a recovery trait never reached absorption or durability writeback.

## Coverage

The no-wear definition guard now accepts six additional complete canonical
source definitions:

- Overforced: Overslime capacity stat addition, including the existing saved
  quarter-level upgrade representation. This change does not reinterpret levels.
- Overcast: Overslime capacity addition and base multiplier.
- Overlord: durability-to-capacity stat copy and the source durability reduction.
- Overgrowth: the inventory-tick refill module.
- Overburn: inventory-tick fuel/refill, tank-capacity stat and its fixed Tank
  Handler child trait.
- Overslime Friend: the moduleless source marker.

These definitions have no ordinary TOOL_DAMAGE or DURABILITY_CHANGED hook.
Complete JSON equality includes module order, explicit hook overrides, priority,
conditions and child traits. An added, missing or changed field remains rejected;
this is not a generic allowlist of module type names.

Two additional static source registrations are recognized before dynamic JSON
lookup: Overworked is a plain Modifier with an empty hook map; Tank Handler wraps
TankModule, whose hooks cover fluid capability, volatile data, validation,
removal, slot interaction and display name. Neither modifies ordinary durability
loss or observes durability changes. Source static registrations cannot be
replaced by a same-ID dynamic JSON file. Overslime itself remains separately
implemented at priority150; it is not classified as a no-wear modifier.

All newly admitted combinations still require every other active modifier to
pass the existing guard. They use the unchanged Tanned200 → Solar185 →
Overslime150 → Unbreakable125 → insertion-ordered Dense/Reinforced100 pipeline,
exact-snapshot pairing and per-event commit. Stored capacity and shield validity,
actor, stock-slot, native-enchantment and broken-state checks are unchanged.

## Limits and integration

This restores wear eligibility only. It does not recalculate capacity, repair
stale saved stats, grant friends, acquire materials, add arbitrary modifier-trait
expansion or implement tank slot actions. Overgrowth #706 and Overburn #715 own
inventory recovery and are separate integration prerequisites for those effects;
this branch does not duplicate their implementations. Fuel/tank/persistent data
is not touched by this change. Existing refill, capacity and friend calculations
remain as they were.

Overwield, Overshield, Barkskin and other unreviewed modifiers still reject the
ordinary armor path. In particular this does not implement Overshield health
protection. The earlier deferred-writeback, callback visibility, break-animation
and batched health-ordering limits still apply. No native change or
provenance-blocked fluid implementation is included. The base defense chain
still requires native signed-protection 09f0179db and upper-bound 5e47547e2.
Gameplay, save/reload and dedicated/guest/LAN remain unverified.

## Sources and review

TConstruct f4e77d43f39ba1047bf93ae816b7e1beaab514c2 (MIT): the six generated modifier
JSON files, TinkerModifiers static registrations, Modifier.registerHooks,
StatBoostModule, StatCopyModule, ModifierTraitModule, OvergrowthModule,
OverburnModule and TankModule. Retained JSON uses the existing full license
notice at `tconstruct/third_party/tconstruct/LICENSE`.

Brief source/API and whitespace review only. Three regression cases cover exact
canonical definitions, changed hooks/child traits and the narrow static ID set;
they were added but not run. Builds/planner/tests belong to the separate batch
integration pass. No CI inspection or merge was performed for this slice.

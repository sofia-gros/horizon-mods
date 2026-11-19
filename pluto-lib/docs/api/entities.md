# Entity types and events

Declare types derived from existing Minecraft implementations, and listen for entities entering/leaving worlds, damage and death. Type declarations currently have a native-loader stub; the events below have their own support status.

## Basic Example

Hear when an entity enters or leaves a world:

```rust
use pluto_lib::ServerEntityEvents;

ServerEntityEvents::ENTITY_LOAD.register(|entity| {
    if entity.kind == "minecraft:zombie" {
        println!("a zombie appeared in {}", entity.dimension);
    }
});
```

## Preparing and Completing Mob Spawns

`MobSpawnCallback` separates pre-initialization selection from post-initialization work. The preparation sees spawn metadata and the mob's RNG; return an owned `MobSpawnAction` to run after inherited equipment, enchantment, seasonal and class-tail work. Keep resource snapshots in that action rather than reading newer globals later. Returning `None` discards no RNG draws already consumed.

```rust
use pluto_lib::{Identifier, MobSpawnAction, MobSpawnCallback, MobSpawnCompletion};
MobSpawnCallback::register(Identifier::parse("example:spawn_observer").unwrap(), |before| {
    let original_type = before.info().entity_type.clone();
    Ok(Some(Box::new(move |after: &mut dyn MobSpawnCompletion| {
        assert_eq!(after.info().entity_type, original_type);
        let _final_main_hand = after.equipment("mainhand")?;
        Ok(())
    }) as MobSpawnAction))
}).unwrap();
```

> **Use with Caution.** Dispatch/owned-action/RNG order is pure-tested. The runtime adapter now targets the shared native spawn entry, retaining level/created/fixed yaw selection, individual RNG and UUID registration before join; a production-helper trace verifies these rules and no-finalization/disabled fallback. This changed attachment is not yet native-build/game verified. Runtime/natural, normal/trial-spawner, patrol/raid, marked-structure and finalizing frog-conversion callers, native provider forwarding and Mob-owned transient state are source-declared, not native plan/type/build/game validated. Full caller review remains pending; only the pinned frog conversion has the later finalize call. Other conversions must not select gear. A narrow `spawning::finalize` or entity LOAD hook is not this complete lifecycle. A prepared action is consumed once and dropped on abort/error; Mob render/transport clones discard it. Native source defers join/publication only for selected completion work, and runs it after inherited jockey tails; this changes join timing for those selected mobs and requires game/other-callback validation. Unselected mobs retain native join ordering. Inherited child/world effects are not rolled back after completion failure. Spawn reasons are backend labels (including `natural`, `runtime`, `spawner`, `trial_spawner`, `patrol`, `event`, `structure` and `conversion`), not guaranteed Forge reason enums. Gear is a reduced `StackData` view, not every native component. Named main/off hand/head/chest/legs/feet slots are supported in source; offhand data/count use native gear storage and its save path rather than plain identity alone. Unsupported slots reject. Native offhand render/sync/consumer validation remains pending. Mob RNG and the world's enchantment RNG remain separate. Callback registration order is execution order; newly registered callbacks affect subsequent spawns, never an existing prepared action.

## Cancelling Damage

`ALLOW_DAMAGE` is asked before damage is applied. Answer `false` to cancel it; one `false` among all the callbacks is enough:

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|damage: &LivingDamage| damage.damage_type != "minecraft:fall");
```

`attacker` and `direct` are entity ids, negative when there is none: a skeleton's arrow has the skeleton as `attacker` and the arrow as `direct`.

> **Tip.** `AFTER_DEATH` is called when a living entity's death goes ahead, just before it is removed (Horizon's death hook can still cancel it for other mods). `attacker` is the killer's id, negative when there is none; `direct` is always negative and `amount` 0 here. Players are not reported yet.

## Changing Knockback and Item Wear

`ServerLivingEntityEvents::MODIFY_KNOCKBACK` (NeoForge's `LivingKnockBackEvent`) is heard on a server for each knockback: the `entity` pushed and its `kind`, the `attacker` entity id (negative for none), the stacks in the attacker's `attacker_main` and `attacker_off` hands (damage, max damage and custom data; `None` for an empty hand) and the `strength`, `x` and `z` the game worked out. A callback changes them with `Cell::set`, or sets `cancel` for no knockback. Every callback is called in turn and sees the last one's values.

`ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE` is heard on a server when a player's item is about to lose durability (vanilla `ItemStack.hurtAndBreak`, after Unbreaking and not for a creative player or a LAN host). It carries the player's `player` uuid and `name`, the `item` id, the `stack` as the player holds or wears it (main hand first, then feet, legs, chest and head; `None` when no stack of that item is found) and the `amount`. Change the wear with `amount.set` (0 or less leaves the item whole) or set `cancel` for none.

```rust
use pluto_lib::{ItemDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear: &ItemDamage| {
    if wear.stack.as_ref().is_some_and(|stack| stack.custom_data.contains_key("example_mending")) {
        wear.amount.set(wear.amount.get() / 2);
    }
});
```

> **Use with Caution.** Both events ride the host's own hooks and run only in the game that owns the world. The item wear event names the item but not the slot, so a player carrying two stacks of one item may see the other stack's data.

## Derived Entity Types

`EntityTypeRegistry::register_derived` declares a new type ID that reuses a vanilla entity implementation. Choose the actual base class: a different render model does not justify changing simulation to another mob. Width/height are the default (size-one) runtime dimensions in blocks, and the tracking range is in chunks. If a source class scales its EntityType builder dimensions (for example Slime's 0.255 factor), apply that class conversion before registration; do not copy the unscaled builder box.

```rust
use pluto_lib::{EntityTypeRegistry, Identifier};
EntityTypeRegistry::register_derived(
    Identifier::parse("example:cloud_slime").unwrap(),
    Identifier::parse("minecraft:slime").unwrap(),
    2.04, 2.04, 20, true,
).unwrap();
```

> **Use with Caution.** This is not a Rust entity factory or a spawn command. The source adapter appends derived rows to entity-types.json while preserving inherited raw float fields, allocates matching network registry IDs and publishes command lists. Living factories also inherit their separate attributes.json DefaultAttributes suppliers, retaining exact bit strings without creating fake suppliers for nonliving factories. Both catalogs resolve before either is passed to the writer; missing/invalid living suppliers reject the candidate. Native JSON_TABLES does not visit attributes.json, so the entity-table write explicitly sends its prepared companion through the native unlink-before-write operation. Cache identity has changed to prevent reuse of older roots without suppliers; native loader/overlay validation remains pending. The complete declaration snapshot participates in content-root cache identity. Category and unspecified defaults come from the named factory, whose exported row must exist; existing type IDs cannot be replaced. Native execution/model validation, spawn rules, custom attributes, persistent/synchronized state and simulation callbacks remain incomplete or unverified. Tracking/velocity requests are explicit metadata, not yet consumed by a native tracking-policy adapter. The fixed content-file whitelist is bypassed by this source export path, not edited or evidence that runtime parity is complete. Invalid values and duplicate/late registrations return errors; no unrelated template is substituted.

## Entity-Class Query Methods

Use `LivingEntityBehavior` for per-type `getJumpPower()`, `calculateFallDamage()` and contact-attack `getAttackDamage()` overrides. This is class behaviour, not a post-jump notification or a damage event. Return `None` to retain inheritance; `Some(0.0)` explicitly overrides it. Registration does not create or replace the entity type.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior, LivingEntityBehaviors, JumpPowerContext};
let kind = Identifier::parse("example:spring_cube").unwrap();
LivingEntityBehaviors::register(LivingEntityBehavior::new(kind.clone())
    .jump_power(|c| Some(c.block_jump_factor * 0.5))
    .calculate_fall_damage(|_| Some(0))
    .attack_damage(|c| Some(c.attribute as f32 + 2.0))).unwrap();
let query = JumpPowerContext { entity: 7, entity_type: kind, slime_size: Some(4), block_jump_factor: 0.5 };
assert_eq!(LivingEntityBehaviors::jump_power(&query), Some(0.25));
```

> **Use with Caution.** The native no-argument jump query and cube contact-damage query are source-wired only; their signatures/sites still await native attachment plan/native validation. The jump factor is the native block factor and `slime_size` is a subtype capability, not a generic scale; neither an attribute nor Jump Boost is added after an explicit override. Contact attack receives the effective double attribute before the class bonus. This is not a general melee-damage hook. Fall calculation has a separate source-declared LOAD after the living-fall event and class math, before block effects; cancelled/fall-immune paths do not reach it. `Some(0)` replaces only the calculation, not the surrounding lifecycle or passenger propagation. Pinned scope analysis ignores compound-write places and distinguishes the later shadowed damage local; the actual LOAD/site/types still need native plan validation. Other entity methods, client/server synchronization and custom factories remain separate. Callbacks run outside the registry lock, duplicates reject without replacement, and absent/poisoned registration falls back to inheritance.

## Transient Class Attributes

A class can declare server-side attribute modifiers with `transient_attribute_modifier`. These are instance modifiers, not multiplied final damage/armor values or permanent saved modifiers. Use `AddMultipliedTotal` with amount `3.0` for a fourfold multiplier; native equipment contributions and effective-value limits remain in the attribute pipeline.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior, LivingEntityBehaviors,
    TransientAttributeModifier, AttributeModifierOperation};
let kind = Identifier::parse("example:armored_class").unwrap();
LivingEntityBehaviors::register(LivingEntityBehavior::new(kind.clone())
    .transient_attribute_modifier(TransientAttributeModifier {
        attribute: Identifier::parse("minecraft:armor").unwrap(),
        id: Identifier::parse("example:class_armor").unwrap(), amount: 3.0,
        operation: AttributeModifierOperation::AddMultipliedTotal,
    })).unwrap();
assert_eq!(LivingEntityBehaviors::transient_attribute_modifiers(&kind).unwrap()[0].amount, 3.0);
```

> **Use with Caution.** The source backend supports armor, armor toughness and knockback resistance for native Mob instances only. Other consumers explicitly reject instead of inventing a working effect. A server-only pre-join attachment applies defaults before other join callbacks to both fresh/restored mobs, but is **not** a general constructor callback: pre-finalization timing remains different/unverified. Missing attributes are skipped, missing suppliers/services reject, and existing same-ID modifiers are retained. Source uses native transient instance sets, never permanent insertion. Armor/toughness stay in the native equipment-aware effective calculation. Scalar resistance uses a conditional entry hook: without additional class operations it retains the original function; otherwise its pinned-source calculation inserts class operations after permanent modifiers and before native transient/effect/extra operations, retaining native arithmetic order, NaN handling and clamping. It preserves permanent replacements and exposes has/amount/removal without restoring a removed class modifier. Unsupported initialization rejects the join rather than publishing a falsely initialized mob. Pure tests check declarations, actual adapter control/data flow and scalar helper arithmetic through storage facades, not native host execution, observers, cached gear interaction, save/reopen, packet synchronization, role/constructor/lifecycle timing or native types/sites. Those remain native attachment validation gates. Explicit native `set_base(Armor, value)` writes are source-mirrored into an already-owned equipment-aware instance without replacing modifiers; a newly created pre-join map imports an explicit earlier scalar armor base, while restored maps keep their own base. Missing maps/armor attributes are not manufactured by the setter. Load-time class callbacks can restore bases as described next; subsequent mod-controlled metal/size changes and arbitrary attribute mutation APIs remain separate.

## Restore Class Bases on Load

Use `attribute_bases_on_load` when saved class data determines an attribute base. The server callback runs before ordinary entity-load observers, after transient defaults, with the normal attachment-capable scope. It returns owned writes so no live mob borrow overlaps mod code. The source adapter also reruns the base callback after a successful in-place native `Entity.load` (`/data` and spawn-egg data), using the newly loaded size while preserving existing transient modifiers; it does not reinstall removed class modifiers. Native failures and non-mob targets retain their original result without a callback.

```rust
use pluto_lib::{AttributeBaseValue, Identifier, LivingEntityBehavior, LivingEntityBehaviors};
LivingEntityBehaviors::register(LivingEntityBehavior::new(Identifier::parse("example:load_bases").unwrap())
    .attribute_bases_on_load(|context| {
        let size = context.slime_size.ok_or("size unavailable")?;
        Ok(vec![AttributeBaseValue { attribute: Identifier::parse("minecraft:armor").unwrap(), value: f64::from(size) }])
    })).unwrap();
```

> **Use with Caution.** This source-only backend writes armor/toughness bases through their native instance observers, retaining modifiers. Other base consumers reject; missing entity attributes are skipped. Duplicate or unsupported entries reject the batch before any base write. The source adapter forwards the join event's dimension, rejects a mismatched current dimension before mutation and rechecks it after callback dispatch; an already-cancelled join skips initialization. A borrowed owner token also rejects a replacement mob with the same numeric ID/type. It uses the existing clone-shared native lifecycle identity without an entity-ID map or extra per-mob allocation; in-place NBT reload retains that identity through the native AI field. Callback attachment/world side effects are not rolled back on failure. Errors reject the join; an in-place reload reports failure after the native data change, without rolling that change back. This is load restoration, not constructor timing, a metal setter or interception of every direct size change. Native Host forwarding, application/order, save/sync and gameplay remain unverified behind the native attachment gate.

## Reference

### `AttributeBaseValue`

`AttributeBaseValue { attribute: Identifier, value: f64 }` is an owned explicit base write. Zero is a value, not a fallback.

```rust
use pluto_lib::{AttributeBaseValue, Identifier};
let base = AttributeBaseValue { attribute: Identifier::parse("minecraft:armor").unwrap(), value: 0.0 };
assert_eq!(base.value, 0.0);
```

### `LivingEntityLoadContext`

`LivingEntityLoadContext { entity: EntityInfo, slime_size: Option<i32> }` carries callback-local entity identity/dimension and a native cube/slime size, absent for other implementations. The entity's typed attachment methods use the current host scope; they are unavailable outside it.

```rust
use pluto_lib::{EntityInfo, LivingEntityLoadContext};
let context = LivingEntityLoadContext { entity: EntityInfo { id: 7, kind: "example:slime".into(), dimension: "minecraft:overworld".into() }, slime_size: Some(4) };
assert_eq!(context.slime_size, Some(4));
```

### `LivingEntityBehavior::attribute_bases_on_load`

`attribute_bases_on_load(self, callback: impl Fn(&LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String> + Send + Sync + 'static) -> Self` installs the class's load-time base callback. Empty output leaves bases unchanged. The callback executes outside registration locks and may read existing attachments or explicitly report failure.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let behavior = LivingEntityBehavior::new(Identifier::parse("example:load_policy").unwrap())
    .attribute_bases_on_load(|_| Ok(Vec::new()));
```

### `LivingEntityBehaviors::attribute_bases_on_load`

`attribute_bases_on_load(context: &LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String>` dispatches and validates duplicate attribute IDs. An unregistered class returns an empty vector. Calling this method alone does not create a host scope or apply writes to an entity.

```rust
use pluto_lib::{EntityInfo, LivingEntityLoadContext, LivingEntityBehaviors};
let context = LivingEntityLoadContext { entity: EntityInfo { id: 7, kind: "example:no_load_policy".into(), dimension: "minecraft:overworld".into() }, slime_size: None };
assert!(LivingEntityBehaviors::attribute_bases_on_load(&context).unwrap().is_empty());
```

### `AttributeModifierOperation`

The three Minecraft operations: `AddValue` adds to the base; `AddMultipliedBase` adds a proportion of that additive-phase base; `AddMultipliedTotal` multiplies the accumulated result by `1 + amount`, before effective-value sanitization. This enum describes operations, not a standalone calculator.

```rust
use pluto_lib::AttributeModifierOperation;
let operation = AttributeModifierOperation::AddMultipliedTotal;
assert_ne!(operation, AttributeModifierOperation::AddMultipliedBase);
assert_ne!(operation, AttributeModifierOperation::AddValue);
```

### `TransientAttributeModifier`

Owned declaration fields: `attribute: Identifier`, modifier `id: Identifier`, `amount: f64` and `operation: AttributeModifierOperation`. IDs are native identifier keys (not the source era's display-name/random-UUID representation); declarations contain no live entity handles or instances.

```rust
use pluto_lib::{Identifier, TransientAttributeModifier, AttributeModifierOperation};
let modifier = TransientAttributeModifier { attribute: Identifier::parse("minecraft:armor_toughness").unwrap(),
    id: Identifier::parse("example:toughness").unwrap(), amount: 3.0,
    operation: AttributeModifierOperation::AddMultipliedTotal };
assert_eq!(modifier.clone(), modifier);
```

### `LivingEntityBehavior::transient_attribute_modifier`

`transient_attribute_modifier(self, modifier: TransientAttributeModifier) -> Self` adds a class declaration. Registration rejects duplicate attribute/ID pairs; the same modifier ID on different attributes is permitted. Register one complete behavior per type.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior, TransientAttributeModifier, AttributeModifierOperation};
let _behavior = LivingEntityBehavior::new(Identifier::parse("example:resistant").unwrap())
    .transient_attribute_modifier(TransientAttributeModifier {
        attribute: Identifier::parse("minecraft:knockback_resistance").unwrap(),
        id: Identifier::parse("example:resistance").unwrap(), amount: 3.0,
        operation: AttributeModifierOperation::AddMultipliedTotal,
    });
```

### `LivingEntityBehaviors::transient_attribute_modifiers`

`transient_attribute_modifiers(entity_type: &Identifier) -> Result<Vec<TransientAttributeModifier>, String>` returns an owned declaration snapshot, empty for an unregistered type. Poisoned registration returns an error, not an empty policy; mutating the returned vector does not alter registrations or a live mob.

```rust
use pluto_lib::{Identifier, LivingEntityBehaviors};
assert!(LivingEntityBehaviors::transient_attribute_modifiers(
    &Identifier::parse("example:no_attribute_policy").unwrap()).unwrap().is_empty());
```

### `CalculateFallDamageContext`

Owned class-query data: `entity: u32`, `entity_type: Identifier`, post-event `distance: f64`, `multiplier: f32` and inherited native calculation `inherited: f64`. This calculation is before block-specific alterations; the host's result representation is not a promise of the original Java's baseline rounding.

```rust
use pluto_lib::{Identifier, CalculateFallDamageContext};
let query = CalculateFallDamageContext { entity: 1, entity_type: Identifier::parse("example:fall").unwrap(),
    distance: 10.0, multiplier: 0.5, inherited: 3.0 };
assert_eq!(query.inherited, 3.0);
```

### `LivingEntityBehavior::calculate_fall_damage`

`calculate_fall_damage(self, callback: impl Fn(&CalculateFallDamageContext) -> Option<i32> + Send + Sync + 'static) -> Self` configures the class calculation. The integer result follows the Minecraft method's shape. `None` delegates, while zero means no calculated damage, not an event cancellation.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let _behavior = LivingEntityBehavior::new(Identifier::parse("example:no_fall_damage").unwrap())
    .calculate_fall_damage(|_| Some(0));
```

### `LivingEntityBehaviors::calculate_fall_damage`

`calculate_fall_damage(context: &CalculateFallDamageContext) -> Option<i32>` dispatches the registered class calculation outside the lock. It applies neither block effects nor damage itself.

```rust
use pluto_lib::{Identifier, CalculateFallDamageContext, LivingEntityBehaviors};
let query = CalculateFallDamageContext { entity: 1, entity_type: Identifier::parse("example:missing_fall").unwrap(),
    distance: 10.0, multiplier: 1.0, inherited: 7.0 };
assert_eq!(LivingEntityBehaviors::calculate_fall_damage(&query), None);
```

### `JumpPowerContext`

Owned query data: `entity: u32`, `entity_type: Identifier`, `slime_size: Option<i32>` and `block_jump_factor: f32`. A missing subtype size must not be guessed as one.

```rust
use pluto_lib::{Identifier, JumpPowerContext};
let query = JumpPowerContext { entity: 3, entity_type: Identifier::parse("example:cube").unwrap(),
    slime_size: Some(9), block_jump_factor: 0.5 };
assert_eq!(query.slime_size, Some(9));
```

### `AttackDamageContext`

Owned contact-query data: `entity: u32`, `entity_type: Identifier`, `attribute: f64`. `attribute` is the effective attribute, without an inherited class bonus or a prior conversion to float.

```rust
use pluto_lib::{Identifier, AttackDamageContext};
let query = AttackDamageContext { entity: 3, entity_type: Identifier::parse("example:cube").unwrap(), attribute: 3.0 };
assert_eq!(query.attribute, 3.0);
```

### `LivingEntityBehavior`

A cloneable set of per-type class methods; the builder retains thread-safe callbacks, not entities or world handles.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let behavior = LivingEntityBehavior::new(Identifier::parse("example:behavior").unwrap());
let _owned_copy = behavior.clone();
```

### `LivingEntityBehavior::new`

`new(entity_type: Identifier) -> Self` starts with inherited methods.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let _behavior = LivingEntityBehavior::new(Identifier::parse("example:inherited").unwrap());
```

### `LivingEntityBehavior::jump_power`

`jump_power(self, callback: impl Fn(&JumpPowerContext) -> Option<f32> + Send + Sync + 'static) -> Self` configures the no-argument jump-power override. `None` delegates; zero is explicit.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let _behavior = LivingEntityBehavior::new(Identifier::parse("example:jump").unwrap())
    .jump_power(|c| c.slime_size.map(|n| (n as f64).sqrt() as f32 * c.block_jump_factor / 2.0));
```

### `LivingEntityBehavior::attack_damage`

`attack_damage(self, callback: impl Fn(&AttackDamageContext) -> Option<f32> + Send + Sync + 'static) -> Self` configures contact attack damage, independently of jump power.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let _behavior = LivingEntityBehavior::new(Identifier::parse("example:contact").unwrap())
    .attack_damage(|c| Some(c.attribute as f32 + 2.0));
```

### `LivingEntityBehaviors`

Process-local registry and dispatch for class queries, keyed by symbolic entity type rather than runtime entity ID. No serialization or entity ownership is provided.

```rust
use pluto_lib::{Identifier, AttackDamageContext, LivingEntityBehaviors};
let query = AttackDamageContext { entity: 4, entity_type: Identifier::parse("example:absent").unwrap(), attribute: 3.0 };
assert_eq!(LivingEntityBehaviors::attack_damage(&query), None);
```

### `LivingEntityBehaviors::register`

`register(behavior: LivingEntityBehavior) -> Result<(), String>` rejects duplicate type IDs, duplicate class attribute/ID pairs and poisoned registration without changing the old entry. Callbacks may register another type during dispatch, without changing the selected callback.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior, LivingEntityBehaviors};
let id = Identifier::parse("example:once").unwrap();
LivingEntityBehaviors::register(LivingEntityBehavior::new(id.clone())).unwrap();
assert!(LivingEntityBehaviors::register(LivingEntityBehavior::new(id)).is_err());
```

### `LivingEntityBehaviors::jump_power`

`jump_power(context: &JumpPowerContext) -> Option<f32>` invokes the registered method or returns `None` for inheritance. Native callers must honor the difference between `None` and zero.

```rust
use pluto_lib::{Identifier, JumpPowerContext, LivingEntityBehaviors};
let query = JumpPowerContext { entity: 1, entity_type: Identifier::parse("example:missing_jump").unwrap(),
    slime_size: None, block_jump_factor: 1.0 };
assert_eq!(LivingEntityBehaviors::jump_power(&query), None);
```

### `LivingEntityBehaviors::attack_damage`

`attack_damage(context: &AttackDamageContext) -> Option<f32>` invokes the registered contact method or delegates. It does not apply armor, combat events, cooldowns, hit effects or damage itself.

```rust
use pluto_lib::{Identifier, AttackDamageContext, LivingEntityBehaviors};
let query = AttackDamageContext { entity: 1, entity_type: Identifier::parse("example:missing_contact").unwrap(), attribute: 5.0 };
assert_eq!(LivingEntityBehaviors::attack_damage(&query), None);
```

### `MobSpawnInfo`

Owned spawn origin metadata: `entity_type: Identifier`, `dimension: Identifier`, `position: [f64; 3]` and backend `reason: String`. No temporary numeric entity ID is a handle here. The native pending work retains this origin metadata for completion. Native source stores pending work in the Mob's owned Navigation, whose private fields and concrete Default constructor avoid the pinned validator's prohibition on adding state to externally constructible Mob. Spawn tails retain it; navigation loans occur only during later ticks. This source route still needs native attachment plan/native validation. The source adapter also checks a Mob-owned identity token before writing equipment: a replacement world/entity with the same numeric ID cannot receive the old work. Consumed or aborted native state cannot be prepared again; render/network clones start without it. Native attachment remains unvalidated.

```rust
use pluto_lib::{Identifier, MobSpawnInfo};
let info = MobSpawnInfo { entity_type: Identifier::parse("minecraft:zombie").unwrap(),
    dimension: Identifier::parse("minecraft:overworld").unwrap(), position: [1.0, 64.0, 2.0], reason: "natural".into() };
assert_eq!(info.position[1], 64.0);
assert_eq!(info.reason, "natural");
```

### `MobSpawnPreparation`

Borrowed pre-initialization view: spawn info, the mob RNG and owned gear reads. It deliberately has no gear writer. Implementations are backend-provided; retain copies/resource leases, never this borrow.

```rust
use pluto_lib::MobSpawnPreparation;
fn origin(view: &dyn MobSpawnPreparation) -> [f64; 3] { view.info().position }
```

### Method `MobSpawnPreparation::info`

`fn info(&self) -> &MobSpawnInfo`. Returns the spawn origin metadata, not an already published entity handle.

```rust
use pluto_lib::MobSpawnPreparation;
fn kind(view: &dyn MobSpawnPreparation) -> String { view.info().entity_type.to_string() }
```

### Method `MobSpawnPreparation::next_float`

`fn next_float(&mut self) -> f32`. One draw from the mob's stream in `[0, 1)`. It is not the world's enchantment RNG. Draws remain consumed even when no completion is selected.

```rust
use pluto_lib::MobSpawnPreparation;
fn selected(view: &mut dyn MobSpawnPreparation) -> bool { view.next_float() < 0.05 }
```

### Method `MobSpawnPreparation::next_double`

`fn next_double(&mut self) -> f64`. One double draw from the same mob RNG stream in `[0, 1)`.

```rust
use pluto_lib::MobSpawnPreparation;
fn choice(view: &mut dyn MobSpawnPreparation) -> f64 { view.next_double() }
```

### Method `MobSpawnPreparation::next_int`

`fn next_int(&mut self, bound: i32) -> Result<i32, String>`. One bounded mob-stream draw in `[0, bound)`; the bound must be positive. Unsupported/invalid requests return an error, not a guessed random value.

```rust
use pluto_lib::MobSpawnPreparation;
fn index(view: &mut dyn MobSpawnPreparation) -> Result<i32, String> { view.next_int(4) }
```

### Method `MobSpawnPreparation::equipment`

`fn equipment(&self, slot: &str) -> Result<Option<StackData>, String>`. Returns an owned reduced stack, `Ok(None)` for an empty slot or an error for unavailable/unsupported access. Native source accepts `mainhand`, `offhand`, `head`, `chest`, `legs` and `feet`.

```rust
use pluto_lib::{MobSpawnPreparation, StackData};
fn held(view: &dyn MobSpawnPreparation) -> Result<Option<StackData>, String> { view.equipment("mainhand") }
```

### `MobSpawnCompletion`

Post-inherited view extending `MobSpawnPreparation` with gear writes. Backends call it only after the whole inherited lifecycle; invoking a completion must not re-run finalization.

```rust
use pluto_lib::MobSpawnCompletion;
fn completed_type(view: &dyn MobSpawnCompletion) -> String { view.info().entity_type.to_string() }
```

### Method `MobSpawnCompletion::set_equipment`

`fn set_equipment(&mut self, slot: &str, stack: StackData) -> Result<(), String>`. Replaces a slot, not a component-preserving patch to every native component. Unsupported slots/data reject; source complex-offhand writes are unsupported. Native validation is pending. Errors stop subsequent completion work but do not roll back earlier side effects.

```rust
use pluto_lib::{MobSpawnCompletion, StackData};
fn replace(view: &mut dyn MobSpawnCompletion, stack: StackData) -> Result<(), String> {
    view.set_equipment("mainhand", stack)
}
```

### `MobSpawnAction`

`Box<dyn FnOnce(&mut dyn MobSpawnCompletion) -> Result<(), String> + Send + Sync>`. An owned once-only completion. Captures must be owned/static; the action is not cloned or saved and is dropped if its unpublished spawn is aborted.

```rust
use pluto_lib::MobSpawnAction;
let action: MobSpawnAction = Box::new(|after| { let _origin = after.info().position; Ok(()) });
drop(action); // abort: no completion runs
```

### `MobSpawnCallback`

Registers split-phase mob callbacks. No instance constructor; use `register`. Preparation runs in registration order, outside registry locks.

```rust
use pluto_lib::{Identifier, MobSpawnCallback};
MobSpawnCallback::register(Identifier::parse("example:no_completion").unwrap(), |_| Ok(None)).unwrap();
```

### `MobSpawnCallback::register`

`fn register(id: Identifier, prepare: impl Fn(&mut dyn MobSpawnPreparation) -> Result<Option<MobSpawnAction>, String> + Send + Sync + 'static) -> Result<(), String>`. Duplicate IDs fail without replacement. `None` means no pending completion, not undoing RNG. Runtime/natural, normal/trial-spawner, patrol/raid, marked-structure and finalizing frog-conversion callers are source-wired but full caller review/native validation remain pending. Selected conversion completes before join and skips the later inherited-finalize call; unselected conversions retain native ordering. NBT-loaded spawner mobs, ordinary saved mobs and NBT summons without the native finalize marker do not select again. Explicit spawner equipment still follows finalization and can override generated gear. The actual spawner-publication and converted-tail helpers pass trace-host ordering/abort/once-only tests; those do not validate native types or attachment. Registration alone does not make a game feature work.

```rust
use pluto_lib::{Identifier, MobSpawnCallback};
let id = Identifier::parse("example:once").unwrap();
MobSpawnCallback::register(id.clone(), |_| Ok(None)).unwrap();
assert!(MobSpawnCallback::register(id, |_| Ok(None)).is_err());
```

### `EntityTypeRegistry`

The registry for derived entity-type declarations. Register during initialization, before the native resource adapter collects its owned snapshot. An ID cannot be replaced by a later registration.

```rust
use pluto_lib::{EntityTypeRegistry, Identifier};
EntityTypeRegistry::register_derived(Identifier::parse("example:registry_slime").unwrap(),
    Identifier::parse("minecraft:slime").unwrap(), 1.0, 1.0, 8, true).unwrap();
```

### `EntityTypeRegistry::register_derived`

`fn register_derived(id: Identifier, factory: Identifier, width: f32, height: f32, tracking_chunks: u32, velocity_updates: bool) -> Result<(), String>`. `id` is a custom type, not a replacement in `minecraft`; `factory` names an existing vanilla runtime type (existence still needs native validation). Dimensions must be finite and positive; tracking must be `1..=i32::MAX` chunks. `velocity_updates` records whether the type requests velocity synchronization. The method rejects duplicate IDs, a poisoned registry and registrations after collection without changing earlier entries.

```rust
use pluto_lib::{EntityTypeRegistry, Identifier};
let id = Identifier::parse("example:derived_slime").unwrap();
let factory = Identifier::parse("minecraft:slime").unwrap();
assert!(EntityTypeRegistry::register_derived(id.clone(), factory.clone(), f32::NAN, 1.0, 8, true).is_err());
EntityTypeRegistry::register_derived(id.clone(), factory.clone(), 2.04, 2.04, 32, true).unwrap();
assert!(EntityTypeRegistry::register_derived(id, factory, 1.0, 1.0, 8, false).is_err());
```

## Data

### `EntityInfo`

An entity entering or leaving a world. `id` is the entity id, `kind` its type id such as `minecraft:zombie`, `dimension` the dimension id.

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_LOAD.register(|entity: &EntityInfo| {
    if entity.kind == "minecraft:zombie" && entity.dimension == "minecraft:overworld" {
        let _id = entity.id;
    }
});
ServerEntityEvents::ENTITY_LOAD.invoke(&EntityInfo { id: 5, kind: "minecraft:zombie".into(), dimension: "minecraft:overworld".into() });
```

### `LivingDamage`

Damage about to be applied to a living entity. `entity` is the victim's id and `kind` its type. `damage_type` is the damage type id such as `minecraft:player_attack`. `attacker` is the id of the entity that caused it (negative if none) and `direct` the entity that dealt it directly, a projectile for example (negative if none). `amount` is the damage.

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|damage: &LivingDamage| {
    // no fall damage for anyone
    damage.damage_type != "minecraft:fall"
});
let fall = LivingDamage {
    entity: 1, kind: "minecraft:player".into(), damage_type: "minecraft:fall".into(), attacker: -1, direct: -1, amount: 4.0,
};
assert!(!ServerLivingEntityEvents::ALLOW_DAMAGE.invoke(&fall));
```

## Events

### `ServerEntityEvents`

Entity load and unload events.

```rust
use pluto_lib::ServerEntityEvents;

ServerEntityEvents::ENTITY_LOAD.register(|_| {});
ServerEntityEvents::ENTITY_UNLOAD.register(|_| {});
```

### `ServerEntityEvents::ENTITY_LOAD`

An entity joined a world (spawned, or loaded with its chunk).

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_LOAD.register(|e| println!("{} {} loaded", e.kind, e.id));
ServerEntityEvents::ENTITY_LOAD.invoke(&EntityInfo { id: 9, kind: "minecraft:cow".into(), dimension: "minecraft:overworld".into() });
```

### `ServerEntityEvents::ENTITY_UNLOAD`

An entity left a world (died, was removed, or its chunk unloaded).

```rust
use pluto_lib::{EntityInfo, ServerEntityEvents};

ServerEntityEvents::ENTITY_UNLOAD.register(|e| println!("{} {} unloaded", e.kind, e.id));
ServerEntityEvents::ENTITY_UNLOAD.invoke(&EntityInfo { id: 9, kind: "minecraft:cow".into(), dimension: "minecraft:overworld".into() });
```

### `ServerLivingEntityEvents`

Events for living entities.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|_| true);
```

### `ServerLivingEntityEvents::ALLOW_DAMAGE`

Damage is about to be applied. Answer `false` to cancel it; any `false` cancels.

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|d| d.amount < 100.0);
let hit = LivingDamage { entity: 2, kind: "minecraft:cow".into(), damage_type: "minecraft:generic".into(), attacker: -1, direct: -1, amount: 3.0 };
assert!(ServerLivingEntityEvents::ALLOW_DAMAGE.invoke(&hit));
```

### `KnockbackModification`

Knockback about to be applied, for a callback to change or cancel. `entity` is the id of the entity pushed and `kind` its type; `attacker` is the id of what caused it, negative for none. `attacker_main` and `attacker_off` are the stacks in the attacker's hands, with damage, max damage and custom data; `None` for an empty hand. `strength`, `x` and `z` are the strength and the horizontal direction now (each a `Cell`: change it with `set`); `cancel` set to true gives no knockback.

```rust
use pluto_lib::KnockbackModification;
use std::cell::Cell;

let push = KnockbackModification { entity: 4, kind: "minecraft:zombie".into(), attacker: 1, attacker_main: None, attacker_off: None, strength: Cell::new(0.4), x: Cell::new(1.0), z: Cell::new(0.0), cancel: Cell::new(false) };
push.strength.set(push.strength.get() * 2.0);
assert_eq!((push.strength.get(), push.cancel.get()), (0.8, false));
```

### `ItemDamage`

A player's item about to lose durability, for a callback to change or cancel. `player` is the player's uuid and `name` their name; `item` is the id of the item worn and `stack` the stack as the player holds or wears it (`None` when none is found). `amount` is the wear now (a `Cell`: change it with `set`; 0 or less leaves the item whole); `cancel` set to true gives no wear; `slot` is the stack as an item slot, with its components; `replacement` is a whole new stack a callback may set (a `RefCell`), written to the main-inventory stack that carried the old data, after which the game wears it by 0. A stack that is not in the main inventory (worn armor) is not rewritten and the game wears it by `amount`, so a callback that sets `replacement` also sets `amount` to the wear left. `holder` is what the player was doing as the stack wore, as an [`EntityState`](#entitystate) (their health, height, light, biome temperature and effects as of that moment); `None` when the game did not name the player.

```rust
use pluto_lib::ItemDamage;
use std::cell::{Cell, RefCell};

let wear = ItemDamage { player: 1, name: "alex".into(), item: "example:pickaxe".into(), stack: None, slot: None, amount: Cell::new(3), cancel: Cell::new(false), replacement: RefCell::new(None), holder: None };
wear.amount.set(wear.amount.get() - 1);
assert_eq!((wear.amount.get(), wear.cancel.get(), wear.replacement.borrow().is_none()), (2, false, true));
```

### `PlayerTick`

A server player's tick: `player` is the uuid and `entity` the entity id, `armor` the pieces worn that carry custom data (as (slot, stack)), `effects` the effects on the player as (effect id, amplifier, ticks left), `remove_effects` the ids callbacks push to take off, and `attribute_additions` the (attribute id, amount) pairs callbacks push to add to the base value of the player's attributes for as long as they push them (amounts for one attribute add up; one no callback pushes any more is taken off again), and `actions` the [`EntityAction`](#entityaction)s callbacks push to be done to the ticking player (as `Attacker`; `Victim` is no one) at the end of the tick: a player the host can name by entity id is reached, any other is skipped. The tick is heard once more after the last armor piece with custom data comes off, so an amount tied to armor is let go. The host gives no transient modifier call, so the amount moves the saved base value: it is exact for an addition, taken off again when the player leaves (best effort), and a crash while it is applied leaves it in the save.

```rust
use pluto_lib::PlayerTick;
use std::cell::RefCell;

let tick = PlayerTick { player: 1, entity: 4, armor: Vec::new(), effects: vec![("minecraft:poison".into(), 0, 100)], remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
tick.remove_effects.borrow_mut().push("minecraft:poison".into());
tick.attribute_additions.borrow_mut().push(("minecraft:knockback_resistance".into(), 0.1));
assert_eq!((tick.effects.len(), tick.remove_effects.borrow().len()), (1, 1));
```

### `VisibilityModification`

How far a mob sees a player, for a callback to scale: `wearer` is the player's uuid, `observer` the type id of the mob that is looking, `armor` the pieces the player wore as of their last tick (as (slot, stack); only those carrying custom data), `armor_revision` a number that changes whenever what any player wears changes (never 0; keep what you worked out from `armor` while it stays the same) and `multiplier` the factor on the range now (a `Cell`: 1 leaves it whole, 0.5 halves it).

```rust
use pluto_lib::VisibilityModification;
use std::cell::Cell;

let look = VisibilityModification { wearer: 1, observer: "minecraft:skeleton".into(), armor: Vec::new(), armor_revision: 1, multiplier: Cell::new(1.0) };
look.multiplier.set(look.multiplier.get() * 0.5);
assert_eq!(look.multiplier.get(), 0.5);
```

### `CounterDamage`

Damage for a callback to deal to the mob that hurt the player: `damage_type` is the damage type id (`minecraft:thorns`) and `amount` the amount.

```rust
use pluto_lib::CounterDamage;

let damage = CounterDamage { damage_type: "minecraft:thorns".into(), amount: 2.5 };
assert_eq!(damage.amount, 2.5);
```

### `ActionTarget`

Which party of a hit an [`EntityAction`](#entityaction) is done to: `Attacker` or `Victim`. In a weapon hit the attacker is the player and the victim the mob; in a hit on a player (`WearerHit`) the attacker is the mob and the victim the wearer.

```rust
use pluto_lib::ActionTarget;

assert_ne!(ActionTarget::Attacker, ActionTarget::Victim);
```

### `KnockbackDirection`

How a knockback action picks the way the mob is pushed: `AwayFromOther` pushes it away from the other party of the hit, as the game's damage knockback does; `AgainstFacing` pushes it against the way it faces, as `Entity.knockback(strength, -sin(yaw), cos(yaw))` does.

```rust
use pluto_lib::KnockbackDirection;

assert_ne!(KnockbackDirection::AwayFromOther, KnockbackDirection::AgainstFacing);
```

### `EntityAction`

Something for a callback to do to one party of a hit, pushed onto the hit's `actions`. A weapon hit's actions are done just before its damage lands; those of a hit on a player on the player's next tick. Each action is guarded, and one the target cannot take is skipped without a word: `SetFire { target, seconds }` sets it burning for `seconds` when that is longer than it already burns (a fire-immune mob is skipped; a player is reached only when the host can reach them); `Heal { target, amount }` gives a mob or a player `amount` health, cut to their maximum; `Freeze { target, ticks }` freezes a mob (its frozen ticks become the larger of 140 and what it has, plus `ticks`, and its fire goes out; not a player, not a kind the game makes freeze-immune such as the stray); `Knockback { target, strength, direction }` pushes a mob along the horizontal plane (and up when it stands on the ground) as `LivingEntity.knockback` does, without the mob's own knockback resistance (a player is not pushed: the host has no call for it). `AddEffect { target, effect, amplifier, duration }` puts a status effect on a player or a mob (`amplifier` is the level less one, `duration` is in ticks; an unknown effect id, a negative amplifier or no duration gives nothing; the player who shot or sheared is reached by their uuid, the world owner included). `EditStackData { target, slot, item, edits }` applies the [`StackDataEdit`](#stackdataedit)s to the custom data of the stack the target player carries in [`slot`](#stackslot), but only when that stack is still of `item`; it is done to the stack as it is when the action lands (a weapon hit's before its damage and the hold item's wear, a hit on a player's on their next tick), so wear in between stays. It reaches players only (the acting player of the hit, the world owner included: the host names a player's stack by the player acting in the hook, and a hit on a player lands on that player's own tick); a mob, a slot holding another item and an edit that changes nothing are skipped. A guest's change reaches them through the host's inventory sync, which has not been watched in a game. `Hurt { target, amount, damage_type }` deals a player or a mob `amount` damage of `damage_type` with no attacker, through the host's hurt call: armor, effects, invulnerability time and the damage hooks all apply and hear it, and a creative player or a dead mob is skipped. A mod's own damage type id works once its data pack declares it (`data/<namespace>/damage_type/<name>.json`). `SetEffectDuration { target, effect, duration }` sets the ticks left of an effect a player has, keeping its level and flags (the host takes it off and gives it back, so it can shorten as well as lengthen); an effect the player lacks, a duration that is not positive and a mob are skipped.

```rust
use pluto_lib::{ActionTarget, EntityAction, KnockbackDirection, ServerLivingEntityEvents};

// An action for the ticking player: hurt them, and halve the time left on their poison.
ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
    if tick.armor.iter().any(|(_, piece)| piece.item == "example:cursed_helmet") {
        tick.actions.borrow_mut().push(EntityAction::Hurt { target: ActionTarget::Attacker, amount: 1.0, damage_type: "minecraft:magic".into() });
    }
    if let Some((effect, _, left)) = tick.effects.iter().find(|(effect, _, _)| effect == "minecraft:poison") {
        tick.actions.borrow_mut().push(EntityAction::SetEffectDuration { target: ActionTarget::Attacker, effect: effect.clone(), duration: (left / 2).max(1) });
    }
});

ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
    if hit.weapon.as_ref().is_some_and(|weapon| weapon.item == "example:flame_sword") {
        hit.actions.borrow_mut().push(EntityAction::SetFire { target: ActionTarget::Victim, seconds: 5 });
        hit.actions.borrow_mut().push(EntityAction::Knockback { target: ActionTarget::Victim, strength: 0.5, direction: KnockbackDirection::AwayFromOther });
    }
});
```

### `StackSlot`

Which stack of a player an `EntityAction::EditStackData` edits: `MainHand` (the selected hotbar slot), `OffHand`, `Feet`, `Legs`, `Chest` or `Head`.

```rust
use pluto_lib::{armor_stack_slot, StackSlot};

assert_eq!(armor_stack_slot("head"), Some(StackSlot::Head));
assert_ne!(StackSlot::MainHand, StackSlot::OffHand);
```

### `armor_stack_slot`

The [`StackSlot`](#stackslot) of an armor slot name that `armor` lists use (`feet`, `legs`, `chest`, `head`); None for any other name.

```rust
use pluto_lib::armor_stack_slot;

assert!(armor_stack_slot("chest").is_some() && armor_stack_slot("body").is_none());
```

### `StackDataEdit`

One change to a stack's custom data for `EntityAction::EditStackData`: `value` is put at `path` (compound keys from the root of the custom data, the compounds on the way made when missing, a value in the way replaced by a compound) or, when `value` is None, the entry at `path` is removed. Edits are applied in order and an empty path is skipped. Edits state the value they want, not a change: a bar that two hits fill in one tick is set by the second from what the first had not yet written.

```rust
use pluto_lib::{ActionTarget, EntityAction, Nbt, ServerLivingEntityEvents, StackDataEdit, StackSlot};

ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
    if hit.weapon.as_ref().is_some_and(|weapon| weapon.item == "example:charged_sword") {
        hit.actions.borrow_mut().push(EntityAction::EditStackData {
            target: ActionTarget::Attacker, slot: StackSlot::MainHand, item: "example:charged_sword".into(),
            edits: vec![StackDataEdit { path: vec!["example:charge".into()], value: Some(Nbt::Int(7)) }],
        });
    }
});
```

### `ServerEntityEvents::SHEAR`

A player is about to shear a mob (Bukkit: `PlayerShearEntityEvent`; Fabric has no counterpart), heard on a server before the shears act. Set `attempt.cancel` to stop the shearing, or push `EntityAction`s for the player (`ActionTarget::Attacker`) onto `attempt.actions`; they land even when a later callback cancels the shearing. `attempt.tool` is the stack in the shearing hand. The game reports only a guest's shearing on the world owner's LAN world; the owner's own shearing is heard by no hook.

```rust
use pluto_lib::{ActionTarget, EntityAction, ServerEntityEvents};

ServerEntityEvents::SHEAR.register(|attempt| {
    if attempt.tool.as_ref().is_some_and(|tool| tool.item == "example:cutters") {
        attempt.actions.borrow_mut().push(EntityAction::AddEffect {
            target: ActionTarget::Attacker, effect: "minecraft:speed".into(), amplifier: 0, duration: 100,
        });
    }
    if attempt.off_hand { attempt.cancel.set(true); }
});
```

### `ShearAttempt`

A player about to shear a mob. `player` is the player's uuid and `name` their name, `entity` the mob's entity id, `off_hand` whether the shears are in the off hand, `tool` the stack in that hand (its damage, max damage and custom data; `None` for an empty hand), `cancel` a `Cell<bool>` to set to stop the shearing and `actions` the `EntityAction`s to do to the player.

```rust
use pluto_lib::{ServerEntityEvents, ShearAttempt};

fn tool_item(attempt: &ShearAttempt) -> Option<String> {
    attempt.tool.as_ref().map(|tool| tool.item.clone())
}
ServerEntityEvents::SHEAR.register(|attempt| {
    let _ = (attempt.player, &attempt.name, attempt.entity, tool_item(attempt));
    attempt.cancel.set(false);
    assert!(attempt.actions.borrow().is_empty());
});
```

### `EffectGrant`

A status effect for a callback to put on the mob a weapon hit: `effect` is the effect id (an id the game does not know is skipped), `amplifier` the level less one (0 is level I) and `duration` how long it lasts, in ticks.

```rust
use pluto_lib::EffectGrant;

let grant = EffectGrant { effect: "minecraft:wither".into(), amplifier: 2, duration: 120 };
assert_eq!((grant.amplifier + 1, grant.duration), (3, 120));
```

### `WeaponHit`

A player's weapon hit on a mob: `entity` is the mob's id and `kind` its type id, `attacker` the player's entity id, `weapon` the stack used (its damage, max damage and custom data; None for an empty hand), `attacker_state` and `target_state` what the player and the mob were doing, `armor` the pieces the player wore as of their last tick (as (slot, stack): `feet`, `legs`, `chest`, `head`; only those carrying custom data; empty when the player is not known, and the local player of a single-player world counts only while they are the one wearer), `effects` the effects callbacks push for the mob, and `actions` the [`EntityAction`](#entityaction)s they push for the player (`Attacker`) or the mob (`Victim`), which the game does just before the hit's damage lands.

```rust
use pluto_lib::WeaponHit;
use std::cell::RefCell;

let hit = WeaponHit { entity: 4, kind: "minecraft:husk".into(), attacker: 1, weapon: None, attacker_state: None, target_state: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
assert!(hit.effects.borrow().is_empty() && hit.armor.is_empty() && hit.actions.borrow().is_empty());
```

### `WearerHit`

A mob hurting the player: `attacker` is the mob's id and `kind` its type id, `damage_type` the damage type id and `amount` the amount the game works the hit out as (both not reported yet: an empty id and 0.0), `attacker_state` what the mob was doing, `armor` the pieces the player wore (as (slot, stack): `feet`, `legs`, `chest`, `head`; only those carrying custom data) `effects` the effects callbacks push for the mob and `self_effects` the ones they push for the wearer and `counter_damage` the damage they push for the mob, and `actions` the [`EntityAction`](#entityaction)s they push for the mob (`Attacker`) or the wearer (`Victim`), which land on the player's next tick.

```rust
use pluto_lib::{EntityState, WearerHit};
use std::cell::RefCell;

let hit = WearerHit { attacker: 4, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
assert!(hit.armor.is_empty() && hit.effects.borrow().is_empty() && hit.self_effects.borrow().is_empty() && hit.counter_damage.borrow().is_empty() && hit.actions.borrow().is_empty());
```

### `EntityState`

What a living entity was doing when a player's weapon hit it, and the facts about it a condition may ask: `kind` is its entity type id; `sprinting` whether it was sprinting; `on_ground` whether it stood on the ground (false in the air); `in_water` whether it was in water; `fire_ticks` how long it burns for (0 or less when it does not); `fire_immune` whether fire cannot hurt it; `effects` the ids of the effects on it; `effect_levels` the same effects as (id, amplifier), the amplifier being the level less one (see [`EntityState::effect_level`](#entitystateeffect_level)); `health` and `max_health` its health points and its maximum (the `minecraft:max_health` attribute), `None` when not reported; `biome` the biome it stands in, `biome_temperature` that biome's temperature at its height (`Biome.getTemperature`), `None` for a biome PlutoLib has no temperature for; `eyes_in_water` whether its eyes are in water, where `in_water` is a touch of it; `rain_exposed` whether rain falls on it; `armor_coverage` the share of its four armor slots that hold armor, 0.0 to 1.0, `None` when not reported; `crouching` whether it crouches (a player holding sneak; a mob never does); `y` the height of its feet, `None` when not reported; `block_light` and `sky_light` the light of the two layers at the block it stands in, each 0 to 15 (`Level.getBrightness`), `None` when not reported; `sky_darken` how much the time of day and the weather take off the sky light, 0 to 11 (see [`sky_darken`](#sky_darken)). Every field has a default, so build one with `..EntityState::default()`.

```rust
use pluto_lib::EntityState;

let state = EntityState { kind: "minecraft:husk".into(), on_ground: true, fire_ticks: 40, effects: vec!["minecraft:poison".into()],
    health: Some(6.0), max_health: Some(20.0), biome_temperature: Some(2.0), rain_exposed: false, y: Some(64.0), sky_light: Some(15), sky_darken: 0, ..EntityState::default() };
assert!(state.fire_ticks > 0 && state.effects.contains(&"minecraft:poison".to_string()));
assert!(state.health.unwrap() < state.max_health.unwrap());
assert_eq!((state.crouching, state.y, state.block_light), (false, Some(64.0), None));
```

> **Use with Caution.** A mob's health, maximum and armor come from its saved data: the maximum is only known when the mob saved a `minecraft:max_health` attribute, and armor only when it saved equipment. A player's come from the server's player list and their equipment. Temperature is a table of the vanilla biomes with the game's cooling above y 80 (its noise term, at most about 0.01, and the frozen-biome noise are left out); a data pack's biome has `biome` but no temperature. `rain_exposed` is: raining, full sky light at the entity's block, and a biome that rains rather than snows or stays dry (an unknown biome is taken to rain). `eyes_in_water` reads the block at the eyes: water, a bubble column, kelp, seagrass or a waterlogged block. The state of a mob that hurt a wearer (`WearerHit::attacker_state`) carries only what the hurt report holds: type, ground, water, fire and effect ids. When a hit on a player wearing armor that carries custom data brings no target state, `DamageModification::target_state` is the player's as of their last tick, so their health, height and light are up to a tick old. A player's `crouching` is the server's sneak flag, and their `y` is the height of their feet; a mob's `y` is read from its position, and its light from the block it stands in. The light is read as of the moment the state is made, so a player's `sky_darken` follows the time of day and the weather of the world they are in; it is 0 in the Nether and the End, which have no weather, and the End and the Nether have no sky light to take it from.

### `EntityState::effect_level`

The level of an effect on the entity: the amplifier plus one, so 1 for level I, and 0 when it does not have the effect or the game did not report levels.

```rust
use pluto_lib::EntityState;

let state = EntityState { effect_levels: vec![("minecraft:poison".into(), 1)], ..EntityState::default() };
assert_eq!(state.effect_level("minecraft:poison"), 2);
assert_eq!(state.effect_level("minecraft:speed"), 0);
```


### `sky_darken`

`sky_darken(rain, thunder, day_time)` is `Level.getSkyDarken`: how much of the sky light the weather and the time of day take off, from 0 (clear noon) to 11 (midnight in a thunderstorm). `rain` and `thunder` are the levels, 0.0 to 1.0, and `day_time` the world's time of day in ticks (it may run past 24000). Subtract it from `EntityState::sky_light` for the light daylight leaves.

```rust
use pluto_lib::sky_darken;

assert_eq!(sky_darken(0.0, 0.0, 6000), 0);
assert_eq!(sky_darken(0.0, 0.0, 18000), 11);
assert!(sky_darken(1.0, 1.0, 6000) > sky_darken(1.0, 0.0, 6000));
```

### `EffectCategory`

A status effect's category, as `MobEffectCategory`: `Beneficial`, `Harmful` or `Neutral`.

```rust
use pluto_lib::{effect_category, EffectCategory};

assert_ne!(EffectCategory::Beneficial, EffectCategory::Harmful);
assert_eq!(effect_category("minecraft:speed"), Some(EffectCategory::Beneficial));
```

### `effect_category`

`effect_category(id)` is the category of a vanilla effect id (`minecraft:poison` is harmful), or `None` for an id that is not vanilla's: PlutoLib does not know another mod's effects. A vanilla effect is one milk can cure.

```rust
use pluto_lib::{effect_category, EffectCategory};

assert_eq!(effect_category("minecraft:poison"), Some(EffectCategory::Harmful));
assert_eq!(effect_category("example:unknown"), None);
```

### `DamageModification`

Damage about to be applied, for a callback to change. `damage` is what is known of it (its `amount` is the game's and does not change), `amount` the amount now (a `Cell`: change it with `set`) and `attacker_stack` the stack a player attacked with, with its damage, max damage and custom data; None when no player's attack caused it. `armor` lists what a player wore as of their last tick, as (slot, stack) for `feet`, `legs`, `chest` and `head` (only pieces carrying custom data, which is where a mod keeps its armor's state); it is empty for anything but a player. The event fires after vanilla's armor and enchantment protection. When a player's weapon hit a mob, `attacker_state` and `target_state` say what the player and the mob were doing as the hit landed (`None` otherwise); for a damaged player wearing armor that carries custom data, `target_state` is their state as of their last tick when the hit brought none. `actions` takes [`EntityAction`](#entityaction)s for the damaged player (`ActionTarget::Victim`), which land on their next tick; only a player whose armor carries custom data is served, so they are dropped for any other entity. Damage heard in the same tick all work from the armor as of the last tick, so an `EditStackData` one callback queues does not see the edit another queued a moment before.

```rust
use pluto_lib::{DamageModification, LivingDamage};
use std::cell::Cell;

let damage = LivingDamage { entity: 4, kind: "minecraft:zombie".into(), damage_type: "minecraft:player_attack".into(), attacker: 1, direct: 1, amount: 3.0 };
let hit = DamageModification { damage, amount: Cell::new(3.0), attacker_stack: None, armor: Vec::new(), attacker_state: None, target_state: None, actions: Default::default() };
hit.amount.set(hit.amount.get() + 2.0);
assert_eq!((hit.damage.amount, hit.amount.get()), (3.0, 5.0));
assert!(hit.attacker_stack.is_none());
```

### `ServerLivingEntityEvents::PLAYER_TICK`

A server player's tick, heard while they wear armor carrying custom data (a mod's own armor keeps its state there). It carries the armor and the effects on the player; take effects off them with `remove_effects`. It is how a mod grants immunity to an effect the game gives no call to refuse: the effect is gone by the next tick.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
    if tick.armor.iter().any(|(_, piece)| piece.item == "example:antidote_helmet") && tick.effects.iter().any(|(id, _, _)| id == "minecraft:poison") {
        tick.remove_effects.borrow_mut().push("minecraft:poison".into());
    }
});
```

### `ServerLivingEntityEvents::WEARER_HIT`

A mob hurting the player, heard on a server as the hit starts. It carries the mob's id, type and state, the damage type and amount, and the armor the player wore as of their last tick (only the pieces carrying custom data); push status effects on the mob with `effects`, and the game puts them on it, or on the wearer with `self_effects`, which land on the player's next tick, and deal damage to the mob with `counter_damage` (an armor's thorns), which lands on the next tick too as hurt by the player; set it on fire, freeze it or knock it back with `actions`, which land on the next tick as well. The hit is heard once for each attacker in a game tick, and only when the attacking mob is within a few blocks of a player. Only a single-player or one-wearer world names the armor; with several players online `armor` is empty.

```rust
use pluto_lib::{EffectGrant, ServerLivingEntityEvents};

ServerLivingEntityEvents::WEARER_HIT.register(|hit| {
    if hit.armor.iter().any(|(_, piece)| piece.item == "example:thorny_helmet") {
        hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:weakness".into(), amplifier: 0, duration: 100 });
    }
});
```

### `ServerLivingEntityEvents::WEAPON_HIT`

A player's weapon hit on a mob, heard on a server before the hit's damage lands. It carries the weapon and what the player and the mob were doing; push status effects on the mob with `effects`, and the game puts them on it right after.

```rust
use pluto_lib::{EffectGrant, ServerLivingEntityEvents};

ServerLivingEntityEvents::WEAPON_HIT.register(|hit| {
    if hit.weapon.as_ref().is_some_and(|weapon| weapon.item == "example:frost_blade") {
        hit.effects.borrow_mut().push(EffectGrant { effect: "minecraft:slowness".into(), amplifier: 1, duration: 60 });
    }
});
```

### `ServerLivingEntityEvents::MODIFY_KNOCKBACK`

NeoForge: `LivingKnockBackEvent`. Heard on a server for each knockback: change the strength or the direction with `set`, or set `cancel` for none. It carries the stacks in the attacker's hands.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_KNOCKBACK.register(|push| {
    if push.attacker_main.as_ref().is_some_and(|stack| stack.item == "example:hammer") {
        push.strength.set(push.strength.get() * 2.0);
    }
});
```

### `ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE`

A player's item is about to lose durability: change the amount with `amount.set`, set `cancel` for no wear, or put a whole new stack in `replacement`. The game's own hook is heard for a LAN player only; PlutoLib reports the local player's wear too.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE.register(|wear| {
    if wear.item == "example:unbreaking_pickaxe" {
        wear.cancel.set(true);
    }
});
```

### `ServerLivingEntityEvents::MODIFY_VISIBILITY`

NeoForge: `LivingVisibilityEvent`. Heard on a server each time a mob works out how far it sees a player who wears armor carrying custom data; the game's own factors (sneaking, invisibility, worn heads) are already counted, and a callback scales the range further with `multiplier.set` (a disguise that halves it for one kind of mob). Nothing is heard for a player wearing no such armor, and the factor never blocks a mob that is already targeting.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_VISIBILITY.register(|look| {
    if look.observer == "minecraft:skeleton" && look.armor.iter().any(|(_, piece)| piece.item == "example:bone_helmet") {
        look.multiplier.set(look.multiplier.get() * 0.5);
    }
});
```

### `ServerLivingEntityEvents::MODIFY_DAMAGE`

NeoForge: `LivingDamageEvent.Pre`. Called for damage that goes ahead, before armor and effects take their share; change the amount with `amount.set`. For a player's attack on the server it carries the stack used.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
    if hit.attacker_stack.as_ref().is_some_and(|stack| stack.item == "example:sword") {
        hit.amount.set(hit.amount.get() * 2.0);
    }
});
```

### `ServerLivingEntityEvents::AFTER_DEATH`

An entity died: called when a living entity's death goes ahead, just before it is removed. Players are not reported yet.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::AFTER_DEATH.register(|death| {
    let _ = death.entity;
});
```

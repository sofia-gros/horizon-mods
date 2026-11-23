# Entity types and events

Declare types derived from existing Minecraft implementations, and listen for entities entering/leaving worlds, damage and death. Type declarations are published into the game's own entity tables by the overlay export (see Derived Entity Types); what remains open for them is runtime policy validation and standalone distribution, not loading.

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
        Ok(())
    }) as MobSpawnAction))
}).unwrap();
```

> **Use with Caution.** Dispatch, owned-action and RNG order are pure-tested. The runtime adapter targets the shared native spawn entry, retaining level/created/fixed yaw selection, individual RNG and UUID registration before join; a production-helper trace verifies these rules and the no-finalization/disabled fallback, but the changed attachment is not yet native-build/game verified. The other callers (runtime/natural, trial-spawner, patrol/raid, marked-structure, frog conversion), native provider forwarding and Mob-owned transient state are source-declared, not native validated; only the pinned frog conversion has the later finalize call, so other conversions must not select gear. A prepared action is consumed once and dropped on abort/error. Native source defers join/publication only for selected completion work; unselected mobs retain native join ordering, and inherited child/world effects are not rolled back after completion failure. Spawn reasons are backend labels (`natural`, `runtime`, `spawner`, `trial_spawner`, `patrol`, `event`, `structure`, `conversion`), not guaranteed Forge reason enums. Gear is a reduced `StackData` view; unsupported slots reject. Mob RNG and the world's enchantment RNG remain separate. Callback registration order is execution order; new callbacks affect subsequent spawns, never an existing prepared action.

## Cancelling Damage

`ALLOW_DAMAGE` is asked before damage is applied. Answer `false` to cancel it; one `false` among all the callbacks is enough:

```rust
use pluto_lib::{LivingDamage, ServerLivingEntityEvents};

ServerLivingEntityEvents::ALLOW_DAMAGE.register(|damage: &LivingDamage| damage.damage_type != "minecraft:fall");
```

`attacker` and `direct` are entity ids, negative when there is none: a skeleton's arrow has the skeleton as `attacker` and the arrow as `direct`.

> **Tip.** `AFTER_DEATH` is called when a living entity's death goes ahead, just before it is removed (the death hook can still cancel it for other mods). `attacker` is the killer's id, negative when there is none; `direct` is always negative and `amount` 0 here. Players are not reported yet.

## Changing Knockback and Item Wear

`ServerLivingEntityEvents::MODIFY_KNOCKBACK` (NeoForge's `LivingKnockBackEvent`) is heard on a server for each knockback: the `entity` pushed and its `kind`, the `attacker` entity id (negative for none), the attacker's `attacker_main` and `attacker_off` hand stacks and the `strength`, `x` and `z` the game worked out. A callback changes them with `Cell::set`, or sets `cancel` for no knockback. Every callback sees the last one's values.

`ServerLivingEntityEvents::MODIFY_ITEM_DAMAGE` is heard on a server when a player's item is about to lose durability (vanilla `ItemStack.hurtAndBreak`, after Unbreaking and not for a creative player or a LAN host). It carries the player's uuid and name, the item id, the stack as held or worn (`None` when no stack of that item is found) and the amount. Change the wear with `amount.set` (0 or less leaves the item whole) or set `cancel` for none.

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

`EntityTypeRegistry::register_derived` declares a new type ID that reuses a vanilla entity implementation. Choose the actual base class: a different render model does not justify another mob's simulation. Width/height are the default (size-one) runtime dimensions in blocks, and the tracking range is in chunks. If a source class scales its EntityType builder dimensions (for example Slime's 0.255 factor), apply that class conversion before registration; do not copy the unscaled builder box.

```rust
use pluto_lib::{EntityTypeRegistry, Identifier};
EntityTypeRegistry::register_derived(
    Identifier::parse("example:cloud_slime").unwrap(),
    Identifier::parse("minecraft:slime").unwrap(),
    2.04, 2.04, 20, true,
).unwrap();
```

> **Use with Caution.** Not a Rust entity factory or a spawn command. The source adapter appends derived rows to entity-types.json preserving inherited raw float fields, allocates matching network registry IDs and publishes command lists. Living factories also inherit their separate attributes.json DefaultAttributes suppliers, retaining exact bit strings without fake suppliers for nonliving factories; missing/invalid living suppliers reject the candidate. existing type IDs cannot be replaced. Validation of a declaration against the game's runtime policy (a factory whose exported row exists but whose model or consumers are missing) remains pending, and the export runs only as part of a content overlay, so a standalone install cannot publish declarations yet. Native execution/model validation, spawn rules, custom attributes, persistent/synchronized state and simulation callbacks remain incomplete or unverified. Invalid values and duplicate/late registrations error; no unrelated template is substituted.

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

> **Use with Caution.** The native no-argument jump query and cube contact-damage query are source-wired only; their signatures/sites still await native attachment plan/validation. The jump factor is the native block factor and `slime_size` a subtype capability, not a generic scale; neither an attribute nor Jump Boost is added after an explicit override. Fall calculation has a separate source-declared LOAD after the living-fall event and class math, before block effects; cancelled/fall-immune paths do not reach it, and `Some(0)` replaces only the calculation, not the surrounding lifecycle or passenger propagation. Callbacks run outside the registry lock, duplicates reject without replacement, and absent/poisoned registration falls back to inheritance.

## Transient Class Attributes

A class can declare server-side attribute modifiers with `transient_attribute_modifier`. These are instance modifiers, not multiplied final damage/armor values or permanent saved modifiers. Use `AddMultipliedTotal` with amount `3.0` for a fourfold multiplier.

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

> **Use with Caution.** The source backend supports armor, armor toughness and knockback resistance for native Mob instances only; other consumers explicitly reject instead of inventing a working effect. A server-only pre-join attachment applies defaults before other join callbacks to fresh and restored mobs, but is **not** a general constructor callback: pre-finalization timing remains different/unverified. Missing attributes are skipped, missing suppliers/services reject, and existing same-ID modifiers are retained. Scalar resistance uses a conditional entry hook: without additional class operations it retains the original function; otherwise its pinned-source calculation inserts class operations after permanent modifiers and before native transient/effect/extra operations, retaining native arithmetic order, NaN handling and clamping, and preserves permanent replacements without restoring a removed class modifier. Unsupported initialization rejects the join rather than publishing a falsely initialized mob. Pure tests check declarations, adapter control/data flow and scalar helper arithmetic through storage facades — not native host execution, observers, cached gear interaction, save/reopen, packet synchronization, role/constructor/lifecycle timing or native types/sites, which remain native attachment validation gates. Load-time class callbacks can restore bases as described next; subsequent mod-controlled metal/size changes and arbitrary attribute mutation APIs remain separate.

## Restore Class Bases on Load

Use `attribute_bases_on_load` when saved class data determines an attribute base. The server callback runs before ordinary entity-load observers, after transient defaults, with the normal attachment-capable scope. The source adapter also reruns the base callback after a successful in-place native `Entity.load` (`/data` and spawn-egg data), using the newly loaded size while preserving transient modifiers; it does not reinstall removed class modifiers. Native failures and non-mob targets retain their original result without a callback.

```rust
use pluto_lib::{AttributeBaseValue, Identifier, LivingEntityBehavior, LivingEntityBehaviors};
LivingEntityBehaviors::register(LivingEntityBehavior::new(Identifier::parse("example:load_bases").unwrap())
    .attribute_bases_on_load(|context| {
        let size = context.slime_size.ok_or("size unavailable")?;
        Ok(vec![AttributeBaseValue { attribute: Identifier::parse("minecraft:armor").unwrap(), value: f64::from(size) }])
    })).unwrap();
```

> **Use with Caution.** This source-only backend writes armor/toughness bases through their native instance observers, retaining modifiers; other base consumers reject and missing entity attributes are skipped. Duplicate or unsupported entries reject the batch before any base write. The source adapter forwards the join event's dimension, rejects a mismatched one before mutation and rechecks it after callback dispatch; an already-cancelled join skips initialization, Callback attachment/world side effects are not rolled back on failure: errors reject the join, and an in-place reload reports failure after the native data change, without rolling it back. Native Host forwarding, application/order, save/sync and gameplay remain unverified behind the native attachment gate.

## Reaching the Entities Around a Ticker

A block entity ticker can reach the entities around it through [`EntityReach`](#entityreach): list the entities in a box, replace or discard a dropped item, hurt a living entity under a damage type of the mod's choosing, read where an entity stands, and move one. The reach answers only while a host scope is open; outside one it answers nothing and every action fails. An entity is either an item (its stack) or a living thing (the facts a consumer's rules read: fire immunity, invulnerability, fire resistance); anything else is listed with neither.

```rust
use pluto_lib::{EntityReach, Identifier, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 7,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    let found = EntityReach::in_box([0.0, 60.0, 0.0], [4.0, 70.0, 4.0]);
    assert_eq!(found.len(), 1);
    assert!(EntityReach::hurt(7, 2.0, "minecraft:in_fire"));
});
assert_eq!(ops.hurt.len(), 1);
```

> **Use with Caution.** The pure rules and the in-memory world are tested. The host adapter is written, not yet compiled or game-verified: `HostEntities` in `hooks.rs` lists the dimension's entities, discards a dropped item by aging it out through the host's entity data merge, replaces an item's stack by merging its `Item` id and count (components stay), and hurts through the host's damage call under the type's own name. The generated `Host` signatures can only be checked by a native build.

## Riding and Entity Ticks

The entity reach also answers who rides whom, and can change it: `EntityReach::passengers_of(vehicle)`, `EntityReach::vehicle_of(passenger)`, `EntityReach::start_riding(passenger, vehicle)` and `EntityReach::stop_riding(passenger)`. A callback that names a player (a `ServerPlayer`) finds their entity id with `EntityReach::id_of_uuid(player.uuid)`, and an entity's type with `EntityReach::kind_of(id)`. The host refuses what the game refuses: the same entity twice, a passenger that already rides something, a cycle, and the local player as a passenger. An accepted mount or dismount is applied by the game's own rider synchronisation, so it is observable through `vehicle_of` from the next tick, and a refusal there is final. The local player can be a *vehicle*: a mob mounted on the player is kept and seated like any other mount.

The reach is open not only while a block entity ticker runs, but also during the entity interaction events, the entity tick events below, and player inventory ticks (`PlayerInventoryEvents::TICK`, in the player documentation), each in the dimension the event names.

An entity's own tick is heard through `ServerEntityEvents::ENTITY_TICK_PRE` and `ENTITY_TICK_POST`, for every kind the game ticks — derived types included. A type declared with `EntityTypeRegistry` carries no code of its own; these events, keyed by the entity's kind, are its tick behaviour surface. Return `false` from a `ENTITY_TICK_PRE` callback to cancel that entity's tick (the post event is then not heard for it).

> **Use with Caution.** The pure rules and the in-memory world are tested, including the refusal rules. The host side is written, not yet compiled or game-verified: the adapter answers ride questions from a snapshot the game rebuilds at each rider synchronisation, queues requests for that synchronisation to apply through the game's own mount paths, and answers `Game::seat` for the local player so a mob riding them is kept and seated. Remote players as vehicles are not in the snapshot: only the local player, mobs, armor stands, displays and interactions are.

## Knocking Back, Pushing and Lifting

The reach can also move a living entity the way the game itself moves one: `EntityReach::knockback(id, strength, x, z)` is `LivingEntity.knockback`, `EntityReach::push(id, delta)` is `Entity.push`, and `EntityReach::leave_ground(id)` is the launch hop of the original's sling leap. A player is reached by all three, and their client is told the new motion. A true answer means the request was taken: the host applies it at its next rider synchronisation, so the motion is observable from the next tick; an entity that is gone, or not living, by then is simply not moved. The reference sections below give each call's exact terms.

> **Use with Caution.** The pure rules and the in-memory world are tested (the requests are recorded). The host side is written, not yet compiled or game-verified: the adapter queues the requests, and a drain woven at the head of the game's rider synchronisation applies a mob's knockback through `Mob::knockback` with a sourceless damage source (the original's bare `knockback` call carries none), a player's through the game's own player knockback with the velocity marked for their client, and a guest's through the game's guest motion path. Armor stands and other non-living kinds are not reached.

## Reference

### `AttributeBaseValue`

`AttributeBaseValue { attribute: Identifier, value: f64 }` is an owned explicit base write. Zero is a value, not a fallback.

```rust
use pluto_lib::{AttributeBaseValue, Identifier};
let base = AttributeBaseValue { attribute: Identifier::parse("minecraft:armor").unwrap(), value: 0.0 };
assert_eq!(base.value, 0.0);
```

### `LivingEntityLoadContext`

`LivingEntityLoadContext { entity: EntityInfo, slime_size: Option<i32> }` carries callback-local entity identity/dimension and a native cube/slime size, absent for other implementations. The entity's typed attachment methods use the current host scope.

```rust
use pluto_lib::{EntityInfo, LivingEntityLoadContext};
let context = LivingEntityLoadContext { entity: EntityInfo { id: 7, kind: "example:slime".into(), dimension: "minecraft:overworld".into() }, slime_size: Some(4) };
assert_eq!(context.slime_size, Some(4));
```

### `LivingEntityBehavior::attribute_bases_on_load`

`attribute_bases_on_load(self, callback: impl Fn(&LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String> + Send + Sync + 'static) -> Self` installs the class's load-time base callback. Empty output leaves bases unchanged. The callback executes outside registration locks and may read existing attachments.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let behavior = LivingEntityBehavior::new(Identifier::parse("example:load_policy").unwrap())
    .attribute_bases_on_load(|_| Ok(Vec::new()));
```

### `LivingEntityBehaviors::attribute_bases_on_load`

`attribute_bases_on_load(context: &LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String>` dispatches and validates duplicate attribute IDs. An unregistered class returns an empty vector.

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

Declaration fields: `attribute: Identifier`, modifier `id: Identifier`, `amount: f64` and `operation: AttributeModifierOperation`. IDs are native identifier keys (not the source era's display-name/random-UUID representation); declarations contain no live entity handles.

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

Owned class-query data: `entity: u32`, `entity_type: Identifier`, post-event `distance: f64`, `multiplier: f32` and inherited native calculation `inherited: f64`. The calculation runs before block-specific alterations; the host's result representation does not promise the original Java's baseline rounding.

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

Process-local registry and dispatch for class queries, keyed by symbolic entity type rather than runtime entity ID.

```rust
use pluto_lib::{Identifier, AttackDamageContext, LivingEntityBehaviors};
let query = AttackDamageContext { entity: 4, entity_type: Identifier::parse("example:absent").unwrap(), attribute: 3.0 };
assert_eq!(LivingEntityBehaviors::attack_damage(&query), None);
```

### `LivingEntityBehaviors::register`

`register(behavior: LivingEntityBehavior) -> Result<(), String>` rejects duplicate type IDs, duplicate class attribute/ID pairs and poisoned registration without changing the old entry.

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

`attack_damage(context: &AttackDamageContext) -> Option<f32>` invokes the registered contact method or delegates.

```rust
use pluto_lib::{Identifier, AttackDamageContext, LivingEntityBehaviors};
let query = AttackDamageContext { entity: 1, entity_type: Identifier::parse("example:missing_contact").unwrap(), attribute: 5.0 };
assert_eq!(LivingEntityBehaviors::attack_damage(&query), None);
```

### `MobSpawnInfo`

Owned spawn origin metadata: `entity_type: Identifier`, `dimension: Identifier`, `position: [f64; 3]` and backend `reason: String`. No temporary numeric entity ID is a handle here. Native source stores pending work in the Mob's owned Navigation, whose private fields avoid the pinned validator's prohibition on adding state to externally constructible Mob. The adapter also checks a Mob-owned identity token before writing equipment: a replacement world/entity with the same numeric ID cannot receive the old work. Consumed or aborted native state cannot be prepared again; render/network clones start without it. This source route still needs native attachment plan/validation.

```rust
use pluto_lib::{Identifier, MobSpawnInfo};
let info = MobSpawnInfo { entity_type: Identifier::parse("minecraft:zombie").unwrap(),
    dimension: Identifier::parse("minecraft:overworld").unwrap(), position: [1.0, 64.0, 2.0], reason: "natural".into(), special_multiplier: 0.5 };
assert_eq!(info.position[1], 64.0);
assert_eq!(info.reason, "natural");
```

### `MobSpawnPreparation`

Borrowed pre-initialization view: spawn info, the mob RNG and owned gear reads. It deliberately has no gear writer. Backends provide implementations; retain copies, never this borrow.

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

`fn set_equipment(&mut self, slot: &str, stack: StackData) -> Result<(), String>`. Replaces a slot; it is not a component-preserving patch. Unsupported slots/data reject, and source complex-offhand writes are unsupported. Errors stop subsequent completion work but do not roll back earlier side effects. Native validation is pending.

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

`fn register(id: Identifier, prepare: impl Fn(&mut dyn MobSpawnPreparation) -> Result<Option<MobSpawnAction>, String> + Send + Sync + 'static) -> Result<(), String>`. Duplicate IDs fail without replacement. `None` means no pending completion, not undoing RNG. Runtime/natural, normal/trial-spawner, patrol/raid, marked-structure and finalizing frog-conversion callers are source-wired but full caller review/native validation remain pending. Selected conversion completes before join and skips the later inherited-finalize call; unselected conversions retain native ordering. NBT-loaded spawner mobs, ordinary saved mobs and NBT summons without the native finalize marker do not select again. Registration alone does not make a game feature work.

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

`fn register_derived(id: Identifier, factory: Identifier, width: f32, height: f32, tracking_chunks: u32, velocity_updates: bool) -> Result<(), String>`. `id` is a custom type, not a replacement in `minecraft`; `factory` names an existing vanilla runtime type (existence still needs native validation). Dimensions must be finite and positive; tracking must be `1..=i32::MAX` chunks. `velocity_updates` whether the type requests velocity synchronization. It rejects duplicate IDs, a poisoned registry and registrations after collection.

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

A player's item about to lose durability, for a callback to change or cancel. `player` is the uuid, `name` the player's name; `item` the id of the item worn, `stack` the stack as the player holds or wears it (`None` when none is found). `amount` is the wear now (a `Cell`: change it with `set`; 0 or less leaves the item whole); `cancel` set to true gives no wear; `slot` is the stack as an item slot, with its components; `replacement` a whole new stack a callback may set (a `RefCell`), written to the main-inventory stack that carried the old data, after which the game wears it by 0. A stack outside the main inventory (worn armor) is not rewritten; the game wears it by `amount`, so a callback that sets `replacement` also sets `amount` to the wear left. `holder` what the player was doing as the stack wore, as an [`EntityState`](#entitystate); `None` when the game did not name the player.

```rust
use pluto_lib::ItemDamage;
use std::cell::{Cell, RefCell};

let wear = ItemDamage { player: 1, name: "alex".into(), item: "example:pickaxe".into(), stack: None, slot: None, amount: Cell::new(3), cancel: Cell::new(false), replacement: RefCell::new(None), holder: None, worn: None, carried: Vec::new(), edits: RefCell::new(Vec::new()) };
wear.amount.set(wear.amount.get() - 1);
assert_eq!((wear.amount.get(), wear.cancel.get(), wear.replacement.borrow().is_none()), (2, false, true));
```

### `PlayerTick`

A server player's tick: `player` is the uuid, `entity` the entity id, `dimension` the dimension the player is in, `position` their feet position, `on_ground` whether they stand on the ground, `armor` the pieces worn that carry custom data (as (slot, stack)), `hands` the stacks held that carry custom data (as (slot, stack): `mainhand`, `offhand`), `block_light`, `sky_light` and `sky_darken` the light where the player stands, sampled at their position raised by half a block, `effects` the player's effects as (effect id, amplifier, ticks left), `remove_effects` the ids callbacks push to take off, `attribute_additions` the (attribute id, amount) pairs callbacks push to add to the player's attribute base values for as long as they push them (amounts for one attribute add up; one no callback pushes any more is taken off again), and `actions` the [`EntityAction`](#entityaction)s callbacks push to be done to the ticking player (as `Attacker`; `Victim` is no one) at tick's end: a player the host can name by entity id is reached, any other is skipped. The tick is heard once more after the last piece with custom data is put away, so an equipment-tied amount is let go. The host gives no transient modifier call, so the amount moves the saved base value: exact for an addition, taken off again when the player leaves (best effort), and a crash while it is applied leaves it in the save.

```rust
use pluto_lib::PlayerTick;
use std::cell::RefCell;

let tick = PlayerTick { player: 1, entity: 4, dimension: "minecraft:overworld".into(), position: [0.0; 3], on_ground: true, armor: Vec::new(), hands: Vec::new(), block_light: 0, sky_light: 15, sky_darken: 0, effects: vec![("minecraft:poison".into(), 0, 100)], remove_effects: RefCell::new(Vec::new()), attribute_additions: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
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

What a callback does to one party of a hit, pushed onto the hit's `actions`. A weapon hit's actions are done just before its damage lands; a hit on a player's, on that player's next tick. Each action is guarded; one the target cannot take is skipped silently: `SetFire { target, seconds }` sets it burning for `seconds` when longer than it already burns (a fire-immune mob is skipped; a player only when the host can reach them); `Heal { target, amount }` gives a mob or player `amount` health, cut to their maximum; `Freeze { target, ticks }` freezes a mob (its frozen ticks become the larger of 140 and what it has, plus `ticks`, and its fire goes out; never a player or a freeze-immune kind); `Knockback { target, strength, direction }` pushes a mob horizontally (and up when it stands on the ground), as `LivingEntity.knockback` does, without the mob's own knockback resistance (a player is not pushed: the host has no call for it). `AddEffect { target, effect, amplifier, duration }` puts a status effect on a player or a mob (`amplifier` is the level less one, `duration` in ticks; an unknown effect id, a negative amplifier or no duration gives nothing; the shooter or shearer is reached by uuid, the world owner included). `EditStackData { target, slot, item, edits }` applies the [`StackDataEdit`](#stackdataedit)s to the custom data of the stack the target player carries in [`slot`](#stackslot), but only when that stack is still of `item`, and to the stack as it is when the action lands, so wear in between stays. It reaches players only (the acting player of the hit, the world owner included); a mob, a slot holding another item and an edit that changes nothing are skipped. `Hurt { target, amount, damage_type }` deals a player or mob `amount` damage of `damage_type` with no attacker, through the host's hurt call: armor, effects, invulnerability time and the damage hooks all apply and hear it, and a creative player or a dead mob is skipped. `SetEffectDuration { target, effect, duration }` sets the ticks left of an effect a player has, keeping level and flags (so it can shorten as well as lengthen); an effect the player lacks, a non-positive duration and a mob are skipped. `Teleport { target, position }` moves the target to `position` (`Entity.teleportTo`): a mob's navigation stops and its fall distance resets, as the game's own teleport does, and another entity keeps its facing; a player is moved by the game's own server teleport, queued for the rider synchronisation, so their teleport lands by the next tick. `DropItem { target, stack }` drops `stack` at the target on its next tick. `Summon { target, kind }` summons `kind` at the target.

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

### `equipment_of`

The equipment of the player `uuid`, as (slot, stack) pairs: the armor the game's ticks reported for them, in `feet`, `legs`, `chest`, `head` order, then the off hand under the name `offhand`. Only pieces carrying custom data are recorded. Empty when the player is not known.

```rust
use pluto_lib::entity_events_v1::equipment_of;

assert!(equipment_of(0).is_empty());
```

### `StackDataEdit`

One change to a stack's custom data for `EntityAction::EditStackData`: `value` is put at `path` (compound keys from the root of the custom data, the compounds on the way made when missing, a value in the way replaced by a compound) or, when `value` is None, the entry at `path` is removed. Edits are applied in order; an empty path is skipped. Edits state the value they want, not a change.

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

A player is about to shear a mob (Bukkit: `PlayerShearEntityEvent`), heard on a server before the shears act. Set `attempt.cancel` to stop the shearing, or push `EntityAction`s for the player (`ActionTarget::Attacker`) onto `attempt.actions`; they land even when a later callback cancels the shearing. `attempt.tool` is the stack in the shearing hand. The game reports only a guest's shearing on the world owner's LAN world; the owner's own shearing is heard by no hook.

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

A player's weapon hit on a mob: `entity` the mob's id, `kind` its type id, `attacker` the player's entity id, `weapon` the stack used (its damage, max damage and custom data; None for an empty hand), `attacker_state` and `target_state` what the player and the mob were doing, `target_position` the mob's position, `attacker_position` where the attacking player stands (their feet), `armor` the pieces the player wore as of their last tick (as (slot, stack): `feet`, `legs`, `chest`, `head`; only those carrying custom data; empty when the player is not known), `effects` the effects callbacks push for the mob, and `actions` the [`EntityAction`](#entityaction)s they push for the player (`Attacker`) or the mob (`Victim`), done just before the hit's damage lands.

```rust
use pluto_lib::WeaponHit;
use std::cell::RefCell;

let hit = WeaponHit { entity: 4, kind: "minecraft:husk".into(), attacker: 1, weapon: None, attacker_state: None, target_state: None, target_position: None, attacker_position: None, armor: Vec::new(), effects: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
assert!(hit.effects.borrow().is_empty() && hit.armor.is_empty() && hit.actions.borrow().is_empty());
```

### `WearerHit`

A mob hurting the player: `attacker` the mob's id, `kind` its type id, `damage_type` the damage type id and `amount` the amount the game works the hit out as (both not reported yet: an empty id and 0.0), `attacker_state` what the mob was doing, `attacker_position` its position, `armor` the pieces the player wore (as (slot, stack): `feet`, `legs`, `chest`, `head`; only those carrying custom data), `effects` the effects callbacks push for the mob, `self_effects` the ones for the wearer, `counter_damage` the damage for the mob, and `actions` the [`EntityAction`](#entityaction)s they push for the mob (`Attacker`) or the wearer (`Victim`), which land on the player's next tick.

```rust
use pluto_lib::{EntityState, WearerHit};
use std::cell::RefCell;

let hit = WearerHit { attacker: 4, kind: "minecraft:zombie".into(), damage_type: "minecraft:mob_attack".into(), amount: 3.0, attacker_position: None, attacker_state: EntityState::default(), armor: Vec::new(), effects: RefCell::new(Vec::new()), self_effects: RefCell::new(Vec::new()), counter_damage: RefCell::new(Vec::new()), actions: RefCell::new(Vec::new()) };
assert!(hit.armor.is_empty() && hit.effects.borrow().is_empty() && hit.self_effects.borrow().is_empty() && hit.counter_damage.borrow().is_empty() && hit.actions.borrow().is_empty());
```

### `EntityState`

What a living entity was doing when a player's weapon hit it, and the facts a condition may ask: `kind` is its entity type id; `sprinting` whether it was sprinting; `on_ground` whether it stood on the ground (false in the air); `in_water` whether it was in water; `fire_ticks` how long it burns for (0 or less when it does not); `fire_immune` whether fire cannot hurt it; `effects` the ids of the effects on it; `effect_levels` the same effects as (id, amplifier), the amplifier being the level less one (see [`EntityState::effect_level`](#entitystateeffect_level)); `health` and `max_health` its health and maximum (the `minecraft:max_health` attribute), `None` when not reported; `biome` the biome it stands in, `biome_temperature` that biome's temperature at its height (`Biome.getTemperature`), `None` for a biome PlutoLib has no temperature for; `eyes_in_water` whether its eyes are in water, where `in_water` is a touch of it; `rain_exposed` whether rain falls on it; `thundering` whether it is thundering where it stands; `enchantments` the ids of the enchantments on its equipment (presence only, not levels); `armor_coverage` the share of its four armor slots holding armor, 0.0 to 1.0, `None` when not reported; `equipment` the names of the equipment slots holding an item (`mainhand`, `offhand`, `head`, `chest`, `legs`, `feet`), `None` when not reported; `armor_stacks` its worn armor as (slot, stack) pairs (custom-data pieces only); `crouching` whether it crouches (a player holding sneak; a mob never does); `y` the height of its feet, `None` when not reported; `block_light` and `sky_light` the two light layers at the block it stands in, each 0 to 15 (`Level.getBrightness`), `None` when not reported; `sky_darken` how much the time of day and the weather take off the sky light, 0 to 11 (see [`sky_darken`](#sky_darken)). Every field has a default, so build one with `..EntityState::default()`.

```rust
use pluto_lib::EntityState;

let state = EntityState { kind: "minecraft:husk".into(), on_ground: true, fire_ticks: 40, effects: vec!["minecraft:poison".into()],
    health: Some(6.0), max_health: Some(20.0), biome_temperature: Some(2.0), rain_exposed: false, y: Some(64.0), sky_light: Some(15), sky_darken: 0, ..EntityState::default() };
assert!(state.fire_ticks > 0 && state.effects.contains(&"minecraft:poison".to_string()));
assert!(state.health.unwrap() < state.max_health.unwrap());
assert_eq!((state.crouching, state.y, state.block_light), (false, Some(64.0), None));
```

> **Use with Caution.** A mob's health, maximum and armor come from its saved data: the maximum is only known when the mob saved a `minecraft:max_health` attribute, and armor only when it saved equipment. A player's come from the server's player list and their equipment. Temperature is a table of the vanilla biomes with the game's cooling above y 80 (its noise term, at most about 0.01, and the frozen-biome noise are left out); a data pack biome has `biome` but no temperature. `rain_exposed` is: raining, full sky light at the entity's block, and a biome that rains rather than snows or stays dry (an unknown biome is taken to rain). `eyes_in_water` reads the block at the eyes: water, a bubble column, kelp, seagrass or a waterlogged block. When a hit on a player wearing custom-data armor brings no target state, `DamageModification::target_state` is the player's as of their last tick, so their health, height and light are up to a tick old. A player's `crouching` is the server's sneak flag and their `y` the height of their feet; a mob's `y` is read from its position and its light from the block it stands in; `sky_darken` is 0 in the Nether and the End, which have no weather and no sky light to take it from.

### `EntityState::effect_level`

The level of an effect on the entity: the amplifier plus one, so 1 for level I, and 0 when it does not have the effect or the game did not report levels.

```rust
use pluto_lib::EntityState;

let state = EntityState { effect_levels: vec![("minecraft:poison".into(), 1)], ..EntityState::default() };
assert_eq!(state.effect_level("minecraft:poison"), 2);
assert_eq!(state.effect_level("minecraft:speed"), 0);
```


### `sky_darken`

`sky_darken(rain, thunder, day_time)` is `Level.getSkyDarken`: how much sky light the weather and time take off, from 0 (clear noon) to 11 (midnight in a thunderstorm). `rain` and `thunder` are the levels, 0.0 to 1.0, and `day_time` the world's time of day in ticks (it may run past 24000).

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

`effect_category(id)` is the category of a vanilla effect id (`minecraft:poison` is harmful), or `None` for an id that is not vanilla's: PlutoLib does not know another mod's effects.

```rust
use pluto_lib::{effect_category, EffectCategory};

assert_eq!(effect_category("minecraft:poison"), Some(EffectCategory::Harmful));
assert_eq!(effect_category("example:unknown"), None);
```

### `DamageModification`

Damage about to be applied, for a callback to change. `damage` what is known of the hit, `amount` the amount now (a `Cell`: change it with `set`), `attacker_stack` the stack a player attacked with (damage, max damage, custom data; None when no player's attack caused it). `armor` lists what a player wore as of their last tick, as (slot, stack) for `feet`, `legs`, `chest` and `head` (custom-data pieces only); empty otherwise. It fires after armor and enchantment protection. `attacker_state`/`target_state` say what the parties were doing as a player's weapon hit landed (`None` otherwise); a damaged player in custom-data armor gets their last tick's state instead. Any attacker is reported (`attacker_kind`, `attacker_is_player`, `attacker_main_hand`, `attacker_chest`, `attacker_attack_damage`), the victim's health and attack attribute as `target_health`/`target_attack_damage`. `actions` takes [`EntityAction`](#entityaction)s for the damaged player (`ActionTarget::Victim`), landing on their next tick; only a custom-data-armor player is served by that queue; a mob party's action is kept for its next tick.

```rust
use pluto_lib::{DamageModification, LivingDamage};
use std::cell::Cell;

let damage = LivingDamage { entity: 4, kind: "minecraft:zombie".into(), damage_type: "minecraft:player_attack".into(), attacker: 1, direct: 1, amount: 3.0 };
let hit = DamageModification { damage, amount: Cell::new(3.0), attacker_stack: None, armor: Vec::new(), attacker_state: None, target_state: None, attacker_kind: "".into(), attacker_is_player: None, attacker_main_hand: None, attacker_chest: None, attacker_attack_damage: None, target_health: None, target_attack_damage: None, actions: Default::default() };
hit.amount.set(hit.amount.get() + 2.0);
```

### `ServerLivingEntityEvents::PLAYER_TICK`

A server player's tick, heard while they wear armor or hold a tool carrying custom data (a mod's own armor keeps its state there). It carries the armor, the held stacks, the light where the player stands and the effects on the player; take effects off them with `remove_effects`. It is how a mod grants immunity to an effect the game gives no call to refuse: the effect is gone by the next tick.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::PLAYER_TICK.register(|tick| {
    if tick.armor.iter().any(|(_, piece)| piece.item == "example:antidote_helmet") && tick.effects.iter().any(|(id, _, _)| id == "minecraft:poison") {
        tick.remove_effects.borrow_mut().push("minecraft:poison".into());
    }
});
```

### `ServerLivingEntityEvents::WEARER_HIT`

A mob hurting the player, heard on a server as the hit starts. It carries the mob's id, type and state, the damage type and amount, and the armor the player wore as of their last tick (only the pieces carrying custom data); push status effects on the mob with `effects`, and the game puts them on it, or on the wearer with `self_effects`, which land on the player's next tick, and deal damage to the mob with `counter_damage` (an armor's thorns), which lands on the next tick too as hurt by the player; set it on fire, freeze it or knock it back with `actions`, which land on the next tick as well. The hit is heard once per attacker per tick, only when the mob is within a few blocks of a player. Only a single-player or one-wearer world names the armor; with several players online `armor` is empty.

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

NeoForge: `LivingVisibilityEvent`. Heard on a server each time a mob works out how far it sees a player who wears armor carrying custom data; the game's own factors (sneaking, invisibility, worn heads) are already counted, and a callback scales the range further with `multiplier.set` (a disguise). Nothing is heard for a player wearing no such armor, and the factor never blocks a mob that is already targeting.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_VISIBILITY.register(|look| {
    if look.observer == "minecraft:skeleton" && look.armor.iter().any(|(_, piece)| piece.item == "example:bone_helmet") {
        look.multiplier.set(look.multiplier.get() * 0.5);
    }
});
```

### `ServerLivingEntityEvents::EXPERIENCE_DROP`

NeoForge: `LivingExperienceDropEvent`. The `ExperienceDrop` a mob killed by a player is about to drop, before the orbs split: change the total with `experience.set`.

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

On a player's attack, `actions` aimed at `ActionTarget::Attacker` are done to the attacking player on their next tick, so a callback can act on the damage it just saw (a lifesteal heals by a share of `amount`). Actions aimed at `Victim` keep their older rule: they land on a damaged player wearing armor with custom data.

```rust
use pluto_lib::{ActionTarget, EntityAction, ServerLivingEntityEvents};

ServerLivingEntityEvents::MODIFY_DAMAGE.register(|hit| {
    if hit.attacker_stack.as_ref().is_some_and(|stack| stack.item == "example:vampire_sword") {
        hit.actions.borrow_mut().push(EntityAction::Heal { target: ActionTarget::Attacker, amount: hit.amount.get() * 0.1 });
    }
});
```

### `ServerLivingEntityEvents::MODIFY_HURT`

NeoForge: `LivingIncomingDamageEvent`. Called as damage that goes ahead arrives, before armor and enchantment protection take a share and before `MODIFY_DAMAGE` hears of it: the amount as it arrives, which callbacks may change with `amount.set`. It carries the same `DamageModification` as the damage event — the armor worn, the states of the hit, the attacking stack — and its actions queue the same way, for the wearer or the attacker; the damage event that follows reads the same hit, only later.

```rust
use pluto_lib::ServerLivingEntityEvents;

ServerLivingEntityEvents::MODIFY_HURT.register(|hit| {
    if hit.damage.damage_type == "minecraft:fall" {
        hit.amount.set(hit.amount.get() * 0.5);
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

### `EntityReach`

The facade a ticker uses to reach the entities around it. Outside a scope every call answers nothing and every action fails.

- `fn in_box(min: [f64; 3], max: [f64; 3]) -> Vec<WorldEntity>`: the entities whose positions fall in the box, in the host's order.
- `fn discard(id: u64) -> bool`: removes an item entity; true when the host accepted the removal.
- `fn set_item(id: u64, stack: &ItemSlot) -> bool`: replaces an item entity's stack.
- `fn hurt(id: u64, amount: f32, damage_type: &str) -> bool`: hurts a living entity by `amount` under the damage type named; whether it took any.
- `fn position_of(id: u64) -> Option<[f64; 3]>`: where the entity stands (its feet position); `None` when the host cannot reach it.
- `fn teleport(id: u64, pos: [f64; 3]) -> bool`: moves the entity to `pos` (`Entity.teleportTo`); a mob's navigation stops and its fall distance resets, as the game's own teleport does. A player is moved by the game's own server teleport (the call `/tp` uses), queued for the level's rider synchronisation: true means it was accepted and lands by the next tick.
- `fn id_of_uuid(uuid: u128) -> Option<u64>`: the entity id of the player whose uuid is `uuid`; `None` when no such player is there.
- `fn kind_of(id: u64) -> Option<Identifier>`: the type of the entity `id`; `None` when there is no such entity.

```rust
use pluto_lib::{EntityReach, Identifier, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

assert!(EntityReach::in_box([0.0, 0.0, 0.0], [4.0, 70.0, 4.0]).is_empty(), "outside a scope");
let mut ops = MemoryEntityOps::default();
ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || {
    assert_eq!(EntityReach::in_box([0.0, 60.0, 0.0], [4.0, 70.0, 4.0]).len(), 1);
    assert_eq!(EntityReach::kind_of(2), Identifier::parse("minecraft:zombie").ok());
    assert_eq!(EntityReach::id_of_uuid(1), None, "the memory world holds no players");
});
```

### `EntityReach::knockback`

`fn knockback(id: u64, strength: f64, x: f64, z: f64) -> bool`. Knocks entity `id` back with `strength` along the horizontal direction (`x`, `z`), as `LivingEntity.knockback` does: the entity's own knockback resistance scales the strength, its horizontal motion is halved against the push, and it rises by the strength (at most 0.4) when it stands on the ground. A player is knocked back the same way, and their client is told the new motion. Only a living entity or a player can be knocked back. A host may apply an accepted knockback at its next rider synchronisation rather than instantly; a refusal there (the entity is gone, or not living) is final. False outside a scope.

```rust
use pluto_lib::{EntityReach, Identifier, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([0.0, 64.0, 0.0], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || assert!(EntityReach::knockback(2, 1.5, -0.5, 0.25)));
assert_eq!(ops.knocked, vec![(2, 1.5, -0.5, 0.25)]);
assert!(!EntityReach::knockback(2, 1.0, 1.0, 0.0), "outside a scope");
```

### `EntityReach::push`

`fn push(id: u64, delta: [f64; 3]) -> bool`. Adds `delta` to entity `id`'s velocity, as `Entity.push` does: no resistance and no halving, the motion simply grows by the vector. A player is pushed the same way, and their client is told. Only a living entity or a player is pushed. A host may apply an accepted push at its next rider synchronisation rather than instantly, as [`EntityReach::knockback`](#entityreachknockback) describes. False outside a scope.

```rust
use pluto_lib::{EntityReach, Identifier, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([0.0, 64.0, 0.0], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || assert!(EntityReach::push(2, [0.1, 0.4, -0.2])));
assert_eq!(ops.pushed, vec![(2, [0.1, 0.4, -0.2])]);
```

### `EntityReach::leave_ground`

`fn leave_ground(id: u64) -> bool`. Lifts the player `id` off the ground for a launch, as `Entity.move(MoverType.SELF, (0, 1.3, 0))` does at the head of one: standing on the ground, they rise by 1.3, or to the first ceiling the move would stop at. Only a player is lifted, and only while they stand on the ground. A host may apply an accepted lift at its next rider synchronisation rather than instantly, as [`EntityReach::knockback`](#entityreachknockback) describes. False outside a scope.

```rust
use pluto_lib::{EntityReach, Identifier, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
ops.entities.push(([0.0, 64.0, 0.0], WorldEntity {
    id: 2,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
}));
with_entities(&mut ops, || assert!(EntityReach::leave_ground(2)));
assert_eq!(ops.lifted, vec![2]);
```

### `EntityOps`

`trait EntityOps`. The operations behind [`EntityReach`](#entityreach): `entities_in(min, max)`, `discard(id)`, `set_item(id, stack)`, `hurt(id, amount, damage_type)`, `position_of(id)` (default `None`), `teleport(id, pos)` (default `false`), `id_of_uuid(uuid)` (default `None`) and `kind_of(id)` (default `None`). An adapter implements it and opens it with `with_entities`.

```rust
use pluto_lib::{EntityOps, EntityReach, ItemSlot, WorldEntity, with_entities};

#[derive(Default)]
struct EmptyWorld;
impl EntityOps for EmptyWorld {
    fn entities_in(&mut self, _min: [f64; 3], _max: [f64; 3]) -> Vec<WorldEntity> { Vec::new() }
    fn discard(&mut self, _id: u64) -> bool { false }
    fn set_item(&mut self, _id: u64, _stack: &ItemSlot) -> bool { false }
    fn hurt(&mut self, _id: u64, _amount: f32, _damage_type: &str) -> bool { false }
}

let mut world = EmptyWorld;
with_entities(&mut world, || {
    assert!(EntityReach::in_box([0.0, 0.0, 0.0], [4.0, 70.0, 4.0]).is_empty());
    assert!(!EntityReach::teleport(7, [0.0, 64.0, 0.0]));
});
```

### `WorldEntity`

`struct WorldEntity { id: u64, kind: Identifier, item: Option<ItemSlot>, living: Option<LivingFacts> }`. One entity in a box: an item entity carries its `item` stack and no living facts, a living entity carries `living` facts and no stack, anything else neither.

```rust
use pluto_lib::{Identifier, LivingFacts, WorldEntity};

let zombie = WorldEntity {
    id: 7,
    kind: Identifier::parse("minecraft:zombie").unwrap(),
    item: None,
    living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
};
assert_eq!(zombie.id, 7);
assert!(zombie.item.is_none() && zombie.living.is_some());
```

### `LivingFacts`

`struct LivingFacts { fire_immune: bool, invulnerable: bool, fire_resistant: bool }`. The facts about a living entity a consumer's rules read: it does not burn, it cannot be hurt by the damage being dealt, and it has fire resistance.

```rust
use pluto_lib::LivingFacts;

let facts = LivingFacts { fire_immune: true, invulnerable: false, fire_resistant: false };
assert!(facts.fire_immune && !facts.invulnerable && !facts.fire_resistant);
```

### `MemoryEntityOps`

An in-memory [`EntityOps`](#entityops) for tests: `entities` placed at fixed positions, with `discarded`, `left` (the stacks `set_item` left) and `hurt` recording what was done, a `hurts_land` switch for a damage the host refuses, and `mounts` recording who rides whom as `(passenger, vehicle)` pairs in the order they mounted, and `knocked`, `pushed` and `lifted` recording the knockbacks, pushes and lifts asked for.

```rust
use pluto_lib::{EntityReach, Identifier, LivingFacts, MemoryEntityOps, WorldEntity, with_entities};

let mut ops = MemoryEntityOps::default();
for (id, kind) in [(2, "minecraft:zombie"), (3, "minecraft:cow")] {
    ops.entities.push(([1.5, 64.5, 1.5], WorldEntity {
        id,
        kind: Identifier::parse(kind).unwrap(),
        item: None,
        living: Some(LivingFacts { fire_immune: false, invulnerable: false, fire_resistant: false }),
    }));
}
with_entities(&mut ops, || {
    assert!(EntityReach::start_riding(2, 3));
    assert!(EntityReach::knockback(2, 1.5, -0.5, 0.25));
    assert!(EntityReach::push(3, [0.1, 0.4, -0.2]));
    assert!(EntityReach::leave_ground(2));
});
assert_eq!(ops.mounts, vec![(2, 3)]);
assert_eq!(ops.knocked, vec![(2, 1.5, -0.5, 0.25)]);
assert_eq!(ops.pushed, vec![(3, [0.1, 0.4, -0.2])]);
assert_eq!(ops.lifted, vec![2]);
```

### `with_entities`

`fn with_entities(ops: &mut dyn EntityOps, body: impl FnOnce() -> R) -> R`. Opens the entity reach over `ops` for `body`; the host opens it around a block entity ticker. The previous scope is restored when `body` returns.

```rust
use pluto_lib::{EntityReach, MemoryEntityOps, with_entities};

let mut ops = MemoryEntityOps::default();
let found = with_entities(&mut ops, || EntityReach::in_box([0.0, 0.0, 0.0], [4.0, 70.0, 4.0]));
assert!(found.is_empty());
assert!(EntityReach::in_box([0.0, 0.0, 0.0], [4.0, 70.0, 4.0]).is_empty(), "the scope is closed again");
```



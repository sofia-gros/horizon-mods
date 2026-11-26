# Attributes a mod declares

Attributes of a mod's own, for the ones the host does not have. The host's attribute set is fixed: a stack entry can name a host attribute and the game applies it, but a mod's own attribute — Tinkers' bouncy, jump count, experience multiplier and their kin — has no host counterpart, so a stack entry naming one would be data nothing applies. A mod declares such an attribute here (its id, its default, the range its value is clamped to), collects the contributions its equipment, tools and effects make to it, and works the value out with the game's own modifier arithmetic, to read where the original reads the attribute. Nothing here applies a value to an entity: the mod keeps the values it works out where its readers can reach them (Tinkers keeps them per player from the player tick). A contribution to one of the host's own attributes still goes through the host's machinery.

### `AttributeOperation`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

How one contribution bends an attribute's value, in the data's own names — the game's three modifier operations. `Addition` adds the amount; `MultiplyBase` adds the amount times the value after additions; `MultiplyTotal` multiplies the value by one plus the amount, one such contribution at a time. The order the operations apply in is fixed by [`attribute_value`](#attribute_value): every addition first, then every multiply-base against the added value, then each multiply-total in turn.

```rust
use pluto_lib::AttributeOperation;

assert_eq!(AttributeOperation::from_name("multiply_total"), Some(AttributeOperation::MultiplyTotal));
assert_eq!(AttributeOperation::MultiplyBase.name(), "multiply_base");
```

### `AttributeOperation::from_name`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn from_name(name: &str) -> Option<AttributeOperation>`. The operation the data names, or None for a name that is not one of the three.

```rust
use pluto_lib::AttributeOperation;

assert_eq!(AttributeOperation::from_name("addition"), Some(AttributeOperation::Addition));
assert_eq!(AttributeOperation::from_name("add_value"), None, "the stack entry name is not the data name");
```

### `AttributeOperation::name`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn name(self) -> &'static str`. The name the data uses for this operation.

```rust
use pluto_lib::AttributeOperation;

assert_eq!(AttributeOperation::Addition.name(), "addition");
assert_eq!(AttributeOperation::MultiplyTotal.name(), "multiply_total");
```

### `AttributeDef`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

One attribute a mod declares: `id` is the attribute's id, such as `tconstruct:generic.bouncy`; `default` the value an entity has with no contributions; `min` and `max` the range the worked-out value is clamped into.

```rust
use pluto_lib::{AttributeDef, Identifier};

let def = AttributeDef { id: Identifier::parse("example:bouncy").unwrap(), default: 0.0, min: 0.0, max: 100.0 };
assert_eq!((def.default, def.min, def.max), (0.0, 0.0, 100.0));
```

### `AttributeContribution`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

One amount contributed to an attribute: by an equipment or tool module on a worn or held stack, by a max-armor claim, or by an effect active on the entity. `operation` says how the amount bends the value, and `amount` is the amount, already scaled by whatever level produced it (a module's formula result, an effect's per-level amount times its level).

```rust
use pluto_lib::{AttributeContribution, AttributeOperation};

let contribution = AttributeContribution { operation: AttributeOperation::Addition, amount: 0.25 };
assert_eq!(contribution.amount, 0.25);
```

### `AttributeContribution::scaled`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn scaled(self, count: i32) -> AttributeContribution`. This contribution scaled by a count, as an effect's modifier is scaled by its level (the game's effect attribute modifiers grow by the per-level amount times amplifier + 1).

```rust
use pluto_lib::{AttributeContribution, AttributeOperation};

let per_level = AttributeContribution { operation: AttributeOperation::MultiplyBase, amount: 0.25 };
assert_eq!(per_level.scaled(3).amount, 0.75);
```

### `attribute_value`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn attribute_value(def: &AttributeDef, contributions: &[AttributeContribution]) -> f64`. An attribute's value under `contributions`, worked out as the game works out an attribute instance: the default, plus every addition; that sum grown by each multiply-base amount's share of it; the result then multiplied by one plus each multiply-total amount in turn; and the whole clamped into the declared range. A contribution whose amount is not finite is ignored, and a result that is not finite falls back to the default.

```rust
use pluto_lib::{AttributeContribution, AttributeDef, AttributeOperation, Identifier, attribute_value};

let def = AttributeDef { id: Identifier::parse("example:multiplier").unwrap(), default: 1.0, min: 0.0, max: 100.0 };
let contributions = [
    AttributeContribution { operation: AttributeOperation::Addition, amount: 3.0 },
    AttributeContribution { operation: AttributeOperation::MultiplyBase, amount: 0.25 },
    AttributeContribution { operation: AttributeOperation::MultiplyTotal, amount: 1.0 },
];
assert_eq!(attribute_value(&def, &contributions), 10.0);
assert_eq!(attribute_value(&def, &[]), 1.0, "no contributions, the default");
```

### `AttributeRegistry`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

The attributes mods have declared, by id. Registration is process-lifetime: declaring an id again with the same declaration stands as it was, and a different declaration for an id already declared is refused, so the first declaration for an id is the one that stands.

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:registry_bouncy").unwrap(), default: 0.0, min: 0.0, max: 100.0 }).unwrap();
assert!(AttributeRegistry::is_registered("example:registry_bouncy"));
```

### `AttributeRegistry::register`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn register(def: AttributeDef) -> Result<(), String>`. Declares an attribute. The declaration is validated whole first — the numbers finite and the default inside the range — so a refused declaration leaves any previous one intact. A declaration for an id already declared is refused the same way when it differs from the one standing (a different default or range); declaring the same declaration again is accepted and changes nothing.

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

let def = AttributeDef { id: Identifier::parse("example:declared").unwrap(), default: 1.0, min: 0.0, max: 100.0 };
AttributeRegistry::register(def.clone()).unwrap();
assert!(AttributeRegistry::register(AttributeDef { default: 5.0, ..def }).is_err(), "a different declaration is refused");
```

### `AttributeRegistry::get`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn get(id: &str) -> Option<AttributeDef>`. The declaration of the attribute `id`, or None when no mod declared one.

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:gettable").unwrap(), default: 2.0, min: 0.0, max: 10.0 }).unwrap();
assert_eq!(AttributeRegistry::get("example:gettable").map(|def| def.default), Some(2.0));
assert!(AttributeRegistry::get("example:missing").is_none());
```

### `AttributeRegistry::is_registered`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn is_registered(id: &str) -> bool`. Whether a mod declared the attribute `id`.

```rust
use pluto_lib::AttributeRegistry;

assert!(!AttributeRegistry::is_registered("example:never_declared"));
```

### `AttributeRegistry::registered`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn registered() -> Vec<AttributeDef>`. Every declared attribute, sorted by id (the registry's own census, for tests and tools).

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:census").unwrap(), default: 0.0, min: 0.0, max: 1.0 }).unwrap();
assert!(AttributeRegistry::registered().iter().any(|def| def.id.to_string() == "example:census"));
```

### `AttributeRegistry::value`

![Horizon: works](https://img.shields.io/badge/Horizon-works-blue) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`fn value(id: &str, contributions: &[AttributeContribution]) -> Option<f64>`. The value of the declared attribute `id` under `contributions`, or None when no mod declared it.

```rust
use pluto_lib::{AttributeContribution, AttributeDef, AttributeOperation, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:valued").unwrap(), default: 1.0, min: 0.0, max: 100.0 }).unwrap();
let bonus = [AttributeContribution { operation: AttributeOperation::Addition, amount: 1.0 }];
assert_eq!(AttributeRegistry::value("example:valued", &bonus), Some(2.0));
assert_eq!(AttributeRegistry::value("example:missing", &[]), None);
```

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

> **Use with Caution.** This source-only backend writes armor/toughness bases through their native instance observers, retaining modifiers; other base consumers reject and missing entity attributes are skipped. Duplicate or unsupported entries reject the batch before any base write. The adapter forwards the join event's dimension (rejecting a mismatch before mutation, rechecking after dispatch); an already-cancelled join skips initialization, and errors reject the join without rolling back attachment or world side effects. Native Host forwarding, application/order, save/sync and gameplay remain unverified behind the native attachment gate.

### `AttributeBaseValue`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`AttributeBaseValue { attribute: Identifier, value: f64 }` is an owned explicit base write. Zero is a value, not a fallback.

```rust
use pluto_lib::{AttributeBaseValue, Identifier};
let base = AttributeBaseValue { attribute: Identifier::parse("minecraft:armor").unwrap(), value: 0.0 };
assert_eq!(base.value, 0.0);
```

### `LivingEntityLoadContext`

![Horizon: inert](https://img.shields.io/badge/Horizon-inert-yellow) ![Sunrise: works](https://img.shields.io/badge/Sunrise-works-blue)

`LivingEntityLoadContext { entity: EntityInfo, slime_size: Option<i32> }` carries callback-local entity identity/dimension and a native cube/slime size, absent for other implementations. The entity's typed attachment methods use the current host scope.

```rust
use pluto_lib::{EntityInfo, LivingEntityLoadContext};
let context = LivingEntityLoadContext { entity: EntityInfo { id: 7, kind: "example:slime".into(), dimension: "minecraft:overworld".into() }, slime_size: Some(4) };
assert_eq!(context.slime_size, Some(4));
```

### `LivingEntityBehavior::attribute_bases_on_load`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`attribute_bases_on_load(self, callback: impl Fn(&LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String> + Send + Sync + 'static) -> Self` installs the class's load-time base callback. Empty output leaves bases unchanged. The callback executes outside registration locks and may read existing attachments.

```rust
use pluto_lib::{Identifier, LivingEntityBehavior};
let behavior = LivingEntityBehavior::new(Identifier::parse("example:load_policy").unwrap())
    .attribute_bases_on_load(|_| Ok(Vec::new()));
```

### `LivingEntityBehaviors::attribute_bases_on_load`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`attribute_bases_on_load(context: &LivingEntityLoadContext) -> Result<Vec<AttributeBaseValue>, String>` dispatches and validates duplicate attribute IDs. An unregistered class returns an empty vector.

```rust
use pluto_lib::{EntityInfo, LivingEntityLoadContext, LivingEntityBehaviors};
let context = LivingEntityLoadContext { entity: EntityInfo { id: 7, kind: "example:no_load_policy".into(), dimension: "minecraft:overworld".into() }, slime_size: None };
assert!(LivingEntityBehaviors::attribute_bases_on_load(&context).unwrap().is_empty());
```

### `AttributeModifierOperation`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

The three Minecraft operations: `AddValue` adds to the base; `AddMultipliedBase` adds a proportion of that additive-phase base; `AddMultipliedTotal` multiplies the accumulated result by `1 + amount`, before effective-value sanitization. This enum describes operations, not a standalone calculator.

```rust
use pluto_lib::AttributeModifierOperation;
let operation = AttributeModifierOperation::AddMultipliedTotal;
assert_ne!(operation, AttributeModifierOperation::AddMultipliedBase);
assert_ne!(operation, AttributeModifierOperation::AddValue);
```

### `TransientAttributeModifier`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

Declaration fields: `attribute: Identifier`, modifier `id: Identifier`, `amount: f64` and `operation: AttributeModifierOperation`. IDs are native identifier keys (not the source era's display-name/random-UUID representation); declarations contain no live entity handles.

```rust
use pluto_lib::{Identifier, TransientAttributeModifier, AttributeModifierOperation};
let modifier = TransientAttributeModifier { attribute: Identifier::parse("minecraft:armor_toughness").unwrap(),
    id: Identifier::parse("example:toughness").unwrap(), amount: 3.0,
    operation: AttributeModifierOperation::AddMultipliedTotal };
assert_eq!(modifier.clone(), modifier);
```

### `LivingEntityBehavior::transient_attribute_modifier`

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

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

![Horizon: absent](https://img.shields.io/badge/Horizon-absent-lightgrey) ![Sunrise: inert](https://img.shields.io/badge/Sunrise-inert-yellow)

`transient_attribute_modifiers(entity_type: &Identifier) -> Result<Vec<TransientAttributeModifier>, String>` returns an owned declaration snapshot, empty for an unregistered type. Poisoned registration returns an error, not an empty policy; mutating the returned vector does not alter registrations or a live mob.

```rust
use pluto_lib::{Identifier, LivingEntityBehaviors};
assert!(LivingEntityBehaviors::transient_attribute_modifiers(
    &Identifier::parse("example:no_attribute_policy").unwrap()).unwrap().is_empty());
```

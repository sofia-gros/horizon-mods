# Attributes a mod declares

Attributes of a mod's own, for the ones the host does not have. The host's attribute set is fixed: a stack entry can name a host attribute and the game applies it, but a mod's own attribute — Tinkers' bouncy, jump count, experience multiplier and their kin — has no host counterpart, so a stack entry naming one would be data nothing applies. A mod declares such an attribute here (its id, its default, the range its value is clamped to), collects the contributions its equipment, tools and effects make to it, and works the value out with the game's own modifier arithmetic, to read where the original reads the attribute. Nothing here applies a value to an entity: the mod keeps the values it works out where its readers can reach them (Tinkers keeps them per player from the player tick). A contribution to one of the host's own attributes still goes through the host's machinery.

### `AttributeOperation`

How one contribution bends an attribute's value, in the data's own names — the game's three modifier operations. `Addition` adds the amount; `MultiplyBase` adds the amount times the value after additions; `MultiplyTotal` multiplies the value by one plus the amount, one such contribution at a time. The order the operations apply in is fixed by [`attribute_value`](#attribute_value): every addition first, then every multiply-base against the added value, then each multiply-total in turn.

```rust
use pluto_lib::AttributeOperation;

assert_eq!(AttributeOperation::from_name("multiply_total"), Some(AttributeOperation::MultiplyTotal));
assert_eq!(AttributeOperation::MultiplyBase.name(), "multiply_base");
```

### `AttributeOperation::from_name`

`fn from_name(name: &str) -> Option<AttributeOperation>`. The operation the data names, or None for a name that is not one of the three.

```rust
use pluto_lib::AttributeOperation;

assert_eq!(AttributeOperation::from_name("addition"), Some(AttributeOperation::Addition));
assert_eq!(AttributeOperation::from_name("add_value"), None, "the stack entry name is not the data name");
```

### `AttributeOperation::name`

`fn name(self) -> &'static str`. The name the data uses for this operation.

```rust
use pluto_lib::AttributeOperation;

assert_eq!(AttributeOperation::Addition.name(), "addition");
assert_eq!(AttributeOperation::MultiplyTotal.name(), "multiply_total");
```

### `AttributeDef`

One attribute a mod declares: `id` is the attribute's id, such as `tconstruct:generic.bouncy`; `default` the value an entity has with no contributions; `min` and `max` the range the worked-out value is clamped into.

```rust
use pluto_lib::{AttributeDef, Identifier};

let def = AttributeDef { id: Identifier::parse("example:bouncy").unwrap(), default: 0.0, min: 0.0, max: 100.0 };
assert_eq!((def.default, def.min, def.max), (0.0, 0.0, 100.0));
```

### `AttributeContribution`

One amount contributed to an attribute: by an equipment or tool module on a worn or held stack, by a max-armor claim, or by an effect active on the entity. `operation` says how the amount bends the value, and `amount` is the amount, already scaled by whatever level produced it (a module's formula result, an effect's per-level amount times its level).

```rust
use pluto_lib::{AttributeContribution, AttributeOperation};

let contribution = AttributeContribution { operation: AttributeOperation::Addition, amount: 0.25 };
assert_eq!(contribution.amount, 0.25);
```

### `AttributeContribution::scaled`

`fn scaled(self, count: i32) -> AttributeContribution`. This contribution scaled by a count, as an effect's modifier is scaled by its level (the game's effect attribute modifiers grow by the per-level amount times amplifier + 1).

```rust
use pluto_lib::{AttributeContribution, AttributeOperation};

let per_level = AttributeContribution { operation: AttributeOperation::MultiplyBase, amount: 0.25 };
assert_eq!(per_level.scaled(3).amount, 0.75);
```

### `attribute_value`

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

The attributes mods have declared, by id. Registration is process-lifetime: declaring an id again with the same declaration stands as it was, and a different declaration for an id already declared is refused, so the first declaration for an id is the one that stands.

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:registry_bouncy").unwrap(), default: 0.0, min: 0.0, max: 100.0 }).unwrap();
assert!(AttributeRegistry::is_registered("example:registry_bouncy"));
```

### `AttributeRegistry::register`

`fn register(def: AttributeDef) -> Result<(), String>`. Declares an attribute. The declaration is validated whole first — the numbers finite and the default inside the range — so a refused declaration leaves any previous one intact. A declaration for an id already declared is refused the same way when it differs from the one standing (a different default or range); declaring the same declaration again is accepted and changes nothing.

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

let def = AttributeDef { id: Identifier::parse("example:declared").unwrap(), default: 1.0, min: 0.0, max: 100.0 };
AttributeRegistry::register(def.clone()).unwrap();
assert!(AttributeRegistry::register(AttributeDef { default: 5.0, ..def }).is_err(), "a different declaration is refused");
```

### `AttributeRegistry::get`

`fn get(id: &str) -> Option<AttributeDef>`. The declaration of the attribute `id`, or None when no mod declared one.

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:gettable").unwrap(), default: 2.0, min: 0.0, max: 10.0 }).unwrap();
assert_eq!(AttributeRegistry::get("example:gettable").map(|def| def.default), Some(2.0));
assert!(AttributeRegistry::get("example:missing").is_none());
```

### `AttributeRegistry::is_registered`

`fn is_registered(id: &str) -> bool`. Whether a mod declared the attribute `id`.

```rust
use pluto_lib::AttributeRegistry;

assert!(!AttributeRegistry::is_registered("example:never_declared"));
```

### `AttributeRegistry::registered`

`fn registered() -> Vec<AttributeDef>`. Every declared attribute, sorted by id (the registry's own census, for tests and tools).

```rust
use pluto_lib::{AttributeDef, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:census").unwrap(), default: 0.0, min: 0.0, max: 1.0 }).unwrap();
assert!(AttributeRegistry::registered().iter().any(|def| def.id.to_string() == "example:census"));
```

### `AttributeRegistry::value`

`fn value(id: &str, contributions: &[AttributeContribution]) -> Option<f64>`. The value of the declared attribute `id` under `contributions`, or None when no mod declared it.

```rust
use pluto_lib::{AttributeContribution, AttributeDef, AttributeOperation, AttributeRegistry, Identifier};

AttributeRegistry::register(AttributeDef { id: Identifier::parse("example:valued").unwrap(), default: 1.0, min: 0.0, max: 100.0 }).unwrap();
let bonus = [AttributeContribution { operation: AttributeOperation::Addition, amount: 1.0 }];
assert_eq!(AttributeRegistry::value("example:valued", &bonus), Some(2.0));
assert_eq!(AttributeRegistry::value("example:missing", &[]), None);
```

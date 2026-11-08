# Fluids

A fluid in PlutoLib is a definition: an id and how the fluid looks and behaves. Tanks hold a fluid by its id, screens and renderers draw it with its colour, and a mod's recipes refer to it by id. This page first walks through defining and using fluids, then lists every item in the [Reference](#reference).

## Basic Example

Define a fluid once, when your mod starts, with `FluidRegistry::register`. Everything is optional except the id: a new definition has water's attributes.

```rust
use pluto_lib::{FluidDef, FluidRegistry, Identifier};

FluidRegistry::register(
    FluidDef::new(Identifier::new("example", "molten_copper").unwrap())
        .temperature(1100)
        .luminance(12)
        .density(2000)
        .viscosity(10000),
);

let copper = FluidRegistry::get("example:molten_copper").unwrap();
assert_eq!(copper.attributes.luminance, 12);
```

Any code can look a fluid up by id later with `FluidRegistry::get`, for example to find out how hot it is.

## Hot, Cool and Light Fluids

The attributes follow the units of the game's own fluids, so water is the comparison: temperature 300 kelvin, density and viscosity 1000. A fluid that is hotter than lava (1300) should burn, a negative density rises, and a higher viscosity flows slower. A mod usually describes a few families of fluids with the same numbers, so keep them in one place:

```rust
use pluto_lib::{FluidDef, Identifier};

/// A molten metal: hot, heavy and slow, like the original Tinkers' Construct.
fn molten(name: &str, kelvin: i32, light: u8) -> FluidDef {
    FluidDef::new(Identifier::new("example", name).unwrap())
        .temperature(kelvin)
        .luminance(light)
        .density(2000)
        .viscosity(10000)
}

let iron = molten("molten_iron", 1100, 12);
assert_eq!(iron.attributes.density, 2000);
let gas = FluidDef::new(Identifier::new("example", "swamp_gas").unwrap()).density(-1000);
assert!(gas.attributes.density < 0, "a negative fluid rises");
```

## Tinting

`color` is `0xAARRGGBB`. The default, white, leaves the fluid's texture as it is; any other colour multiplies it, so one grey texture can serve many fluids:

```rust
use pluto_lib::{FluidDef, Identifier};

let stew = FluidDef::new(Identifier::new("example", "beetroot_soup").unwrap()).color(0xFF84160D);
assert_eq!(stew.attributes.color >> 24, 0xFF, "fully opaque");
```

## Using a Fluid in a Tank

A definition does not hold any fluid: the amounts live in tanks. Declare tanks on a block entity (see [block-entity](block-entity.md)) and put the fluid in by id:

```rust
use pluto_lib::{BlockEntityType, FluidStack, Identifier};

let kind = BlockEntityType::new(Identifier::new("example", "melter").unwrap())
    .fluid_tanks([4000])
    .ticker(|tick| {
        // fill the tank with molten copper, one bucket a tick, until it is full
        let have = tick.fluid(0).map_or(0, |f| f.amount);
        if have < 4000 {
            let copper = Identifier::new("example", "molten_copper").unwrap();
            tick.set_fluid(0, Some(FluidStack { fluid: copper, amount: (have + 1000).min(4000) }));
        }
    });
let _ = kind;
```

## Source Blocks and Buckets

Many mods want a fluid to be a real thing in the world: placed from a bucket, flowing across the ground. `with_world_blocks` and `with_bucket` record that wish.

> **Use with Caution.** Horizon has no source or flowing blocks of a mod's fluid, and no buckets of one yet, so these two do nothing today. They are kept and listed in the support matrix, and start working without a change in your mod once the game offers them. Until then a fluid exists only in tanks.

```rust
use pluto_lib::{FluidDef, Identifier};

let lake = FluidDef::new(Identifier::new("example", "lake_water").unwrap()).with_world_blocks().with_bucket();
assert!(lake.world_blocks && lake.bucket);
```

## Names in the Player's Language

A fluid's name is looked up by a translation key, `fluid.<namespace>.<path>`, in the language the player chose (see [i18n](i18n.md)). Register the names for each language you support and ask the registry for the display name:

```rust
use pluto_lib::{FluidDef, FluidRegistry, I18n, Identifier, Translations};

FluidRegistry::register(FluidDef::new(Identifier::new("example", "molten_gold").unwrap()).temperature(1000));
Translations::register("en_us", [("fluid.example.molten_gold".to_owned(), "Molten Gold".to_owned())]);
Translations::register("ja_jp", [("fluid.example.molten_gold".to_owned(), "溶けた金".to_owned())]);

// the same fluid, named for a player who plays in Japanese
assert_eq!(FluidRegistry::display_name_in("ja_jp", "example:molten_gold").unwrap(), "溶けた金");
// and for the language the game is set to right now
I18n::use_language(Some("en_us"));
assert_eq!(FluidRegistry::display_name("example:molten_gold").unwrap(), "Molten Gold");
I18n::use_language(None);
```

> **Tip.** When the player switches language, a name you drew last frame is simply asked for again. Only text you kept (a built string) needs rebuilding, on `LanguageEvents::CHANGED`.

## Reference

### `FluidAttributes`

How a fluid looks and behaves. `temperature` is in kelvin (water 300, lava 1300), `luminance` the light level 0 to 15, `density` (water 1000, lava 3000; a negative fluid rises), `viscosity` (water 1000, lava 6000; higher flows slower) and `color` as `0xAARRGGBB` (white leaves the texture as it is). The default is water's.

```rust
use pluto_lib::FluidAttributes;

let water = FluidAttributes::default();
assert_eq!((water.temperature, water.luminance, water.density, water.viscosity), (300, 0, 1000, 1000));
assert_eq!(water.color, 0xFFFF_FFFF);
```

### `FluidDef`

A fluid a mod defines. `id` is its id, `attributes` how it looks and behaves, `world_blocks` whether it wants source and flowing blocks in the world (stub), `bucket` whether it wants a bucket (stub).

```rust
use pluto_lib::{FluidDef, Identifier};

let def = FluidDef::new(Identifier::new("example", "honey").unwrap());
assert_eq!(def.id.to_string(), "example:honey");
assert_eq!(def.attributes.temperature, 300);
assert!(!def.world_blocks && !def.bucket);
```

### `FluidDef::new`

`fn new(id: Identifier) -> FluidDef`. A fluid with water's attributes, no world blocks and no bucket.

```rust
use pluto_lib::{FluidDef, Identifier};

assert_eq!(FluidDef::new(Identifier::new("example", "oil").unwrap()).attributes.density, 1000);
```

### `FluidDef::temperature`

`fn temperature(self, kelvin: i32) -> FluidDef`. The temperature in kelvin.

```rust
use pluto_lib::{FluidDef, Identifier};

let lava_like = FluidDef::new(Identifier::new("example", "magma").unwrap()).temperature(1300);
assert_eq!(lava_like.attributes.temperature, 1300);
```

### `FluidDef::luminance`

`fn luminance(self, level: u8) -> FluidDef`. The light level the fluid gives, 0 to 15 (larger values are clamped to 15).

```rust
use pluto_lib::{FluidDef, Identifier};

assert_eq!(FluidDef::new(Identifier::new("example", "glow").unwrap()).luminance(40).attributes.luminance, 15);
```

### `FluidDef::density`

`fn density(self, density: i32) -> FluidDef`. The density; a negative fluid rises.

```rust
use pluto_lib::{FluidDef, Identifier};

assert_eq!(FluidDef::new(Identifier::new("example", "gas").unwrap()).density(-1000).attributes.density, -1000);
```

### `FluidDef::viscosity`

`fn viscosity(self, viscosity: i32) -> FluidDef`. The viscosity; the higher, the slower it flows.

```rust
use pluto_lib::{FluidDef, Identifier};

assert_eq!(FluidDef::new(Identifier::new("example", "tar").unwrap()).viscosity(8000).attributes.viscosity, 8000);
```

### `FluidDef::color`

`fn color(self, argb: u32) -> FluidDef`. The colour as `0xAARRGGBB`.

```rust
use pluto_lib::{FluidDef, Identifier};

assert_eq!(FluidDef::new(Identifier::new("example", "ink").unwrap()).color(0xFF101030).attributes.color, 0xFF101030);
```

### `FluidDef::with_world_blocks`

`fn with_world_blocks(self) -> FluidDef`. The fluid wants source and flowing blocks in the world. Kept; does nothing until the game has mod fluid blocks.

```rust
use pluto_lib::{FluidDef, Identifier};

assert!(FluidDef::new(Identifier::new("example", "lake").unwrap()).with_world_blocks().world_blocks);
```

### `FluidDef::with_bucket`

`fn with_bucket(self) -> FluidDef`. The fluid wants a bucket. Kept; does nothing until the game has mod fluid buckets.

```rust
use pluto_lib::{FluidDef, Identifier};

assert!(FluidDef::new(Identifier::new("example", "milk_like").unwrap()).with_bucket().bucket);
```

### `FluidDef::translation_key`

`fn translation_key(&self) -> String`. The key of the fluid's name in the language files: `fluid.<namespace>.<path>`.

```rust
use pluto_lib::{FluidDef, Identifier};

assert_eq!(FluidDef::new(Identifier::new("tconstruct", "molten_iron").unwrap()).translation_key(), "fluid.tconstruct.molten_iron");
```

### `FluidRegistry::display_name`

`fn display_name(id: &str) -> Option<String>`. The fluid's name in the language in use (see `I18n`); the translation key itself if nobody translated it, and `None` if no mod defined the fluid.

```rust
use pluto_lib::{FluidDef, FluidRegistry, Identifier};

FluidRegistry::register(FluidDef::new(Identifier::new("example", "untranslated").unwrap()));
assert_eq!(FluidRegistry::display_name("example:untranslated").unwrap(), "fluid.example.untranslated");
assert!(FluidRegistry::display_name("example:never_defined").is_none());
```

### `FluidRegistry::display_name_in`

`fn display_name_in(language: &str, id: &str) -> Option<String>`. The fluid's name in `language`, whatever the game is set to; `None` if no mod defined the fluid.

```rust
use pluto_lib::{FluidDef, FluidRegistry, Identifier, Translations};

FluidRegistry::register(FluidDef::new(Identifier::new("example", "juice").unwrap()));
Translations::register("ja_jp", [("fluid.example.juice".to_owned(), "ジュース".to_owned())]);
assert_eq!(FluidRegistry::display_name_in("ja_jp", "example:juice").unwrap(), "ジュース");
```

### `FluidRegistry`

Registers fluids (Fabric: `Registry.register(Registries.FLUID, ...)`).

```rust
use pluto_lib::{FluidDef, FluidRegistry, Identifier};

FluidRegistry::register(FluidDef::new(Identifier::new("example", "syrup").unwrap()).viscosity(5000));
```

### `FluidRegistry::register`

`fn register(def: FluidDef)`. Defines a fluid. Registering the same id again replaces the earlier definition. Register in your mod's `hooks()`.

```rust
use pluto_lib::{FluidDef, FluidRegistry, Identifier};

let id = Identifier::new("example", "swapped").unwrap();
FluidRegistry::register(FluidDef::new(id.clone()).temperature(400));
FluidRegistry::register(FluidDef::new(id).temperature(500));
assert_eq!(FluidRegistry::get("example:swapped").unwrap().attributes.temperature, 500);
```

### `FluidRegistry::get`

`fn get(id: &str) -> Option<FluidDef>`. The definition of the fluid `id` (such as `example:syrup`), if a mod defined it.

```rust
use pluto_lib::FluidRegistry;

assert!(FluidRegistry::get("example:never_defined").is_none());
```

### `FluidRegistry::registered`

`fn registered() -> Vec<FluidDef>`. Every defined fluid, sorted by id.

```rust
use pluto_lib::{FluidDef, FluidRegistry, Identifier};

FluidRegistry::register(FluidDef::new(Identifier::new("example", "zzz").unwrap()));
FluidRegistry::register(FluidDef::new(Identifier::new("example", "aaa").unwrap()));
let ids: Vec<String> = FluidRegistry::registered().iter().map(|f| f.id.to_string()).collect();
let mut sorted = ids.clone();
sorted.sort();
assert_eq!(ids, sorted);
```

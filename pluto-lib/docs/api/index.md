# PlutoLib API

PlutoLib is a library mod for Horizon: events, player access, data, config and commands in one place, so a mod does not have to hook the game itself. Everything below is plain Rust; every example on these pages is compiled and run by the test suite.

## Start a mod on PlutoLib

Declare the dependency in the mod's `horizon-mixin.toml`, and give the mod an `[api]` package:

```toml
[mod]
id = "mymod"
version = "0.1.0"

[api]
path = "api"

[dependencies]
pluto_lib = "^0.1"
```

The package's `lib.rs` registers what the mod wants and returns the hooks entry point. A mod that registers everything through PlutoLib returns `no_hooks()`; the mod never names a Horizon type:

```rust,ignore
use pluto_lib::{ModHooks, ServerPlayConnectionEvents};

pub fn hooks() -> ModHooks {
    ServerPlayConnectionEvents::JOIN.register(|join| {
        join.player.send_message("Welcome!");
    });
    pluto_lib::no_hooks()
}
```

(This one block is not compiled by the page tests, because `ModHooks` and `hooks()` belong to the game-facing half of PlutoLib. Everything else on these pages is.)

## How these pages are written

Each page starts with a **Basic Example** and then walks through the feature one section at a time: what it is for, the code, and what to watch out for. The **Reference** at the end of a page lists every public item with its types, its arguments and an example for each form. All code on these pages is compiled and run by the test suite, so it does what it says.

## Pages

| Page | What it covers |
|---|---|
| [base](base.md) | `Event`, `Identifier`, `ActionResult`, the support matrix |
| [lifecycle](lifecycle.md) | server start, stopping and successful stopped events, ticks, chunk load and unload |
| [networking](networking.md) | a player joins or leaves |
| [interaction](interaction.md) | using items and blocks, finishing eating or drinking, attacking, breaking blocks and how fast |
| [entities](entities.md) | entities entering and leaving worlds, damage (allow it or change its amount, with the attacker's stack), death |
| [loot](loot.md) | change loot tables as they load |
| [resources](resources.md) | resource reload listeners |
| [items](items.md) | tooltips (with the stack's damage and data), using items over time |
| [block](block.md) | where a mod's block may stand, which state it is placed in, how neighbours change it, entities inside it |
| [color](color.md) | the tint of a block's faces, which may depend on where the block stands (stub: the game does not ask yet) |
| [block-entity](block-entity.md) | NBT, block entities for a mod's blocks (saved data, ticking, sync to clients and menu data slots) |
| [screen](screen.md) | windows for a block's menu: static and dynamic slot positions, drawing, clicks, tooltips, menu buttons, what each slot takes, shift-click routing, what a hopper may put in |
| [renderer](renderer.md) | drawing a mod block's block entity in the world |
| [fluid](fluid.md) | fluids a mod defines: id, temperature, light, density, viscosity, colour |
| [i18n](i18n.md) | translations: text per language, the language in use, switching |
| [world](world.md) | reading and changing blocks of a world, block tags |
| [projectile](projectile.md) | a projectile hits something: act on the world, cancel the hit |
| [creative](creative.md) | creative inventory tabs, data-carrying stacks, entries for existing tabs |
| [player](player.md) | messages, items and item stacks with their data (damage, max damage, custom data) for a player, effects, hunger, and testing with `MemoryPlayerOps` |
| [attachments](attachments.md) | values saved with a player |
| [config](config.md) | config files |
| [commands](commands.md) | commands |

## What a call does when the host cannot

PlutoLib always offers the whole API. A call the pinned Horizon cannot honour yet is accepted and does nothing (or returns its documented default); `base.md` describes the support matrix, and each such call says "Stub" on its page.

## Test a mod's logic

Code that uses a `ServerPlayer` runs against an in-memory game in a plain `cargo test`:

```rust
use pluto_lib::{with_ops, MemoryPlayerOps, ServerPlayConnectionEvents, PlayerConnection, ServerPlayer};

ServerPlayConnectionEvents::JOIN.register(|join| {
    join.player.give_item("minecraft:bread", 1);
});

let player = ServerPlayer { uuid: 1, name: "Alex".into() };
let mut game = MemoryPlayerOps::default();
with_ops(&mut game, || {
    ServerPlayConnectionEvents::JOIN.invoke(&PlayerConnection { player: player.clone(), name: player.name.clone() });
});
assert_eq!(game.items[&(1, "minecraft:bread".to_owned())], 1);
```

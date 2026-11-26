# Commands

Add commands when the server builds its command tree. A command is one usage line: words, then arguments `<name:kind>`, optional ones last in brackets `[name:kind]`. Kinds: `word`, `text` (the rest of the line, last), `integer`, `number`, `boolean`, `player`, `item`, `block_pos`. The handler gets the arguments as text, in order, and returns the command's result (1 for success, 0 for failure).

## Basic Example

Register a command when the server builds its command tree. The usage line is the command: words, then arguments in angle brackets.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example hello", |ctx| {
        ctx.source.send_message("Hello!");
        1 // success
    });
});
```

## Arguments

An argument is written `<name:kind>`; an optional one goes last in square brackets, `[name:kind]`. The handler receives the words in `ctx.args`, in the order the usage lists them:

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example give <item:item> [count:integer]", |ctx| {
        let item = &ctx.args[0];
        let count: u32 = ctx.args.get(1).and_then(|c| c.parse().ok()).unwrap_or(1);
        ctx.source.give_item(item, count);
        1
    });
});
```

The kinds are `word`, `text` (the rest of the line, last), `integer`, `number`, `boolean`, `player`, `item` and `block_pos`.

## Who May Run It

`register` is for every player. `register_op` takes an operator level from 1 to 4:

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register_op("example reset", 2, |_| 1);
});
```

> **Tip.** A handler is a plain closure over a `CommandContext`, so you can test it without a game: build the context by hand and run it inside `with_ops` with a `MemoryPlayerOps` (see [player](player.md)).

## Reference

### `CommandRegistrationCallback`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Holder of the registration event.

> Its `EVENT` runs when the server builds its command tree, and the callback receives a
> `CommandRegistrar` to add commands with: `register` for a command every player may run,
> `register_op` for an operator-only one.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example hello", |_| 1);
});
```

### `CommandRegistrationCallback::EVENT`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Runs when the server builds its command tree.

> Register your commands in this callback, through the `CommandRegistrar` it receives: the
> tree is built per server, so this runs again for each new server. Commands added
> anywhere else are not in the tree.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example pay <player:player> <amount:integer>", |ctx| {
        let amount: i32 = ctx.args[1].parse().unwrap_or(0);
        ctx.source.send_message(&format!("paid {} {}", ctx.args[0], amount));
        1
    });
});
```

### `CommandRegistrar`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Collects the commands one registration pass adds.

> It is handed to the callback of `CommandRegistrationCallback::EVENT`, and it lives only
> for that pass: add every command inside the callback, with `register` or `register_op`.
> There is no registrar to hold onto afterwards.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example a", |_| 1);
    registrar.register_op("example b", 2, |_| 1);
});
```

### `CommandRegistrar::register`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A command every player may run.

`fn register(&self, usage: &str, handler: impl Fn(&CommandContext) -> i32 + Send + Sync + 'static)`

> `usage` is the command's text — its name and argument shape — and the handler runs the
> command, receiving a `CommandContext` and returning an `i32` result code. For a command
> only operators may run, use `register_op` instead.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example balance [player:player]", |ctx| {
        let who = ctx.args.first().cloned().unwrap_or_else(|| ctx.source.name.clone());
        ctx.source.send_message(&format!("{who} has 5 coins"));
        1
    });
});
```

### `CommandRegistrar::register_op`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A command for operators of `level` (1 to 4) and above.

`fn register_op(&self, usage: &str, level: u8, handler: ...)`

> `level` is the operator permission level the command requires, from 1 to 4, and players
> below it cannot run the command. The `usage` text and the handler work as in `register`.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register_op("example reset", 2, |_| 1);
});
```

### `CommandContext`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What a handler receives.

> `source` is the player who ran the command (its methods work inside the handler), `args`
> the argument words in the order the usage lists them; an omitted optional argument is
> absent. Build one yourself to test a handler.

```rust
use pluto_lib::{with_ops, CommandContext, MemoryPlayerOps, ServerPlayer};

// the handler of "example give <item:item>" as a plain function
fn give(ctx: &CommandContext) -> i32 {
    if ctx.source.give_item(&ctx.args[0], 1) == 0 { 1 } else { 0 }
}

let mut game = MemoryPlayerOps::default();
let ctx = CommandContext { source: ServerPlayer { uuid: 1, name: "Alex".into() }, args: vec!["minecraft:apple".into()] };
assert_eq!(with_ops(&mut game, || give(&ctx)), 1);
assert_eq!(game.items[&(1, "minecraft:apple".to_owned())], 1);
```

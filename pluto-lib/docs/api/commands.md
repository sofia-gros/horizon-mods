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

Holder of the registration event.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example hello", |_| 1);
});
```

### `CommandRegistrationCallback::EVENT`

Runs when the server builds its command tree; the callback receives a `CommandRegistrar`.

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

Collects the commands one registration pass adds; it is handed to the callback of `CommandRegistrationCallback::EVENT`.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register("example a", |_| 1);
    registrar.register_op("example b", 2, |_| 1);
});
```

### `CommandRegistrar::register`

`fn register(&self, usage: &str, handler: impl Fn(&CommandContext) -> i32 + Send + Sync + 'static)`. A command every player may run.

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

`fn register_op(&self, usage: &str, level: u8, handler: ...)`. A command for operators of `level` (1 to 4) and above.

```rust
use pluto_lib::CommandRegistrationCallback;

CommandRegistrationCallback::EVENT.register(|registrar| {
    registrar.register_op("example reset", 2, |_| 1);
});
```

### `CommandContext`

What a handler receives. `source` is the player who ran the command (its methods work inside the handler), `args` the argument words in the order the usage lists them; an omitted optional argument is absent. Build one yourself to test a handler.

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

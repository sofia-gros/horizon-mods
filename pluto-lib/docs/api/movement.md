# Block movement factors

A mod gives its blocks the movement factors the original gives them: the speed factor that scales an
entity's movement speed on the block and the jump factor that scales its jump height, as
`BlockBehaviour.Properties.speedFactor` and `jumpFactor` set them. A block with no registration keeps
its template's factors, which are 1.0 and 1.0 for the vanilla templates, so only blocks whose originals
differ need registering.

```rust
use pluto_lib::movement::{get, register, MovementFactors};

register("example:soul_glass", 0.2, 1.0).unwrap();
assert_eq!(get("example:soul_glass"), Some(MovementFactors { speed_factor: 0.2, jump_factor: 1.0 }));
assert_eq!(get("example:lantern"), None);
```

Register before the first movement-table load: the game loads the host's movement table once per world
load, and the `pluto_movement` mixin applies the registered factors to that table as it loads. A
registration made after a load applies on the next one. The host's block declarations carry no
speed-factor key, so the factors cannot ride in a block declaration; they ride here instead.

> **Use with Caution.** The registry is pure-tested. The transport patches the host's movement table
> at load (a RETURN injection on the table load, source-declared, native-unvalidated), replacing only
> the registered blocks' speed and jump factors and leaving the rest of each entry alone.

## Reference

### MovementFactors

The two movement factors of one block: `speed_factor` scales an entity's movement speed on the block,
`jump_factor` its jump height. Both are 1.0 unless the block sets them.

```rust
use pluto_lib::movement::MovementFactors;

let factors = MovementFactors { speed_factor: 0.4, jump_factor: 0.5 };
assert_eq!(factors.speed_factor, 0.4);
assert_eq!(factors.jump_factor, 0.5);
```

### register

Register the movement factors of one block, by its namespaced block ID. The factors must be finite and
0 or more. Registering the same block with the same factors again is a no-op; registering it with
different factors is an error, so two mods cannot silently disagree about a block.

```rust
use pluto_lib::movement::register;

assert!(register("example:glass", 0.1, 1.0).is_ok());
assert!(register("example:glass", 0.1, 1.0).is_ok());
assert!(register("example:glass", 0.2, 1.0).is_err());
assert!(register("glass", 0.1, 1.0).is_err());
assert!(register("example:glass", f32::NAN, 1.0).is_err());
```

### get

The factors registered for a block, if any. A block with no registration keeps its template's factors.

```rust
use pluto_lib::movement::{get, register, MovementFactors};

register("example:cheese", 0.4, 0.5).unwrap();
assert_eq!(get("example:cheese"), Some(MovementFactors { speed_factor: 0.4, jump_factor: 0.5 }));
assert_eq!(get("example:stone"), None);
```

### registered

Every registration, in block-ID order, for the movement-table injection.

```rust
use pluto_lib::movement::{register, registered};

register("example:zeta", 0.4, 0.5).unwrap();
register("example:alpha", 0.2, 1.0).unwrap();
let all = registered();
let listed: Vec<&str> = all.iter().map(|(block, _)| block.as_str()).collect();
assert!(listed.windows(2).all(|pair| pair[0] < pair[1]));
```

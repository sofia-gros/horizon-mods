# Potions

The effects a potion carries, read from the game's own potion registry: which effects, at what duration and amplifier. The registry lives in the game's code, so PlutoLib asks the game rather than copying the vanilla table — a potion registered with the game by content is answered like a vanilla one. Asking is all this page does: applying the effects is the caller's work, through the entity actions of [entities](entities.md).

### `PotionEffectFact`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

One effect of a potion, as the game's registry lists it.

> Its fields are `effect` (the effect's id), `duration` (in ticks; an instantaneous
> effect's stored duration is 1), `amplifier` (the effect's level less one) and `instant`
> (whether the effect applies at once instead of lasting).

```rust
use pluto_lib::{Identifier, PotionEffectFact};

let fact = PotionEffectFact { effect: Identifier::parse("minecraft:poison").unwrap(), duration: 432, amplifier: 1, instant: false };
assert_eq!(fact.effect.to_string(), "minecraft:poison");
assert_eq!((fact.duration, fact.amplifier, fact.instant), (432, 1, false));
```

### `Potions`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

Looking potions up in the game's registry.

> Outside the game (unit tests, tools) there is no registry to read, and a lookup finds
> nothing.

```rust
use pluto_lib::{Identifier, Potions};

let water = Identifier::parse("minecraft:water").unwrap();
assert_eq!(Potions::effects(&water), None, "no game is running in a unit test");
```

### `Potions::effects`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

The effects of a potion, in the order the game lists them.

> None when the game has no potion under the id — an invalid id is not an error, it simply
> carries no effects. A known potion with no effects (water, mundane) answers Some with an
> empty list.

```rust
use pluto_lib::{Identifier, Potions};

let poison = Identifier::parse("minecraft:strong_poison").unwrap();
assert_eq!(Potions::effects(&poison), None, "the registry is the game's, and no game is running here");
```

### `Potions::contents`

![Horizon: absent](badges/horizon-absent.svg) ![Sunrise: works](badges/sunrise-works.svg)

The effects a potion's contents apply, as a stack carries them: the potion's own effects followed by the contents' custom effects, in that order.

> An unknown potion contributes no effects of its own, so the custom effects are the whole
> list.

```rust
use pluto_lib::{Identifier, PotionEffectFact, Potions};

let custom = PotionEffectFact { effect: Identifier::parse("minecraft:poison").unwrap(), duration: 200, amplifier: 0, instant: false };
let water = Identifier::parse("minecraft:water").unwrap();
assert_eq!(Potions::contents(&water, &[custom.clone()]), vec![custom]);
```

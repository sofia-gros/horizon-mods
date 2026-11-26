# Trade offers

Register factories that join a merchant's candidate pool **before selection**. A factory creates its offer only when selected, using the caller's RNG and borrowed registry view. Repeating a factory gives it additional weight; adding a completed offer afterwards is not equivalent.

## Basic Example

```rust
use pluto_lib::{TradeOfferFactory, TradeOfferHelper};
TradeOfferHelper::register_wandering_trader_offers(2, || {
    let factory = TradeOfferFactory::new(|_, _| Ok(None));
    vec![factory.clone(), factory] // two candidate entries, not two guaranteed offers
}).unwrap();
```

Level 1 is common; level 2 is rare. Providers return owned snapshots in registration order and run outside registry locks. Configuration can change the next snapshot without changing a pool already being sampled. A factory returns `Ok(None)` to decline or `Err` to report construction failure, not a replacement stack.

> **Use with Caution.** The v13 full attachment plan and host/API/game-entry typechecks include this path; binary linking and gameplay remain unverified, and the combined attachment review is pending. A concrete Game trade-generation HEAD (not a generic-method injection) checks native server authority before snapshotting factories and borrowing the existing per-set RandomSequence. Its native helper appends candidates before the original amount/duplicate-policy selection. Common maps to the host's `common` set, rare to `uncommon`; this does not prove the original 1.20 rare-list distribution. No custom factories leaves the original function untouched. Declined listings follow native removal policy, while errors abort without publishing a successful offer prefix. Explicit errors return the lent RNG before aborting and do not register prefix exploration maps. The concrete Game site and its helper/module inputs were reconciled against the pinned plan. Native offer creation is delegated to the current host, including cost-provider RNG and component matching; this adapter does not replace menu/session handling. Save/reopen, demand/XP/UI and LAN behavior remain unverified. Registry poisoning is an error, not a silent empty pool.

The native source view uses authoritative ordered item tags and actual item existence. Selected factories forward count/damage/max-damage/custom data through the existing stack writer; all item IDs are validated before an offer is returned. Fresh offers start at zero uses/demand/special price and retain native experience/reward handling. These are source contracts, not runtime transport proof.

## Reference

### `TradeRandom`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The authoritative caller RNG: `next_int(&mut self, bound: i32) -> i32` for a positive bound, `next_float(&mut self) -> f32` and `next_double(&mut self) -> f64`.

> A backend must not substitute a newly seeded generator or erase singleton draws: the
> draws a set makes are the draws it gets, in order. Native generation forwards the
> borrowed per-set sequence, so generation consumes the caller's RNG rather than a private
> one.

```rust
use pluto_lib::TradeRandom;
struct First;
impl TradeRandom for First {
    fn next_int(&mut self, bound: i32) -> i32 { assert!(bound > 0); 0 }
    fn next_float(&mut self) -> f32 { 0.0 }
    fn next_double(&mut self) -> f64 { 0.0 }
}
assert_eq!(First.next_int(1), 0);
```

### `TradeWorld`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Borrowed item registry access: `item_tag(&self, tag: &str) -> Option<Vec<String>>` returns authoritative ordered members (None is missing/unavailable), and `item_exists(&self, item: &str) -> bool` checks the actual registry.

> Never synthesize tag members from a static fallback: `None` from `item_tag` means the
> tag is missing or unavailable, and that answer is authoritative. `item_exists` likewise
> checks the actual registry, not a list of your own.

```rust
use pluto_lib::TradeWorld;
struct Unavailable;
impl TradeWorld for Unavailable {
    fn item_tag(&self, _: &str) -> Option<Vec<String>> { None }
    fn item_exists(&self, _: &str) -> bool { false }
}
assert!(Unavailable.item_tag("example:tools").is_none());
```

### `TradeOfferContext`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The context a trade offer is generated in.

`TradeOfferContext<'a>`

> It carries `world: &'a dyn TradeWorld`, `position: [f64; 3]` and `variant: &'a str`.
> These are borrowed from the current generation request, not retained global merchant
> handles — when the request ends, the context goes with it.

```rust
use pluto_lib::{TradeOfferContext, TradeWorld};
struct World;
impl TradeWorld for World {
    fn item_tag(&self, _: &str) -> Option<Vec<String>> { Some(vec![]) }
    fn item_exists(&self, _: &str) -> bool { false }
}
let context = TradeOfferContext { world: &World, position: [1.0, 64.0, 2.0], variant: "minecraft:plains" };
assert_eq!(context.position[1], 64.0);
```

### `TradeOffer`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A fresh offer has `first_buy: StackData`, `second_buy: Option<StackData>`, `sell: StackData`, `max_uses: i32`, `experience: i32` and `price_multiplier: f32`.

> Native demand, uses and special prices belong to the native merchant after creation.
> Stack damage, max damage and custom data are not flattened to item IDs.

```rust
use pluto_lib::{StackData, TradeOffer};
let stack = |item: &str, count| StackData { item: item.into(), count, ..Default::default() };
let offer = TradeOffer {
    first_buy: stack("minecraft:emerald", 6), second_buy: None,
    sell: stack("minecraft:iron_pickaxe", 1), max_uses: 1,
    experience: 15, price_multiplier: 1.0,
};
assert_eq!(offer.first_buy.count, 6);
```

### `TradeOfferFactory`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A cloneable selected-offer callback.

> Cloning preserves callback identity/state and does not invoke it; duplicate candidate
> entries are retained.

```rust
use pluto_lib::TradeOfferFactory;
let factory = TradeOfferFactory::new(|_, _| Ok(None));
let weighted = vec![factory.clone(), factory];
assert_eq!(weighted.len(), 2);
```

### `TradeOfferFactory::new`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The callback can inspect registry context and consume the same RNG.

`fn new(create: impl Fn(&TradeOfferContext<'_>, &mut dyn TradeRandom) -> Result<Option<TradeOffer>, String> + Send + Sync + 'static) -> TradeOfferFactory`

> The callback can inspect the registry context and consume the same RNG the caller is
> using; it must not retain the borrowed context beyond the call. It answers the offer to
> add, `None` for none, or an error — the two stay distinct.

```rust
use pluto_lib::TradeOfferFactory;
let factory = TradeOfferFactory::new(|context, _| {
    if context.world.item_tag("example:tools").is_none() { return Ok(None); }
    Err("a real tool builder is required".into())
});
```

### `TradeOfferFactory::create`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Invokes exactly this selected listing.

`fn create(&self, context: &TradeOfferContext<'_>, random: &mut dyn TradeRandom) -> Result<Option<TradeOffer>, String>`

> None and error remain distinct; pool retry policy is the adapter's responsibility.

```rust
use pluto_lib::{TradeOfferContext, TradeOfferFactory, TradeRandom, TradeWorld};
struct World;
impl TradeWorld for World {
    fn item_tag(&self, _: &str) -> Option<Vec<String>> { None }
    fn item_exists(&self, _: &str) -> bool { false }
}
struct Random;
impl TradeRandom for Random {
    fn next_int(&mut self, _: i32) -> i32 { panic!("no RNG for this declined listing") }
    fn next_float(&mut self) -> f32 { panic!() }
    fn next_double(&mut self) -> f64 { panic!() }
}
let context = TradeOfferContext { world: &World, position: [0.0; 3], variant: "minecraft:plains" };
assert!(TradeOfferFactory::new(|_, _| Ok(None)).create(&context, &mut Random).unwrap().is_none());
assert!(TradeOfferFactory::new(|_, _| Err("failed".into())).create(&context, &mut Random).is_err());
```

### `TradeOfferHelper`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Fabric-shaped wandering-trader registration.

> Providers append candidate factories; they do not themselves edit a merchant's existing
> saved offers.

```rust
use pluto_lib::TradeOfferHelper;
TradeOfferHelper::register_wandering_trader_offers(1, Vec::new).unwrap();
```

### `TradeOfferHelper::register_wandering_trader_offers`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Accepts only levels 1 and 2.

`fn register_wandering_trader_offers(level: u32, provider: impl Fn() -> Vec<TradeOfferFactory> + Send + Sync + 'static) -> Result<(), String>`

> Providers are additive and may be invoked more than once; avoid generating tools or
> consuming RNG while building a pool.

```rust
use pluto_lib::TradeOfferHelper;
assert!(TradeOfferHelper::register_wandering_trader_offers(3, Vec::new).is_err());
TradeOfferHelper::register_wandering_trader_offers(2, Vec::new).unwrap();
```

### `TradeOfferHelper::wandering_trader_offers`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Gets an owned custom-candidate snapshot, preserving registration order and duplicates.

`fn wandering_trader_offers(level: u32) -> Result<Vec<TradeOfferFactory>, String>`

> This does not select listings or generate native merchant offers; adapters must append
> it to the appropriate native pool before selection.

```rust
use pluto_lib::{TradeOfferFactory, TradeOfferHelper};
TradeOfferHelper::register_wandering_trader_offers(2, || vec![TradeOfferFactory::new(|_, _| Ok(None))]).unwrap();
let snapshot = TradeOfferHelper::wandering_trader_offers(2).unwrap();
assert_eq!(snapshot.len(), 1);
```

# Base: events, identifiers, action results, support

The pieces every other module is built from. A mod author mostly meets `Event` (to register a callback), `Identifier` (to name things) and `ActionResult` (to answer an interaction).

## Basic Example

An `Event` is a list of callbacks. PlutoLib's events are all `static`; your mod registers a closure on them. A mod can also make its own event for other mods to hook:

```rust
use pluto_lib::{Event, Unit};

/// Fired when the machine finishes a recipe; the callbacks receive the recipe's name.
static RECIPE_DONE: Event<String, ()> = Event::new(&Unit);

RECIPE_DONE.register(|name| println!("made {name}"));
RECIPE_DONE.invoke(&"steel".to_owned());
```

## Choosing How Answers Combine

When callbacks return something, a combiner turns the list of answers into one. The three that ship cover the events of the game: `Event::void()` (no answer, every callback runs), `Event::first_not_pass()` (the first answer that is not `Pass` wins) and `Event::allow_all()` (any `false` vetoes).

```rust
use pluto_lib::{ActionResult, Event};

static USE: Event<(), ActionResult> = Event::first_not_pass();
USE.register(|_| ActionResult::Pass); // not mine
USE.register(|_| ActionResult::Fail); // cancel it
assert_eq!(USE.invoke(&()), ActionResult::Fail);
```

## Ordering Callbacks with Phases

Callbacks run in the order they were registered. When one mod must run before another, put it in a named phase and order the phases:

```rust
use pluto_lib::{default_phase, Event, Identifier};

static E: Event<(), ()> = Event::void();
let early = Identifier::new("example", "early").unwrap();
E.add_phase_ordering(early.clone(), default_phase()).unwrap();
E.register_with_phase(early, |_| { /* runs before callbacks registered with `register` */ });
E.register(|_| {});
E.invoke(&());
```

> **Tip.** A phase ordering that contradicts an earlier one is refused with `PhaseOrderingCycle` and the event keeps working, so one mod's wish can never break another mod's event.

## Reference

## Events

### `Event`

`Event<A, R>` is a list of callbacks. `A` is what each callback receives (by reference), `R` is what it returns. Callbacks run in registration order unless a phase says otherwise, and the combiner turns the list of answers into one `R`.

```rust
use pluto_lib::ServerLifecycleEvents;

// every PlutoLib event is a static Event; register a closure on it
ServerLifecycleEvents::SERVER_STARTING.register(|server| {
    let _ = server.dedicated;
});
```

A mod can also define its own event:

```rust
use pluto_lib::{Event, Unit};

static ON_SCORE: Event<u32, ()> = Event::new(&Unit);

ON_SCORE.register(|points| assert!(*points > 0));
ON_SCORE.invoke(&5);
```

### `Event::new`

`const fn new(combiner: &'static dyn Combiner<R>) -> Event<A, R>`. Makes an event with the given combiner. Use it in a `static`. The shorthand constructors below cover the usual combiners.

```rust
use pluto_lib::{Event, FirstNotPass, ActionResult};

static CLICKED: Event<u8, ActionResult> = Event::new(&FirstNotPass);
assert_eq!(CLICKED.invoke(&1), ActionResult::Pass); // no callback: the combiner's default
```

### `Event::void`

`const fn void() -> Event<A, ()>`. An event whose callbacks return nothing (every callback runs).

```rust
use pluto_lib::Event;

static PING: Event<(), ()> = Event::void();
PING.register(|_| {});
PING.invoke(&());
```

### `Event::first_not_pass`

`const fn first_not_pass() -> Event<A, ActionResult>`. Callbacks answer with an `ActionResult`; the first answer that is not `Pass` wins and stops the rest. With no such answer the result is `Pass`.

```rust
use pluto_lib::{ActionResult, Event};

static USE: Event<(), ActionResult> = Event::first_not_pass();
USE.register(|_| ActionResult::Pass);
USE.register(|_| ActionResult::Fail);
USE.register(|_| ActionResult::Success); // never reached
assert_eq!(USE.invoke(&()), ActionResult::Fail);
```

### `Event::allow_all`

`const fn allow_all() -> Event<A, bool>`. Callbacks answer yes (`true`) or no. Any `false` vetoes and stops the rest; all `true` (or none) allows.

```rust
use pluto_lib::Event;

static ALLOW: Event<i32, bool> = Event::allow_all();
ALLOW.register(|n| *n >= 0);
assert!(ALLOW.invoke(&3));
assert!(!ALLOW.invoke(&-1));
```

### `Event::register`

`fn register(&self, callback: impl Fn(&A) -> R + Send + Sync + 'static)`. Adds a callback in the default phase. Registering from inside a running callback is allowed.

```rust
use pluto_lib::Event;

static E: Event<String, ()> = Event::void();
E.register(|text| assert_eq!(text, "hi"));
E.invoke(&"hi".to_owned());
```

### `Event::register_with_phase`

`fn register_with_phase(&self, phase: Identifier, callback: ...)`. Adds a callback in a named phase. Phases run in the order set by `add_phase_ordering`; callbacks inside one phase keep registration order.

```rust
use pluto_lib::{Event, Identifier};

static E: Event<(), ()> = Event::void();
let late = Identifier::new("example", "late").unwrap();
E.register_with_phase(late, |_| {});
E.invoke(&());
```

### `Event::add_phase_ordering`

`fn add_phase_ordering(&self, first: Identifier, second: Identifier) -> Result<(), PhaseOrderingCycle>`. Says `first` runs before `second`. A contradicting ordering is refused and the event keeps working.

```rust
use pluto_lib::{Event, Identifier};

static E: Event<(), ()> = Event::void();
let a = Identifier::new("example", "a").unwrap();
let b = Identifier::new("example", "b").unwrap();
assert!(E.add_phase_ordering(a.clone(), b.clone()).is_ok());
assert!(E.add_phase_ordering(b, a).is_err()); // would make a cycle
```

### `Event::invoke`

`fn invoke(&self, args: &A) -> R`. Runs the callbacks and returns the combined answer. PlutoLib calls it for its own events; a mod calls it for events it defines (and tests use it to drive a callback).

```rust
use pluto_lib::Event;

static SUM: Event<u32, ()> = Event::void();
SUM.register(|n| assert_eq!(*n, 7));
SUM.invoke(&7);
```

### `Callback`

`type Callback<A, R> = Arc<dyn Fn(&A) -> R + Send + Sync>`. How an event stores a callback; a `Combiner` receives them.

```rust
use pluto_lib::Callback;
use std::sync::Arc;

let double: Callback<i32, i32> = Arc::new(|n| n * 2);
assert_eq!(double(&4), 8);
```

### `Combiner`

Trait that turns the callbacks into one result. Implement it for a custom event; the three provided ones (`Unit`, `FirstNotPass`, `AllowAll`) cover PlutoLib's events.

```rust
use pluto_lib::{Combiner, Event};

/// Adds up every callback's answer.
struct Sum;
impl Combiner<u32> for Sum {
    fn combine(&self, results: &mut dyn Iterator<Item = u32>) -> u32 {
        results.sum()
    }
}

static TOTAL: Event<(), u32> = Event::new(&Sum);
TOTAL.register(|_| 2);
TOTAL.register(|_| 3);
assert_eq!(TOTAL.invoke(&()), 5);
```

### `Unit`

Combiner for events whose callbacks return `()`: every callback runs.

```rust
use pluto_lib::{Event, Unit};

static E: Event<(), ()> = Event::new(&Unit);
E.invoke(&());
```

### `FirstNotPass`

Combiner for `ActionResult` events: the first answer that is not `Pass` wins.

```rust
use pluto_lib::{ActionResult, Event, FirstNotPass};

static E: Event<(), ActionResult> = Event::new(&FirstNotPass);
E.register(|_| ActionResult::Consume);
assert_eq!(E.invoke(&()), ActionResult::Consume);
```

### `AllowAll`

Combiner for `bool` events: any `false` vetoes.

```rust
use pluto_lib::{AllowAll, Event};

static E: Event<(), bool> = Event::new(&AllowAll);
E.register(|_| false);
assert!(!E.invoke(&()));
```

### `default_phase`

`fn default_phase() -> Identifier`. The phase callbacks join when registered with `register`. Order a custom phase before or after it.

```rust
use pluto_lib::{default_phase, Event, Identifier};

static E: Event<(), ()> = Event::void();
let early = Identifier::new("example", "early").unwrap();
E.add_phase_ordering(early.clone(), default_phase()).unwrap();
E.register_with_phase(early, |_| {});
```

### `PhaseOrderingCycle`

The error `add_phase_ordering` returns when two orderings contradict. The fields name the pair that was refused.

```rust
use pluto_lib::{Event, Identifier};

static E: Event<(), ()> = Event::void();
let a = Identifier::new("example", "a").unwrap();
let b = Identifier::new("example", "b").unwrap();
E.add_phase_ordering(a.clone(), b.clone()).unwrap();
let refused = E.add_phase_ordering(b.clone(), a.clone()).unwrap_err();
// first and second are the identifiers of the refused ordering
assert_eq!((refused.first, refused.second), (b, a));
```

## Interaction results

### `ActionResult`

What an interaction callback decided: `Success` (happened, swing the hand), `Consume` (happened, no swing), `ConsumePartial` (partly happened), `Pass` (not handled here) and `Fail` (cancel).

```rust
use pluto_lib::ActionResult;

let answers = [ActionResult::Success, ActionResult::Consume, ActionResult::ConsumePartial, ActionResult::Pass, ActionResult::Fail];
assert_eq!(answers.iter().filter(|a| a.is_accepted()).count(), 3);
```

### `ActionResult::is_accepted`

`fn is_accepted(self) -> bool`. True for every result except `Pass` and `Fail`.

```rust
use pluto_lib::ActionResult;

assert!(ActionResult::Consume.is_accepted());
assert!(!ActionResult::Pass.is_accepted());
assert!(!ActionResult::Fail.is_accepted());
```

### `ActionResult::should_swing_hand`

`fn should_swing_hand(self) -> bool`. True only for `Success`.

```rust
use pluto_lib::ActionResult;

assert!(ActionResult::Success.should_swing_hand());
assert!(!ActionResult::Consume.should_swing_hand());
```

## Identifiers

### `Identifier`

A namespaced name such as `tconstruct:flint_pickaxe`. The namespace allows `a-z 0-9 _ . -`, the path also `/`. Displays as `namespace:path`.

```rust
use pluto_lib::Identifier;

let id = Identifier::new("tconstruct", "tools/pickaxe").unwrap();
assert_eq!(id.to_string(), "tconstruct:tools/pickaxe");
```

### `Identifier::new`

`fn new(namespace: &str, path: &str) -> Result<Identifier, IdentifierError>`.

```rust
use pluto_lib::Identifier;

assert!(Identifier::new("example", "thing").is_ok());
assert!(Identifier::new("Example", "thing").is_err()); // upper case is refused
assert!(Identifier::new("example", "").is_err());      // so is an empty path
```

### `Identifier::parse`

`fn parse(text: &str) -> Result<Identifier, IdentifierError>`. Reads `namespace:path`; without a colon the namespace is `minecraft`.

```rust
use pluto_lib::Identifier;

assert_eq!(Identifier::parse("tconstruct:seared_brick").unwrap().namespace(), "tconstruct");
assert_eq!(Identifier::parse("stone").unwrap().to_string(), "minecraft:stone");
```

### `Identifier::namespace`

`fn namespace(&self) -> &str`.

```rust
use pluto_lib::Identifier;

assert_eq!(Identifier::new("example", "a").unwrap().namespace(), "example");
```

### `Identifier::path`

`fn path(&self) -> &str`.

```rust
use pluto_lib::Identifier;

assert_eq!(Identifier::new("example", "a/b").unwrap().path(), "a/b");
```

### `DEFAULT_NAMESPACE`

`const DEFAULT_NAMESPACE: &str = "minecraft"`. The namespace `Identifier::parse` uses when the text has none.

```rust
use pluto_lib::{Identifier, DEFAULT_NAMESPACE};

assert_eq!(Identifier::parse("dirt").unwrap().namespace(), DEFAULT_NAMESPACE);
```

### `IdentifierError`

Why an identifier was rejected: `InvalidNamespace` or `InvalidPath`, each with the offending text.

```rust
use pluto_lib::{Identifier, IdentifierError};

match Identifier::new("Bad Space", "x") {
    Err(IdentifierError::InvalidNamespace(text)) => assert_eq!(text, "Bad Space"),
    other => panic!("unexpected: {other:?}"),
}
```

## Support matrix

PlutoLib always offers the whole API. Where Horizon cannot do what a call means yet, the call exists, does nothing (or returns its documented default), and is listed here so missing behaviour is never silent.

### `Support`

Whether a call works on the pinned Horizon: `Implemented`, `Stub` (callable but inert: no host hook exists) or `BlockedByMixins` (inert: the pinned Horizon Mixins refuses the hook that would implement it).

```rust
use pluto_lib::Support;

assert_ne!(Support::Implemented, Support::Stub);
assert_ne!(Support::Stub, Support::BlockedByMixins);
```

### `SupportEntry`

One row of the matrix. `api` names the call, `support` says how far it works, `behaviour` says what it does meanwhile, `unblocked_by` says what host change makes it work.

```rust
use pluto_lib::MATRIX;

let row = &MATRIX[0];
assert!(!row.api.is_empty() && !row.behaviour.is_empty() && !row.unblocked_by.is_empty());
let _ = row.support;
```

### `MATRIX`

`const MATRIX: &[SupportEntry]`. Every call that is not fully implemented.

```rust
use pluto_lib::MATRIX;

assert!(MATRIX.iter().all(|row| !row.api.is_empty()));
```

### `entries_with`

`fn entries_with(support: Support) -> impl Iterator<Item = &'static SupportEntry>`. The rows in one class.

```rust
use pluto_lib::{entries_with, Support};

for row in entries_with(Support::Stub) {
    assert_eq!(row.support, Support::Stub);
}
```

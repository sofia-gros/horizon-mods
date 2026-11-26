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

When callbacks return something, a combiner turns the list of answers into one. The four that ship cover the events of the game: `Event::void()` (no answer, every callback runs), `Event::first_not_pass()` (the first answer that is not `Pass` wins), `Event::allow_all()` (any `false` vetoes) and `Event::first_some()` (the first `Some` answer wins).

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A list of callbacks that together answer one occurrence.

`Event<A, R>`

> `A` is what each callback receives by reference and `R` is what it returns. Callbacks
> run in registration order unless a phase says otherwise, and the combiner turns the list
> of answers into one `R`. PlutoLib's events are `static`s you register closures on, and a
> mod can define its own event the same way.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Makes an event with the given combiner.

`const fn new(combiner: &'static dyn Combiner<R>) -> Event<A, R>`

> Use it in a `static` to build an event whose answers combine in a custom way. The
> shorthand constructors below cover the usual combiners, so reach for `new` when none of
> them fits; with no callbacks registered, `invoke` returns the combiner's default.

```rust
use pluto_lib::{Event, FirstNotPass, ActionResult};

static CLICKED: Event<u8, ActionResult> = Event::new(&FirstNotPass);
assert_eq!(CLICKED.invoke(&1), ActionResult::Pass); // no callback: the combiner's default
```

### `Event::void`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An event whose callbacks return nothing.

`const fn void() -> Event<A, ()>`

> Every registered callback runs on `invoke`; with no return value there is nothing to
> combine, so no callback can stop or veto the others. Use it for notifications — the
> common case for a mod-defined event — and reach for the other constructors when
> callbacks must answer something.

```rust
use pluto_lib::Event;

static PING: Event<(), ()> = Event::void();
PING.register(|_| {});
PING.invoke(&());
```

### `Event::first_not_pass`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An event whose callbacks answer with an `ActionResult`; the first non-`Pass` answer wins.

`const fn first_not_pass() -> Event<A, ActionResult>`

> Callbacks run in order until one returns something other than `Pass`; that answer is the
> result and the rest never run. If every callback passes, or none is registered, the
> result is `Pass`. Use it for interactions, where the first handler to claim the action
> decides it.

```rust
use pluto_lib::{ActionResult, Event};

static USE: Event<(), ActionResult> = Event::first_not_pass();
USE.register(|_| ActionResult::Pass);
USE.register(|_| ActionResult::Fail);
USE.register(|_| ActionResult::Success); // never reached
assert_eq!(USE.invoke(&()), ActionResult::Fail);
```

### `Event::allow_all`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An event whose callbacks answer yes or no, and any no vetoes.

`const fn allow_all() -> Event<A, bool>`

> Each callback returns `true` or `false`; the first `false` stops the rest and the
> combined answer is `false`. If every callback allows, or none is registered, the answer
> is `true`. Use it for permission-style questions where any listener may veto.

```rust
use pluto_lib::Event;

static ALLOW: Event<i32, bool> = Event::allow_all();
ALLOW.register(|n| *n >= 0);
assert!(ALLOW.invoke(&3));
assert!(!ALLOW.invoke(&-1));
```

### `Event::first_some`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

An event whose callbacks answer with an option; the first `Some` wins.

`const fn first_some() -> Event<A, Option<T>>`

> A callback returns `Some` when it knows the answer for the request and `None` when it
> passes. The first `Some` stops the rest and becomes the result; if all pass, the result
> is `None`. Use it for lookups that one listener at a time can satisfy.

```rust
use pluto_lib::Event;

static LOOKUP: Event<i32, Option<&'static str>> = Event::first_some();
LOOKUP.register(|n| if *n == 1 { Some("one") } else { None });
LOOKUP.register(|n| if *n == 2 { Some("two") } else { None });
assert_eq!(LOOKUP.invoke(&2), Some("two"));
assert_eq!(LOOKUP.invoke(&3), None);
```

### `Event::register`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Adds a callback to an event, in the default phase.

`fn register(&self, callback: impl Fn(&A) -> R + Send + Sync + 'static)`

> The callback receives the event's argument by reference and returns its answer.
> Registering from inside a running callback is allowed. For ordering against other mods'
> callbacks, register in a named phase with `register_with_phase` instead.

```rust
use pluto_lib::Event;

static E: Event<String, ()> = Event::void();
E.register(|text| assert_eq!(text, "hi"));
E.invoke(&"hi".to_owned());
```

### `Event::register_with_phase`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Adds a callback to an event in a named phase.

`fn register_with_phase(&self, phase: Identifier, callback: ...)`

> Phases run in the order set by `add_phase_ordering`, and callbacks inside one phase keep
> registration order. Use a phase when your callback must run before or after another
> mod's; plain `register` puts a callback in the default phase.

```rust
use pluto_lib::{Event, Identifier};

static E: Event<(), ()> = Event::void();
let late = Identifier::new("example", "late").unwrap();
E.register_with_phase(late, |_| {});
E.invoke(&());
```

### `Event::add_phase_ordering`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Declares that one phase runs before another on this event.

`fn add_phase_ordering(&self, first: Identifier, second: Identifier) -> Result<(), PhaseOrderingCycle>`

> It says `first` runs before `second`. An ordering that contradicts an earlier one —
> closing a cycle — is refused with `PhaseOrderingCycle` and the event keeps working, so
> one mod's wish can never break another mod's event.

```rust
use pluto_lib::{Event, Identifier};

static E: Event<(), ()> = Event::void();
let a = Identifier::new("example", "a").unwrap();
let b = Identifier::new("example", "b").unwrap();
assert!(E.add_phase_ordering(a.clone(), b.clone()).is_ok());
assert!(E.add_phase_ordering(b, a).is_err()); // would make a cycle
```

### `Event::is_empty`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether no callback is registered on the event.

`fn is_empty(&self) -> bool`

> It lets the game pay for an expensive hook only when someone listens. Check it before
> doing costly work to produce an event's argument when nobody may be registered to hear
> it.

```rust
use pluto_lib::Event;

static E: Event<(), ()> = Event::void();
assert!(E.is_empty());
E.register(|_| {});
assert!(!E.is_empty());
```

### `Event::invoke`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Runs the event's callbacks and returns the combined answer.

`fn invoke(&self, args: &A) -> R`

> PlutoLib calls it for its own events; a mod calls it for events it defines, and tests
> use it to drive a callback. The combiner decides how the individual answers become the
> one returned value.

```rust
use pluto_lib::Event;

static SUM: Event<u32, ()> = Event::void();
SUM.register(|n| assert_eq!(*n, 7));
SUM.invoke(&7);
```

### `Callback`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The stored form of an event callback.

`type Callback<A, R> = Arc<dyn Fn(&A) -> R + Send + Sync>`

> An event stores each callback as a shared `Arc` closure taking the argument by reference
> and returning the answer. A `Combiner` receives the callbacks in this form when it
> combines their answers.

```rust
use pluto_lib::Callback;
use std::sync::Arc;

let double: Callback<i32, i32> = Arc::new(|n| n * 2);
assert_eq!(double(&4), 8);
```

### `Combiner`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The trait that turns an event's callback answers into one result.

> Implement it for a custom event whose answers combine in a way the provided combiners do
> not cover. The four provided ones — `Unit`, `FirstNotPass`, `AllowAll` and `FirstSome` —
> cover PlutoLib's own events and most mod events.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The combiner for events whose callbacks return nothing.

> Every callback runs, and there is no answer to combine. It is the combiner behind
> `Event::void()`; pass `&Unit` to `Event::new` when you build the event explicitly.

```rust
use pluto_lib::{Event, Unit};

static E: Event<(), ()> = Event::new(&Unit);
E.invoke(&());
```

### `FirstNotPass`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The combiner for `ActionResult` events.

> Callbacks run in order until one answers with something other than `Pass`; that answer
> is returned and the rest are skipped. If none does, the result is `Pass`. It is the
> combiner behind `Event::first_not_pass()`.

```rust
use pluto_lib::{ActionResult, Event, FirstNotPass};

static E: Event<(), ActionResult> = Event::new(&FirstNotPass);
E.register(|_| ActionResult::Consume);
assert_eq!(E.invoke(&()), ActionResult::Consume);
```

### `AllowAll`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The combiner for `bool` events.

> The combined answer is `false` as soon as one callback returns `false`, and `true` when
> every callback allows or none is registered. It is the combiner behind
> `Event::allow_all()`.

```rust
use pluto_lib::{AllowAll, Event};

static E: Event<(), bool> = Event::new(&AllowAll);
E.register(|_| false);
assert!(!E.invoke(&()));
```

### `FirstSome`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The combiner for `Option<T>` events.

> Callbacks answer `Some` when they know the answer and `None` to pass; the first `Some`
> is returned and the rest are skipped, and all `None` gives `None`. It is the combiner
> behind `Event::first_some()`.

```rust
use pluto_lib::{Event, FirstSome};

static E: Event<i32, Option<&'static str>> = Event::new(&FirstSome);
E.register(|n| if *n == 1 { Some("one") } else { None });
assert_eq!(E.invoke(&1), Some("one"));
assert_eq!(E.invoke(&2), None);
```

### `default_phase`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The phase callbacks join when registered with `register`.

`fn default_phase() -> Identifier`

> Order a custom phase before or after it with `add_phase_ordering` to place your
> callbacks relative to everyone who registered plainly.

```rust
use pluto_lib::{default_phase, Event, Identifier};

static E: Event<(), ()> = Event::void();
let early = Identifier::new("example", "early").unwrap();
E.add_phase_ordering(early.clone(), default_phase()).unwrap();
E.register_with_phase(early, |_| {});
```

### `PhaseOrderingCycle`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The error returned when two phase orderings contradict.

> Its fields name the pair of phases whose ordering was refused — the `first` and `second`
> identifiers of the call that would have closed the cycle. The event keeps its earlier
> orderings and keeps working.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

What an interaction callback decided.

> `Success` means the interaction happened and swings the hand, `Consume` that it happened
> without a swing, and `ConsumePartial` that it partly happened. `Pass` hands the
> interaction to the next handler and `Fail` cancels it outright. The helpers
> `is_accepted` and `should_swing_hand` read the answer.

```rust
use pluto_lib::ActionResult;

let answers = [ActionResult::Success, ActionResult::Consume, ActionResult::ConsumePartial, ActionResult::Pass, ActionResult::Fail];
assert_eq!(answers.iter().filter(|a| a.is_accepted()).count(), 3);
```

### `ActionResult::is_accepted`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether the result counts as the interaction happening.

`fn is_accepted(self) -> bool`

> True for every result except `Pass` and `Fail`: `Success`, `Consume` and
> `ConsumePartial` all mean the interaction was handled here.

```rust
use pluto_lib::ActionResult;

assert!(ActionResult::Consume.is_accepted());
assert!(!ActionResult::Pass.is_accepted());
assert!(!ActionResult::Fail.is_accepted());
```

### `ActionResult::should_swing_hand`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether the game should swing the player's hand for this result.

`fn should_swing_hand(self) -> bool`

> True only for `Success`. `Consume` reports an action that happened without a swing, and
> `Pass` means the interaction did not happen at all.

```rust
use pluto_lib::ActionResult;

assert!(ActionResult::Success.should_swing_hand());
assert!(!ActionResult::Consume.should_swing_hand());
```

## Identifiers

### `Identifier`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

A namespaced name, such as `tconstruct:flint_pickaxe`.

> The namespace allows `a-z 0-9 _ . -`, and the path also allows `/`. It displays as
> `namespace:path`, and every registry in the API is keyed by it. Build one with
> `Identifier::new`, or parse a single string with `Identifier::parse`.

```rust
use pluto_lib::Identifier;

let id = Identifier::new("tconstruct", "tools/pickaxe").unwrap();
assert_eq!(id.to_string(), "tconstruct:tools/pickaxe");
```

### `Identifier::new`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Builds an identifier from a namespace and a path, validating both parts.

`fn new(namespace: &str, path: &str) -> Result<Identifier, IdentifierError>`

> The Err case carries an `IdentifierError` naming which part was rejected and why, so bad
> data fails at construction instead of downstream. Use `Identifier::parse` instead when
> the name arrives as a single `namespace:path` string.

```rust
use pluto_lib::Identifier;

assert!(Identifier::new("example", "thing").is_ok());
assert!(Identifier::new("Example", "thing").is_err()); // upper case is refused
assert!(Identifier::new("example", "").is_err());      // so is an empty path
```

### `Identifier::parse`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Reads an identifier from a `namespace:path` string.

`fn parse(text: &str) -> Result<Identifier, IdentifierError>`

> Without a colon the namespace is `minecraft` (see `DEFAULT_NAMESPACE`). Both parts are
> validated as in `Identifier::new`, and a bad part fails with an `IdentifierError`.

```rust
use pluto_lib::Identifier;

assert_eq!(Identifier::parse("tconstruct:seared_brick").unwrap().namespace(), "tconstruct");
assert_eq!(Identifier::parse("stone").unwrap().to_string(), "minecraft:stone");
```

### `Identifier::namespace`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The namespace part of the identifier.

`fn namespace(&self) -> &str`

> The part before the colon — for example `tconstruct` in `tconstruct:flint_pickaxe`. The
> namespace names the mod or pack that owns the thing the identifier refers to.

```rust
use pluto_lib::Identifier;

assert_eq!(Identifier::new("example", "a").unwrap().namespace(), "example");
```

### `Identifier::path`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The path part of the identifier.

`fn path(&self) -> &str`

> The part after the colon — for example `flint_pickaxe` in `tconstruct:flint_pickaxe`.
> Together with the namespace it forms the full name every registry in the API is keyed
> by.

```rust
use pluto_lib::Identifier;

assert_eq!(Identifier::new("example", "a/b").unwrap().path(), "a/b");
```

### `DEFAULT_NAMESPACE`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The namespace assumed when an identifier string has none.

`const DEFAULT_NAMESPACE: &str = "minecraft"`

> `Identifier::parse` uses it when the text has no colon, so a bare name like `stone`
> reads as `minecraft:stone`.

```rust
use pluto_lib::{Identifier, DEFAULT_NAMESPACE};

assert_eq!(Identifier::parse("dirt").unwrap().namespace(), DEFAULT_NAMESPACE);
```

### `IdentifierError`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Why an identifier was rejected.

> Either `InvalidNamespace` or `InvalidPath`, each carrying the offending text, so a
> caller can report exactly which part of the name was bad and show it.

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

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

How far a call in the support matrix works.

> The classes are `Implemented` (the call does what it means), `Stub` (callable but inert:
> no host hook exists) and `BlockedByMixins` (inert: the pinned Mixins refuses the hook
> that would implement it). Each matrix row carries one of these classes, so missing
> behaviour is never silent.

```rust
use pluto_lib::Support;

assert_ne!(Support::Implemented, Support::Stub);
assert_ne!(Support::Stub, Support::BlockedByMixins);
```

### `SupportEntry`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

One row of the support matrix.

> Its fields are `api`, naming the call; `support`, saying how far it works; `behaviour`,
> saying what the call does meanwhile; and `unblocked_by`, saying what host change makes
> it work. Read a row to know exactly what a not-yet-implemented call does instead of
> guessing.

```rust
use pluto_lib::MATRIX;

let row = &MATRIX[0];
assert!(!row.api.is_empty() && !row.behaviour.is_empty() && !row.unblocked_by.is_empty());
let _ = row.support;
```

### `MATRIX`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Every call that is not fully implemented, as support rows.

`const MATRIX: &[SupportEntry]`

> Each row is a `SupportEntry` for one such call. Filter it with `entries_with` to list
> the rows of one support class at a time.

```rust
use pluto_lib::MATRIX;

assert!(MATRIX.iter().all(|row| !row.api.is_empty()));
```

### `entries_with`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

The support-matrix rows in one support class.

`fn entries_with(support: Support) -> impl Iterator<Item = &'static SupportEntry>`

> It yields the rows whose support class is the given one. Tools and pages use it to list,
> for one class at a time, which calls are implemented, which are stubs and which are
> unsupported.

```rust
use pluto_lib::{entries_with, Support};

for row in entries_with(Support::Stub) {
    assert_eq!(row.support, Support::Stub);
}
```

### `Nbt::parse_snbt`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Parses SNBT text into an NBT value.

`fn parse_snbt(text: &str) -> Result<Nbt, String>`

> SNBT is the game's stringified NBT. On failure the error says where the text stopped
> making sense, so bad input can be pointed at rather than merely rejected.

```rust
use pluto_lib::Nbt;

assert!(Nbt::parse_snbt("{count: 3, name: \"gem\"}").is_ok());
assert!(Nbt::parse_snbt("{count: 3").is_err());
```

### `compare_nbt`

![Horizon: works](badges/horizon-works.svg) ![Sunrise: works](badges/sunrise-works.svg)

Whether data carries the pattern in `expected`.

`fn compare_nbt(expected: &Nbt, actual: Option<&Nbt>) -> bool`

> The kinds must match exactly — a byte is not an int, and no number widens. A compound
> pattern matches when the data has at least its entries and every one matches; a list
> pattern matches when the data has at least its elements and every pattern element
> matches some data element; anything else must be equal. No data at all never matches.

```rust
use pluto_lib::{Nbt, compare_nbt};

let pattern = Nbt::parse_snbt("{count: 3}").unwrap();
let data = Nbt::parse_snbt("{count: 3, name: \"gem\"}").unwrap();
assert!(compare_nbt(&pattern, Some(&data)));
assert!(!compare_nbt(&pattern, None));
```

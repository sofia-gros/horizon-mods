# Block Behaviour

Say what a block of your mod does that its content file cannot say: where it may stand, which state it takes when placed, how a neighbour changes it, and what happens to an entity inside it. The block itself (its template, its state properties, its shapes, its loot) is declared in the mod's content files; a `BlockBehavior` is attached to it by id. This page first walks through using it, then lists every item in the [Reference](#reference).

## Basic Example

A hook that hangs from the block above it, and breaks when that block goes:

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Direction, Identifier};

BlockBehaviors::register(
    BlockBehavior::new(Identifier::new("example", "hook").unwrap())
        .can_survive(|c| c.view.is_face_sturdy(Direction::Up.relative(c.pos), Direction::Down)),
);
```

`c.view` looks at the blocks around; `Direction::Up.relative(c.pos)` is the block above. The game asks again whenever a neighbour changes, so the hook is removed (and drops its item) as soon as the block above stops being solid.

## Choosing the State When Placed

`state_for_placement` answers with a `Placement`: leave the game's choice alone, refuse, or give a state. This one faces away from the clicked side, and refuses water:

```rust
use pluto_lib::{BlockBehavior, Identifier, Placement};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .state_for_placement(|c| {
        if c.in_water {
            return Placement::Refuse;
        }
        Placement::State(c.state.clone().with("facing", c.face.opposite().name()))
    });
assert_eq!(behavior.block().to_string(), "example:post");
```

## Following the Neighbours

`update_shape` is asked when a neighbour changes. Answer with the new state, or `None` to keep it. This one remembers whether the block to its north is a post as well:

```rust
use pluto_lib::{BlockBehavior, Direction, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .update_shape(|c| {
        if c.direction != Direction::North {
            return None;
        }
        let joined = c.neighbour.as_ref().is_some_and(|n| n.block.path() == "post");
        Some(c.state.clone().with("joined", if joined { "true" } else { "false" }))
    });
assert!(behavior.block().path() == "post");
```

> **Tip.** The game asks at least six times for every block set. Keep `update_shape` short, and answer `None` when nothing changes.

## Testing

`MemoryBlockView` is a small world for tests: the blocks you set are solid, everything else is air. Build the context yourself and call your own functions:

```rust
use pluto_lib::{BlockState, BlockView, Direction, MemoryBlockView};

let mut view = MemoryBlockView::default();
assert!(view.is_air([0, 65, 0]));
view.blocks.insert([0, 65, 0], BlockState::parse("minecraft:stone").unwrap());
assert!(view.is_face_sturdy(Direction::Up.relative([0, 64, 0]), Direction::Down));
```

## Reference

### `Direction`

One of the six block faces: `Down`, `Up`, `North`, `South`, `West`, `East`.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::North.name(), "north");
assert_ne!(Direction::Up, Direction::Down);
```

### `Direction::ALL`

All six directions, in the game's order: down, up, north, south, west, east.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::ALL.len(), 6);
assert_eq!(Direction::ALL[0], Direction::Down);
```

### `Direction::name`

`fn name(self) -> &'static str`. The name a block state spells it with: `down`, `up`, `north`, `south`, `west`, `east`.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::East.name(), "east");
```

### `Direction::parse`

`fn parse(name: &str) -> Option<Direction>`. Reads a name written by `name`; None for anything else.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::parse("west"), Some(Direction::West));
assert_eq!(Direction::parse("sideways"), None);
```

### `Direction::offset`

`fn offset(self) -> [i32; 3]`. The step one block in this direction takes: `[0, 1, 0]` for up, `[0, 0, -1]` for north.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::Up.offset(), [0, 1, 0]);
assert_eq!(Direction::North.offset(), [0, 0, -1]);
```

### `Direction::from_offset`

`fn from_offset(offset: [i32; 3]) -> Option<Direction>`. Reads a step written by `offset`; None for a step that is not one block along an axis.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::from_offset([1, 0, 0]), Some(Direction::East));
assert_eq!(Direction::from_offset([1, 1, 0]), None);
```

### `Direction::opposite`

`fn opposite(self) -> Direction`. The other way: down for up, north for south.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::South.opposite(), Direction::North);
```

### `Direction::clockwise`

`fn clockwise(self) -> Direction`. A quarter turn clockwise seen from above (north, east, south, west); up and down stay as they are.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::North.clockwise(), Direction::East);
assert_eq!(Direction::Down.clockwise(), Direction::Down);
```

### `Direction::relative`

`fn relative(self, pos: BlockPos) -> BlockPos`. The position one block from `pos` in this direction.

```rust
use pluto_lib::Direction;

assert_eq!(Direction::West.relative([5, 64, 5]), [4, 64, 5]);
```

### `BlockView`

What a behaviour can see of the blocks around, with the game's own answers: `block_state(pos) -> Option<BlockState>` is the state with its properties (None where nothing is loaded), `is_air(pos) -> bool`, `is_face_sturdy(pos, face) -> bool` whether the whole face is solid, and `is_in_tag(pos, tag) -> bool` whether the block is in a block tag such as `tconstruct:slimy_leaves`.

```rust
use pluto_lib::{BlockState, BlockView, Direction};

/// A view of a world with one stone block.
struct OneStone;
impl BlockView for OneStone {
    fn block_state(&self, pos: [i32; 3]) -> Option<BlockState> {
        (pos == [0, 0, 0]).then(|| BlockState::parse("minecraft:stone").unwrap())
    }
    fn is_air(&self, pos: [i32; 3]) -> bool { pos != [0, 0, 0] }
    fn is_face_sturdy(&self, pos: [i32; 3], _face: Direction) -> bool { pos == [0, 0, 0] }
    fn is_in_tag(&self, _pos: [i32; 3], tag: &str) -> bool { tag == "minecraft:base_stone_overworld" }
}

assert!(OneStone.is_face_sturdy([0, 0, 0], Direction::Up));
assert!(OneStone.is_air([1, 0, 0]));
```

### `MemoryBlockView`

A `BlockView` for tests. `blocks` holds the states set (each position in it is solid and not air), `weak` the positions that hold a block but not a sturdy one (a slab's top, glass), `tags` the tags the block at each position is in.

```rust
use pluto_lib::{BlockState, BlockView, Direction, MemoryBlockView};

let mut view = MemoryBlockView::default();
view.blocks.insert([0, 0, 0], BlockState::parse("minecraft:glass").unwrap());
view.weak.insert([0, 0, 0]);
assert!(!view.is_air([0, 0, 0]));
assert!(!view.is_face_sturdy([0, 0, 0], Direction::Up));
assert_eq!(view.block_state([0, 0, 0]).unwrap().to_string(), "minecraft:glass");
```

### `SurviveContext`

Where a block is asked whether it may stay. `state` is the block's state (it may not be in the world yet), `pos` the position and `view` the blocks around.

```rust
use pluto_lib::{BlockState, MemoryBlockView, SurviveContext};

let view = MemoryBlockView::default();
let context = SurviveContext { state: BlockState::parse("example:hook").unwrap(), pos: [0, 64, 0], view: &view };
assert_eq!(context.pos, [0, 64, 0]);
assert_eq!(context.state.to_string(), "example:hook");
assert!(context.view.is_air([0, 65, 0]));
```

### `PlacementContext`

What a placement is told. `state` is the block's default state, `pos` where it would go, `face` the face that was clicked, `yaw` and `pitch` the placer's turn and look in degrees, `sneaking` whether the placer sneaks, `in_water` whether the position holds water now, `item` the item in hand and `view` the blocks around.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, PlacementContext};

let view = MemoryBlockView::default();
let context = PlacementContext {
    state: BlockState::parse("example:post").unwrap(),
    pos: [1, 64, 1],
    face: Direction::Up,
    yaw: 90.0,
    pitch: 0.0,
    sneaking: false,
    in_water: false,
    item: "example:post".into(),
    view: &view,
};
assert_eq!(context.face.opposite(), Direction::Down);
assert!(!context.sneaking && !context.in_water);
assert_eq!((context.pos, context.yaw, context.pitch), ([1, 64, 1], 90.0, 0.0));
assert_eq!(context.item, "example:post");
assert_eq!(context.state.to_string(), "example:post");
assert!(context.view.is_air([1, 63, 1]));
```

### `Placement`

What a placement becomes: `Placement::Default` lets the game's own placement run, `Placement::Refuse` places nothing and keeps the item, `Placement::State(state)` places the block in `state` (it has to be a state of the same block, with the properties declared).

```rust
use pluto_lib::{BlockState, Placement};

let state = BlockState::parse("example:post[lit=true]").unwrap();
assert_ne!(Placement::Default, Placement::Refuse);
assert_eq!(Placement::State(state.clone()), Placement::State(state));
```

### `ShapeUpdateContext`

A neighbour changed. `state` is the block's state, `pos` its position, `direction` the side the change came from, `neighbour_pos` the neighbour's position, `neighbour` the neighbour's state if it is a block with a behaviour, and `view` the blocks around.

```rust
use pluto_lib::{BlockState, Direction, MemoryBlockView, ShapeUpdateContext};

let view = MemoryBlockView::default();
let context = ShapeUpdateContext {
    state: BlockState::parse("example:post").unwrap(),
    pos: [0, 64, 0],
    direction: Direction::North,
    neighbour_pos: [0, 64, -1],
    neighbour: None,
    view: &view,
};
assert_eq!(context.direction.relative(context.pos), context.neighbour_pos);
assert!(context.neighbour.is_none());
assert_eq!(context.state.block.path(), "post");
assert!(context.view.is_air(context.neighbour_pos));
```

### `EntityInsideContext`

An entity is inside the block. `dimension` is the dimension id, `state` the block's state, `pos` its position, `entity` the entity's id and `kind` its type.

```rust
use pluto_lib::{BlockState, EntityInsideContext};

let context = EntityInsideContext {
    dimension: "minecraft:overworld".into(),
    state: BlockState::parse("example:post").unwrap(),
    pos: [0, 64, 0],
    entity: 12,
    kind: "minecraft:zombie".into(),
};
assert_eq!(context.kind, "minecraft:zombie");
assert_eq!((context.entity, context.pos), (12, [0, 64, 0]));
assert_eq!(context.dimension, "minecraft:overworld");
assert_eq!(context.state.block.path(), "post");
```

### `BlockBehavior`

What one block of the mod does. Build it with the methods below, then give it to `BlockBehaviors::register`.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.block().path(), "post");
```

### `BlockBehavior::new`

`fn new(block: Identifier) -> BlockBehavior`. A behaviour for `block` that changes nothing yet.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.block().to_string(), "example:post");
```

### `BlockBehavior::can_survive`

`fn can_survive(self, f: impl Fn(&SurviveContext) -> bool) -> BlockBehavior`. Where the block may stay. Answer `false` and it is refused when placed, and breaks (with its drops) when a neighbour changes. The game asks often, so keep it short.

```rust
use pluto_lib::{BlockBehavior, Direction, Identifier};

// a sign that needs the block below
let behavior = BlockBehavior::new(Identifier::new("example", "sign").unwrap())
    .can_survive(|c| c.view.is_face_sturdy(Direction::Down.relative(c.pos), Direction::Up));
assert!(behavior.block().path() == "sign");
```

### `BlockBehavior::state_for_placement`

`fn state_for_placement(self, f: impl Fn(&PlacementContext) -> Placement) -> BlockBehavior`. The state the block takes when placed (see [Choosing the State When Placed](#choosing-the-state-when-placed)).

```rust
use pluto_lib::{BlockBehavior, Identifier, Placement};

// never placed by a sneaking player
let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .state_for_placement(|c| if c.sneaking { Placement::Refuse } else { Placement::Default });
assert!(behavior.block().path() == "post");
```

### `BlockBehavior::update_shape`

`fn update_shape(self, f: impl Fn(&ShapeUpdateContext) -> Option<BlockState>) -> BlockBehavior`. How a neighbour's change turns the block's state: `Some(state)` becomes the block's state, `None` leaves it. Asked at least six times for every block set, so keep it short.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap())
    .update_shape(|c| (c.state.property("powered") == Some("true")).then(|| c.state.clone().with("powered", "false")));
assert!(behavior.block().path() == "post");
```

### `BlockBehavior::entity_inside`

`fn entity_inside(self, f: impl Fn(&EntityInsideContext)) -> BlockBehavior`. An entity is inside the block's space. Stub: the callbacks are accepted and not called yet (see the support matrix).

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "thorns").unwrap())
    .entity_inside(|c| println!("{} touched the thorns at {:?}", c.kind, c.pos));
assert!(behavior.block().path() == "thorns");
```

### `BlockBehavior::block`

`fn block(&self) -> &Identifier`. The block this behaviour belongs to.

```rust
use pluto_lib::{BlockBehavior, Identifier};

let behavior = BlockBehavior::new(Identifier::new("example", "post").unwrap());
assert_eq!(behavior.block().namespace(), "example");
```

### `BlockBehaviors`

The registry of block behaviours.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "registry_page").unwrap()));
assert!(BlockBehaviors::get("example:registry_page").is_some());
```

### `BlockBehaviors::register`

`fn register(behavior: BlockBehavior)`. Gives `behavior.block()` its behaviour. Registering the same block again replaces the earlier behaviour.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

let id = Identifier::new("example", "twice").unwrap();
BlockBehaviors::register(BlockBehavior::new(id.clone()).can_survive(|_| false));
BlockBehaviors::register(BlockBehavior::new(id).can_survive(|_| true));
assert_eq!(BlockBehaviors::registered().iter().filter(|b| b.block().to_string() == "example:twice").count(), 1);
```

### `BlockBehaviors::get`

`fn get(block: &str) -> Option<BlockBehavior>`. The behaviour registered for `block` (an id such as `tconstruct:punji`), if any.

```rust
use pluto_lib::BlockBehaviors;

assert!(BlockBehaviors::get("example:never_registered").is_none());
```

### `BlockBehaviors::registered`

`fn registered() -> Vec<BlockBehavior>`. Every block that has a behaviour.

```rust
use pluto_lib::{BlockBehavior, BlockBehaviors, Identifier};

BlockBehaviors::register(BlockBehavior::new(Identifier::new("example", "listed").unwrap()));
assert!(BlockBehaviors::registered().iter().any(|b| b.block().to_string() == "example:listed"));
```

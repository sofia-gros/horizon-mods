# Highlight

A mod outlines the extra blocks its held tool is about to affect: the hammer's whole seam, the
brush's brushable neighbours, the neighbours a mining tool is effective on. While the local player holds a tool and aims at a block, a provider
registered with `HighlightRegistry` answers the extra positions its own hooks would highlight, and
the game draws the block outline around each of them — the shape of the original's render event
over the area-of-effect iterator's DISPLAY predicate, whose per-module `shouldHighlight` answers
decide which neighbours are outlined, and whose effective-tool half outlines the neighbours the
held tool is effective on. The aimed block itself is never a provider's answer: the
game draws its own outline around the target, and the original's event adds only the extra blocks.

```rust
use pluto_lib::highlight_api_v1::{HighlightFacts, HighlightRegistry};

// Outline the block above the aimed one; answer nothing without a target.
HighlightRegistry::register(|facts: &HighlightFacts<'_>| {
    match &facts.target {
        Some(target) => vec![[target.pos[0], target.pos[1] + 1, target.pos[2]]],
        None => Vec::new(),
    }
});
```

Every ask hands the provider the facts of the frame: the stack in the player's main hand, the
block the view rests on (its position, the face the view enters through, what it is, and its
distance), the player's yaw and pitch, its block reach for the player's game mode, whether a
position sits inside the world border, and reads of the blocks around the target — the reads an
area-of-effect iterator walks by. In the original the positions are recomputed on every render
event from the player's current aim; here the transport asks every provider once a frame while
the world is drawn, so an answer lives exactly as long as the aim that causes it, and a provider
that owns nothing in the facts answers an empty set.

> **Use with Caution.** The registry and the merge are pure-tested. The transport rides the
> game's native world-primitives hook — the frame seam the game asks its mods through each
> frame the world is drawn — so no injection carries it, and the view a provider reads is the
> local player's own, read through the host on the client (native-unvalidated).

## Reference

### `HighlightTarget`

The block the local player's view rests on: `pos` is its position, `face` the face the view enters it through as a unit offset (the face an area-of-effect iterator orients itself by), `block` what it is, and `distance` how far along the view it sits, in blocks.

```rust
use pluto_lib::Identifier;
use pluto_lib::highlight_api_v1::HighlightTarget;

let target = HighlightTarget { pos: [1, 64, -3], face: [0, 0, 1], block: Identifier::new("minecraft", "suspicious_sand").unwrap(), distance: 2.5 };
assert_eq!(target.pos, [1, 64, -3]);
assert_eq!(target.face, [0, 0, 1]);
assert_eq!(target.distance, 2.5);
```

### `HighlightFacts`

The facts of one frame's outline ask: `main_hand` is the stack in the player's main hand (a tool's modifiers and persistent data ride in its custom data), or None when the hand is empty or its stack cannot be named; `target` is the block the view rests on, or None when it rests on nothing; `yaw` and `pitch` are the player's look in degrees; `block_reach` is the block reach the distance gate uses (the `minecraft:block_interaction_range` attribute's base value for the player's game mode: 4.5 in survival, 5.0 in creative); `in_world_border` answers whether a position sits inside the world border (positions outside it are never outlined, and do not spend one of the sixty slots); `block_at` reads the block at a position near the view (None where nothing is loaded); and `in_tag_at` answers whether the block at a position is in a block tag.

```rust
use pluto_lib::highlight_api_v1::HighlightFacts;

let facts = HighlightFacts { main_hand: None, target: None, yaw: 90.0, pitch: 0.0, block_reach: 5.0, in_world_border: &|_| true, block_at: &|_| None, in_tag_at: &|_, _| false };
assert!(facts.main_hand.is_none());
assert!(facts.target.is_none());
assert_eq!(facts.yaw, 90.0);
assert_eq!(facts.block_reach, 5.0);
assert!((facts.in_world_border)([0, 64, 0]));
assert!((facts.block_at)([0, 64, 0]).is_none());
```

### `HighlightProvider`

`type HighlightProvider = fn(&HighlightFacts<'_>) -> Vec<[i32; 3]>`. Answers the extra block positions one mod's hooks highlight under the facts, or an empty set when they highlight nothing this frame.

```rust
use pluto_lib::highlight_api_v1::{HighlightFacts, HighlightProvider};

fn nothing(_: &HighlightFacts<'_>) -> Vec<[i32; 3]> { Vec::new() }
let provider: HighlightProvider = nothing;
let facts = HighlightFacts { main_hand: None, target: None, yaw: 0.0, pitch: 0.0, block_reach: 5.0, in_world_border: &|_| true, block_at: &|_| None, in_tag_at: &|_, _| false };
assert!(provider(&facts).is_empty());
```

### `HighlightRegistry`

The registry of highlight providers. Register once during startup; the transport asks every registered provider on each frame the world is drawn and draws the merged set as block outlines.

```rust
use pluto_lib::highlight_api_v1::{HighlightFacts, HighlightRegistry, highlight_positions};

HighlightRegistry::register(|_| vec![[1, 64, -3]]);
let facts = HighlightFacts { main_hand: None, target: None, yaw: 0.0, pitch: 0.0, block_reach: 5.0, in_world_border: &|_| true, block_at: &|_| None, in_tag_at: &|_, _| false };
assert_eq!(highlight_positions(&facts), vec![[1, 64, -3]]);
```

### `HighlightRegistry::register`

`fn register(provider: HighlightProvider)`. Registers a highlight provider; registering the same provider again does not register it twice.

```rust
use pluto_lib::highlight_api_v1::{HighlightFacts, HighlightProvider, HighlightRegistry, highlight_positions};

fn one(_: &HighlightFacts<'_>) -> Vec<[i32; 3]> { vec![[1, 64, -3]] }
let provider: HighlightProvider = one;
HighlightRegistry::register(provider);
HighlightRegistry::register(provider);
let facts = HighlightFacts { main_hand: None, target: None, yaw: 0.0, pitch: 0.0, block_reach: 5.0, in_world_border: &|_| true, block_at: &|_| None, in_tag_at: &|_, _| false };
assert_eq!(highlight_positions(&facts), vec![[1, 64, -3]], "registered once, drawn once");
```

### `highlight_positions`

`fn highlight_positions(facts: &HighlightFacts<'_>) -> Vec<[i32; 3]>`. The merged outline set over the facts: every registered provider's positions, in the order they answered, each position once, cut to `MAX_HIGHLIGHTS`. A position outside the world border never enters the set, and does not spend one of the sixty slots. Empty when no provider highlights anything.

```rust
use pluto_lib::highlight_api_v1::{HighlightFacts, HighlightRegistry, highlight_positions};

let facts = HighlightFacts { main_hand: None, target: None, yaw: 0.0, pitch: 0.0, block_reach: 5.0, in_world_border: &|_| true, block_at: &|_| None, in_tag_at: &|_, _| false };
HighlightRegistry::register(|_| vec![[1, 64, -3], [1, 64, -3]]);
assert_eq!(highlight_positions(&facts), vec![[1, 64, -3]], "each position is drawn once");
```

### `SURVIVAL_BLOCK_REACH`

`pub const SURVIVAL_BLOCK_REACH: f32 = 4.5`. The block reach in survival (and adventure and spectator): the `minecraft:block_interaction_range` attribute's base value, as the game recomputes it every tick.

```rust
use pluto_lib::highlight_api_v1::SURVIVAL_BLOCK_REACH;

assert_eq!(SURVIVAL_BLOCK_REACH, 4.5);
```

### `CREATIVE_BLOCK_REACH`

`pub const CREATIVE_BLOCK_REACH: f32 = 5.0`. The block reach in creative: the same attribute's base value there.

```rust
use pluto_lib::highlight_api_v1::CREATIVE_BLOCK_REACH;

assert_eq!(CREATIVE_BLOCK_REACH, 5.0);
```

### `block_reach_for_mode`

`fn block_reach_for_mode(mode: i32) -> f32`. The block reach for game mode `mode` (0 survival, 1 creative, 2 adventure, 3 spectator, as the game numbers its modes): creative reaches 5.0, every other mode 4.5. The transport reads the mode the game's mode setter last left, since no host read states the local player's mode on the render path.

```rust
use pluto_lib::highlight_api_v1::{block_reach_for_mode, SURVIVAL_BLOCK_REACH, CREATIVE_BLOCK_REACH};

assert_eq!(block_reach_for_mode(0), SURVIVAL_BLOCK_REACH, "survival");
assert_eq!(block_reach_for_mode(1), CREATIVE_BLOCK_REACH, "creative");
assert_eq!(block_reach_for_mode(2), SURVIVAL_BLOCK_REACH, "adventure reaches as survival does");
assert_eq!(block_reach_for_mode(3), SURVIVAL_BLOCK_REACH, "spectator reaches as survival does");
```

### `MAX_HIGHLIGHTS`

`pub const MAX_HIGHLIGHTS: usize = 60`. The most extra blocks outlined at once, across every provider (the original's `MAX_BLOCKS` in its render event): the merged set is cut to this, in the order providers answered.

```rust
use pluto_lib::highlight_api_v1::MAX_HIGHLIGHTS;

assert_eq!(MAX_HIGHLIGHTS, 60);
```

### `OUTLINE_ARGB`

`pub const OUTLINE_ARGB: u32 = 0x6600_0000`. The outline's colour, as the game draws a targeted block's own: black at 40% opacity, ARGB.

```rust
use pluto_lib::highlight_api_v1::OUTLINE_ARGB;

assert_eq!(OUTLINE_ARGB >> 24, 0x66, "black at 40% opacity");
assert_eq!(OUTLINE_ARGB & 0xFF_FFFF, 0);
```

### `OUTLINE_WIDTH`

`pub const OUTLINE_WIDTH: f32 = 2.0`. The outline's line width in pixels, as the game draws a targeted block's own.

```rust
use pluto_lib::highlight_api_v1::OUTLINE_WIDTH;

assert_eq!(OUTLINE_WIDTH, 2.0);
```

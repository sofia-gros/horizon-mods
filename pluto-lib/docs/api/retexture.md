# Retextured blocks: a block entity's texture on its model

A retextured block is made from a material — a crafting station from oak logs, a smeltery drain from seared bricks — and the original draws the block with that material's texture on the model's retextured slots, while the rest of the model keeps its fixed textures. The block entity carries the material under `texture` (see the consumer's `retextured.rs`: recipe to stack, stack to block entity on placement, block entity to the drop). This page covers the mesh side: which baked faces belong to a retextured slot, and how the mesher swaps their sprite layer. It first walks through the pure face test, then lists every public item in the [Reference](#reference).

> **Use with Caution.** The world-block path is source-wired and pure-tested; the native weave plan and the in-game look remain unverified. Item models (a retextured drain held in the hand or sitting in a chest) keep their default texture: the host bakes item models once per item, with no per-stack model data to swap on, so the item side is a documented remainder, not a claim.

## Basic Example

A crafting station made from oak logs draws its legs with the log texture. The legs sample the same sprite as the table's side, so the sprite alone cannot tell them apart — the UV rectangle can:

```rust
use pluto_lib::is_retextured_face;

// A leg face: the sprite `crafting_station_side`, UVs [12, 4, 16, 16].
let leg = [[0.75, 0.25], [1.0, 0.25], [1.0, 1.0], [0.75, 1.0]];
assert!(is_retextured_face(
    "tconstruct:crafting_station",
    "tconstruct:block/table/crafting_station_side",
    leg,
));
// The table top plate's side face: the same sprite, UVs [0, 0, 16, 4].
let side = [[0.0, 0.0], [1.0, 0.0], [1.0, 0.25], [0.0, 0.25]];
assert!(!is_retextured_face(
    "tconstruct:crafting_station",
    "tconstruct:block/table/crafting_station_side",
    side,
));
```

## How the swap works

The host bakes each block model once, so a retextured block and a plain one share one baked model. The mesh seam therefore runs per face, at `Mesher::face`: the `pluto-retexture-layer` expression injection replaces the `face.layer as u16` value that feeds the face's own tile with the texture block's particle layer — but only when the pure test above says the face is a retextured slot and the block entity at that position names a texture. The injection is an expression value, not a second `MODIFY_VARIABLE` on the same function, because the block-tint injection already owns that place.

The texture reaches the mesher through a position cache: the client block-entity tick pass copies each loaded block entity's `texture` record into it once a tick, and a level replacement clears it. The seam re-checks the current block against the pure table on every face, so a stale cache entry for a position whose block changed cannot swap a face it should not.

The replacement sprite is the texture item's block's particle sprite, read through the host model catalog. Mantle's sources are not in this tree, so the exact `getTexture` resolution is unverified from source; the smoke check — a retextured drain in seared bricks against one in seared stone — decides whether the particle sprite matches the original's look.

## Reference

### `RetextureSlot`

One retextured texture slot of a block's model: the faces whose baked sprite and UVs it names take the block entity's texture instead. `sprite` is the baked sprite name (`namespace:path`) the slot shows by default, and `uv` lists the UV rectangles, in texels `[u0, v0, u1, v1]`, the slot's faces sample; an empty `uv` means the sprite alone names the slot.

```rust
use pluto_lib::RetextureSlot;

// A table leg: the side sprite, sampled in two 4-wide strips.
let slot = RetextureSlot {
    sprite: "tconstruct:block/table/side",
    uv: &[[12, 4, 16, 16], [0, 4, 4, 16]],
};
assert_eq!(slot.sprite, "tconstruct:block/table/side");
assert_eq!(slot.uv.len(), 2);
```

### `slots_for`

The retextured slots of a block, in the original's model order; empty when the block has no retextured model. The fourteen blocks are the ones the original's retextured recipes can make.

```rust
use pluto_lib::slots_for;

let slots = slots_for("tconstruct:crafting_station");
assert_eq!(slots.len(), 2);
assert_eq!(slots[0].sprite, "tconstruct:block/table/crafting_station_side");
assert_eq!(slots[1].sprite, "tconstruct:block/table/bottom");
assert!(slots_for("minecraft:oak_log").is_empty());
```

### `uv_rect`

The texel rectangle a baked quad's UVs cover, from its four normalized corners: the minimum and maximum of each axis, scaled back to texels. The corners arrive in model order (a face `rotation` permutes them), so the rectangle is the same whichever corner the bake started from.

```rust
use pluto_lib::uv_rect;

assert_eq!(uv_rect([[0.75, 0.25], [1.0, 0.25], [1.0, 1.0], [0.75, 1.0]]), [12, 4, 16, 16]);
```

### `is_retextured_face`

Whether a baked face of `block` belongs to one of its retextured slots: its sprite names a slot and its UVs fall inside one of that slot's rectangles (any rectangle when the slot names none). A face of a block with no retextured model is never one.

```rust
use pluto_lib::is_retextured_face;

let whole = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
assert!(is_retextured_face("tconstruct:seared_drain", "tconstruct:block/smeltery/seared/bricks", whole));
assert!(!is_retextured_face("tconstruct:seared_drain", "tconstruct:block/smeltery/io/back", whole));
```

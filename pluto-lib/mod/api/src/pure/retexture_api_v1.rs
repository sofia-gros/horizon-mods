//! Retextured block rendering: which baked faces of a block's model take the
//! texture its block entity names (Mantle's `RetexturedHelper` / `RetexturedModel`).
//!
//! The original chooses through model data: a block model whose JSON declares
//! `loader: "mantle:retextured"` (the four tables and the smeltery/foundry
//! templates) or `loader: "tconstruct:material_block"` (the two anvils) names
//! `retextured` texture slots, and the block entity's `texture` record — the
//! item id of the material the block was made from — replaces the sprite those
//! slots sample. The host bakes each model once, with the slots' default
//! sprites, and its mesher draws a face by the baked sprite layer, so the swap
//! has to happen at mesh time, face by face.
//!
//! A baked face carries no texture-variable name, only the resolved sprite
//! name and its UVs, so a slot is matched by (sprite, UV rectangle): the
//! table below is read off the original model JSONs, and it is exact. The only
//! models where a retextured slot shares its default sprite with a fixed face
//! are the four tables, and there the UV rectangles differ: the legs sample
//! 4-wide strips while the table's side samples a 16-wide strip, and the leg
//! bottoms sample 4x4 corners while the table's underside samples the whole
//! sprite. The host bakes UVs without `uv_lock` for these blockstates, so a
//! baked face's UV rectangle is the model JSON's rectangle, whichever way the
//! block faces.
//!
//! The replacement sprite is the texture item's block's particle sprite — the
//! block's representative sprite, as the host's model catalog exposes it.
//! (Mantle's sources are not in this tree, so the exact `getTexture`
//! resolution is unverified from source; the in-game smoke check — a
//! retextured drain in seared bricks against one in seared stone — decides
//! whether the particle sprite matches the original's look.) The pure side
//! only answers "is this face a retextured slot"; the woven mesh seam owns
//! the position-to-texture cache and the layer lookup.

/// One retextured texture slot of a block's model: the faces whose baked
/// sprite and UVs it names take the block entity's texture instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetextureSlot {
    /// The baked sprite name (`namespace:path`) the slot shows by default.
    pub sprite: &'static str,
    /// The UV rectangles, in texels `[u0, v0, u1, v1]`, the slot's faces
    /// sample; empty when the sprite alone names the slot (the anvil top:
    /// the fallback sprite appears on no other face of the anvil model).
    pub uv: &'static [[u32; 4]],
}

/// The table legs' side faces: 4-wide strips at the sprite's left and right.
const LEG_UV: &[[u32; 4]] = &[[12, 4, 16, 16], [0, 4, 4, 16]];

/// The table legs' undersides: one 4x4 corner per leg.
const LEG_BOTTOM_UV: &[[u32; 4]] = &[[0, 12, 4, 16], [12, 12, 16, 16], [12, 0, 16, 4], [0, 0, 4, 4]];

/// A whole-sprite face: the smeltery and foundry brick faces.
const FULL_UV: &[[u32; 4]] = &[[0, 0, 16, 16]];

/// No UV restriction: the sprite alone names the slot.
const ANY_UV: &[[u32; 4]] = &[];

/// The retextured slots of a block, in the original's model order; empty when
/// the block has no retextured model. The fourteen blocks are the ones the
/// original's retextured recipes can make (see the Deco tab's variants).
pub fn slots_for(block: &str) -> &'static [RetextureSlot] {
    match block {
        "tconstruct:crafting_station" => &[
            RetextureSlot { sprite: "tconstruct:block/table/crafting_station_side", uv: LEG_UV },
            RetextureSlot { sprite: "tconstruct:block/table/bottom", uv: LEG_BOTTOM_UV },
        ],
        "tconstruct:part_builder" | "tconstruct:tinker_station" => &[
            RetextureSlot { sprite: "tconstruct:block/table/side", uv: LEG_UV },
            RetextureSlot { sprite: "tconstruct:block/table/bottom", uv: LEG_BOTTOM_UV },
        ],
        "tconstruct:modifier_worktable" => &[
            RetextureSlot { sprite: "tconstruct:block/table/modifier_workstation_side", uv: LEG_UV },
            RetextureSlot { sprite: "tconstruct:block/table/modifier_workstation_top", uv: LEG_BOTTOM_UV },
        ],
        "tconstruct:smeltery_controller"
        | "tconstruct:seared_drain"
        | "tconstruct:seared_duct"
        | "tconstruct:seared_chute" => &[
            RetextureSlot { sprite: "tconstruct:block/smeltery/seared/bricks", uv: FULL_UV },
        ],
        "tconstruct:foundry_controller"
        | "tconstruct:scorched_drain"
        | "tconstruct:scorched_duct"
        | "tconstruct:scorched_chute" => &[
            RetextureSlot { sprite: "tconstruct:block/foundry/scorched/bricks", uv: FULL_UV },
        ],
        "tconstruct:tinkers_anvil" | "tconstruct:scorched_anvil" => &[
            RetextureSlot { sprite: "tconstruct:block/storage/fallback", uv: ANY_UV },
        ],
        _ => &[],
    }
}

/// The texel rectangle a baked quad's UVs cover, from its four normalized
/// corners: the minimum and maximum of each axis, scaled back to texels. The
/// corners arrive in model order (a face `rotation` permutes them), so the
/// rectangle is the same whichever corner the bake started from.
pub fn uv_rect(uv: [[f32; 2]; 4]) -> [u32; 4] {
    let (mut u0, mut v0) = (f32::INFINITY, f32::INFINITY);
    let (mut u1, mut v1) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
    for [u, v] in uv {
        u0 = u0.min(u);
        v0 = v0.min(v);
        u1 = u1.max(u);
        v1 = v1.max(v);
    }
    [
        (u0 * 16.0).round() as u32,
        (v0 * 16.0).round() as u32,
        (u1 * 16.0).round() as u32,
        (v1 * 16.0).round() as u32,
    ]
}

/// Whether a baked face of `block` belongs to one of its retextured slots:
/// its sprite names a slot and its UVs fall inside one of that slot's
/// rectangles (any rectangle when the slot names none). A face of a block
/// with no retextured model is never one.
pub fn is_retextured_face(block: &str, sprite: &str, uv: [[f32; 2]; 4]) -> bool {
    if slots_for(block).is_empty() {
        return false;
    }
    let rect = uv_rect(uv);
    slots_for(block).iter().any(|slot| {
        slot.sprite == sprite && (slot.uv.is_empty() || slot.uv.contains(&rect))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A leg face's UVs in the bake's normalized form: [12, 4, 16, 16] texels.
    fn leg_uv() -> [[f32; 2]; 4] {
        [[0.75, 0.25], [1.0, 0.25], [1.0, 1.0], [0.75, 1.0]]
    }

    /// The table top plate's side face: [0, 0, 16, 4] texels, the same sprite
    /// as the legs but a different rectangle.
    fn side_uv() -> [[f32; 2]; 4] {
        [[0.0, 0.0], [1.0, 0.0], [1.0, 0.25], [0.0, 0.25]]
    }

    #[test]
    fn the_table_lists_two_slots() {
        let slots = slots_for("tconstruct:crafting_station");
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].sprite, "tconstruct:block/table/crafting_station_side");
        assert_eq!(slots[1].sprite, "tconstruct:block/table/bottom");
        assert!(slots_for("minecraft:oak_log").is_empty());
    }

    #[test]
    fn a_leg_face_matches_but_the_table_side_does_not() {
        let leg = "tconstruct:block/table/crafting_station_side";
        assert!(is_retextured_face("tconstruct:crafting_station", leg, leg_uv()));
        assert!(!is_retextured_face("tconstruct:crafting_station", leg, side_uv()));
    }

    #[test]
    fn a_leg_bottom_corner_matches_but_the_table_underside_does_not() {
        let bottom = "tconstruct:block/table/bottom";
        let corner = [[0.0, 0.75], [0.25, 0.75], [0.25, 1.0], [0.0, 1.0]];
        let whole = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert!(is_retextured_face("tconstruct:crafting_station", bottom, corner));
        assert!(!is_retextured_face("tconstruct:crafting_station", bottom, whole));
    }

    #[test]
    fn the_drain_brick_face_matches_but_its_back_does_not() {
        let bricks = "tconstruct:block/smeltery/seared/bricks";
        let whole = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        assert!(is_retextured_face("tconstruct:seared_drain", bricks, whole));
        assert!(!is_retextured_face(
            "tconstruct:seared_drain",
            "tconstruct:block/smeltery/io/back",
            whole
        ));
    }

    #[test]
    fn the_anvil_top_matches_on_its_sprite_alone() {
        let fallback = "tconstruct:block/storage/fallback";
        let any = [[0.0, 0.0], [0.3125, 0.0], [0.3125, 1.0], [0.0, 1.0]];
        assert!(is_retextured_face("tconstruct:tinkers_anvil", fallback, any));
        assert!(!is_retextured_face(
            "tconstruct:tinkers_anvil",
            "tconstruct:block/table/anvil_base",
            any
        ));
    }

    #[test]
    fn uv_rect_recovers_the_texel_rectangle_whichever_corner_starts_it() {
        assert_eq!(uv_rect(leg_uv()), [12, 4, 16, 16]);
        // The same rectangle with its corners rotated (a face `rotation`).
        let rotated = [[1.0, 1.0], [0.75, 1.0], [0.75, 0.25], [1.0, 0.25]];
        assert_eq!(uv_rect(rotated), [12, 4, 16, 16]);
        // A reversed range, as the anvil's faces write some of theirs.
        let reversed = [[0.8125, 1.0], [1.0, 1.0], [1.0, 0.0], [0.8125, 0.0]];
        assert_eq!(uv_rect(reversed), [13, 0, 16, 16]);
    }
}

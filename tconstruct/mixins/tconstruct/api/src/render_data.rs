//! The original's block entity render data (SlimeKnights, MIT; see LICENSE): the fluid cuboids and item placements
//! the block entity renderers draw, from Mantle's model data (`assets/tconstruct/mantle/model`), the channel's
//! cuboids, and each fluid's still and flowing textures (`assets/tconstruct/mantle/fluid_texture`).
//! Generated from the pinned original by tools/import_original_data.py; do not edit by hand.

/// One face of a fluid cuboid: its direction, whether it uses the flowing texture, and the texture rotation in degrees.
pub(crate) struct Face { pub dir: &'static str, pub flowing: bool, pub rotation: u16 }

/// A fluid cuboid in sixteenths (Mantle's FluidCuboid): the box, and the faces it draws.
pub(crate) struct Cuboid { pub from: [f32; 3], pub to: [f32; 3], pub faces: &'static [Face] }

/// Where a block entity renderer places one item (Mantle's RenderItem): the center and size in sixteenths, the
/// display transform, and the x and y rotation in degrees.
pub(crate) struct RenderItem { pub center: [f32; 3], pub size: f32, pub transform: &'static str, pub x: f32, pub y: f32 }

/// The data of one block state variant: the state properties it asks for (empty: the default variant).
pub(crate) struct Variant<T: 'static> { pub when: &'static [(&'static str, &'static str)], pub data: &'static [T] }

/// The fluid cuboids of a block (`mantle/model/block_fluids`), by block id.
pub(crate) fn fluid_cuboids(block: &str) -> Option<&'static [Variant<Cuboid>]> {
    Some(match block {
        "tconstruct:end_fluid_cannon" => &[
            Variant { when: &[], data: &[Cuboid { from: [1.08, 7.08, 1.08], to: [14.92, 14.92, 14.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:obsidian_gauge" => &[
            Variant { when: &[("facing", "down")], data: &[Cuboid { from: [5.0, 15.5, 6.0], to: [11.0, 15.5, 11.0], faces: &[Face { dir: "down", flowing: false, rotation: 0 }] }] },
            Variant { when: &[("facing", "east")], data: &[Cuboid { from: [0.5, 6.0, 5.0], to: [0.5, 11.0, 11.0], faces: &[Face { dir: "east", flowing: false, rotation: 0 }] }] },
            Variant { when: &[("facing", "north")], data: &[Cuboid { from: [5.0, 6.0, 15.5], to: [11.0, 11.0, 15.5], faces: &[Face { dir: "north", flowing: false, rotation: 0 }] }] },
            Variant { when: &[("facing", "south")], data: &[Cuboid { from: [5.0, 6.0, 0.5], to: [11.0, 11.0, 0.5], faces: &[Face { dir: "south", flowing: false, rotation: 0 }] }] },
            Variant { when: &[("facing", "up")], data: &[Cuboid { from: [5.0, 0.5, 5.0], to: [11.0, 0.5, 10.0], faces: &[Face { dir: "up", flowing: false, rotation: 0 }] }] },
            Variant { when: &[("facing", "west")], data: &[Cuboid { from: [15.5, 6.0, 5.0], to: [15.5, 11.0, 11.0], faces: &[Face { dir: "west", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_alloyer" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 5.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_basin" => &[
            Variant { when: &[], data: &[Cuboid { from: [2.1, 4.0, 2.1], to: [13.9, 15.9, 13.9], faces: &[Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_faucet" => &[
            Variant { when: &[], data: &[Cuboid { from: [6.0, 6.0, 0.0], to: [10.0, 9.0, 6.0], faces: &[Face { dir: "up", flowing: true, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }] }, Cuboid { from: [6.0, 0.0, 6.0], to: [10.0, 9.0, 8.0], faces: &[Face { dir: "up", flowing: true, rotation: 0 }, Face { dir: "north", flowing: true, rotation: 0 }, Face { dir: "south", flowing: true, rotation: 0 }, Face { dir: "west", flowing: true, rotation: 0 }, Face { dir: "east", flowing: true, rotation: 0 }] }] },
            Variant { when: &[("facing", "down")], data: &[Cuboid { from: [6.0, 0.0, 6.0], to: [10.0, 16.0, 10.0], faces: &[Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: true, rotation: 0 }, Face { dir: "south", flowing: true, rotation: 0 }, Face { dir: "west", flowing: true, rotation: 0 }, Face { dir: "east", flowing: true, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_fluid_cannon" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 8.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_fuel_gauge" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_fuel_tank" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_ingot_gauge" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_ingot_tank" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_proxy_tank" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 4.0, 0.08], to: [4.92, 15.92, 4.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }, Cuboid { from: [11.08, 4.0, 0.08], to: [15.92, 15.92, 4.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }, Cuboid { from: [0.08, 4.0, 11.08], to: [4.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }, Cuboid { from: [11.08, 4.0, 11.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:scorched_table" => &[
            Variant { when: &[], data: &[Cuboid { from: [1.0, 15.0, 1.0], to: [15.0, 15.9, 15.0], faces: &[Face { dir: "up", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_basin" => &[
            Variant { when: &[], data: &[Cuboid { from: [2.1, 4.0, 2.1], to: [13.9, 15.9, 13.9], faces: &[Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_casting_tank" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 10.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_faucet" => &[
            Variant { when: &[], data: &[Cuboid { from: [6.0, 6.0, 0.0], to: [10.0, 9.0, 6.0], faces: &[Face { dir: "up", flowing: true, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }] }, Cuboid { from: [6.0, 0.0, 6.0], to: [10.0, 9.0, 8.0], faces: &[Face { dir: "up", flowing: true, rotation: 0 }, Face { dir: "north", flowing: true, rotation: 0 }, Face { dir: "south", flowing: true, rotation: 0 }, Face { dir: "west", flowing: true, rotation: 0 }, Face { dir: "east", flowing: true, rotation: 0 }] }] },
            Variant { when: &[("facing", "down")], data: &[Cuboid { from: [6.0, 0.0, 6.0], to: [10.0, 16.0, 10.0], faces: &[Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: true, rotation: 0 }, Face { dir: "south", flowing: true, rotation: 0 }, Face { dir: "west", flowing: true, rotation: 0 }, Face { dir: "east", flowing: true, rotation: 0 }] }] },
        ],
        "tconstruct:seared_fluid_cannon" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 8.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_fuel_gauge" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_fuel_tank" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_ingot_gauge" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_ingot_tank" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 0.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_melter" => &[
            Variant { when: &[], data: &[Cuboid { from: [0.08, 8.08, 0.08], to: [15.92, 15.92, 15.92], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }, Face { dir: "north", flowing: false, rotation: 0 }, Face { dir: "south", flowing: false, rotation: 0 }, Face { dir: "west", flowing: false, rotation: 0 }, Face { dir: "east", flowing: false, rotation: 0 }] }] },
        ],
        "tconstruct:seared_table" => &[
            Variant { when: &[], data: &[Cuboid { from: [1.0, 15.0, 1.0], to: [15.0, 15.9, 15.0], faces: &[Face { dir: "up", flowing: false, rotation: 0 }] }] },
        ],
        _ => return None,
    })
}

/// The item placements of a block (`mantle/model/item_lists`), by block id.
pub(crate) fn render_items(block: &str) -> Option<&'static [Variant<RenderItem>]> {
    Some(match block {
        "tconstruct:crafting_station" => &[
            Variant { when: &[], data: &[RenderItem { center: [5.0, 17.0, 5.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 17.0, 5.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [11.0, 17.0, 5.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [5.0, 17.0, 8.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 17.0, 8.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [11.0, 17.0, 8.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [5.0, 17.0, 11.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 17.0, 11.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [11.0, 17.0, 11.0], size: 2.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:end_fluid_cannon" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 4.0, 16.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 0.0, y: 0.0 }] },
            Variant { when: &[("facing", "down")], data: &[RenderItem { center: [8.0, 1.0, 8.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 90.0, y: 0.0 }] },
            Variant { when: &[("facing", "up")], data: &[RenderItem { center: [8.0, 15.0, 8.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 270.0, y: 0.0 }] },
        ],
        "tconstruct:modifier_worktable" => &[
            Variant { when: &[], data: &[RenderItem { center: [5.0, 16.25, 8.0], size: 7.5, transform: "tconstruct:table", x: 270.0, y: 0.0 }, RenderItem { center: [11.5, 17.5, 3.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [11.5, 17.5, 12.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:part_builder" => &[
            Variant { when: &[], data: &[RenderItem { center: [4.5, 17.5, 4.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [4.5, 16.3125, 11.5], size: 5.0, transform: "tconstruct:table", x: 270.0, y: 0.0 }, RenderItem { center: [11.5, 17.5, 4.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:scorched_anvil" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 16.25, 9.0], size: 7.5, transform: "tconstruct:table", x: 270.0, y: 0.0 }, RenderItem { center: [2.5, 17.5, 10.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [2.5, 17.5, 5.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 17.5, 4.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [13.5, 17.5, 5.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [13.5, 17.5, 10.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:scorched_basin" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 10.0, 8.0], size: 11.95, transform: "tconstruct:casting_basin", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 10.0, 8.0], size: 12.0, transform: "tconstruct:casting_basin", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:scorched_fluid_cannon" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 4.0, 16.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 0.0, y: 0.0 }] },
            Variant { when: &[("facing", "down")], data: &[RenderItem { center: [8.0, 0.0, 8.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 90.0, y: 0.0 }] },
            Variant { when: &[("facing", "up")], data: &[RenderItem { center: [8.0, 16.0, 8.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 270.0, y: 0.0 }] },
        ],
        "tconstruct:scorched_proxy_tank" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 9.0, 8.0], size: 12.0, transform: "tconstruct:melter", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:scorched_table" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 15.5, 8.0], size: 14.0, transform: "tconstruct:casting_table", x: 270.0, y: 180.0 }, RenderItem { center: [8.0, 15.5, 8.0], size: 14.1, transform: "tconstruct:casting_table", x: 270.0, y: 180.0 }] },
        ],
        "tconstruct:seared_basin" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 10.0, 8.0], size: 11.95, transform: "tconstruct:casting_basin", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 10.0, 8.0], size: 12.0, transform: "tconstruct:casting_basin", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:seared_casting_tank" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 15.5, 8.0], size: 14.0, transform: "tconstruct:casting_table", x: 270.0, y: 180.0 }, RenderItem { center: [8.0, 15.5, 8.0], size: 14.1, transform: "tconstruct:casting_table", x: 270.0, y: 180.0 }] },
        ],
        "tconstruct:seared_fluid_cannon" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 4.0, 16.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 0.0, y: 0.0 }] },
            Variant { when: &[("facing", "down")], data: &[RenderItem { center: [8.0, 0.0, 8.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 90.0, y: 0.0 }] },
            Variant { when: &[("facing", "up")], data: &[RenderItem { center: [8.0, 16.0, 8.0], size: 7.5, transform: "tconstruct:fluid_cannon", x: 270.0, y: 0.0 }] },
        ],
        "tconstruct:seared_melter" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 12.0, 12.0], size: 7.5, transform: "tconstruct:melter", x: 0.0, y: 0.0 }, RenderItem { center: [4.0, 12.0, 4.0], size: 7.5, transform: "tconstruct:melter", x: 0.0, y: 0.0 }, RenderItem { center: [12.0, 12.0, 4.0], size: 7.5, transform: "tconstruct:melter", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:seared_table" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 15.5, 8.0], size: 14.0, transform: "tconstruct:casting_table", x: 270.0, y: 180.0 }, RenderItem { center: [8.0, 15.5, 8.0], size: 14.1, transform: "tconstruct:casting_table", x: 270.0, y: 180.0 }] },
        ],
        "tconstruct:tinker_station" => &[
            Variant { when: &[], data: &[RenderItem { center: [11.0, 16.25, 8.0], size: 7.5, transform: "tconstruct:table", x: 270.0, y: 0.0 }, RenderItem { center: [2.5, 17.5, 8.0], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [4.5, 17.5, 3.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [4.5, 17.5, 12.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }] },
        ],
        "tconstruct:tinkers_anvil" => &[
            Variant { when: &[], data: &[RenderItem { center: [8.0, 16.25, 9.0], size: 7.5, transform: "tconstruct:table", x: 270.0, y: 0.0 }, RenderItem { center: [2.5, 17.5, 10.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [2.5, 17.5, 5.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [8.0, 17.5, 4.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [13.5, 17.5, 5.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }, RenderItem { center: [13.5, 17.5, 10.5], size: 3.0, transform: "tconstruct:table", x: 0.0, y: 0.0 }] },
        ],
        _ => return None,
    })
}

/// The channel's fluid cuboids (Mantle's ChannelFluids, `mantle/model/channel_fluids/channel.json`): the center in
/// its still and flowing forms, the drop below the center, and one side (authored facing south) in its still, in,
/// out and edge forms.
pub(crate) struct ChannelFluids { pub center_still: Cuboid, pub center_flowing: Cuboid, pub down: Cuboid,
    pub side_still: Cuboid, pub side_in: Cuboid, pub side_out: Cuboid, pub side_edge: Cuboid }
pub(crate) const CHANNEL_FLUIDS: ChannelFluids = ChannelFluids {
    center_still: Cuboid { from: [6.0, 6.0, 6.0], to: [10.0, 8.0, 10.0], faces: &[Face { dir: "down", flowing: false, rotation: 0 }, Face { dir: "up", flowing: false, rotation: 0 }] }, center_flowing: Cuboid { from: [6.0, 6.0, 6.0], to: [10.0, 8.0, 10.0], faces: &[Face { dir: "down", flowing: true, rotation: 180 }, Face { dir: "up", flowing: true, rotation: 0 }] },
    down: Cuboid { from: [6.0, 0.0, 6.0], to: [10.0, 6.0, 10.0], faces: &[Face { dir: "north", flowing: true, rotation: 0 }, Face { dir: "south", flowing: true, rotation: 0 }, Face { dir: "west", flowing: true, rotation: 0 }, Face { dir: "east", flowing: true, rotation: 0 }] }, side_still: Cuboid { from: [6.0, 6.0, 10.0], to: [10.0, 8.0, 10.0], faces: &[Face { dir: "south", flowing: false, rotation: 0 }] },
    side_in: Cuboid { from: [6.0, 8.0, 10.0], to: [10.0, 8.0, 16.0], faces: &[Face { dir: "up", flowing: true, rotation: 180 }] }, side_out: Cuboid { from: [6.0, 8.0, 10.0], to: [10.0, 8.0, 16.0], faces: &[Face { dir: "up", flowing: true, rotation: 0 }] },
    side_edge: Cuboid { from: [6.0, 6.0, 16.0], to: [10.0, 8.0, 16.0], faces: &[Face { dir: "south", flowing: true, rotation: 0 }] } };

/// A fluid's still and flowing textures (Mantle's fluid texture data, `mantle/fluid_texture`).
pub(crate) struct FluidTexture { pub still: &'static str, pub flowing: &'static str }

/// The textures of a fluid, by fluid id.
pub(crate) fn fluid_texture(fluid: &str) -> Option<FluidTexture> {
    Some(match fluid {
        "tconstruct:beetroot_soup" => FluidTexture { still: "tconstruct:fluid/food/stew/still", flowing: "tconstruct:fluid/food/stew/flowing" },
        "tconstruct:blazing_blood" => FluidTexture { still: "tconstruct:fluid/molten/blaze/still", flowing: "tconstruct:fluid/molten/blaze/flowing" },
        "tconstruct:earth_slime" => FluidTexture { still: "tconstruct:fluid/slime/earth/still", flowing: "tconstruct:fluid/slime/earth/flowing" },
        "tconstruct:ender_slime" => FluidTexture { still: "tconstruct:fluid/slime/ender/still", flowing: "tconstruct:fluid/slime/ender/flowing" },
        "tconstruct:fiery_liquid" => FluidTexture { still: "tconstruct:fluid/slime/fiery/still", flowing: "tconstruct:fluid/slime/fiery/flowing" },
        "tconstruct:honey" => FluidTexture { still: "tconstruct:fluid/food/honey/still", flowing: "tconstruct:fluid/food/honey/flowing" },
        "tconstruct:ichor" => FluidTexture { still: "tconstruct:fluid/slime/ichor/still", flowing: "tconstruct:fluid/slime/ichor/flowing" },
        "tconstruct:liquid_soul" => FluidTexture { still: "tconstruct:fluid/slime/soul/still", flowing: "tconstruct:fluid/slime/soul/flowing" },
        "tconstruct:magma" => FluidTexture { still: "tconstruct:fluid/slime/magma/still", flowing: "tconstruct:fluid/slime/magma/flowing" },
        "tconstruct:meat_soup" => FluidTexture { still: "tconstruct:fluid/food/stew/still", flowing: "tconstruct:fluid/food/stew/flowing" },
        "tconstruct:molten_aluminum" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/aluminum/still", flowing: "tconstruct:fluid/molten/compat_ore/aluminum/flowing" },
        "tconstruct:molten_amethyst" => FluidTexture { still: "tconstruct:fluid/molten/ore/amethyst/still", flowing: "tconstruct:fluid/molten/ore/amethyst/flowing" },
        "tconstruct:molten_amethyst_bronze" => FluidTexture { still: "tconstruct:fluid/molten/alloy/amethyst_bronze/still", flowing: "tconstruct:fluid/molten/alloy/amethyst_bronze/flowing" },
        "tconstruct:molten_bendalloy" => FluidTexture { still: "tconstruct:fluid/molten/still", flowing: "tconstruct:fluid/molten/flowing" },
        "tconstruct:molten_brass" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/brass/still", flowing: "tconstruct:fluid/molten/compat_alloy/brass/flowing" },
        "tconstruct:molten_bronze" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/bronze/still", flowing: "tconstruct:fluid/molten/compat_alloy/bronze/flowing" },
        "tconstruct:molten_cadmium" => FluidTexture { still: "tconstruct:fluid/molten/still", flowing: "tconstruct:fluid/molten/flowing" },
        "tconstruct:molten_chromium" => FluidTexture { still: "tconstruct:fluid/molten/still", flowing: "tconstruct:fluid/molten/flowing" },
        "tconstruct:molten_cinderslime" => FluidTexture { still: "tconstruct:fluid/molten/alloy/cinderslime/still", flowing: "tconstruct:fluid/molten/alloy/cinderslime/flowing" },
        "tconstruct:molten_clay" => FluidTexture { still: "tconstruct:fluid/molten/stone/still", flowing: "tconstruct:fluid/molten/stone/flowing" },
        "tconstruct:molten_cobalt" => FluidTexture { still: "tconstruct:fluid/molten/ore/cobalt/still", flowing: "tconstruct:fluid/molten/ore/cobalt/flowing" },
        "tconstruct:molten_constantan" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/constantan/still", flowing: "tconstruct:fluid/molten/compat_alloy/constantan/flowing" },
        "tconstruct:molten_copper" => FluidTexture { still: "tconstruct:fluid/molten/ore/copper/still", flowing: "tconstruct:fluid/molten/ore/copper/flowing" },
        "tconstruct:molten_debris" => FluidTexture { still: "tconstruct:fluid/molten/stone/still", flowing: "tconstruct:fluid/molten/stone/flowing" },
        "tconstruct:molten_diamond" => FluidTexture { still: "tconstruct:fluid/molten/ore/diamond/still", flowing: "tconstruct:fluid/molten/ore/diamond/flowing" },
        "tconstruct:molten_duralumin" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/duralumin/still", flowing: "tconstruct:fluid/molten/compat_alloy/duralumin/flowing" },
        "tconstruct:molten_electrum" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/electrum/still", flowing: "tconstruct:fluid/molten/compat_alloy/electrum/flowing" },
        "tconstruct:molten_emerald" => FluidTexture { still: "tconstruct:fluid/molten/ore/emerald/still", flowing: "tconstruct:fluid/molten/ore/emerald/flowing" },
        "tconstruct:molten_ender" => FluidTexture { still: "tconstruct:fluid/molten/stone/still", flowing: "tconstruct:fluid/molten/stone/flowing" },
        "tconstruct:molten_enderium" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/enderium/still", flowing: "tconstruct:fluid/molten/compat_alloy/enderium/flowing" },
        "tconstruct:molten_glass" => FluidTexture { still: "tconstruct:fluid/molten/glass/still", flowing: "tconstruct:fluid/molten/glass/flowing" },
        "tconstruct:molten_gold" => FluidTexture { still: "tconstruct:fluid/molten/ore/gold/still", flowing: "tconstruct:fluid/molten/ore/gold/flowing" },
        "tconstruct:molten_hepatizon" => FluidTexture { still: "tconstruct:fluid/molten/alloy/hepatizon/still", flowing: "tconstruct:fluid/molten/alloy/hepatizon/flowing" },
        "tconstruct:molten_invar" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/invar/still", flowing: "tconstruct:fluid/molten/compat_alloy/invar/flowing" },
        "tconstruct:molten_iron" => FluidTexture { still: "tconstruct:fluid/molten/ore/iron/still", flowing: "tconstruct:fluid/molten/ore/iron/flowing" },
        "tconstruct:molten_knightmetal" => FluidTexture { still: "tconstruct:fluid/molten/alloy/knightmetal/still", flowing: "tconstruct:fluid/molten/alloy/knightmetal/flowing" },
        "tconstruct:molten_knightslime" => FluidTexture { still: "tconstruct:fluid/molten/alloy/knightslime/still", flowing: "tconstruct:fluid/molten/alloy/knightslime/flowing" },
        "tconstruct:molten_lead" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/lead/still", flowing: "tconstruct:fluid/molten/compat_ore/lead/flowing" },
        "tconstruct:molten_lumium" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/lumium/still", flowing: "tconstruct:fluid/molten/compat_alloy/lumium/flowing" },
        "tconstruct:molten_manyullyn" => FluidTexture { still: "tconstruct:fluid/molten/alloy/manyullyn/still", flowing: "tconstruct:fluid/molten/alloy/manyullyn/flowing" },
        "tconstruct:molten_netherite" => FluidTexture { still: "tconstruct:fluid/molten/alloy/netherite/still", flowing: "tconstruct:fluid/molten/alloy/netherite/flowing" },
        "tconstruct:molten_nickel" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/nickel/still", flowing: "tconstruct:fluid/molten/compat_ore/nickel/flowing" },
        "tconstruct:molten_nicrosil" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/nicrosil/still", flowing: "tconstruct:fluid/molten/compat_alloy/nicrosil/flowing" },
        "tconstruct:molten_obsidian" => FluidTexture { still: "tconstruct:fluid/molten/stone/obsidian/still", flowing: "tconstruct:fluid/molten/stone/obsidian/flowing" },
        "tconstruct:molten_osmium" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/osmium/still", flowing: "tconstruct:fluid/molten/compat_ore/osmium/flowing" },
        "tconstruct:molten_pewter" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/pewter/still", flowing: "tconstruct:fluid/molten/compat_alloy/pewter/flowing" },
        "tconstruct:molten_pig_iron" => FluidTexture { still: "tconstruct:fluid/molten/alloy/pig_iron/still", flowing: "tconstruct:fluid/molten/alloy/pig_iron/flowing" },
        "tconstruct:molten_platinum" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/platinum/still", flowing: "tconstruct:fluid/molten/compat_ore/platinum/flowing" },
        "tconstruct:molten_porcelain" => FluidTexture { still: "tconstruct:fluid/molten/stone/porcelain/still", flowing: "tconstruct:fluid/molten/stone/porcelain/flowing" },
        "tconstruct:molten_quartz" => FluidTexture { still: "tconstruct:fluid/molten/ore/quartz/still", flowing: "tconstruct:fluid/molten/ore/quartz/flowing" },
        "tconstruct:molten_queens_slime" => FluidTexture { still: "tconstruct:fluid/molten/alloy/queens_slime/still", flowing: "tconstruct:fluid/molten/alloy/queens_slime/flowing" },
        "tconstruct:molten_refined_glowstone" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/refined_glowstone/still", flowing: "tconstruct:fluid/molten/compat_alloy/refined_glowstone/flowing" },
        "tconstruct:molten_refined_obsidian" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/refined_obsidian/still", flowing: "tconstruct:fluid/molten/compat_alloy/refined_obsidian/flowing" },
        "tconstruct:molten_rose_gold" => FluidTexture { still: "tconstruct:fluid/molten/alloy/rose_gold/still", flowing: "tconstruct:fluid/molten/alloy/rose_gold/flowing" },
        "tconstruct:molten_signalum" => FluidTexture { still: "tconstruct:fluid/molten/compat_alloy/signalum/still", flowing: "tconstruct:fluid/molten/compat_alloy/signalum/flowing" },
        "tconstruct:molten_silver" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/silver/still", flowing: "tconstruct:fluid/molten/compat_ore/silver/flowing" },
        "tconstruct:molten_slimesteel" => FluidTexture { still: "tconstruct:fluid/molten/alloy/slimesteel/still", flowing: "tconstruct:fluid/molten/alloy/slimesteel/flowing" },
        "tconstruct:molten_soulsteel" => FluidTexture { still: "tconstruct:fluid/molten/alloy/soulsteel/still", flowing: "tconstruct:fluid/molten/alloy/soulsteel/flowing" },
        "tconstruct:molten_steel" => FluidTexture { still: "tconstruct:fluid/molten/ore/steel/still", flowing: "tconstruct:fluid/molten/ore/steel/flowing" },
        "tconstruct:molten_steeleaf" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/steeleaf/still", flowing: "tconstruct:fluid/molten/compat_ore/steeleaf/flowing" },
        "tconstruct:molten_tin" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/tin/still", flowing: "tconstruct:fluid/molten/compat_ore/tin/flowing" },
        "tconstruct:molten_tungsten" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/tungsten/still", flowing: "tconstruct:fluid/molten/compat_ore/tungsten/flowing" },
        "tconstruct:molten_uranium" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/uranium/still", flowing: "tconstruct:fluid/molten/compat_ore/uranium/flowing" },
        "tconstruct:molten_zinc" => FluidTexture { still: "tconstruct:fluid/molten/compat_ore/zinc/still", flowing: "tconstruct:fluid/molten/compat_ore/zinc/flowing" },
        "tconstruct:mushroom_stew" => FluidTexture { still: "tconstruct:fluid/food/stew/still", flowing: "tconstruct:fluid/food/stew/flowing" },
        "tconstruct:potion" => FluidTexture { still: "tconstruct:fluid/potion/still", flowing: "tconstruct:fluid/potion/flowing" },
        "tconstruct:powdered_snow" => FluidTexture { still: "tconstruct:fluid/powdered_snow/still", flowing: "tconstruct:fluid/powdered_snow/flowing" },
        "tconstruct:rabbit_stew" => FluidTexture { still: "tconstruct:fluid/food/stew/still", flowing: "tconstruct:fluid/food/stew/flowing" },
        "tconstruct:scorched_stone" => FluidTexture { still: "tconstruct:fluid/molten/stone/still", flowing: "tconstruct:fluid/molten/stone/flowing" },
        "tconstruct:seared_stone" => FluidTexture { still: "tconstruct:fluid/molten/stone/still", flowing: "tconstruct:fluid/molten/stone/flowing" },
        "tconstruct:sky_slime" => FluidTexture { still: "tconstruct:fluid/slime/sky/still", flowing: "tconstruct:fluid/slime/sky/flowing" },
        "tconstruct:venom" => FluidTexture { still: "tconstruct:fluid/slime/venom/still", flowing: "tconstruct:fluid/slime/venom/flowing" },
        _ => return None,
    })
}

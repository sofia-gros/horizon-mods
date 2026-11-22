//! The fancy item frames and armor stands of the original's gadgets module: the six frame variants of
//! `gadgets/entity/FrameType.java` and the four stand variants of `FancyArmorStandEntity.StandType`, their items
//! (`gadgets/item/FancyItemFrameItem.java`, `gadgets/item/FancyArmorStandItem.java`, registered in
//! `gadgets/TinkerGadgets.java`) and the two entity types behind them, at the pinned revision.
//!
//! What ships here:
//! - The ten items as content declarations (written by `tools/import_original_data.py`): the frame items on the
//!   `minecraft:item_frame` template with the original's plain item properties (stack 64), the stand items on
//!   `minecraft:armor_stand` with `max_stack_size` 16 (the stand item is built with `stacksTo(16)`). The original
//!   item classes copy the vanilla placement logic of their base items, so the templates place a working vanilla
//!   item frame or armor stand today.
//! - The two derived entity types, registered below with the original's sizes and tracking: the frame is
//!   0.5 x 0.5 with tracking range 10 and no velocity updates (the original also sets its update interval to
//!   `Integer.MAX_VALUE`, which the derived registration does not carry), the stand is 0.5 x 1.975 with
//!   tracking range 10.
//! - The gray tooltip line of both item classes (each appends the translatable `<id>.tooltip`) through the item
//!   tooltip callback; the keys ship in the imported language files.
//! - The creative tab entries (creative.rs, in `TinkerGadgets.addTabItems` order) and the four recipes whose
//!   ingredients this port declares (the bamboo and bone stands and the two reversed gold conversions); the
//!   other seven convert on a later import once `obsidian_pane`, the clear glass family and `blazewood_slab` are
//!   declared.
//!
//! What does not run, and why: in the original all variant behaviour lives in the entity classes
//! (`FancyItemFrameEntity`, `FancyArmorStandEntity`), and this host gives a mod no way to run it. PlutoLib's
//! derived entity types are declarations whose native registry and loader "remain an explicit stub"
//! (pluto-lib/mod/api/src/pure/entity_type_api.rs), the pure API has no operation to spawn an entity (the world
//! operations cover blocks, sounds, scheduled ticks and block entity data; entity reach only finds, discards or
//! hurts entities that already exist), and there are no simulation, interaction, equipment or rendering
//! callbacks for derived types (docs/status/entities.md: derived type registration is source-level only and
//! simulation callbacks remain incomplete or unverified). The tables below therefore record the behaviour as
//! data, for the day those host surfaces exist:
//! - frames: the rotation timers (gold advances the rotation by one every 20 ticks, reversed gold goes
//!   backwards, diamond instead winds down one rotation every 300 ticks and plays `tconstruct:item_frame_click`),
//!   the 16-step rotation and 0-16 comparator output of the diamond, manyullyn and netherite frames (the others
//!   read rotation % 8 + 1, an empty frame reads 0), `setItem` resetting the rotation of the rotating types, the
//!   clear frame using the block behind it when clicked while filled and hiding its frame model then (its held
//!   item renders at 0.75 scale instead of 0.5), manyullyn's fullbright rendering (packed light 0xF000F0, block
//!   light at least 7), and the netherite frame's fire and explosion immunity;
//! - stands: the placement properties (bamboo small, bone and necrotic bone show arms, necrotic bone renders
//!   fullbright), the left-handed flag taken from the placer's main arm, the yaw snapped to 45 degree steps,
//!   fire immunity for every variant, the clear stand turning invisible while any equipment slot is filled, the
//!   break drop that keeps the stand item with its custom name, and the dispenser behaviour
//!   (`DispenseFancyArmorStand`). Entity rendering generally (the frame block models on a wall, the stand
//!   textures under `textures/entity/armorstand`) has no registration surface for derived types; the models and
//!   item textures still ship as assets. The original's armor stand entity loot table is an empty table (the
//!   drop is code in `brokenByPlayer`) and entity loot tables are outside this port's reviewed content paths,
//!   so nothing ships for it.
use pluto_lib::{EntityTypeRegistry, Identifier, ItemTooltipCallback, TooltipLine};
use std::sync::Once;

/// One fancy frame variant, in `FrameType` declaration order (the order is saved in the entity as an int, so
/// it is part of the format). `rotation_step` is the rotation change the 20-tick timer applies (gold forwards,
/// reversed gold backwards); diamond has no step and instead winds down (module docs). `more_rotations` is
/// `hasMoreRotations()`: 16 rotation steps and a 0-16 comparator output instead of 8.
pub struct FrameKind {
    pub item: &'static str,
    pub variant: i32,
    pub rotation_step: i32,
    pub winds_down: bool,
    pub more_rotations: bool,
    pub fullbright: bool,
    pub hidden_when_filled: bool,
    pub fire_immune: bool,
}

pub const FRAME_KINDS: &[FrameKind] = &[
    FrameKind { item: "reversed_gold_item_frame", variant: 0, rotation_step: -1, winds_down: false, more_rotations: false, fullbright: false, hidden_when_filled: false, fire_immune: false },
    FrameKind { item: "diamond_item_frame", variant: 1, rotation_step: 0, winds_down: true, more_rotations: true, fullbright: false, hidden_when_filled: false, fire_immune: false },
    FrameKind { item: "manyullyn_item_frame", variant: 2, rotation_step: 0, winds_down: false, more_rotations: true, fullbright: true, hidden_when_filled: false, fire_immune: false },
    FrameKind { item: "gold_item_frame", variant: 3, rotation_step: 1, winds_down: false, more_rotations: false, fullbright: false, hidden_when_filled: false, fire_immune: false },
    FrameKind { item: "clear_item_frame", variant: 4, rotation_step: 0, winds_down: false, more_rotations: false, fullbright: false, hidden_when_filled: true, fire_immune: false },
    FrameKind { item: "netherite_item_frame", variant: 5, rotation_step: 0, winds_down: false, more_rotations: true, fullbright: false, hidden_when_filled: false, fire_immune: true },
];

/// One armor stand variant, in `StandType` declaration order (likewise saved as an int). `small` and `arms`
/// are the `onPlace` properties; every variant is fire immune at the class level, and the clear stand hides
/// while it wears anything.
pub struct StandKind {
    pub item: &'static str,
    pub variant: i32,
    pub small: bool,
    pub arms: bool,
    pub fullbright: bool,
    pub hidden_when_equipped: bool,
}

pub const STAND_KINDS: &[StandKind] = &[
    StandKind { item: "bamboo_armor_stand", variant: 0, small: true, arms: false, fullbright: false, hidden_when_equipped: false },
    StandKind { item: "bone_armor_stand", variant: 1, small: false, arms: true, fullbright: false, hidden_when_equipped: false },
    StandKind { item: "necrotic_bone_armor_stand", variant: 2, small: false, arms: true, fullbright: true, hidden_when_equipped: false },
    StandKind { item: "clear_armor_stand", variant: 3, small: false, arms: false, fullbright: false, hidden_when_equipped: true },
];

/// The derived entity types of `TinkerGadgets`, as registration data: (id, vanilla factory, width, height,
/// tracking chunks, velocity updates). The frame's registry name differs from its item suffix; the frame also
/// sets its update interval to `Integer.MAX_VALUE` in the original, which the derived registration does not
/// carry. The stand's tracking is its client tracking range.
pub const ENTITY_TYPES: &[(&str, &str, f32, f32, u32, bool)] = &[
    ("tconstruct:fancy_item_frame", "minecraft:item_frame", 0.5, 0.5, 10, false),
    ("tconstruct:armor_stand", "minecraft:armor_stand", 0.5, 1.975, 10, true),
];

pub fn register() {
    // The original builds both types over the vanilla classes (EntityType.Builder.of with the item frame and
    // armor stand factories); PlutoLib's derived registration is the same declaration. A rejected registration
    // (a duplicate, or the registry already frozen) is a host integration error the native side surfaces, not
    // something the mod can act on, so the results are not unwrapped.
    for (id, factory, width, height, tracking, velocity) in ENTITY_TYPES {
        let _ = EntityTypeRegistry::register_derived(
            Identifier::parse(id).expect("valid id"),
            Identifier::parse(factory).expect("valid id"),
            *width, *height, *tracking, *velocity);
    }
    static TOOLTIPS: Once = Once::new();
    TOOLTIPS.call_once(|| {
        ItemTooltipCallback::EVENT.register(|tip| {
            if let Some(name) = tip.item.strip_prefix("tconstruct:") {
                if FRAME_KINDS.iter().any(|kind| kind.item == name) || STAND_KINDS.iter().any(|kind| kind.item == name) {
                    tip.added.borrow_mut().push(TooltipLine::translatable(format!("item.tconstruct.{name}.tooltip")));
                }
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_kinds_match_frame_type() {
        // FrameType declaration order and ordinals (the enum comment: the order is weird for backwards
        // compatibility, as it is saved in the entity as an int).
        let names: Vec<&str> = FRAME_KINDS.iter().map(|kind| kind.item).collect();
        assert_eq!(names, ["reversed_gold_item_frame", "diamond_item_frame", "manyullyn_item_frame", "gold_item_frame", "clear_item_frame", "netherite_item_frame"]);
        for (index, kind) in FRAME_KINDS.iter().enumerate() { assert_eq!(kind.variant, index as i32, "variant ids are the ordinals"); }
        let more: Vec<&str> = FRAME_KINDS.iter().filter(|kind| kind.more_rotations).map(|kind| kind.item).collect();
        assert_eq!(more, ["diamond_item_frame", "manyullyn_item_frame", "netherite_item_frame"], "hasMoreRotations is diamond, manyullyn, netherite");
        assert_eq!(FRAME_KINDS.iter().find(|kind| kind.item == "gold_item_frame").unwrap().rotation_step, 1);
        assert_eq!(FRAME_KINDS.iter().find(|kind| kind.item == "reversed_gold_item_frame").unwrap().rotation_step, -1);
        assert!(FRAME_KINDS.iter().find(|kind| kind.item == "diamond_item_frame").unwrap().winds_down, "diamond winds down instead of stepping");
        assert!(FRAME_KINDS.iter().find(|kind| kind.item == "manyullyn_item_frame").unwrap().fullbright);
        assert!(FRAME_KINDS.iter().find(|kind| kind.item == "clear_item_frame").unwrap().hidden_when_filled);
        assert!(FRAME_KINDS.iter().find(|kind| kind.item == "netherite_item_frame").unwrap().fire_immune);
    }

    #[test]
    fn the_stand_kinds_match_stand_type() {
        let names: Vec<&str> = STAND_KINDS.iter().map(|kind| kind.item).collect();
        assert_eq!(names, ["bamboo_armor_stand", "bone_armor_stand", "necrotic_bone_armor_stand", "clear_armor_stand"]);
        for (index, kind) in STAND_KINDS.iter().enumerate() { assert_eq!(kind.variant, index as i32, "variant ids are the ordinals"); }
        assert!(STAND_KINDS[0].small, "bamboo is placed small");
        assert!(STAND_KINDS[1].arms && STAND_KINDS[2].arms, "bone and necrotic bone show arms");
        assert!(STAND_KINDS[2].fullbright, "necrotic bone renders fullbright");
        assert!(STAND_KINDS[3].hidden_when_equipped, "the clear stand hides while it wears anything");
    }

    #[test]
    fn the_derived_entity_types_match_tinker_gadgets_and_register() {
        assert_eq!(ENTITY_TYPES, &[
            ("tconstruct:fancy_item_frame", "minecraft:item_frame", 0.5, 0.5, 10, false),
            ("tconstruct:armor_stand", "minecraft:armor_stand", 0.5, 1.975, 10, true),
        ], "sized 0.5x0.5 with tracking range 10 and no velocity updates; sized 0.5x1.975 with client tracking range 10");
        // The registrations themselves are accepted by PlutoLib's validation (the snapshot a consumer would
        // read is PlutoLib-internal, so acceptance is what this crate can assert).
        for (id, factory, width, height, tracking, velocity) in ENTITY_TYPES {
            EntityTypeRegistry::register_derived(
                Identifier::parse(id).unwrap(), Identifier::parse(factory).unwrap(),
                *width, *height, *tracking, *velocity).expect("the declaration registers");
        }
    }
}

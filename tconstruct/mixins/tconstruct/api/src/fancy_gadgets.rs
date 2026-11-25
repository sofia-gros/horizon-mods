//! The fancy item frames and armor stands of the original's gadgets module: the six frame variants of
//! `gadgets/entity/FrameType.java` and the four stand variants of `FancyArmorStandEntity.StandType`, their items
//! (`gadgets/item/FancyItemFrameItem.java`, `gadgets/item/FancyArmorStandItem.java`, registered in
//! `gadgets/TinkerGadgets.java`) and the two entity types behind them, at the pinned revision.
//!
//! What ships here:
//! - The ten items as content declarations (written by `tools/import_original_data.py`): the frame items on the
//!   `minecraft:item_frame` template with the original's plain item properties (stack 64), the stand items on
//!   `minecraft:armor_stand` with `max_stack_size` 16 (the stand item is built with `stacksTo(16)`).
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
//! - The ten variants themselves, registered with PlutoLib's hanging registry (`HangingRegistry`), which runs
//!   their live behaviour over the host's vanilla frame/stand state: the variant rides in the entity's saved
//!   data (the original's `Variant` int), so a placed fancy frame or stand is a vanilla frame or stand whose
//!   mechanics are the variant's.
//!
//! What runs, through that surface: placement of all ten items as their variants (survival checks, sounds and
//! game events as the vanilla items); the frame rotation rules (16 steps for diamond, manyullyn and netherite,
//! diamond capping at 16 instead of wrapping) on interact, on load and on the comparator (16-step output 0-16,
//! the others rotation % 8 + 1, empty 0); the rotation timers (gold +1 and reversed gold -1 every 20 ticks,
//! diamond winding down one step every 300 ticks and playing `tconstruct:item_frame_click`); `setItem`
//! resetting a rotating variant's rotation (interact, item removal and the command path); the clear frame
//! using the block behind it when clicked while filled and hiding its frame model while filled; the netherite
//! frame's explosion immunity (frames do not burn in this host, so its fire immunity has nothing to bite on);
//! the stands' placement properties (bamboo small, bone and necrotic bone arms) and the left-handed flag taken
//! from the placer's hand and main arm, both saved; fire immunity for every stand variant (its fire ticks are
//! cleared around the tick, and `/damage` refuses fire types); the clear stand hiding while any equipment
//! slot is filled; the break drop keeping the variant item with its custom name; and the dispenser behaviour
//! (`DispenseFancyArmorStand`: spawn at the facing position with the facing's yaw, `onPlace` after the stack
//! config).
//!
//! What does not run, and why:
//! - Rendering: the host draws every frame with the vanilla frame model and every stand with the vanilla stand
//!   model and texture, and no surface lets a variant substitute its own — so the per-variant frame models
//!   (`tconstruct:block/frame/<name>`), the stand textures (`textures/entity/armorstand`), manyullyn's and
//!   necrotic bone's fullbright rendering (packed light 0xF000F0, block light at least 7 for manyullyn), the
//!   clear frame's 0.75 held-item scale (the host draws 0.5) and the left-handed arm side do not show. The
//!   models and textures still ship as assets. The clear frame/stand hiding does run, through the host's one
//!   invisibility flag: in this host an invisible stand is also spared by blasts and refused by `/damage`,
//!   which the original's per-player `isInvisibleTo` hiding is not — a known divergence of the flag route.
//! - The clear frame's click-through goes through the level's generic use branch only: the blocks the host's
//!   own use flow special-cases first (jukebox, composter, lectern and kin) fall back to the frame's own
//!   interaction, and a LAN guest never clicks through (the host tracks no guest sneak state server-side, and
//!   the original requires not-sneaking).
//! - Pick-block results follow the host's own rules for frames and stands, not the variants' `getPickedResult`
//!   (the variant item, or the held item for a filled frame).
//! - The guest item-frame-change notice reports the rotation modulo 8, the host hook's shape, even for the
//!   16-step variants.
//! - `updateRotationTimer` is dead API in the original (public, no callers), so nothing ports it.
//! The original's armor stand entity loot table is an empty table (the drop is code in `brokenByPlayer`) and
//! entity loot tables are outside this port's reviewed content paths, so nothing ships for it.
use pluto_lib::{EntityTypeRegistry, FrameVariant, HangingRegistry, Identifier, ItemTooltipCallback, StandVariant, TooltipLine};
use std::sync::Once;

/// One fancy frame variant, in `FrameType` declaration order (the order is saved in the entity as an int, so
/// it is part of the format). `rotation_step` is the rotation change the 20-tick timer applies (gold forwards,
/// reversed gold backwards); diamond has no step and instead winds down (module docs). `more_rotations` is
/// `hasMoreRotations()`: 16 rotation steps and a 0-16 comparator output instead of 8. `click_through` is the
/// clear frame's filled-click behaviour (use the block behind it first).
pub struct FrameKind {
    pub item: &'static str,
    pub variant: i32,
    pub rotation_step: i32,
    pub winds_down: bool,
    pub more_rotations: bool,
    pub fullbright: bool,
    pub hidden_when_filled: bool,
    pub click_through: bool,
    pub fire_immune: bool,
}

pub const FRAME_KINDS: &[FrameKind] = &[
    FrameKind { item: "reversed_gold_item_frame", variant: 0, rotation_step: -1, winds_down: false, more_rotations: false, fullbright: false, hidden_when_filled: false, click_through: false, fire_immune: false },
    FrameKind { item: "diamond_item_frame", variant: 1, rotation_step: 0, winds_down: true, more_rotations: true, fullbright: false, hidden_when_filled: false, click_through: false, fire_immune: false },
    FrameKind { item: "manyullyn_item_frame", variant: 2, rotation_step: 0, winds_down: false, more_rotations: true, fullbright: true, hidden_when_filled: false, click_through: false, fire_immune: false },
    FrameKind { item: "gold_item_frame", variant: 3, rotation_step: 1, winds_down: false, more_rotations: false, fullbright: false, hidden_when_filled: false, click_through: false, fire_immune: false },
    FrameKind { item: "clear_item_frame", variant: 4, rotation_step: 0, winds_down: false, more_rotations: false, fullbright: false, hidden_when_filled: true, click_through: true, fire_immune: false },
    FrameKind { item: "netherite_item_frame", variant: 5, rotation_step: 0, winds_down: false, more_rotations: true, fullbright: false, hidden_when_filled: false, click_through: false, fire_immune: true },
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
    // The variants themselves, with PlutoLib's hanging registry: the tables above are the registration rows,
    // keyed by item and by the saved variant index.
    for kind in FRAME_KINDS {
        let _ = HangingRegistry::register_frame(FrameVariant {
            item: Identifier::parse(&format!("tconstruct:{}", kind.item)).expect("valid id"),
            entity_type: Identifier::parse("tconstruct:fancy_item_frame").expect("valid id"),
            variant: kind.variant,
            rotation_step: kind.rotation_step,
            winds_down: kind.winds_down,
            more_rotations: kind.more_rotations,
            fullbright: kind.fullbright,
            hidden_when_filled: kind.hidden_when_filled,
            click_through: kind.click_through,
            fire_immune: kind.fire_immune,
        });
    }
    for kind in STAND_KINDS {
        let _ = HangingRegistry::register_stand(StandVariant {
            item: Identifier::parse(&format!("tconstruct:{}", kind.item)).expect("valid id"),
            entity_type: Identifier::parse("tconstruct:armor_stand").expect("valid id"),
            variant: kind.variant,
            small: kind.small,
            arms: kind.arms,
            fullbright: kind.fullbright,
            hidden_when_equipped: kind.hidden_when_equipped,
        });
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
        assert!(FRAME_KINDS.iter().find(|kind| kind.item == "clear_item_frame").unwrap().click_through, "the clear frame uses the block behind it while filled");
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

    #[test]
    fn the_variants_register_with_the_hanging_registry() {
        register();
        for kind in FRAME_KINDS {
            let found = HangingRegistry::frame_for_item(
                &Identifier::parse(&format!("tconstruct:{}", kind.item)).unwrap())
                .expect("the frame variant registered");
            assert_eq!(found.variant, kind.variant);
            assert_eq!(found.rotation_step, kind.rotation_step);
            assert_eq!(found.winds_down, kind.winds_down);
            assert_eq!(found.more_rotations, kind.more_rotations);
            assert_eq!(found.click_through, kind.click_through);
            assert_eq!(found.fire_immune, kind.fire_immune);
            assert_eq!(found.entity_type, Identifier::parse("tconstruct:fancy_item_frame").unwrap());
            assert_eq!(HangingRegistry::frame_for_variant(kind.variant), Some(found));
        }
        for kind in STAND_KINDS {
            let found = HangingRegistry::stand_for_item(
                &Identifier::parse(&format!("tconstruct:{}", kind.item)).unwrap())
                .expect("the stand variant registered");
            assert_eq!(found.variant, kind.variant);
            assert_eq!(found.small, kind.small);
            assert_eq!(found.arms, kind.arms);
            assert_eq!(found.hidden_when_equipped, kind.hidden_when_equipped);
            assert_eq!(found.entity_type, Identifier::parse("tconstruct:armor_stand").unwrap());
            assert_eq!(HangingRegistry::stand_for_variant(kind.variant), Some(found));
        }
    }
}

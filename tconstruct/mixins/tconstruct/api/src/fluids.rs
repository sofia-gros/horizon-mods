//! The fluids of SlimeKnights TinkerFluids (MIT; see LICENSE), defined through PlutoLib's `FluidRegistry`.
//!
//! Each of the 72 source fluids takes the attributes its FluidType.Properties give it (tools/import_original_data.py reads them
//! into `fluid_defs`), and its name is the original's `fluid.tconstruct.<name>` translation. The 70 that have a block there (all but the
//! potion and powdered snow, which are `unplacable`) are world fluids with a bucket: the block `<name>_fluid` (the original's block ids), its stepped models and the bucket item `<name>_bucket` are
//! the importer's content, the flow is the family's of FluidDeferredRegisterExtension (slime 50 ticks, stone 40, metal 30, gem 45, glass
//! 30, the level drop and slope distance of each) and PlutoLib flows the block and runs the bucket. The potion is `unplacable` too, but
//! its bucket is its own item and carries the fluid's tag as the item's own data (the original's `PotionBucketItem`), where powdered
//! snow's bucket is the vanilla item, registered in `fluid_items`. Fluids with a negative density (ichor,
//! molten cinderslime) rise, as the original's inverted fluids do. Entity contact (`BurningLiquidBlock`, `MobEffectLiquidBlock`) is kept
//! as data with the actions it makes; the host has no entity-inside hook that can act on the entity yet.
//! The fluid tags (`data/tconstruct/tags/fluid`) are read by the host and reach this mod through `reload_registry("minecraft:fluid")`.
use pluto_lib::{ActionTarget, ColorProviderRegistry, EntityAction, FluidDef, FluidRegistry, Identifier};
use crate::fluid_defs::{Contact, FLUIDS, FluidSpec};

/// The Tinkers namespace.
const NAMESPACE: &str = "tconstruct";

fn id(name: &str) -> Identifier { Identifier::new(NAMESPACE, name).expect("generated fluid ids are valid") }

fn definition(spec: &FluidSpec) -> FluidDef {
    let mut def = FluidDef::new(id(spec.name))
        .temperature(spec.temperature)
        .luminance(spec.luminance)
        .density(spec.density)
        .viscosity(spec.viscosity)
        .color(spec.color)
        .flow(spec.tick_delay, spec.level_drop, spec.slope_distance);
    if spec.placeable { def = def.with_world_blocks(); }
    if spec.bucket { def = def.with_bucket(); }
    if spec.bucket_tagged { def = def.bucket_carries_tag(); }
    // FluidType sounds: the hot builder uses the lava bucket's, the others the water bucket's (PlutoLib's default)
    if spec.hot { def = def.bucket_sounds("minecraft:item.bucket.fill_lava", "minecraft:item.bucket.empty_lava"); }
    def
}

/// All fluid definitions in the original's registration order.
pub(crate) fn definitions() -> Vec<FluidDef> { FLUIDS.iter().map(definition).collect() }

/// What an entity inside the fluid's block suffers, `Contact::None` for a fluid without one.
pub(crate) fn contact(fluid: &str) -> Contact {
    FLUIDS.iter().find(|spec| spec.name == fluid).map_or(Contact::None, |spec| spec.contact)
}

/// The actions an entity standing in the fluid takes: `BurningLiquidBlock.entityInside` sets a non-fire-immune entity on fire for the
/// burn time and hurts it with lava damage; `MobEffectLiquidBlock.entityInside` gives a living entity the effect (levels are the amplifier
/// plus one). `fluid_height_above_zero` is `entity.getFluidTypeHeight > 0`, the entity being in the fluid and not only in the block's cell.
pub(crate) fn contact_actions(contact: Contact, fire_immune: bool, living: bool, in_fluid: bool) -> Vec<EntityAction> {
    if !in_fluid { return Vec::new(); }
    match contact {
        Contact::None => Vec::new(),
        Contact::Burning { seconds, damage } if !fire_immune => vec![
            EntityAction::SetFire { target: ActionTarget::Victim, seconds },
            EntityAction::Hurt { target: ActionTarget::Victim, amount: damage, damage_type: "minecraft:lava".to_owned() },
        ],
        Contact::Burning { .. } => Vec::new(),
        Contact::Effect { effect, amplifier, duration } if living => vec![EntityAction::AddEffect { target: ActionTarget::Victim, effect: effect.to_owned(), amplifier, duration }],
        Contact::Effect { .. } => Vec::new(),
    }
}

/// Defines every fluid. Registering an id again replaces it, so calling this twice is harmless. The fluids with a tint (the soups, the stones
/// and clay) get their block and bucket coloured by it, as the original's fluid texture colour does.
pub(crate) fn register() {
    for def in definitions() { FluidRegistry::register(def); }
    for spec in FLUIDS.iter().filter(|spec| spec.placeable && spec.color != 0xFFFF_FFFF) {
        let rgb = spec.color & 0x00FF_FFFF;
        ColorProviderRegistry::register_block(&[id(spec.name)], move |_| Some(rgb));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{BlockBehaviors, FluidItems, FluidStack, FluidWorld, ItemSlot, MemoryBlockView};

    #[test]
    fn the_72_fluids_register_with_blocks_and_buckets_but_the_powdered_snow() {
        register();
        let all = definitions();
        assert_eq!(all.len(), 72);
        assert_eq!(all.iter().filter(|def| def.world_blocks).count(), 70);
        assert_eq!(all.iter().filter(|def| def.bucket).count(), 71);
        let potion = FluidRegistry::get("tconstruct:potion").unwrap();
        assert!(potion.bucket && potion.bucket_carries_tag && !potion.world_blocks, "the potion's bucket is its own and carries the tag");
        let snow = FluidRegistry::get("tconstruct:powdered_snow").unwrap();
        assert!(!snow.bucket && !snow.bucket_carries_tag && !snow.world_blocks, "powdered snow's bucket is the vanilla item, not the fluid's own");
        let unplaced: Vec<_> = all.iter().filter(|def| !def.world_blocks).map(|def| def.id.path().to_owned()).collect();
        assert_eq!(unplaced, ["powdered_snow", "potion"]);
        assert!(BlockBehaviors::get("tconstruct:molten_iron_fluid").is_some());
        assert!(BlockBehaviors::get("tconstruct:potion_fluid").is_none());
        // families: metal 30 ticks, 3 per block; slime 50 ticks; glass flows one cell; fiery liquid is a plain fluid at 30 ticks
        let flow = |name: &str| FluidRegistry::get(&format!("tconstruct:{name}")).unwrap().flow;
        assert_eq!((flow("molten_iron").tick_delay, flow("molten_iron").level_drop, flow("molten_iron").slope_distance), (30, 3, 3));
        assert_eq!((flow("earth_slime").tick_delay, flow("earth_slime").level_drop), (50, 1));
        assert_eq!((flow("molten_glass").level_drop, flow("seared_stone").tick_delay, flow("molten_emerald").tick_delay), (8, 40, 45));
        assert_eq!((flow("fiery_liquid").tick_delay, flow("fiery_liquid").level_drop, flow("blazing_blood").tick_delay), (30, 1, 5));
        assert_eq!(FluidRegistry::get("tconstruct:molten_iron").unwrap().bucket_fill_sound.as_deref(), Some("minecraft:item.bucket.fill_lava"));
        assert_eq!(FluidRegistry::get("tconstruct:honey").unwrap().bucket_fill_sound, None);
    }

    #[test]
    fn a_bucket_of_molten_iron_holds_a_bucket_and_an_empty_bucket_fills_with_it() {
        register();
        let bucket = ItemSlot::new(FluidWorld::bucket_id(&id("molten_iron")), 1);
        assert_eq!(FluidItems::contents(&bucket), Some(FluidStack { fluid: id("molten_iron"), amount: 1000, tag: None }));
        let empty = ItemSlot::new(Identifier::parse("minecraft:bucket").unwrap(), 1);
        for name in ["molten_iron", "venom", "fiery_liquid"] {
            let filled = FluidItems::fill(&empty, &FluidStack { fluid: id(name), amount: 1000, tag: None });
            assert_eq!(filled.container.item.to_string(), format!("tconstruct:{name}_bucket"), "the empty bucket knows every bucket");
        }
        // the potion's bucket carries the fluid's tag as the item's own data (PotionBucketItem: the stack's tag is the fluid's)
        let mut tag = pluto_lib::NbtCompound::new();
        tag.insert("Potion".to_owned(), pluto_lib::Nbt::String("minecraft:healing".to_owned()));
        let tagged = FluidStack { fluid: id("potion"), amount: 1000, tag: Some(tag.clone()) };
        let filled = FluidItems::fill(&empty, &tagged);
        assert_eq!((filled.amount, filled.container.item.to_string(), filled.container.custom_data.clone()), (1000, "tconstruct:potion_bucket".to_owned(), tag));
        assert_eq!(FluidItems::contents(&filled.container), Some(tagged.clone()));
        let drained = FluidItems::drain(&filled.container, None, 1000).unwrap();
        assert_eq!(drained.fluid, tagged);
        assert_eq!(drained.container.unwrap().item.to_string(), "minecraft:bucket");
    }

    #[test]
    fn molten_iron_flows_with_the_metal_family_and_ichor_climbs() {
        register();
        let iron = FluidRegistry::get("tconstruct:molten_iron").unwrap();
        let mut world = MemoryBlockView::default();
        for x in -6..=6 { for z in -6..=6 { world.blocks.insert([x, 0, z], pluto_lib::BlockState::parse("minecraft:stone").unwrap()); } }
        assert!(FluidWorld::place_source(&mut world, [0, 1, 0], &iron));
        let mut guard = 0;
        while let Some((pos, _, _)) = world.scheduled.first().cloned() { world.scheduled.remove(0); FluidWorld::tick(&mut world, pos, &iron); guard += 1; assert!(guard < 2000); }
        let level = |pos| world.blocks.get(&pos).and_then(|state| FluidWorld::level_of(state, &iron.id));
        assert_eq!((level([1, 1, 0]), level([2, 1, 0]), level([3, 1, 0])), (Some(3), Some(6), None), "3 levels lost per block");
        assert!(FluidRegistry::get("tconstruct:ichor").unwrap().attributes.density < 0);
    }

    #[test]
    fn contact_burns_or_gives_the_effect_only_to_an_entity_in_the_fluid() {
        let lava_like = contact("molten_iron");
        assert_eq!(lava_like, Contact::Burning { seconds: 10, damage: 5.0 });
        let actions = contact_actions(lava_like, false, true, true);
        assert_eq!(actions, vec![
            EntityAction::SetFire { target: ActionTarget::Victim, seconds: 10 },
            EntityAction::Hurt { target: ActionTarget::Victim, amount: 5.0, damage_type: "minecraft:lava".to_owned() },
        ]);
        assert!(contact_actions(lava_like, true, true, true).is_empty(), "fire-immune entities are left alone");
        assert!(contact_actions(lava_like, false, true, false).is_empty(), "standing in the block's cell above the fluid is not being in it");
        let venom = contact("venom");
        assert_eq!(contact_actions(venom, false, true, true), vec![EntityAction::AddEffect { target: ActionTarget::Victim, effect: "minecraft:poison".to_owned(), amplifier: 0, duration: 100 }]);
        assert!(contact_actions(venom, false, false, true).is_empty(), "an item takes no effect");
        assert_eq!(contact("ichor"), Contact::None);
        assert_eq!(contact("fiery_liquid"), Contact::Burning { seconds: 20, damage: 6.0 });
    }
}

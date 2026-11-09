use super::*;
use pluto_lib::MemoryBlockView;

#[test]
fn seed_callback_preserves_vine_sides_and_consumes_only_successful_survival_use() {
    use pluto_lib::{MemoryPlayerOps, MemoryWorldOps, ServerPlayer, UseItemOnBlockContext};
    use pluto_lib::player_api::with_ops;
    use pluto_lib::world_api_v1::with_world;
    register_seeds();
    for (creative, ground, expected, consumed) in [
        (false, "minecraft:dirt", Some("tconstruct:sky_vanilla_slime_grass"), true),
        (true, "minecraft:dirt", Some("tconstruct:sky_vanilla_slime_grass"), false),
        (false, "minecraft:stone", None, false),
        (false, "minecraft:vine[north=true,up=true]", Some("tconstruct:sky_slime_vine[north=true,stage=start,up=true]"), true),
    ] {
        let key = ("minecraft:overworld".to_owned(), [0, 64, 0]);
        let item = "tconstruct:sky_slime_grass_seeds";
        let mut world = MemoryWorldOps::default();
        world.blocks.insert(key.clone(), ground.to_owned());
        let mut player = MemoryPlayerOps::default();
        player.items.insert((7, item.to_owned()), 2);
        let used = UseItemOnBlockContext { item: item.to_owned(), count: 2, pos: key.1, face: [0, 1, 0], dimension: key.0.clone(), player: ServerPlayer { uuid: 7, name: "Test".into() }, creative };
        let result = with_world(&mut world, || with_ops(&mut player, || UseItemOnBlockCallback::EVENT.invoke(&used)));
        assert_eq!(result, if expected.is_some() { ActionResult::Success } else { ActionResult::Pass });
        assert_eq!(state(&world.blocks[&key]), state(expected.unwrap_or(ground)));
        assert_eq!(player.items[&(7, item.to_owned())], if consumed { 1 } else { 2 });
        assert_eq!(world.sounds.len(), usize::from(expected.is_some()));
    }
}

fn state(id: &str) -> BlockState { BlockState::parse(id).unwrap() }

// SlimeGrassBlock.growGrass: plant == 0 is a sapling only when enabled;
// otherwise plant < 6 is a fern, with the remaining 26 choices tall grass.
#[test]
fn bonemeal_uses_the_java_plant_boundaries_and_does_not_replace_existing_plants() {
    for (roll, sapling, expected) in [(0, true, "slime_sapling"), (0, false, "slime_fern"), (5, false, "slime_fern"), (6, false, "slime_tall_grass")] {
        let mut world = MemoryBlockView::default();
        world.blocks.insert([0, 64, 0], state("tconstruct:earth_slime_dirt"));
        world.rolls = vec![roll];
        grow_grass(&mut world, [0, 64, 0], "test:ground", "earth", sapling, false);
        assert_eq!(world.blocks[&[0, 65, 0]], state(&format!("tconstruct:earth_{expected}")));
        grow_grass(&mut world, [0, 64, 0], "test:ground", "sky", sapling, false);
        assert_eq!(world.blocks[&[0, 65, 0]], state(&format!("tconstruct:earth_{expected}")));
    }
}

// First plant chooses 31 (tall). The first walk at i=16 consumes four
// nextInt(3) draws: 2,1,0,1 -> offset (1,0,0), then 0 chooses a fern.
#[test]
fn bonemeal_walk_respects_draw_order_support_tags_and_opaque_targets() {
    for blocked in [false, true] {
        let mut world = MemoryBlockView::default();
        for x in [0, 1] {
            world.blocks.insert([x, 64, 0], state("tconstruct:earth_slime_dirt"));
            world.tags.entry([x, 64, 0]).or_default().insert("test:ground".into());
        }
        if blocked { world.blocks.insert([1, 65, 0], state("minecraft:stone")); }
        world.rolls = vec![31, 2, 1, 0, 1, 0];
        grow_grass(&mut world, [0, 64, 0], "test:ground", "earth", false, false);
        assert_eq!(world.blocks[&[0, 65, 0]], state("tconstruct:earth_slime_tall_grass"));
        assert_eq!(world.blocks[&[1, 65, 0]], state(if blocked { "minecraft:stone" } else { "tconstruct:earth_slime_fern" }));
    }
    let mut unsupported = MemoryBlockView::default();
    unsupported.blocks.insert([0, 64, 0], state("minecraft:stone"));
    grow_grass(&mut unsupported, [0, 64, 0], "test:ground", "earth", false, false);
    assert!(unsupported.is_air([0, 65, 0]));
}

#[test]
fn seeds_and_plants_keep_original_soil_and_namespace_constraints() {
    for foliage in ["earth", "sky", "ender", "blood", "ichor"] {
        for soil in ["vanilla", "earth", "sky", "ender", "ichor"] {
            let dirt = state(&if soil == "vanilla" { "minecraft:dirt".into() } else { format!("tconstruct:{soil}_slime_dirt") });
            assert_eq!(grass_on(&dirt, foliage), Some(state(&format!("tconstruct:{foliage}_{soil}_slime_grass"))));
        }
    }
    for ground in ["minecraft:dirt", "minecraft:stone", "other:earth_slime_dirt", "tconstruct:blood_slime_dirt"] {
        let mut world = MemoryBlockView::default();
        world.blocks.insert([0, 63, 0], state(ground));
        assert!(!plant_can_stand(&world, [0, 64, 0]), "{ground}");
    }
    for ground in ["tconstruct:earth_slime_dirt", "tconstruct:sky_slime_dirt", "tconstruct:ichor_slime_dirt", "tconstruct:ender_slime_dirt", "tconstruct:earth_vanilla_slime_grass"] {
        let mut world = MemoryBlockView::default();
        world.blocks.insert([0, 63, 0], state(ground));
        assert!(plant_can_stand(&world, [0, 64, 0]), "{ground}");
    }
    for ground in ["minecraft:coarse_dirt", "minecraft:grass_block", "tconstruct:earth_earth_slime_grass"] {
        assert!(grass_on(&state(ground), "earth").is_none());
    }
}

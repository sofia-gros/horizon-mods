//! SlimePropaguleBlock/SlimePropaguleLeavesBlock gameplay through PlutoLib.
//! Native mangrove-template shapes, fluid state and shape water ticks remain
//! inherited; only the original Tinkers support/growth rules are overridden.
use pluto_lib::{BlockBehaviors, BlockEditor, BlockState, Direction, Placement};

type Pos = [i32; 3];

fn hanging(state: &BlockState) -> bool { state.property("hanging") == Some("true") }
fn immature(state: &BlockState) -> bool {
    state.property("age").and_then(|v| v.parse::<u8>().ok()).is_some_and(|age| age < 4)
}

fn mature(world: &mut dyn BlockEditor, pos: Pos, state: &BlockState) {
    if let Some(age) = state.property("age").and_then(|v| v.parse::<u8>().ok()).filter(|age| *age < 4) {
        world.set_block_state_with_flags(pos, &state.clone().with("age", &(age + 1).to_string()), 2);
    }
}

fn advance(world: &mut dyn BlockEditor, pos: Pos, state: &BlockState) {
    match state.property("stage") {
        Some("0") => { world.set_block_state_with_flags(pos, &state.clone().with("stage", "1"), 4); },
        Some("1") => {
            // Same conservative loaded-chunk boundary as the previous live
            // grower: never access a chunk merely to force tree generation.
            if !world.is_area_loaded(pos, 24) { return }
            let tall = world.random_float() < 0.85;
            let replacement = if state.property("waterlogged") == Some("true") { "minecraft:water[level=0]" } else { "minecraft:air" };
            if !world.set_block_state_with_flags(pos, &BlockState::parse(replacement).expect("fixed state"), 4) { return }
            if !crate::worldgen::grow_ender_tree(world, pos, tall) {
                // AbstractTreeGrower restores only its sapling after failure;
                // it does not roll back any other completed feature writes.
                world.set_block_state_with_flags(pos, state, 4);
            }
        },
        _ => {},
    }
}

fn random_tick(world: &mut dyn BlockEditor, pos: Pos, state: &BlockState) {
    if hanging(state) { mature(world, pos, state); }
    else if world.random_int(7) == 0 { advance(world, pos, state); }
}

fn bonemeal(world: &mut dyn BlockEditor, pos: Pos, state: &BlockState) {
    if hanging(state) { mature(world, pos, state); }
    else { advance(world, pos, state); }
}

pub fn register() {
    let sapling = BlockBehaviors::get("tconstruct:ender_slime_sapling").expect("propagule survival registered first");
    BlockBehaviors::register(sapling
        .state_for_placement(|context| Placement::State(context.state.clone()
            .with("hanging", "false").with("stage", "0").with("age", "4")
            .with("waterlogged", if context.in_water { "true" } else { "false" })))
        .random_tick(|context| random_tick(context.world, context.pos, &context.state))
        .can_bonemeal(|context| !hanging(&context.state) || immature(&context.state))
        .bonemeal_success(|context| if hanging(&context.state) { immature(&context.state) } else { context.world.random_float() < 0.45 })
        .on_bonemeal(|context| bonemeal(context.world, context.pos, &context.state)));
    let leaves = BlockBehaviors::get("tconstruct:ender_slime_leaves").expect("slime leaf behaviour registered first");
    // Keep the existing leaf distance/placement callbacks while adding the
    // original hanging-propagule action, rather than replacing its behaviour.
    BlockBehaviors::register(leaves
        .can_bonemeal(|context| context.view.is_air(Direction::Down.relative(context.pos)))
        .bonemeal_success(|_| true)
        .on_bonemeal(|context| {
            let below = Direction::Down.relative(context.pos);
            if context.world.is_air(below) {
                context.world.set_block_state_with_flags(below, &BlockState::parse("tconstruct:ender_slime_sapling[age=0,hanging=true,stage=0,waterlogged=false]").expect("fixed state"), 2);
            }
        }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use pluto_lib::{BlockView, MemoryBlockView, RootPlacement, TreeDecoration};
    fn sapling(properties: &str) -> BlockState { BlockState::parse(&format!("tconstruct:ender_slime_sapling[{properties}]")).unwrap() }

    #[test]
    fn maturation_and_two_stage_growth_preserve_flags_and_do_not_draw_unneeded_randomness() {
        let mut world = MemoryBlockView::default();
        world.rolls = vec![6]; world.floats = vec![0.5];
        random_tick(&mut world, [0,64,0], &sapling("hanging=true,age=3,stage=0"));
        assert_eq!(world.blocks[&[0,64,0]].property("age"), Some("4"));
        assert_eq!(world.writes[0].2, 2);
        random_tick(&mut world, [0,64,0], &sapling("hanging=true,age=4,stage=0"));
        assert_eq!(world.writes.len(), 1);
        assert_eq!(world.rolls, [6]); assert_eq!(world.floats, [0.5]);
        random_tick(&mut world, [0,64,0], &sapling("hanging=false,age=4,stage=0"));
        assert_eq!(world.writes.len(), 1);
        bonemeal(&mut world, [0,64,0], &sapling("hanging=false,age=4,stage=0"));
        assert_eq!(world.blocks[&[0,64,0]].property("stage"), Some("1"));
        assert_eq!(world.writes[1].2, 4);
        world.unloaded.insert([0,64,0]);
        advance(&mut world, [0,64,0], &sapling("hanging=false,age=4,stage=1"));
        assert_eq!(world.writes.len(), 2); assert_eq!(world.floats, [0.5]);
    }

    #[derive(Default)]
    struct Recorder { memory: MemoryBlockView, success: bool, bases: Vec<i64> }
    impl BlockView for Recorder {
        fn block_state(&self,p:Pos)->Option<BlockState>{self.memory.block_state(p)}
        fn is_air(&self,p:Pos)->bool{self.memory.is_air(p)}
        fn is_face_sturdy(&self,p:Pos,d:Direction)->bool{self.memory.is_face_sturdy(p,d)}
        fn is_in_tag(&self,p:Pos,t:&str)->bool{self.memory.is_in_tag(p,t)}
        fn brightness(&self,p:Pos)->u8{self.memory.brightness(p)}
        fn light_passes_up(&self,p:Pos)->bool{self.memory.light_passes_up(p)}
        fn is_water(&self,p:Pos)->bool{self.memory.is_water(p)}
        fn is_fluid_full(&self,p:Pos)->bool{self.memory.is_fluid_full(p)}
        fn is_area_loaded(&self,p:Pos,r:i32)->bool{self.memory.is_area_loaded(p,r)}
        fn is_full_block(&self,p:Pos)->bool{self.memory.is_full_block(p)}
        fn build_height(&self)->[i32;2]{self.memory.build_height()}
    }
    impl BlockEditor for Recorder {
        fn set_block_state(&mut self,p:Pos,s:&BlockState)->bool{self.memory.set_block_state(p,s)}
        fn set_block_state_with_flags(&mut self,p:Pos,s:&BlockState,f:i32)->bool{self.memory.set_block_state_with_flags(p,s,f)}
        fn random_int(&mut self,b:i32)->i32{self.memory.random_int(b)}
        fn random_float(&mut self)->f32{self.memory.random_float()}
        fn schedule_tick(&mut self,p:Pos,b:&str,d:i32)->bool{self.memory.schedule_tick(p,b,d)}
        fn place_feature(&mut self,f:&str,p:Pos)->bool{self.memory.place_feature(f,p)}
        fn place_tree_with_plan(&mut self,c:&serde_json::Value,_:Pos,r:Option<&RootPlacement>,p:&[TreeDecoration])->bool {
            assert!(r.is_some());
            assert!(matches!(p, [TreeDecoration::Custom(_), TreeDecoration::Native(0)]));
            self.bases.push(c["trunk_placer"]["base_height"].as_i64().unwrap());
            self.success
        }
    }
    #[test]
    fn tree_choice_uses_java_float_boundary_and_restores_waterlogged_sapling_only_on_failure() {
        crate::worldgen::register();
        for (draw, expected_base, success) in [(0.849,4,true),(0.85,2,true),(0.0,4,false)] {
            let mut world = Recorder { success, ..Default::default() };
            world.memory.floats = vec![draw,0.3];
            let state = sapling("hanging=false,age=4,stage=1,waterlogged=true");
            advance(&mut world, [0,64,0], &state);
            assert_eq!(world.bases, [expected_base]);
            assert_eq!(world.memory.floats, [0.3]);
            assert_eq!(world.memory.writes[0].1.block.to_string(), "minecraft:water");
            assert_eq!(world.memory.writes[0].2, 4);
            assert_eq!(world.memory.writes.len(), if success {1} else {2});
            if !success { assert_eq!(world.memory.writes[1], ([0,64,0], state, 4)); }
        }
    }
}

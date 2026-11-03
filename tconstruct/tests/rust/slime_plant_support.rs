#[path = "../../mixins/tconstruct/source-slime-plant-support.rs"]
mod slime_plant_support;

#[derive(Clone, Copy)]
struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}
impl BlockPos {
    fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}
type BlockStateId = usize;
struct State {
    name: String,
}
struct Registry(Vec<State>);
impl Registry {
    fn state(&self, id: usize) -> Option<&State> {
        self.0.get(id)
    }
}
struct GameLevel<'a> {
    registry: &'a Registry,
    ground: usize,
}
impl GameLevel<'_> {
    fn block_state(&self, pos: BlockPos) -> usize {
        let _ = (pos.x, pos.y, pos.z);
        self.ground
    }
}
mod tconstruct_slime_plant_support {
    pub(super) use super::slime_plant_support::{is_plant, supports};
}
mod adapter {
    use super::*;
    include!("../../mixins/tconstruct/source-slime-plant-survive.rs");
    #[test]
    fn applies_only_to_plants_and_handles_missing_or_invalid_ground() {
        let registry = Registry(vec![
            State {
                name: "tconstruct:blood_slime_fern".into(),
            },
            State {
                name: "tconstruct:sky_earth_slime_grass".into(),
            },
            State {
                name: "minecraft:dirt".into(),
            },
            State {
                name: "minecraft:fern".into(),
            },
        ]);
        let pos = BlockPos::new(1, 10, 2);
        let mut result = None;
        tconstruct_slime_plant_survive(
            &GameLevel {
                registry: &registry,
                ground: 1,
            },
            pos,
            0,
            &mut result,
        );
        assert_eq!(result, Some(true));
        tconstruct_slime_plant_survive(
            &GameLevel {
                registry: &registry,
                ground: 2,
            },
            pos,
            0,
            &mut result,
        );
        assert_eq!(result, Some(false));
        result = None;
        tconstruct_slime_plant_survive(
            &GameLevel {
                registry: &registry,
                ground: 1,
            },
            pos,
            3,
            &mut result,
        );
        assert_eq!(result, None);
        tconstruct_slime_plant_survive(
            &GameLevel {
                registry: &registry,
                ground: 1,
            },
            BlockPos::new(1, i32::MIN, 2),
            0,
            &mut result,
        );
        assert_eq!(result, Some(false));
    }
}

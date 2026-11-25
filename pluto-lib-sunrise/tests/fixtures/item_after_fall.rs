// Trace-only host double: execute the production callback adapter, not a
// second collision solver. The pinned host supplies the resolved contact.
#![allow(dead_code)]
extern crate self as sunrise_core;
extern crate self as sunrise_world;
pub mod position {
    #[derive(Clone, Copy)]
    pub struct BlockPos { pub x: i32, pub y: i32, pub z: i32 }
}
pub mod entities {
    #[derive(Clone, Copy)]
    pub struct Vec3 { pub x: f64, pub y: f64, pub z: f64 }
    pub struct ItemEntity { pub id: u32, pub position: Vec3, pub velocity: Vec3, pub on_ground: bool }
}
pub mod world {
    pub struct World;
    impl World {
        pub fn block(&self, x: i32, y: i32, z: i32) -> bool { [x, y, z] == [0, 63, 0] }
    }
}
pub mod block_collision {
    pub struct BlockCollisionRegistry<'a>(pub &'a ());
    pub struct Definition { pub name: String, pub properties: Vec<(String, String)> }
    impl BlockCollisionRegistry<'_> {
        pub fn registry(&self) -> &Self { self }
        pub fn state(&self, support: bool) -> Option<Definition> {
            Some(Definition { name: if support { "test:support" } else { "minecraft:air" }.into(), properties: vec![] })
        }
    }
}
pub mod level {
    pub mod pluto_block_access {
        pub struct StateInfo { pub block: String, pub properties: Vec<(String, String)> }
        pub struct AfterFall {
            pub pos: [i32; 3], pub entity: i32, pub kind: String,
            pub living: bool, pub item: bool, pub suppressing_bounce: bool,
            pub velocity: [f64; 3], pub fall_distance: f64, pub on_ground: bool,
        }
        pub struct Sink { pub after_fall: fn(&StateInfo) -> Option<fn(&mut AfterFall)> }
        pub fn landing_sink() -> Option<Sink> {
            Some(Sink { after_fall: |info| {
                if info.block != "test:support" { return None; }
                Some(|request| {
                    assert_eq!(request.pos, [0, 63, 0]);
                    assert_eq!(request.velocity, [0.1, -1.0, 0.2]);
                    request.velocity = [0.3, 0.8, 0.4];
                    request.on_ground = false;
                })
            } })
        }
    }
}
include!(PRODUCTION_HELPER);
fn item() -> entities::ItemEntity {
    entities::ItemEntity { id: 7,
        position: entities::Vec3 { x: 1.05, y: 64.0, z: 0.5 },
        velocity: entities::Vec3 { x: 0.1, y: -1.0, z: 0.2 }, on_ground: true }
}
#[test]
fn edge_landing_uses_native_support_not_center() {
    let mut item = item();
    let result = pluto_item_after_fall(0.0, &mut item, &world::World,
        &block_collision::BlockCollisionRegistry(&()),
        Some(position::BlockPos { x: 0, y: 63, z: 0 }));
    assert_eq!(result, 0.8);
    assert_eq!((item.velocity.x, item.velocity.z), (0.3, 0.4));
    assert!(!item.on_ground);
}
#[test]
fn absent_unhandled_and_airborne_contacts_keep_native_result() {
    for (ground, contact) in [
        (true, None),
        (true, Some(position::BlockPos { x: 1, y: 63, z: 0 })),
        (false, Some(position::BlockPos { x: 0, y: 63, z: 0 })),
    ] {
        let mut item = item();
        item.on_ground = ground;
        assert_eq!(pluto_item_after_fall(0.25, &mut item, &world::World,
            &block_collision::BlockCollisionRegistry(&()), contact), 0.25);
        assert_eq!((item.velocity.x, item.velocity.y, item.velocity.z), (0.1, -1.0, 0.2));
        assert_eq!(item.on_ground, ground);
    }
}

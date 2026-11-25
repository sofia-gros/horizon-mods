// Compile the actual native adapter body against reduced snapshots, not a
// reimplementation. This tests branching/math only, not native type bindings.
mod sunrise_world { pub mod level { pub mod pluto_item_behavior_access {
    include!("pluto-item-behavior-access.rs");
} } }
#[derive(Clone, Copy)]
struct Vec3 { x: f64, y: f64, z: f64 }
impl Vec3 { fn new(x: f64, y: f64, z: f64) -> Self { Self { x, y, z } } }
struct Mob { id: u32, position: Vec3 }
impl Mob { fn eye_y(&self) -> f64 { self.position.y + 1.0 } }
struct Player { head: Option<String>, uuid: [i32; 4], alive: bool, look: [f64; 3], position: Vec3, eye: [f64; 3] }
struct MobWorld<'a> { player: Option<Player>, sight: &'a std::cell::Cell<usize>, visible: bool }
impl MobWorld<'_> {
    fn line_of_sight(&self, from: Vec3, to: Vec3) -> bool {
        assert_eq!([from.x, from.y, from.z], [0.0, 1.0, 0.0]);
        assert_eq!([to.x, to.y, to.z], [0.0, 1.0, 10.0]);
        self.sight.set(self.sight.get() + 1); self.visible
    }
}
include!("pluto-item-mask-head.rs");
#[test]
fn actual_mask_adapter_preserves_fallback_explicit_false_and_native_gaze_geometry() {
    use std::ops::ControlFlow::{Break, Continue};
    sunrise_world::level::pluto_item_behavior_access::set_mask_dispatcher(|item, wearer, observer| {
        assert_eq!(wearer, 0xffff_ffff_0000_0002_8000_0000_0000_0004);
        assert_eq!(observer, 42);
        match item { "test:mask" => Some(true), "minecraft:carved_pumpkin" => Some(false), _ => None }
    }).unwrap();
    assert!(sunrise_world::level::pluto_item_behavior_access::set_mask_dispatcher(|_, _, _| None).is_err());
    let sight = std::cell::Cell::new(0);
    let mob = Mob { id: 42, position: Vec3::new(0.0, 0.0, 10.0) };
    let mut world = MobWorld { player: None, sight: &sight, visible: true };
    assert_eq!(pluto_item_ender_mask(&mob, &world), Continue(()));
    world.player = Some(Player { head: None, uuid: [-1, 2, i32::MIN, 4], alive: true,
        look: [0.0, 0.0, 1.0], position: Vec3::new(0.0, 0.0, 0.0), eye: [0.0, 1.0, 0.0] });
    assert_eq!(pluto_item_ender_mask(&mob, &world), Continue(()));
    world.player.as_mut().unwrap().head = Some("test:unregistered".into());
    assert_eq!(pluto_item_ender_mask(&mob, &world), Continue(()));
    world.player.as_mut().unwrap().head = Some("test:mask".into());
    assert_eq!(pluto_item_ender_mask(&mob, &world), Break(false));
    assert_eq!(sight.get(), 0);
    world.player.as_mut().unwrap().head = Some("minecraft:carved_pumpkin".into());
    assert_eq!(pluto_item_ender_mask(&mob, &world), Break(true));
    assert_eq!(sight.get(), 1);
    world.visible = false;
    assert_eq!(pluto_item_ender_mask(&mob, &world), Break(false));
    world.visible = true;
    world.player.as_mut().unwrap().look = [1.0, 0.0, 0.0];
    assert_eq!(pluto_item_ender_mask(&mob, &world), Break(false));
    assert_eq!(sight.get(), 2); // outside the cone never queries occlusion
    world.player.as_mut().unwrap().alive = false;
    assert_eq!(pluto_item_ender_mask(&mob, &world), Break(false));
    assert_eq!(sight.get(), 2);
}

// Exercise the actual position filter against a stand-in for the existing
// spawn-protection callback. Real server permission behavior needs a game test.
#[path = "../../mixins/tconstruct/source-area-shape.rs"]
mod area_shape;
mod game { pub(crate) use crate::area_shape as tconstruct_source_area_shape; }
pub struct Guest { pub conn: u32, pub yaw: f32, pub op: bool }
pub struct Hub { pub guests: Vec<Guest> }
pub struct LanHost { pub hub: Hub }
pub struct Game<'a> { pub marker: std::marker::PhantomData<&'a ()> }
impl Game<'_> {
    fn under_spawn_protection(&self, host: &LanHost, gi: usize, pos: [i32;3]) -> bool {
        !host.hub.guests[gi].op && pos[0] <= 1
    }
}
#[path = "../../mixins/tconstruct/source-area-protection.rs"]
mod area_protection;
#[test]
fn boundary_checks_each_candidate_and_operators_remain_allowed() {
    let game = Game { marker: std::marker::PhantomData };
    let host = LanHost { hub: Hub { guests: vec![Guest { conn: 1, yaw: 0.0, op: false }, Guest { conn: 2, yaw: 0.0, op: true }] } };
    let blocked = area_protection::protected(&game, &host, 1, [2,70,0], [0,0,1]).unwrap();
    assert_eq!(blocked.len(), 3);
    assert!(blocked.iter().all(|pos| pos[0] == 1));
    let all = area_shape::plane([2,70,0], [0,0,1], 0.0).unwrap();
    assert_eq!(all.iter().filter(|pos| !blocked.contains(*pos)).count(), 5);
    assert!(area_protection::protected(&game, &host, 2, [2,70,0], [0,0,1]).unwrap().is_empty());
    assert!(area_protection::protected(&game, &host, 99, [2,70,0], [0,0,1]).is_none());
}

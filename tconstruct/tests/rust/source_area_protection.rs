// Exercise the actual position filter against a stand-in for the existing
// spawn-protection callback. Real server permission behavior needs a game test.
#[path = "../../mixins/tconstruct/source-pure-rules.rs"]
mod area_shape;
mod game { pub(crate) use crate::area_shape as tconstruct_source_area_shape; }
pub struct Stack { pub item: String }
pub struct Guest { pub conn: u32, pub yaw: f32, pub op: bool, pub inventory: Vec<Option<Stack>>, pub selected: usize, pub offhand: Option<Stack> }
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
    let host = LanHost { hub: Hub { guests: vec![Guest { conn: 1, yaw: 0.0, op: false, inventory: vec![], selected: 0, offhand: None }, Guest { conn: 2, yaw: 0.0, op: true, inventory: vec![], selected: 0, offhand: None }] } };
    let blocked = area_protection::protected(&game, &host, 1, [2,70,0], [0,0,1]).unwrap();
    assert_eq!(blocked.len(), 1);
    assert!(blocked.iter().all(|pos| pos.0 == 1));
    let all = area_shape::plane([2,70,0], [0,0,1], 0.0).unwrap();
    assert_eq!(all.iter().filter(|pos| !blocked.contains(&(pos[0],pos[2]))).count(), 5);
    assert!(area_protection::protected(&game, &host, 2, [2,70,0], [0,0,1]).unwrap().is_empty());
    assert!(area_protection::protected(&game, &host, 99, [2,70,0], [0,0,1]).is_none());
}

#[test]
fn distant_tree_columns_are_checked_at_every_height() {
    let game = Game { marker: std::marker::PhantomData };
    let host = LanHost { hub: Hub { guests: vec![Guest { conn: 3, yaw: 0.0, op: false, inventory: vec![Some(Stack { item: "tconstruct:broad_axe".into() })], selected: 0, offhand: None }] } };
    let blocked = area_protection::protected(&game, &host, 3, [8,70,0], [0,1,0]).unwrap();
    assert!(blocked.contains(&(1,0)));
    assert!(!blocked.contains(&(2,0)));
}

#[test]
fn scythe_checks_protected_columns_at_depth_and_width() {
    let game=Game {marker:std::marker::PhantomData};
    let host=LanHost {hub:Hub {guests:vec![Guest {conn:3,yaw:0.0,op:false,inventory:vec![Some(Stack {item:"tconstruct:scythe".into()})],selected:0,offhand:None}]}};
    let blocked=area_protection::protected(&game,&host,3,[4,70,0],[1,0,0]).unwrap();
    assert!(blocked.contains(&(1,0)));
    assert!(!blocked.contains(&(2,0)));
}

#[test]
fn scythe_in_offhand_keeps_full_protection_radius() {
    let game=Game {marker:std::marker::PhantomData};
    let host=LanHost {hub:Hub {guests:vec![Guest {conn:3,yaw:0.0,op:false,inventory:vec![],selected:0,offhand:Some(Stack{item:"tconstruct:scythe".into()})}]}};
    let blocked=area_protection::protected_for(&game,&host,3,[4,70,0],[1,0,0],true).unwrap();
    assert!(blocked.contains(&(1,0)));
}

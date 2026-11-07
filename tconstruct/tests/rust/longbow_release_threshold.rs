// Actual release/draw/stat helpers with host actor and dispatch boundary doubles.
extern crate horizon_items as horizon_client;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
#[path = "../../mixins/tconstruct/source-pure-rules.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-projectile-stats.rs"] pub(crate) mod projectile;
#[path = "../../mixins/tconstruct/source-ranged-draw.rs"] pub(crate) mod draw;
#[path = "../../mixins/tconstruct/source-ranged-wear.rs"] pub(crate) mod wear;
mod game {
    use super::*;
    pub(crate) use crate::{projectile as tconstruct_projectile_stats,draw as tconstruct_ranged_draw,wear as tconstruct_source_ranged_wear};
    pub mod tconstruct_projectile_airborne {pub fn airborne(_: &super::Game<'_>)->bool{false}}
    pub mod tconstruct_glowing_world {pub fn has_glowing(_: &super::Game<'_>,_: &ItemStack)->bool{false}use super::ItemStack;}
    pub struct Position {pub y:f64}
    pub struct Health;
    impl Health {pub fn health(&self)->f32{20.0}}
    pub struct Player {pub position:Position,pub health:Health}
    impl Player {pub fn attribute(&mut self,_:&str)->Result<f64,()>{Ok(20.0)}}
    type Context=(f32,f32,Vec<(f32,f32,bool)>,bool,f64,i32,bool);
    #[derive(Default)] pub struct Chunks {pub contexts:std::collections::HashMap<u32,Context>}
    impl Chunks {pub fn __horizon_mixins_tconstruct_material_projectile_context_mut(&mut self)->&mut std::collections::HashMap<u32,Context>{&mut self.contexts}}
    pub struct Shape;
    impl Shape {pub fn max_y(&self)->i32{320}}
    pub struct World {pub chunks:Chunks,pub shape:Shape}
    pub struct Game<'a> {pub player:Option<Player>,pub world:World,pub lan_acting:u32,pub dispatched:Vec<(u32,Option<Context>)>,pub marker:std::marker::PhantomData<&'a ()>}
    impl Game<'_> {
        pub fn new()->Self{Self{player:None,world:World{chunks:Chunks::default(),shape:Shape},lan_acting:7,dispatched:Vec::new(),marker:std::marker::PhantomData}}
        pub fn release_bow(&mut self,_:&ItemStack,ticks:u32){self.dispatched.push((ticks,self.world.chunks.contexts.get(&self.lan_acting).cloned()));}
    }
}
include!("../../mixins/tconstruct/material-bow-release.rs");
fn bow(velocity:Option<f32>,broken:bool)->ItemStack {
    let mut stack=ItemStack::new("tconstruct:longbow",1);
    let mut fields=vec![("tic_max_damage",Tag::Int(100)),("tic_broken",Tag::Byte(i8::from(broken))),("tic_accuracy",Tag::Float(1.0f32.to_bits()))];
    if let Some(velocity)=velocity {fields.push(("tic_velocity",Tag::Float(velocity.to_bits())));}
    stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound(fields)));stack
}
#[test] fn source_threshold_distinguishes_both_reassociated_boundary_directions() {
    for (ticks,bits,expected) in [(1,0x403b512b,false),(3,0x3f6e23b7,true)] {
        let velocity=f32::from_bits(bits);let charge=projectile::bow_charge(ticks,20);
        assert_eq!(charge*velocity>=0.1,expected);
        assert_ne!((charge*3.0)*velocity>=0.3,expected);
        let mut game=game::Game::new();
        assert_eq!(tconstruct_bow_release(&mut game,&bow(Some(velocity),false),ticks,false),std::ops::ControlFlow::Break(()));
        assert_eq!(!game.dispatched.is_empty(),expected);
        if expected {
            let (dispatch,context)=&game.dispatched[0];
            assert_eq!(*dispatch,ticks.max(3));
            let context=context.as_ref().unwrap();
            assert_eq!(context.0.to_bits(),velocity.to_bits());
            assert_eq!(context.2[0].1.to_bits(),(charge*3.0).to_bits());
        }
        assert!(game.world.chunks.contexts.is_empty());
    }
}
#[test] fn equality_nearby_and_normal_charge_dispatch_are_preserved() {
    for velocity in [0.0,f32::from_bits(0.1f32.to_bits()-1),0.1,f32::from_bits(0.1f32.to_bits()+1),1.0] {
        for ticks in [0,1,3,20,40] {
            let mut game=game::Game::new();let charge=projectile::bow_charge(ticks,20);
            let _ = tconstruct_bow_release(&mut game,&bow(Some(velocity),false),ticks,false);
            assert_eq!(game.dispatched.len(),usize::from(charge*velocity>=0.1));
            assert!(game.world.chunks.contexts.is_empty());
        }
    }
}
#[test] fn release_gates_and_missing_stats_fallback_are_unchanged() {
    for (broken,hand_changed) in [(true,false),(false,true),(true,true)] {
        let mut game=game::Game::new();let _ = tconstruct_bow_release(&mut game,&bow(Some(1.0),broken),20,hand_changed);
        assert!(game.dispatched.is_empty());assert!(game.world.chunks.contexts.is_empty());
    }
    let mut game=game::Game::new();let mut stack=bow(Some(1.0),false);stack.item="minecraft:bow".into();
    assert_eq!(tconstruct_bow_release(&mut game,&stack,20,false),std::ops::ControlFlow::Continue(()));
    assert!(game.dispatched.is_empty());
    let _ = tconstruct_bow_release(&mut game,&bow(None,false),1,false);
    assert_eq!(game.dispatched.len(),1);assert_eq!(game.dispatched[0].0,1);assert!(game.dispatched[0].1.is_none());
}

include!("../../mixins/tconstruct/material-crossbow-charge.rs");
#[test]
fn extreme_draw_speed_reaches_both_active_bow_callers() {
    for (speed, expected) in [(0.0,i32::MAX as u32),(f32::from_bits(1),i32::MAX as u32),(1.0,20),(1.5,14),(1024.0,1),(f32::NAN,1),(-1.0,i32::MAX as u32)] {
        let mut stack=bow(Some(1.0),false);
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
            ("tic_draw_speed",Tag::Float(speed.to_bits())),
            ("tic_velocity",Tag::Float(1.0f32.to_bits())),
            ("tic_accuracy",Tag::Float(1.0f32.to_bits())),
        ])));
        assert_eq!(draw::draw_ticks(&stack),expected);
        let mut game=game::Game::new();
        let _=tconstruct_bow_release(&mut game,&stack,expected,false);
        assert_eq!(game.dispatched.len(),1);
        assert_eq!(game.dispatched[0].0,20);
        assert_eq!(game.dispatched[0].1.as_ref().unwrap().2[0].1.to_bits(),3.0f32.to_bits());
        if expected==i32::MAX as u32 {
            let mut early=game::Game::new();
            let _=tconstruct_bow_release(&mut early,&stack,20,false);
            assert!(early.dispatched.is_empty());
            assert_eq!(draw::vanilla_charge_ticks(expected,expected),20);
        }
        stack.item="tconstruct:crossbow".into();
        assert_eq!(tconstruct_crossbow_charge(&stack),std::ops::ControlFlow::Break(expected));
        stack.item="minecraft:crossbow".into();
        assert_eq!(tconstruct_crossbow_charge(&stack),std::ops::ControlFlow::Continue(()));
    }
}

#[test]
fn draw_saturation_preserves_saved_stat_fallback_and_overwield_bonus() {
    let mut stack=ItemStack::new("tconstruct:crossbow",1);
    assert_eq!(draw::draw_ticks(&stack),20);
    for value in [Tag::Int(0),Tag::String("bad".into())] {
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_draw_speed",value)])));
        assert_eq!(draw::draw_ticks(&stack),20);
    }
    stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
        ("tic_draw_speed",Tag::Float(0.0f32.to_bits())),
        ("tic_overwield_ranged",Tag::Int(1)),("tic_overslime",Tag::Int(1)),
        ("tic_overwield_draw_multiplier",Tag::Float(1.0f32.to_bits())),
    ])));
    assert_eq!(draw::draw_ticks(&stack),334);
}

#[test] fn clamped_accuracy_reaches_dispatch_without_changing_saved_metadata() {
    for (accuracy,spread) in [(0.05f32,27.0f32),(0.1,27.0),(1.0,0.0),(2.0,0.0)] {
        let mut stack=bow(Some(1.0),false);
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
            ("tic_max_damage",Tag::Int(100)),("tic_velocity",Tag::Float(1.0f32.to_bits())),
            ("tic_accuracy",Tag::Float(accuracy.to_bits())),
        ])));
        let mut game=game::Game::new();let _=tconstruct_bow_release(&mut game,&stack,20,false);
        assert_eq!(game.dispatched.len(),1);
        let context=game.dispatched[0].1.as_ref().unwrap();
        assert_eq!(context.0,1.0);assert_eq!(context.1.to_bits(),spread.to_bits());
        assert!(game.world.chunks.contexts.is_empty());
        assert_eq!(stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_accuracy"),Some(&Tag::Float(accuracy.to_bits())));
    }
}

#[test] fn fallback_velocity_clamp_reaches_dispatch_without_an_actor() {
    let mut stack=bow(Some(1.0),false);
    stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
        ("tic_max_damage",Tag::Int(100)),("tic_velocity",Tag::Float(2000.0f32.to_bits())),
        ("tic_accuracy",Tag::Float(1.0f32.to_bits())),
    ])));
    let mut game=game::Game::new();assert!(game.player.is_none());
    let _=tconstruct_bow_release(&mut game,&stack,20,false);
    assert_eq!(game.dispatched.len(),1);
    let context=game.dispatched[0].1.as_ref().unwrap();
    assert_eq!(context.0,1024.0);assert_eq!(context.1,0.0);
    assert!(game.world.chunks.contexts.is_empty());
    assert_eq!(stack.data().unwrap().other("minecraft:custom_data").unwrap().get("tic_velocity"),Some(&Tag::Float(2000.0f32.to_bits())));
}

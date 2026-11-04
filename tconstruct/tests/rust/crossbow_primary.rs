// Actual crossbow-use/stat helpers with host actor and dispatch boundary doubles.
extern crate horizon_items as horizon_client;
use horizon_items::items::ItemStack;
use horizon_world::nbt::Tag;
#[path = "../../mixins/tconstruct/source-dwarven.rs"] mod tconstruct_source_dwarven;
#[path = "../../mixins/tconstruct/source-projectile-stats.rs"] pub(crate) mod projectile;
#[path = "../../mixins/tconstruct/source-ranged-wear.rs"] pub(crate) mod wear;
mod game {
    use super::*;
    pub(crate) use crate::{projectile as tconstruct_projectile_stats,wear as tconstruct_source_ranged_wear};
    pub mod tconstruct_projectile_airborne {pub fn airborne(_: &super::Game<'_>)->bool{false}}
    pub mod tconstruct_glowing_world {pub fn has_glowing(_: &super::Game<'_>,_: &ItemStack)->bool{false}use super::ItemStack;}
    pub struct Position {pub y:f64}
    pub struct Health;
    impl Health {pub fn health(&self)->f32{20.0}}
    pub struct Player {pub position:Position,pub health:Health}
    impl Player {pub fn attribute(&mut self,_:&str)->Result<f64,()>{Ok(20.0)}}
    type Context=(f32,f32,Vec<(f64,f32,bool)>,bool,f64,i32,bool);
    #[derive(Default)] pub struct Chunks {pub contexts:std::collections::HashMap<u32,Context>}
    impl Chunks {pub fn __horizon_mixins_tconstruct_material_projectile_context_mut(&mut self)->&mut std::collections::HashMap<u32,Context>{&mut self.contexts}}
    pub struct Shape;
    impl Shape {pub fn max_y(&self)->i32{320}}
    pub struct World {pub chunks:Chunks,pub shape:Shape}
    pub struct Game<'a> {pub using:Option<()>,pub creative:bool,pub started:bool,pub player:Option<Player>,pub world:World,pub lan_acting:u32,pub dispatched:Vec<(u32,Option<Context>)>,pub marker:std::marker::PhantomData<&'a ()>}
    impl Game<'_> {
        pub fn new()->Self{Self{using:None,creative:false,started:false,player:None,world:World{chunks:Chunks::default(),shape:Shape},lan_acting:7,dispatched:Vec::new(),marker:std::marker::PhantomData}}
        pub fn shoot_crossbow(&mut self){self.dispatched.push((0,self.world.chunks.contexts.get(&self.lan_acting).cloned()));}
        pub fn projectile_slot(&self,_:&str)->Option<usize>{Some(0)}
        pub fn start_using_item(&mut self){self.started=true;}
    }
}
include!("../../mixins/tconstruct/material-crossbow-use.rs");
fn crossbow(ammo:Vec<ItemStack>)->ItemStack {
    let mut stack=ItemStack::new("tconstruct:crossbow",1);
    stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([
        ("tic_max_damage",Tag::Int(100)),("tic_velocity",Tag::Float(1.0f32.to_bits())),("tic_accuracy",Tag::Float(1.0f32.to_bits())),
    ])));
    stack.data_mut().charged=ammo;stack
}
fn ammo(id:&str)->ItemStack{ItemStack::new(id,1)}
#[test] fn homogeneous_supported_arrows_mark_source_middle_without_reordering() {
    for id in ["minecraft:arrow","minecraft:tipped_arrow","minecraft:spectral_arrow"] {
        for count in [1,2,3,4,5] {
            let stack=crossbow(vec![ammo(id);count]);let mut game=game::Game::new();
            assert_eq!(tconstruct_crossbow_use(&mut game,&stack),std::ops::ControlFlow::Break(true));
            let context=game.dispatched[0].1.as_ref().unwrap();
            assert_eq!(context.2.len(),count);
            for (index,(angle,power,primary)) in context.2.iter().enumerate() {
                assert_eq!(*angle,projectile::crossbow_angle(index,count));
                assert_eq!(*power,3.0);assert_eq!(*primary,index==count/2);
            }
            assert_eq!(context.2.iter().filter(|s|s.2).count(),1);
            assert!(game.world.chunks.contexts.is_empty());
        }
    }
}
#[test] fn mixed_rocket_unknown_and_different_component_payloads_keep_legacy_primary() {
    let mut changed=ammo("minecraft:tipped_arrow");changed.data_mut().set_other("test:variant",Some(Tag::Int(1)));
    for loaded in [
        vec![ammo("minecraft:arrow"),ammo("minecraft:firework_rocket"),ammo("minecraft:arrow")],
        vec![ammo("minecraft:firework_rocket"),ammo("minecraft:arrow"),ammo("minecraft:arrow")],
        vec![ammo("minecraft:firework_rocket");3],
        vec![ammo("minecraft:arrow"),ammo("minecraft:spectral_arrow"),ammo("minecraft:arrow")],
        vec![ammo("test:custom_arrow");3],
        vec![ammo("minecraft:tipped_arrow"),changed,ammo("minecraft:tipped_arrow")],
    ] {
        let expected:Vec<_>=loaded.iter().enumerate().filter(|(_,s)|&*s.item!="minecraft:firework_rocket").map(|(i,_)|(projectile::crossbow_angle(i,loaded.len()),3.0,i==0)).collect();
        let mut game=game::Game::new();let stack=crossbow(loaded);
        let _=tconstruct_crossbow_use(&mut game,&stack);
        assert_eq!(game.dispatched[0].1.as_ref().unwrap().2,expected);
        assert!(game.world.chunks.contexts.is_empty());
    }
}
#[test] fn broken_using_empty_and_missing_stats_gates_are_unchanged() {
    let mut stack=crossbow(vec![ammo("minecraft:arrow");3]);
    for broken in [false,true] {
        let mut game=game::Game::new();
        if broken {stack.damage=100;} else {game.using=Some(());}
        let _=tconstruct_crossbow_use(&mut game,&stack);assert!(game.dispatched.is_empty());
    }
    stack.damage=99;
    let mut game=game::Game::new();let _=tconstruct_crossbow_use(&mut game,&stack);
    // The use hook snapshots the entire volley before dispatch even at one durability.
    assert_eq!(game.dispatched[0].1.as_ref().unwrap().2.iter().map(|s|s.2).collect::<Vec<_>>(),vec![false,true,false]);
    let mut game=game::Game::new();let _=tconstruct_crossbow_use(&mut game,&crossbow(vec![]));
    assert!(game.started);assert!(game.dispatched.is_empty());
    stack.data_mut().set_other("minecraft:custom_data",None);
    let mut game=game::Game::new();let _=tconstruct_crossbow_use(&mut game,&stack);
    assert_eq!(game.dispatched.len(),1);assert!(game.dispatched[0].1.is_none());
}

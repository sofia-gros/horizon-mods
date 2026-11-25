//! Executes the actual pre-join adapter against structural game facades. This
//! checks dispatch/gating, not native Host/attachments/attribute observers/sites.
use std::{marker::PhantomData,sync::Arc};
struct Dim(&'static str);
impl Dim {fn id(&self)->&str {self.0}}
struct Motion {attributes:Arc<()>}
struct Game<'a> {remote:Option<()>,dim:Dim,motion:Motion,mobs:Vec<Mob>,callbacks:usize,switch_dimension:bool,replace_owner:bool,_lifetime:PhantomData<&'a ()>}
struct Kind {id:String}
struct Extra {size:i32}
#[derive(Default)]
struct Nav {state:crate::entity_spawn_state_tests::SpawnState}
impl Nav {fn __horizon_mixins_pluto_lib_pluto_entity_spawn_state(&self)->&crate::entity_spawn_state_tests::SpawnState {&self.state}}
#[derive(Default)]
struct Ai {nav:Nav}
struct Mob {id:u32,removed:bool,kind:Kind,extra:Extra,ai:Ai,initialized:usize,bases:Vec<(String,f64)>}
impl Mob {fn species(&self)->sunrise_client::mobs::Species {sunrise_client::mobs::Species::Slime}}
mod sunrise_client {
    pub mod game {pub mod pluto_living_attributes_join {
        use super::super::super::{Game,adapter};
        pub fn after_nbt_load(game:&mut Game<'_>,entity:u32)->Result<(),String> {adapter::after_nbt_load(game,entity)}
    }}
    pub mod mobs {
        #[allow(dead_code)]
        pub enum Species {Slime,MagmaCube,SulfurCube}
        pub mod pluto_living_entity_access {
            pub struct Sink {pub attributes:fn(&str)->Result<Vec<()>,String>}
            static SINK:Sink=Sink {attributes:|kind|Ok(if kind=="load_test:slime" {vec![()]} else {vec![]})};
            pub fn sink()->Option<&'static Sink> {Some(&SINK)}
        }
        pub mod finalize {pub mod pluto_living_attribute_ops {
            use super::super::super::super::Mob;
            use std::sync::Arc;
            pub fn initialize(mob:&mut Mob,_:&Arc<()>,_:Vec<()>)->Result<(),String> {mob.initialized+=1;Ok(())}
            pub fn apply_load_bases(mob:&mut Mob,_:&Arc<()>,values:Vec<(String,f64)>)->Result<(),String> {mob.bases=values;Ok(())}
        }}
    }
    pub mod entry {pub mod pluto_living_load_access {
        use super::super::super::{Game,Dim};
        pub struct LoadInfo {pub entity:u32,pub kind:String,pub dimension:String,pub slime_size:Option<i32>}
        pub struct AttributeBase {pub attribute:String,pub value:f64}
        pub fn has(kind:&str)->Result<bool,String> {Ok(kind=="load_test:slime")}
        pub fn bases(game:&mut Game<'_>,info:&LoadInfo)->Result<Vec<AttributeBase>,String> {
            assert_eq!((info.entity,info.kind.as_str(),info.dimension.as_str(),info.slime_size),(7,"load_test:slime","minecraft:overworld",Some(game.mobs[0].extra.size)));
            game.callbacks+=1;
            if game.switch_dimension {game.dim=Dim("minecraft:the_end");}
            if game.replace_owner {game.mobs[0].ai=Default::default();game.mobs[0].initialized=0;game.mobs[0].bases.clear();}
            Ok(vec![AttributeBase {attribute:"minecraft:armor".into(),value:f64::from(info.slime_size.unwrap())}])
        }
    }}
}
mod sunrise_client_lib {pub mod pluto_living_load_access {
    pub use super::super::sunrise_client::entry::pluto_living_load_access::{bases,has,AttributeBase,LoadInfo};
}}
macro_rules! load_error { ($($arg:tt)*) => { eprintln!($($arg)*) }; }
mod log {pub(crate) use load_error as error;}
mod adapter {
    use super::{sunrise_client,sunrise_client_lib,log};
    include!("../../../mod/src/pluto-living-attributes-join.rs");
}
mod return_adapter {
    use super::{Game,sunrise_client,log};
    type R<T>=Result<T,String>;
    pub(super) enum Target {Mob(u32),Item}
    fn err(key:&str,_:&[()])->String {key.into()}
    include!("../../../mod/src/pluto-living-attributes-reload.rs");
    pub(super) fn run(value:R<()>,game:&mut Game<'_>,target:Target)->R<()> {pluto_living_attributes_reload(value,game,target)}
}
#[test]
fn actual_join_adapter_preserves_event_dimension_and_skips_cancelled_remote_and_unregistered_paths() {
    let fresh=||Game {remote:None,dim:Dim("minecraft:overworld"),motion:Motion {attributes:Arc::new(())},
        mobs:vec![Mob {id:7,removed:false,kind:Kind {id:"load_test:slime".into()},extra:Extra {size:4},ai:Default::default(),initialized:0,bases:vec![]}],callbacks:0,switch_dimension:false,replace_owner:false,_lifetime:PhantomData};
    let mut game=fresh();let mut canceled=false;
    assert!(adapter::before_join(&mut game,"minecraft:overworld",7,"load_test:slime",&mut canceled));
    assert!(!canceled);assert_eq!(game.callbacks,1);assert_eq!(game.mobs[0].initialized,1);
    assert_eq!(game.mobs[0].bases,vec![("minecraft:armor".into(),4.0)]);
    game.mobs[0].extra.size=2; // native NBT reload has already changed size
    return_adapter::run(Ok(()),&mut game,return_adapter::Target::Mob(7)).unwrap();
    assert_eq!(game.mobs[0].bases,vec![("minecraft:armor".into(),2.0)]);
    assert_eq!(game.callbacks,2);assert_eq!(game.mobs[0].initialized,1,"NBT refresh must not reinstall removed transient modifiers");
    assert_eq!(return_adapter::run(Err("native reload refused".into()),&mut game,return_adapter::Target::Mob(7)),Err("native reload refused".into()));
    return_adapter::run(Ok(()),&mut game,return_adapter::Target::Item).unwrap();
    assert_eq!(game.callbacks,2,"failed reloads and non-mobs retain native behavior without class callbacks");
    let mut game=fresh();let mut canceled=false;
    assert!(!adapter::before_join(&mut game,"minecraft:the_end",7,"load_test:slime",&mut canceled));
    assert!(canceled);assert_eq!(game.callbacks,0);assert_eq!(game.mobs[0].initialized,0);assert!(game.mobs[0].bases.is_empty());
    let mut game=fresh();let mut canceled=true;
    assert!(adapter::before_join(&mut game,"minecraft:overworld",7,"load_test:slime",&mut canceled));
    assert!(canceled);assert_eq!(game.callbacks,0);assert_eq!(game.mobs[0].initialized,0);
    let mut game=fresh();game.remote=Some(());let mut canceled=false;
    assert!(adapter::before_join(&mut game,"minecraft:overworld",7,"load_test:slime",&mut canceled));
    assert!(!canceled);assert_eq!(game.callbacks,0);assert_eq!(game.mobs[0].initialized,0);
    let mut game=fresh();game.mobs.clear();let mut canceled=false;
    assert!(adapter::before_join(&mut game,"minecraft:the_end",99,"minecraft:pig",&mut canceled));
    assert!(!canceled);assert_eq!(game.callbacks,0);
    let mut game=fresh();game.switch_dimension=true;let mut canceled=false;
    assert!(!adapter::before_join(&mut game,"minecraft:overworld",7,"load_test:slime",&mut canceled));
    assert!(canceled);assert_eq!(game.callbacks,1);assert!(game.mobs[0].bases.is_empty(),"never apply owned writes after the callback changes the world");
    let mut game=fresh();game.switch_dimension=true;
    assert_eq!(return_adapter::run(Ok(()),&mut game,return_adapter::Target::Mob(7)),Err("commands.data.merge.failed".into()));
    assert_eq!(game.callbacks,1);assert_eq!(game.dim.id(),"minecraft:the_end");
    assert!(game.mobs[0].bases.is_empty(),"reload errors report failure without promising rollback of earlier world effects");
    let mut game=fresh();game.replace_owner=true;let mut canceled=false;
    assert!(!adapter::before_join(&mut game,"minecraft:overworld",7,"load_test:slime",&mut canceled),"same ID and kind must not accept another entity's base writes");
    assert!(canceled);assert!(game.mobs[0].bases.is_empty());
    let mut game=fresh();game.replace_owner=true;
    assert_eq!(return_adapter::run(Ok(()),&mut game,return_adapter::Target::Mob(7)),Err("commands.data.merge.failed".into()));
    assert!(game.mobs[0].bases.is_empty());
}


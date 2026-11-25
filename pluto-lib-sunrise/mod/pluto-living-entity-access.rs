// General per-type living query transport; no entity-id maps or world pointers.
use std::sync::OnceLock;
#[derive(Clone,Debug)]
pub struct AttributeModifier {pub attribute:String,pub id:String,pub amount:f64,pub operation:u8}
/// Native mirror of the pure causeFallDamage outcome. The sound is interned
/// by the API crate: MobEvent sounds are &'static str.
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum FallOutcome {
    Cancel,
    Bounce {velocity:[f64;3],sound:Option<&'static str>},
}
#[derive(Clone,Copy)]
pub struct Sink {
    pub attributes:fn(&str)->Result<Vec<AttributeModifier>,String>,
    pub has_fall:fn(&str)->bool,
    pub fall:fn(u32,&str,f64,f32,f64)->Option<i32>,
    pub has_jump:fn(&str)->bool,
    pub jump:fn(u32,&str,Option<i32>,f32)->Option<f32>,
    pub has_attack:fn(&str)->bool,
    pub attack:fn(u32,&str,f64)->Option<f32>,
    pub has_cause_fall:fn(&str)->bool,
    pub cause_fall:fn(u32,&str,f64,f32,[f64;3],bool)->Option<FallOutcome>,
    pub has_drop_chance:fn(&str)->bool,
    pub drop_chance:fn(&str,&str,Option<i32>,f32,i32)->Option<f32>,
}
static SINK:OnceLock<Sink>=OnceLock::new();
pub fn set_sink(sink:Sink)->Result<(),&'static str> {SINK.set(sink).map_err(|_|"living query sink already installed")}
pub fn sink()->Option<&'static Sink> {SINK.get()}

// What the game tells the mods when a mob hurts the player: the mob's state and the damage. The mods answer with the status
// effects to put on the mob (an armor's counter-attack).
use std::sync::OnceLock;
#[derive(Clone,Debug)]
pub struct PlayerHurt {
    pub attacker:i32,
    pub attacker_kind:String,
    pub position:[f64;3],
    pub on_ground:bool,
    pub in_water:bool,
    pub fire_ticks:i32,
    pub fire_immune:bool,
    pub effects:Vec<String>,
    pub damage_type:String,
    pub amount:f32,
}
#[derive(Clone,Copy)]
pub struct Sink {pub hurt:fn(&PlayerHurt)->Vec<(String,i32,i32)>}
static SINK:OnceLock<Sink>=OnceLock::new();
pub fn set_sink(sink:Sink)->Result<(),&'static str> {SINK.set(sink).map_err(|_|"player hurt sink already installed")}
pub fn sink()->Option<&'static Sink> {SINK.get()}

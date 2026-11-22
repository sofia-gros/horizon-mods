// What the game tells the mods about the parties of a hit as it is delivered: the entity behind a synchronous
// attack (a mob's melee swing) is named as the attack opens, with what it holds and wears, and a damaged mob
// or the ender dragon is named with the health the hit found. The mod answers nothing here; the facts are
// remembered for the damage events of the same hit. Drops a damage callback asks for at an entity are queued
// here too, for the game to spawn at that entity (see pluto-damage-drop-apply).
use std::sync::{Mutex, OnceLock};
#[derive(Clone, Debug)]
pub struct AttackerFacts {
    pub attacker: i32,
    pub kind: String,
    pub main_hand: Option<crate::items::ItemStack>,
    pub chest: Option<crate::items::ItemStack>,
    pub attack_damage: f32,
    pub on_ground: bool,
    pub in_water: bool,
    pub fire_ticks: i32,
    pub fire_immune: bool,
    pub effects: Vec<String>,
}
#[derive(Clone, Copy, Debug)]
pub struct VictimFacts {
    pub entity: i32,
    pub health: f32,
    pub attack_damage: Option<f32>,
}
#[derive(Clone, Copy)]
pub struct Sink {
    pub attacker: fn(&AttackerFacts),
    pub victim: fn(&VictimFacts),
}
static SINK: OnceLock<Sink> = OnceLock::new();
pub fn set_sink(sink: Sink) -> Result<(), &'static str> { SINK.set(sink).map_err(|_| "damage facts sink already installed") }
pub fn sink() -> Option<&'static Sink> { SINK.get() }
static DROPS: Mutex<Vec<(u32, crate::items::ItemStack)>> = Mutex::new(Vec::new());
pub fn push_drop(entity: u32, stack: crate::items::ItemStack) { if let Ok(mut queue) = DROPS.lock() { queue.push((entity, stack)); } }
pub fn take_drops() -> Vec<(u32, crate::items::ItemStack)> { DROPS.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default() }

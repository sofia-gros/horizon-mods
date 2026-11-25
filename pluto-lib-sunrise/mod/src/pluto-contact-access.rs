// Transport for what a mod's entity-inside callback does to the entity inside its block: the API side queues the actions while
// the movement seams run (where the entity may not be changed), and the game applies them where it applies the mob effects
// (see pluto-mob-effect-apply and pluto-mob-effect-apply-tick).
use std::sync::Mutex;
/// One thing to do to the entity, in plain data: set it on fire for so many seconds, give it a status
/// effect (id, ticks, amplifier), or hurt it (damage type, amount).
pub enum Contact { Fire(i32), Effect(String, i32, i32), Hurt(String, f32) }
static QUEUE: Mutex<Vec<(u32, Contact)>> = Mutex::new(Vec::new());
pub fn push(entity: u32, action: Contact) {
    if let Ok(mut queue) = QUEUE.lock() {
        if queue.len() < 4096 { queue.push((entity, action)); }
    }
}
pub fn take() -> Vec<(u32, Contact)> { QUEUE.lock().map(|mut queue| std::mem::take(&mut *queue)).unwrap_or_default() }
/// The effect or damage type id as the game keeps it: a name that lives for the whole run, one allocation per distinct id.
pub fn interned(name: &str) -> Option<&'static str> {
    static NAMES: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
    let mut names = NAMES.lock().ok()?;
    if let Some(known) = names.iter().find(|known| **known == name) { return Some(known); }
    let leaked: &'static str = Box::leak(name.to_string().into_boxed_str());
    names.push(leaked);
    Some(leaked)
}

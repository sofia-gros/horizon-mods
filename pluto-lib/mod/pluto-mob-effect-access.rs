// Transport for the status effects a mod's weapon-hit callback puts on a mob: the API side queues them while the callbacks run, and
// the game applies them to the mob right after the hit's own hooks (see pluto-mob-effect-apply).
use std::sync::Mutex;
static QUEUE:Mutex<Vec<(u32,String,i32,i32)>>=Mutex::new(Vec::new());
pub fn push(entity:u32,effect:&str,ticks:i32,amplifier:i32) {if let Ok(mut queue)=QUEUE.lock() {queue.push((entity,effect.to_string(),ticks,amplifier));}}
pub fn take()->Vec<(u32,String,i32,i32)> {QUEUE.lock().map(|mut queue|std::mem::take(&mut *queue)).unwrap_or_default()}
/// The effect id as the game keeps it: a name that lives for the whole run, one allocation per distinct effect.
pub fn interned(effect:&str)->Option<&'static str> {
    static NAMES:Mutex<Vec<&'static str>>=Mutex::new(Vec::new());
    let mut names=NAMES.lock().ok()?;
    if let Some(known)=names.iter().find(|known|**known==effect) {return Some(known);}
    let leaked:&'static str=Box::leak(effect.to_string().into_boxed_str());
    names.push(leaked);
    Some(leaked)
}

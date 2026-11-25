// What the items registered through PlutoLib's `ItemUseRegistry` need from the game side: food, an early-use rule and a remainder.
// The registry lives in the game's own crate so that PlutoLib's injections (which run inside the game) and its API crate (which the
// mods call) see the same entries.
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// Nutrition and saturation eaten when the use completes, if the item feeds.
    pub food: Option<(i32, f32)>,
    /// A feeding item can be started when the player is not hungry.
    pub can_always_eat: bool,
    /// Item and count handed back when one of the item is used up.
    pub remainder: Option<(String, u32)>,
}

static ALL: Mutex<Option<HashMap<String, Spec>>> = Mutex::new(None);

pub fn set(item: &str, spec: Spec) {
    if let Ok(mut all) = ALL.lock() {
        all.get_or_insert_with(HashMap::new).insert(item.to_owned(), spec);
    }
}

pub fn get(item: &str) -> Option<Spec> {
    ALL.lock().ok()?.as_ref()?.get(item).cloned()
}

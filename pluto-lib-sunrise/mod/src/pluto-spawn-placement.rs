// Spawn placement transport: the game reads the candidate's facts once and
// asks the API crate for the registered predicate's verdict.
use std::sync::OnceLock;
#[derive(Clone)]
pub struct Facts {
    pub entity_type:String,
    pub dimension:String,
    pub position:[i32;3],
    pub reason:String,
    pub peaceful:bool,
    pub block_below:String,
    pub fluid_at:String,
    pub fluid_below:String,
    pub block_below_valid_spawn:bool,
    pub dark_enough:bool,
}
pub struct Provider {
    pub active:fn()->bool,
    pub evaluate:fn(&Facts)->Option<bool>,
    pub active_or:fn()->bool,
    pub evaluate_or:fn(&Facts)->Option<bool>,
    pub or_types:fn()->Vec<String>,
}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),String> {
    PROVIDER.set(provider).map_err(|_|"spawn placement provider already installed".into())
}
pub fn active()->bool { PROVIDER.get().map_or(false,|provider|(provider.active)()) }
pub fn evaluate(facts:&Facts)->Option<bool> {
    PROVIDER.get().and_then(|provider|(provider.evaluate)(facts))
}
/// Whether any OR (permit-add) placement predicate is registered.
pub fn active_or()->bool { PROVIDER.get().map_or(false,|provider|(provider.active_or)()) }
/// The OR predicate's verdict for a permit-add candidate, or None when the type has none.
pub fn evaluate_or(facts:&Facts)->Option<bool> {
    PROVIDER.get().and_then(|provider|(provider.evaluate_or)(facts))
}
/// The types carrying an OR predicate: the permit-add pass proposes candidates for these.
pub fn or_types()->Vec<String> {
    PROVIDER.get().map_or(Vec::new(),|provider|(provider.or_types)())
}

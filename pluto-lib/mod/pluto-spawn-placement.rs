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
}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),String> {
    PROVIDER.set(provider).map_err(|_|"spawn placement provider already installed".into())
}
pub fn active()->bool { PROVIDER.get().map_or(false,|provider|(provider.active)()) }
pub fn evaluate(facts:&Facts)->Option<bool> {
    PROVIDER.get().and_then(|provider|(provider.evaluate)(facts))
}

// The block experience reach's boundary: the game's own break path asks the
// installed provider what a break drops, while the sampled amount can still
// change (pluto-block-exp.rs). With no provider the amount stands.
use sunrise_client::mod_api::Host;
use std::sync::OnceLock;
pub struct Provider {pub adjust:fn(&mut dyn Host,[i32;3],u32)->u32}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),&'static str> {PROVIDER.set(provider).map_err(|_|"block experience provider already installed")}
pub fn adjust(host:&mut dyn Host,pos:[i32;3],value:u32)->u32 {PROVIDER.get().map_or(value,|provider|(provider.adjust)(host,pos,value))}

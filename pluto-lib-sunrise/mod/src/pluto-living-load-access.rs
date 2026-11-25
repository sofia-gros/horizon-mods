// The Host-capable boundary lives in entry, not the lower mobs library. A
// callback borrows the game only while producing owned writes; it never holds
// a live Mob borrow at the same time, or retains a game pointer/entity-id map.
use sunrise_client::mod_api::Host;
use std::sync::OnceLock;
#[derive(Clone,Debug)]
pub struct AttributeBase {pub attribute:String,pub value:f64}
pub struct LoadInfo {pub entity:u32,pub kind:String,pub dimension:String,pub slime_size:Option<i32>}
pub struct Provider {pub has:fn(&str)->Result<bool,String>,pub bases:fn(&mut dyn Host,&LoadInfo)->Result<Vec<AttributeBase>,String>}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),&'static str> {PROVIDER.set(provider).map_err(|_|"living load provider already installed")}
pub fn has(kind:&str)->Result<bool,String> {PROVIDER.get().map_or(Ok(false),|provider|(provider.has)(kind))}
pub fn bases(host:&mut dyn Host,info:&LoadInfo)->Result<Vec<AttributeBase>,String> {
    PROVIDER.get().map_or_else(||Ok(Vec::new()),|provider|(provider.bases)(host,info))
}

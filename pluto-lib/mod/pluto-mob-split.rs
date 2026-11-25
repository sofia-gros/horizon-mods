// Mob split transport: the native split hook tunes count/cancel before the
// game spawns; after the split, the inheritance pass dresses the children.
// No game pointers escape: child handles live only for the return call.
use horizon_client::items::ItemStack;
use horizon_client::mod_api::Host;
use std::sync::OnceLock;
#[derive(Clone)]
pub struct Info {
    pub entity:u32,
    pub entity_type:String,
    pub dimension:String,
    pub position:[f64;3],
    pub size:i32,
    pub child_size:i32,
    pub head:Option<ItemStack>,
    pub head_drop_chance:f32,
    pub custom_name:Option<String>,
    pub no_ai:bool,
    pub invulnerable:bool,
}
pub trait Random {
    fn next_float(&mut self)->f32;
    fn next_int(&mut self,bound:i32)->Result<i32,String>;
}
pub trait Child {
    fn id(&self)->u32;
    fn set_equipment(&mut self,slot:&str,stack:ItemStack)->Result<(),String>;
    fn set_custom_name(&mut self,name:Option<String>)->Result<(),String>;
    fn set_no_ai(&mut self,no_ai:bool)->Result<(),String>;
    fn set_invulnerable(&mut self,invulnerable:bool)->Result<(),String>;
}
pub struct Provider {
    pub active:fn()->bool,
    pub prepare:fn(&Info,&mut u32,&mut bool)->Result<(),String>,
    pub inherit:fn(&mut dyn Host,&Info,&mut dyn Random,&mut [Box<dyn Child>])->Result<(),String>,
}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),String> {
    PROVIDER.set(provider).map_err(|_|"mob split provider already installed".into())
}
pub fn active()->bool { PROVIDER.get().map_or(false,|provider|(provider.active)()) }
pub fn prepare(info:&Info,count:&mut u32,cancel:&mut bool)->Result<(),String> {
    let Some(provider)=PROVIDER.get() else { return Ok(()) };
    (provider.prepare)(info,count,cancel)
}
pub fn inherit(host:&mut dyn Host,info:&Info,random:&mut dyn Random,children:&mut [Box<dyn Child>])->Result<(),String> {
    let Some(provider)=PROVIDER.get() else { return Ok(()) };
    (provider.inherit)(host,info,random,children)
}

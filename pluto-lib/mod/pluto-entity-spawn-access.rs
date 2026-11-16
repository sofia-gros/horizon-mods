// Generic borrowed native spawn boundary. No game pointers or entity-ID maps
// escape this call; the unpublished Mob owns Pending through SpawnState.
use horizon_client::items::ItemStack;
use std::sync::OnceLock;
#[derive(Clone)]
pub struct Info { pub entity_type:String,pub dimension:String,pub position:[f64;3],pub reason:String }
pub trait Random {
    fn next_float(&mut self)->f32;
    fn next_double(&mut self)->f64;
    fn next_int(&mut self,bound:i32)->Result<i32,String>;
}
pub trait Equipment {
    fn get(&self,slot:&str)->Result<Option<ItemStack>,String>;
    fn set(&mut self,slot:&str,stack:ItemStack)->Result<(),String>;
}
pub trait Completion:Send+Sync {
    fn complete(self:Box<Self>,info:&Info,random:&mut dyn Random,equipment:&mut dyn Equipment)->Result<(),String>;
}
pub struct Provider {
    pub enabled:fn()->Result<bool,String>,
    pub prepare:fn(&Info,&mut dyn Random,&mut dyn Equipment)->Result<Option<Box<dyn Completion>>,String>,
}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(provider:Provider)->Result<(),String> {
    PROVIDER.set(provider).map_err(|_|"spawn provider already installed".into())
}
pub fn enabled()->Result<bool,String> { PROVIDER.get().map_or(Ok(false),|provider|(provider.enabled)()) }
pub struct Pending { info:Info,completion:Box<dyn Completion> }
pub fn prepare(info:Info,random:&mut dyn Random,equipment:&mut dyn Equipment)->Result<Option<Pending>,String> {
    let Some(provider)=PROVIDER.get() else { return Ok(None) };
    Ok((provider.prepare)(&info,random,equipment)?.map(|completion|Pending { info,completion }))
}
impl Pending {
    pub fn complete(self,random:&mut dyn Random,equipment:&mut dyn Equipment)->Result<(),String> {
        self.completion.complete(&self.info,random,equipment)
    }
}

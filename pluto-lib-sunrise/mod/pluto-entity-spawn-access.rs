// Generic borrowed native spawn boundary. No game pointers or entity-ID maps
// escape this call; the unpublished Mob owns Pending through SpawnState.
use sunrise_client::items::ItemStack;
use sunrise_client::mod_api::Host;
use std::sync::OnceLock;
#[derive(Clone)]
pub struct Info { pub entity_type:String,pub dimension:String,pub position:[f64;3],pub reason:String,pub special_multiplier:f32 }
pub trait Random {
    fn next_float(&mut self)->f32;
    fn next_double(&mut self)->f64;
    fn next_int(&mut self,bound:i32)->Result<i32,String>;
}
pub trait Equipment {
    fn get(&self,slot:&str)->Result<Option<ItemStack>,String>;
    fn set(&mut self,slot:&str,stack:ItemStack)->Result<(),String>;
    /// `Mob.setCanPickUpLoot` on the unpublished Mob.
    fn set_pick_up_loot(&mut self,can:bool)->Result<(),String>;
    /// Record one slot's death-drop chance (`Mob.setDropChance`).
    fn set_drop_chance(&mut self,slot:&str,chance:f32)->Result<(),String>;
    /// `EnchantmentHelper.enchantItem` at `level` on the slot's stack, drawn
    /// from a clone of the Mob's own stream (the completion's stream adapter
    /// keeps the canonical position; the selection's draws are not replayed
    /// into it). An empty slot is left alone.
    fn enchant(&mut self,slot:&str,level:i32)->Result<(),String>;
    /// Queue an attachment write for the unpublished Mob. The queue is applied
    /// by the completion owner once the Mob is back in the Game (before it
    /// joins), through the installed attachment writer; it is not applied if
    /// the completion fails and the Mob is dropped.
    fn attach(&mut self,attachment:&str,json:&str,synced:bool)->Result<(),String>;
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
/// Writes one attachment on a published-or-joining entity through the game's
/// own attachment system (the API crate's JSON codec kinds). Installed once by
/// the API crate, which owns those kinds; the game side only forwards.
pub type AttachmentWriter=fn(&mut dyn Host,u32,&str,&str,&str,bool)->bool;
static ATTACHMENT_WRITER:OnceLock<AttachmentWriter>=OnceLock::new();
pub fn set_attachment_writer(writer:AttachmentWriter)->Result<(),&'static str> {
    ATTACHMENT_WRITER.set(writer).map_err(|_|"spawn attachment writer already installed")
}
pub fn write_attachment(host:&mut dyn Host,entity:u32,dimension:&str,attachment:&str,json:&str,synced:bool)->bool {
    ATTACHMENT_WRITER.get().map_or(false,|writer|writer(host,entity,dimension,attachment,json,synced))
}

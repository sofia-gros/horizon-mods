// Generic callback-local native death-drop boundary; no Game/Mob pointers are
// retained by a provider. Concrete collection/counter ownership stays native.
use std::sync::OnceLock;
use horizon_client::items::ItemStack;
// Native built-in MobType IDs may be bare; declared types already carry a
// namespace. LootEntity::new does not normalize either representation.
pub fn entity_kind(kind:&str)->std::borrow::Cow<'_,str> {
    if kind.contains(':') {kind.into()} else {format!("minecraft:{kind}").into()}
}
#[derive(Clone,Debug)]
pub struct Info {
    pub entity_id:u32,pub entity_type:String,pub dimension:String,pub position:[f64;3],
    pub damage_type:Option<String>,pub killer_type:Option<String>,
}
pub trait Context {
    fn info(&self)->&Info;
    fn can_drop_head(&self)->Result<bool,String>;
    fn claim_head_drop(&mut self)->Result<bool,String>;
    fn append(&mut self,stack:ItemStack)->Result<(),String>;
}
struct Provider {active:fn()->Result<bool,String>,dispatch:fn(&mut dyn Context)->Result<(),String>}
static PROVIDER:OnceLock<Provider>=OnceLock::new();
pub fn set_provider(active:fn()->Result<bool,String>,dispatch:fn(&mut dyn Context)->Result<(),String>)->Result<(), &'static str> {
    PROVIDER.set(Provider {active,dispatch}).map_err(|_|"living drop provider already installed")
}
pub fn active()->Result<bool,String> {PROVIDER.get().map_or(Ok(false),|provider|(provider.active)())}
pub fn dispatch(context:&mut dyn Context)->Result<(),String> {
    PROVIDER.get().map_or(Ok(()),|provider|(provider.dispatch)(context))
}

// Game-side mob split adapters: the native split hook's tuning call and the
// post-split inheritance pass over the children split_dead_cube just spawned.
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use super::Game;
use sunrise_client::{entry::pluto_mob_split as access,items::ItemStack,mobs::{Mob,Species,gear}};
use sunrise_core::random::{LegacyRandom,RandomSource};
use sunrise_world::player_inventory::EquipmentSlot;

struct Random<'a>(&'a mut LegacyRandom);
impl access::Random for Random<'_> {
    fn next_float(&mut self)->f32 { self.0.next_float() }
    fn next_int(&mut self,bound:i32)->Result<i32,String> { self.0.next_int_bound(bound).map_err(|_| "split RNG bound must be positive".into()) }
}
fn head_stack(m:&Mob)->Option<ItemStack> {
    let stack=m.item_by_slot(EquipmentSlot::Head)?;
    if let Some(saved)=gear::gear(m).and_then(|g|g.stacks[gear::HEAD].as_ref()).filter(|s|s.item==stack.item) {
        return Some(saved.clone());
    }
    Some(stack)
}
fn split_info(game:&Game<'_>,m:&Mob,kind:&str,size:i32)->access::Info {
    access::Info { entity:m.id,entity_type:kind.to_string(),dimension:game.dim.id().into(),
        position:[m.position.x,m.position.y,m.position.z],size,child_size:size/2,
        head:head_stack(m),head_drop_chance:sunrise_client::mobs::drops::drop_chance(m,"head"),
        custom_name:m.custom_name.clone(),no_ai:m.no_ai,invulnerable:m.invulnerable_flag }
}
pub(crate) fn prepare(game:&mut Game<'_>,parent:u32,kind:&str,size:i32,_child_size:i32,count:&mut u32,cancel:&mut bool) {
    if !access::active() { return; }
    let Some(m)=game.mobs.iter().find(|m|m.id==parent) else { return };
    let info=split_info(game,m,kind,size);
    if let Err(error)=access::prepare(&info,count,cancel) {
        eprintln!("PlutoLib: mob split preparation failed: {error}");
    }
}
/// One dressing write a split child was asked for during the inheritance
/// pass, in the order the provider asked for them. A child handle holds no
/// game: it records the writes, checked against the item registry as it
/// stands when the pass begins, and the writes land on the children once
/// the pass returns (see `inherit`), which still holds the game. The
/// provider reads nothing back through a child, so the children end the
/// pass dressed exactly as if each write had landed when asked for.
enum Dress {
    Equipment(usize,ItemStack),
    CustomName(Option<String>),
    NoAi(bool),
    Invulnerable(bool),
}
struct SplitChild {
    id:u32,
    registered:Rc<HashSet<Arc<str>>>,
    writes:Rc<RefCell<Vec<(u32,Dress)>>>,
}
impl SplitChild {
    fn record(&self,dress:Dress) {
        self.writes.borrow_mut().push((self.id,dress));
    }
}
impl access::Child for SplitChild {
    fn id(&self)->u32 { self.id }
    fn set_equipment(&mut self,slot:&str,stack:ItemStack)->Result<(),String> {
        let index=gear::KEYS.iter().position(|key|*key==slot).ok_or_else(||format!("unsupported mob equipment slot {slot}"))?;
        if !self.registered.contains(stack.item.as_ref()) { return Err(format!("unregistered split equipment {}",stack.item)); }
        self.record(Dress::Equipment(index,stack));
        Ok(())
    }
    fn set_custom_name(&mut self,name:Option<String>)->Result<(),String> { self.record(Dress::CustomName(name)); Ok(()) }
    fn set_no_ai(&mut self,no_ai:bool)->Result<(),String> { self.record(Dress::NoAi(no_ai)); Ok(()) }
    fn set_invulnerable(&mut self,invulnerable:bool)->Result<(),String> { self.record(Dress::Invulnerable(invulnerable)); Ok(()) }
}
/// Lands one recorded dressing write on its child: the same gear write the
/// spawn adapter performs, and the same name and flag writes, in the order
/// the provider asked for them.
fn apply_dress(game:&mut Game<'_>,id:u32,dress:Dress)->Result<(),String> {
    let mob=game.mobs.iter_mut().find(|m|m.id==id).ok_or_else(||"split child was removed".to_string())?;
    match dress {
        Dress::Equipment(index,stack)=>{
            // The same gear write the spawn adapter performs.
            let present=stack.count>0 && &*stack.item!="minecraft:air";
            gear::set(mob,index,Vec::new());
            let item=present.then(||stack.item.clone());
            match index {
                gear::MAIN=>mob.extra.set_main_hand_stack(present.then(||stack.clone())),
                gear::OFF=>mob.extra.off_hand=item,
                _=>{
                    let armor=index-gear::HEAD;
                    mob.extra.armor[armor]=present.then(||stack.clone());
                    match index {
                        gear::HEAD=>mob.extra.head=item,
                        gear::CHEST=>mob.extra.chest=item,
                        gear::LEGS=>mob.extra.legs=item,
                        gear::FEET=>mob.extra.feet=item,
                        _=>unreachable!(),
                    }
                }
            }
            if present { gear::set_stack(mob,index,stack); }
            mob.gear_knockback_resistance=None;
            mob.needs_sync=true;
            Ok(())
        }
        Dress::CustomName(name)=>{ mob.custom_name=name; Ok(()) }
        Dress::NoAi(no_ai)=>{ mob.no_ai=no_ai; Ok(()) }
        Dress::Invulnerable(invulnerable)=>{ mob.invulnerable_flag=invulnerable; Ok(()) }
    }
}
pub(crate) fn inherit(game:&mut Game<'_>,m:&Mob) {
    if !access::active() { return; }
    if m.species()==Species::SulfurCube { return; }
    let size=m.extra.size;
    if size<=1 { return; }
    // The children split_dead_cube just spawned sit at its own offset
    // formula, at half size, in spawn order.
    let off=|kk:i32|(f64::from(kk%2)-0.5)*f64::from(size)/4.0;
    // The pass's child handles hold no game: they record the dressing they
    // are asked for, checked against the item registry as it stands, and
    // the writes land below, once the pass returns, in the order they were
    // asked for.
    let registered=Rc::new(game.items().items.keys().cloned().collect::<HashSet<Arc<str>>>());
    let writes=Rc::new(RefCell::new(Vec::new()));
    let mut children:Vec<Box<dyn access::Child>>=Vec::new();
    for k in 0..64i32 {
        let at=[m.position.x+off(k),m.position.y+0.5,m.position.z+off(k/2)];
        if let Some(child)=game.mobs.iter().find(|c|c.id!=m.id && c.kind.id==m.kind.id && c.extra.size==size/2
            && (c.position.x-at[0]).abs()<1e-9 && (c.position.y-at[1]).abs()<1e-9 && (c.position.z-at[2]).abs()<1e-9).map(|c|c.id) {
            children.push(Box::new(SplitChild { id:child,registered:Rc::clone(&registered),writes:Rc::clone(&writes) }));
        }
    }
    if children.is_empty() { return; }
    let kind=super::native_hooks::mob_kind(m);
    let info=split_info(game,m,&kind,size);
    let mut random=m.random.clone();
    let result=access::inherit(game,&info,&mut Random(&mut random),&mut children);
    // The writes the pass asked for land even when it ends in an error:
    // each one was accepted as it was asked for, as it would have landed
    // then. A write that cannot land reports the same failure.
    let recorded=std::mem::take(&mut *writes.borrow_mut());
    let mut apply_error=None;
    for (id,dress) in recorded {
        if let Err(error)=apply_dress(game,id,dress) { apply_error=Some(error); break; }
    }
    if let Err(error)=result {
        eprintln!("PlutoLib: mob split inheritance failed: {error}");
    } else if let Some(error)=apply_error {
        eprintln!("PlutoLib: mob split inheritance failed: {error}");
    }
}

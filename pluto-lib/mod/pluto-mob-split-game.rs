// Game-side mob split adapters: the native split hook's tuning call and the
// post-split inheritance pass over the children split_dead_cube just spawned.
use super::Game;
use horizon_client::{entry::pluto_mob_split as access,items::ItemStack,mobs::{Mob,Species,gear}};
use horizon_core::random::{LegacyRandom,RandomSource};
use horizon_world::player_inventory::EquipmentSlot;

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
        head:head_stack(m),head_drop_chance:horizon_client::mobs::drops::drop_chance(m,"head"),
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
struct SplitChild<'g> { game:*mut Game<'g>,id:u32 }
impl SplitChild<'_> {
    fn mob(&mut self)->Result<&mut Mob,String> {
        // The Game outlives this synchronous pass; each handle dresses one
        // child and handles are used one call at a time.
        let game=unsafe { &mut *self.game };
        game.mobs.iter_mut().find(|m|m.id==self.id).ok_or_else(||"split child was removed".to_string())
    }
}
impl access::Child for SplitChild<'_> {
    fn id(&self)->u32 { self.id }
    fn set_equipment(&mut self,slot:&str,stack:ItemStack)->Result<(),String> {
        let index=gear::KEYS.iter().position(|key|*key==slot).ok_or_else(||format!("unsupported mob equipment slot {slot}"))?;
        let game=unsafe { &mut *self.game };
        if game.items().get(&stack.item).is_none() { return Err(format!("unregistered split equipment {}",stack.item)); }
        let mob=game.mobs.iter_mut().find(|m|m.id==self.id).ok_or("split child was removed")?;
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
    fn set_custom_name(&mut self,name:Option<String>)->Result<(),String> { self.mob()?.custom_name=name; Ok(()) }
    fn set_no_ai(&mut self,no_ai:bool)->Result<(),String> { self.mob()?.no_ai=no_ai; Ok(()) }
    fn set_invulnerable(&mut self,invulnerable:bool)->Result<(),String> { self.mob()?.invulnerable_flag=invulnerable; Ok(()) }
}
pub(crate) fn inherit(game:&mut Game<'_>,m:&Mob) {
    if !access::active() { return; }
    if m.species()==Species::SulfurCube { return; }
    let size=m.extra.size;
    if size<=1 { return; }
    // The children split_dead_cube just spawned sit at its own offset
    // formula, at half size, in spawn order.
    let off=|kk:i32|(f64::from(kk%2)-0.5)*f64::from(size)/4.0;
    let mut children:Vec<Box<dyn access::Child>>=Vec::new();
    for k in 0..64i32 {
        let at=[m.position.x+off(k),m.position.y+0.5,m.position.z+off(k/2)];
        if let Some(child)=game.mobs.iter().find(|c|c.id!=m.id && c.kind.id==m.kind.id && c.extra.size==size/2
            && (c.position.x-at[0]).abs()<1e-9 && (c.position.y-at[1]).abs()<1e-9 && (c.position.z-at[2]).abs()<1e-9) {
            children.push(Box::new(SplitChild { game:game as *mut Game<'_>,id:child.id }));
        }
    }
    if children.is_empty() { return; }
    let kind=super::native_hooks::mob_kind(m);
    let info=split_info(game,m,&kind,size);
    let mut random=m.random.clone();
    if let Err(error)=access::inherit(game,&info,&mut Random(&mut random),&mut children) {
        eprintln!("PlutoLib: mob split inheritance failed: {error}");
    }
}

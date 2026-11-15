// Owned Mob/RNG/gear adapter for the complete spawn callers, not the narrow
// spawning::finalize callee. Source attachment remains gated pending all D.
use super::Game;
use horizon_client::{entry::pluto_entity_spawn_access as access,items::{Items,ItemStack},mobs::{Mob,gear}};
use horizon_core::random::{LegacyRandom,RandomSource};
use horizon_world::player_inventory::EquipmentSlot;

struct Random<'a>(&'a mut LegacyRandom);
impl access::Random for Random<'_> {
    fn next_float(&mut self)->f32 { self.0.next_float() }
    fn next_double(&mut self)->f64 { self.0.next_double() }
    fn next_int(&mut self,bound:i32)->Result<i32,String> { self.0.next_int_bound(bound).map_err(|_| "mob RNG bound must be positive".into()) }
}
struct Equipment<'a> { mob:&'a mut Mob,items:&'a Items }
fn slot(name:&str)->Result<usize,String> { gear::KEYS.iter().position(|slot|*slot==name).ok_or_else(||format!("unsupported mob equipment slot {name}")) }
impl access::Equipment for Equipment<'_> {
    fn get(&self,name:&str)->Result<Option<ItemStack>,String> {
        let index=slot(name)?;
        let native=[EquipmentSlot::MainHand,EquipmentSlot::OffHand,EquipmentSlot::Head,EquipmentSlot::Chest,EquipmentSlot::Legs,EquipmentSlot::Feet][index];
        let stack=self.mob.item_by_slot(native);
        if let Some(stack)=&stack {
            if let Some(saved)=gear::gear(self.mob).and_then(|g|g.stacks[index].as_ref()).filter(|s|s.item==stack.item) {
                return Ok(Some(saved.clone()));
            }
        }
        Ok(stack)
    }
    fn set(&mut self,name:&str,stack:ItemStack)->Result<(),String> {
        let index=slot(name)?;
        if self.items.get(&stack.item).is_none() { return Err(format!("unregistered spawn equipment {}",stack.item)); }
        // Extra.off_hand keeps the item identity; gear::set_stack below keeps
        // the full offhand stack, as for native finalized equipment. Native
        // finalize::stored_stack and entity_nbt save it without losing tool NBT.
        let present=stack.count>0 && &*stack.item!="minecraft:air";
        gear::set(self.mob,index,Vec::new());
        let item=present.then(||stack.item.clone());
        match index {
            gear::MAIN=>self.mob.extra.set_main_hand_stack(present.then(||stack.clone())),
            gear::OFF=>self.mob.extra.off_hand=item,
            _=>{
                let armor=index-gear::HEAD;
                self.mob.extra.armor[armor]=present.then(||stack.clone());
                match index {
                    gear::HEAD=>self.mob.extra.head=item,
                    gear::CHEST=>self.mob.extra.chest=item,
                    gear::LEGS=>self.mob.extra.legs=item,
                    gear::FEET=>self.mob.extra.feet=item,
                    _=>unreachable!(),
                }
            }
        }
        if present { gear::set_stack(self.mob,index,stack); }
        self.mob.gear_knockback_resistance=None;
        self.mob.needs_sync=true;
        Ok(())
    }
}
pub(super) fn prepare(game:&Game<'_>,mob:&mut Mob,reason:&str)->Result<bool,String> {
    let info=access::Info { entity_type:horizon_client::command::args::normalize(&mob.kind.id),
        dimension:game.dim.id().into(),position:[mob.position.x,mob.position.y,mob.position.z],reason:reason.into() };
    // The borrowed equipment view and the RNG must not alias a mutable Mob.
    // Native halloween handling uses the same clone/run/restore RNG pattern.
    let mut random=mob.random.clone();
    let pending=access::prepare(info,&mut Random(&mut random),&mut Equipment { mob,items:game.items() });
    mob.random=random;
    let Some(pending)=pending? else { return Ok(false) };
    mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().prepare(pending).map_err(|(error,_)|error)?;
    Ok(true)
}
pub(super) struct Prepared {
    pending:access::Pending,
    token:horizon_client::mobs::nav::pluto_entity_spawn_state::SpawnToken,
}
pub(super) fn take(mob:&Mob)->Result<Option<Prepared>,String> {
    mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().take_with_token::<access::Pending>()
        .map(|value|value.map(|(pending,token)|Prepared { pending,token }))
}
pub(super) fn complete(game:&mut Game<'_>,id:u32,prepared:Prepared)->Result<(),String> {
    let Some(index)=game.mobs.iter().position(|m|m.id==id) else {
        return Err("prepared spawn was removed during inherited initialization".into());
    };
    if !game.mobs[index].ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().owns(&prepared.token)? {
        return Err("prepared spawn owner changed during inherited initialization".into());
    }
    let pending=prepared.pending;
    let mut mob=game.mobs.remove(index);
    let mut random=mob.random.clone();
    let result=pending.complete(&mut Random(&mut random),&mut Equipment { mob:&mut mob,items:game.items() });
    mob.random=random;
    // On failure the still-unpublished Mob is dropped; no partially replaced
    // parent is joined. Inherited child/world effects are not rolled back.
    if result.is_ok() { game.mobs.insert(index,mob); }
    result
}

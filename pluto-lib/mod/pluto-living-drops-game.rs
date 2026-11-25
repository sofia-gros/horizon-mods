// Adapted from pinned MIT Horizon game/creatures.rs::drop_death_loot.
// License: pluto-lib/LICENSE.horizon. Keep native table/equipment/RNG collection;
// capture carried-item drops and XP before the living-drop callback, and keep
// Creeper head claims independent of its powered state.
use super::*;
use horizon_client::mobs::drops;
use crate::game::pluto_living_drops as access;

fn killer_id(m:&Mob)->Option<u32> {
    match m.last_damage.and_then(|(damage,_)|damage.attacker) {Some(Who::Mob(id))=>Some(id),_=>None}
}
/// The killing blow's Looting level (`EnchantmentHelper.getMobLooting`):
/// the attacker's main-hand Looting. A mob attacker's hand is read from its
/// equipment; a player attacker's is the main hand the host already resolved
/// into the killer `LootEntity` (`Game::killer_loot_entity` in the pinned
/// host: the local player's hand for a local kill, the guest's own hand —
/// read through `as_guest` — for a LAN guest's kill), so the drop-chance
/// context sees the guest's Looting instead of the host's.
fn killer_looting(game:&Game<'_>,killer:&horizon_world::loot::LootEntity,m:&Mob)->i32 {
    match m.last_damage.and_then(|(damage,_)|damage.attacker) {
        Some(Who::Mob(id))=>game.mobs.iter().find(|other|other.id==id).and_then(|mob|mob.extra.main_hand_stack())
            .map_or(0,|stack|stack.enchantment_level("minecraft:looting") as i32),
        Some(Who::Player)=>access::killer_mainhand_looting(killer),
        _=>0,
    }
}
/// The class-adjusted equipment drop chances for this death, when a living
/// behaviour overrides any slot's chance for this kind. Built from the
/// effective chances the death roll itself reads.
fn pluto_drop_chance_override(m:&Mob,looting:i32)->Option<Box<[f32;8]>> {
    let sink=horizon_client::mobs::pluto_living_entity_access::sink()?;
    if !(sink.has_drop_chance)(&m.kind.id) {return None;}
    let slots=horizon_world::loot::EQUIPMENT_SLOTS;
    let size=match m.species() {
        Species::Slime|Species::MagmaCube|Species::SulfurCube=>Some(m.extra.size),
        _=>None,
    };
    let mut adjusted=Box::new(std::array::from_fn(|i|drops::drop_chance(m,slots[i])));
    let mut changed=false;
    for (index,slot) in slots.iter().enumerate() {
        if let Some(chance)=(sink.drop_chance)(&m.kind.id,slot,size,adjusted[index],looting) {
            // Store the class chance unadjusted. The class's own roll is
            // (nextFloat - looting*0.01) < chance, but the native roll
            // applies the attacker's equipment_drops effect (Looting,
            // +0.01 per level) to the swapped chance itself before testing
            // nextFloat >= chance (pinned host drops.rs:518-529 ->
            // horizon-world loot.rs:965 equipment_drop_chance), so the
            // stored chance must not carry the looting term: folding it in
            // here as well counted Looting twice (issue #1163: the capped
            // 0.25 slime head chance rolled at 0.31 under Looting III
            // instead of canon's 0.28). A class chance of exactly 0 makes
            // the native roll skip the slot outright, where the class roll
            // could still succeed on the looting term alone; no registered
            // behaviour returns 0 (the sole sink caps the slime head slot
            // at 0.25), so that corner is not exercised.
            adjusted[index]=chance;changed=true;
        }
    }
    changed.then_some(adjusted)
}
fn can_claim(game:&Game<'_>,killer:Option<u32>)->std::result::Result<bool,String> {
    let Some(id)=killer else {return Ok(false)};
    if let Some(mob)=game.mobs.iter().find(|mob|mob.id==id) {
        return Ok(mob.species()==Species::Creeper && mob.extra.charged
            && mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().head_drop_available()
            && game.combat.__horizon_mixins_pluto_lib_pluto_living_drop_state().available(id)?);
    }
    if !game.combat.charged_exploders.contains(&id) {return Ok(false);}
    game.combat.__horizon_mixins_pluto_lib_pluto_living_drop_state().available(id)
}
fn claim(game:&mut Game<'_>,killer:Option<u32>)->std::result::Result<bool,String> {
    let Some(id)=killer else {return Ok(false)};
    if let Some(mob)=game.mobs.iter().find(|mob|mob.id==id) {
        if mob.species()!=Species::Creeper || !mob.extra.charged
            || !game.combat.__horizon_mixins_pluto_lib_pluto_living_drop_state().available(id)?
            || !mob.ai.nav.__horizon_mixins_pluto_lib_pluto_entity_spawn_state().claim_head_drop() {return Ok(false);}
        // Carry this spent flag if the still-live creeper explodes later in the
        // current wave; begin_wave seeds future waves from the live Nav flags.
        return game.combat.__horizon_mixins_pluto_lib_pluto_living_drop_state().claim(id);
    }
    if !game.combat.charged_exploders.contains(&id) {return Ok(false);}
    // Keep powered source identity available for ordinary loot of every later
    // victim; head availability is separate, not removal from the native list.
    game.combat.__horizon_mixins_pluto_lib_pluto_living_drop_state().claim(id)
}
struct Context<'a,'g> {game:&'a mut Game<'g>,info:access::Info,killer:Option<u32>,at:[f64;3]}
impl access::Context for Context<'_,'_> {
    fn info(&self)->&access::Info {&self.info}
    fn can_drop_head(&self)->std::result::Result<bool,String> {can_claim(self.game,self.killer)}
    fn claim_head_drop(&mut self)->std::result::Result<bool,String> {claim(self.game,self.killer)}
    fn append(&mut self,stack:ItemStack)->std::result::Result<(),String> {
        if stack.count==0 || self.game.res.items().get(&stack.item).is_none() {return Err("death drop item unavailable or empty".into());}
        // WorldEvents uses spawnAtLocation while processing the captured drop
        // event. Publish this head once now; do not also enqueue a second copy.
        self.game.spawn_item_own(stack,self.at,10);
        Ok(())
    }
}
impl Game<'_> {
    pub(super) fn pluto_drop_death_loot(&mut self,m:&mut Mob,player_uuid:Option<u128>,by_player:bool,
        killer:horizon_world::loot::LootEntity,consumed:bool,pos:[f64;3]) {
        use horizon_world::loot::{LootDamage,LootEntity};
        let Some(loot)=horizon_client::structure_blocks::loot_data(&self.options.resources,&self.pack_files()) else {return};
        let source_id=killer_id(m);
        let eligible=match can_claim(self,source_id) {
            Ok(eligible)=>eligible,
            Err(error)=>{eprintln!("PlutoLib: head eligibility unavailable: {error}");false},
        };
        let player=||killer.clone();
        let damage=m.last_damage.map(|(damage,_)| {
            let source=match damage.attacker {
                Some(Who::Player)=>Some(player()),
                Some(Who::Mob(id))=>self.mobs.iter().find(|other|other.id==id).map(|other|drops::loot_entity(other,None))
                    .or_else(||self.combat.charged_exploders.contains(&id).then(|| {
                        // ExplosionSource retains the real type after removal;
                        // do not report every derived creeper as vanilla.
                        let kind=self.combat.exploded_kinds.iter().find(|(owner,_)|*owner==id)
                            .map(|(_,kind)|kind.as_str()).unwrap_or("minecraft:creeper");
                        LootEntity {powered:true,..LootEntity::new(&access::entity_kind(kind))}
                    })),
                None=>None,
            };
            let direct=match damage.kind {
                DamageKind::Arrow=>Some(LootEntity {kind:"minecraft:arrow".into(),..LootEntity::default()}),
                DamageKind::Fireball=>Some(LootEntity {kind:"minecraft:fireball".into(),..LootEntity::default()}),
                DamageKind::WitherSkull=>Some(LootEntity {kind:"minecraft:wither_skull".into(),..LootEntity::default()}),
                DamageKind::Fireworks=>Some(LootEntity {kind:"minecraft:firework_rocket".into(),..LootEntity::default()}),
                _=>source.clone(),
            };
            LootDamage {kind:source_damage_type(damage).into(),source,direct}
        });
        let vehicle=m.extra.mount.and_then(|id|self.mobs.iter().find(|other|other.id==id)).map(|other|LootEntity::new(&other.kind.id));
        let death=drops::Death {loot:&loot,damage,player:player_uuid.is_some().then(player),player_killed:by_player,luck:0.0,vehicle,
            mob_drops:self.ui.rules.bool("mob_drops"),world_seed:self.level.seed};
        let origin=[m.position.x,m.position.y,m.position.z];
        let place=self.loot_place_at(origin,None);
        // A class drop-chance override (e.g. the armored slime head cap)
        // replaces the recorded chances for this death's rolls only; the
        // mob's own record is restored untouched afterwards.
        let chance_override=pluto_drop_chance_override(m,killer_looting(self,&killer,m));
        let saved_chances=chance_override.map(|adjusted| {
            let saved=m.extra.drop_chances.clone();
            m.extra.drop_chances=Some(adjusted);
            saved
        });
        let (items,skulls)=horizon_world::loot::with_native_loot_place(place,|| {
            horizon_client::mobs::nav::pluto_entity_spawn_state::with_head_drop_eligibility(eligible,|| {
                drops::die(m,&death,&mut self.rand_sequences,&mut self.world.random)
            })
        });
        if let Some(saved)=saved_chances {m.extra.drop_chances=saved;}
        if skulls && let Err(error)=claim(self,source_id) {eprintln!("PlutoLib: native head claim failed: {error}");}
        let raider=horizon_client::mobs::raider::is_raider(m.species());
        let mut stacks=Vec::new();let mut rest=Vec::new();
        for drop in items {
            match drop {
                drops::Drop::Item(stack)=>stacks.push(if raider && stack.item==horizon_client::mobs::raider::BANNER {
                    horizon_client::mobs::raider::ominous_banner()
                } else {from_loot(&stack)}),
                drops::Drop::Stack(stack)=>stacks.push(stack),
                other=>rest.push(other),
            }
        }
        let kind=crate::game::native_hooks::mob_kind(m);
        let table=drops::death_loot_table(m,death.mob_drops).filter(|table|loot.has_table(table));
        let context=NativeLootDrops {table:table.as_deref().unwrap_or_default(),dimension:self.dim.id(),origin,
            block:None,entity:Some((m.id,&kind)),player:player_uuid,tool:None,explosion:None,used:false};
        self.native_loot_drops(&context,&mut stacks);
        // Keep Horizon's experience hook even when the living-drop provider
        // replaces drop_death_loot. Unchanged XP retains the original split;
        // changed XP is re-split, and nonpositive XP produces no orbs.
        if rest.iter().any(|drop|matches!(drop,drops::Drop::Orb(_))) {
            let mut xp:i32=rest.iter().filter_map(|drop|match drop {
                drops::Drop::Orb(value)=>Some(*value),_=>None,
            }).sum();
            let before=xp;
            self.native_living_experience_drop(m.id,&kind,&mut xp);
            if xp!=before {
                rest.retain(|drop|!matches!(drop,drops::Drop::Orb(_)));
                while xp>0 {
                    let value=drops::orb_value(xp);
                    xp-=value;
                    rest.push(drops::Drop::Orb(value));
                }
            }
        }
        // Preserve the inherited loot callback's input and RNG boundary: carried
        // block drops are rolled afterward, not presented as the victim's table
        // items. They are still captured before the living-death callback.
        for drop in rest {
            match drop {
                drops::Drop::CarriedBlock(state)=>stacks.extend(horizon_client::mobs::enderman::carried_block_drops(
                    world_items!(self),self.pack.assets.registry,state,&mut self.world.random,Some((&mut self.rand_sequences,self.level.seed)))),
                // Forge's capture ends after dropExperience. Appended heads can
                // publish during the callback, before captured ordinary drops.
                // Preserve the native extended lifetime (e.g. the Wither's
                // nether star starts at -6000), outside table-drop callbacks.
                drops::Drop::Aged(stack,age)=>self.spawn_item_aged(from_loot(&stack),pos,age),
                drops::Drop::Orb(value) if !consumed=>self.spawn_orbs(pos,value as u32),
                drops::Drop::Orb(_)=>{},
                drops::Drop::Item(_) | drops::Drop::Stack(_)=>unreachable!("ordinary drops were partitioned above"),
            }
        }
        let info=access::Info {entity_id:m.id,entity_type:kind,dimension:self.dim.id().to_string(),position:origin,
            damage_type:death.damage.as_ref().map(|damage|damage.kind.clone()),
            killer_type:death.damage.as_ref().and_then(|damage|damage.source.as_ref()).map(|source|source.kind.clone())};
        if let Err(error)=access::dispatch(&mut Context {game:self,info,killer:source_id,at:origin}) {
            eprintln!("PlutoLib: living drop callback failed after prior effects: {error}");
        }
        stacks.retain(|stack|stack.count>0 && &*stack.item!="minecraft:air");
        for item in stacks {self.spawn_item_own(item,pos,10);}
    }
}

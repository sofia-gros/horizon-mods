use horizon_items::items::ItemStack;
use horizon_world::registry::BlockStateId;
use horizon_world::nbt::Tag;

pub(super) fn mode(id:&str)->Option<(&'static str,u8)> {
    match id {
        "tconstruct:mattock"|"tconstruct:kama"|"tconstruct:scythe"=>Some(("minecraft:wooden_hoe",0)),
        "tconstruct:excavator"=>Some(("minecraft:wooden_shovel",1)),
        "tconstruct:hand_axe"|"tconstruct:broad_axe"=>Some(("minecraft:wooden_axe",2)),
        _=>None,
    }
}
pub(super) fn maximum(stack:&ItemStack)->Option<u32> {
    stack.data().and_then(|d|d.other("minecraft:custom_data")).and_then(|d|match d.get("tic_max_damage") { Some(Tag::Int(n)) if *n>0=>Some(*n as u32),_=>None })
}
// The original shovel transform accepts replaceable plants above the ground;
// evaluate the existing vanilla transformer against a read-only air projection
// and let the authoritative caller destroy that plant with normal drops.
pub(super) fn probe(game:&super::Game<'_>, fake:&ItemStack, kind:u8, at:[i32;3],face:[i32;3])->Option<(BlockStateId,String,Option<String>,u32,String,bool)> {
    if let Some((id,sound,loot,damage,particle))=game.block_transform(fake,at,face) {
        return Some((id,sound,loot,damage,particle,false));
    }
    if kind!=1 {return None;}
    let shield=game.offhand.as_ref().and_then(|s|game.res.items().get(&s.item)).is_some_and(|i|i.blocks_attacks.is_some());
    if shield && !game.keys.contains(&super::KeyCode::ShiftLeft) {return None;}
    let above=[at[0],at[1].checked_add(1)?,at[2]];
    let above_state=game.world.block(above[0],above[1],above[2]);
    if game.res.facts().is_air(above_state) || !game.res.facts().get(above_state).replaceable {return None;}
    struct Projected<'a> { game:&'a super::Game<'a>, at:[i32;3] }
    impl horizon_client::block_transform::Around for Projected<'_> {
        fn state(&self,o:[i32;3])->horizon_world::chunk::BlockState {
            let registry=self.game.pack.assets.registry;
            if o==[0,1,0] {return registry.state(self.game.res.facts().air).cloned().unwrap();}
            let state=self.game.world.block(self.at[0]+o[0],self.at[1]+o[1],self.at[2]+o[2]);
            registry.state(state).cloned().unwrap_or_else(||horizon_world::chunk::BlockState{name:"minecraft:air".into(),properties:Default::default()})
        }
    }
    let face_name=match face {[0,-1,0]=>"down",[0,1,0]=>"up",[0,0,-1]=>"north",[0,0,1]=>"south",[-1,0,0]=>"west",[1,0,0]=>"east",_=>return None};
    let registry=game.pack.assets.registry;
    let (_,rule,state)=game.res.transformers().apply(game.res.items(),registry,"minecraft:shovel",face_name,&Projected{game,at})?;
    Some((registry.resolve(&state).ok()?,rule.sound.clone(),rule.loot.clone(),rule.damage,rule.particle.clone(),true))
}

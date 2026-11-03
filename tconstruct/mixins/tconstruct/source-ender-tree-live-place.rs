// Run the shared Ender tree against the host's normal live feature world.
use super::*;
pub fn place<L: LevelAccess + ?Sized>(behaviors: &Behaviours, level: &mut L, ticks: &mut Ticks, pos: horizon_core::position::BlockPos, tall: bool) -> bool {
    let Some(features)=behaviors.live_features().cloned() else {return false;};
    let Ok(tree)=crate::worldgen::live::tconstruct_ender_live_decode::decode(&features,tall) else {return false;};
    let mut sim=behaviors.sim(level,ticks);
    let mut random=sim.random().clone();
    let placed={
        let mut world=feature_world::LevelFeatureWorld {sim:&mut sim,features:&features,entities:Vec::new()};
        tree.place(&mut world,&mut random,[pos.x,pos.y,pos.z]).unwrap_or(false)
    };
    *sim.random()=random;
    placed
}

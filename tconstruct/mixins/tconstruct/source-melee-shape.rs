// Original CircleWeaponAttack and SweepWeaponAttack tool definitions. The
// size is the original unexpanded radius despite its source field name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Attack { Circle(f64), Sweep(f64) }
pub(super) fn attack(item:&str)->Option<Attack> {
    match item {
        "tconstruct:scythe"=>Some(Attack::Circle(2.0)),
        "tconstruct:kama"=>Some(Attack::Circle(1.0)),
        "tconstruct:sword"=>Some(Attack::Sweep(1.0)),
        "tconstruct:cleaver"=>Some(Attack::Sweep(2.0)),
        _=>None,
    }
}
// Dynamic tool records retain a broken stack instead of deleting it.
pub(super) fn intact(stack:&horizon_items::items::ItemStack)->Option<u32> {
    use horizon_world::nbt::Tag;
    let custom=stack.data()?.other("minecraft:custom_data")?;
    let max=match custom.get("tic_max_damage")? { Tag::Int(n) if *n>0=>*n as u32,_=>return None };
    if stack.damage>=max || matches!(custom.get("tic_broken"),Some(Tag::Byte(value)) if *value!=0) {return None;}
    Some(max)
}
// The caller also checks intersection with the primary target's expanded box,
// the entity's eligibility, team, and the original tool's live durability.
pub(super) fn in_distance(kind:Attack, primary:[f64;3], attacker:[f64;3], candidate:[f64;3])->bool {
    let (origin,radius)=match kind { Attack::Circle(r)=>(primary,r),Attack::Sweep(r)=>(attacker,2.0+r) };
    let sum=(0..3).map(|i| {let d=origin[i]-candidate[i];d*d}).sum::<f64>();
    sum < radius*radius
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn broken_flag_and_damage_both_stop_actions() {
        use horizon_items::items::ItemStack;
        use horizon_world::nbt::Tag;
        let mut stack=ItemStack::new("tconstruct:scythe",1);
        assert_eq!(intact(&stack),None);
        stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(4))])));
        assert_eq!(intact(&stack),Some(4));
        stack.damage=4;assert_eq!(intact(&stack),None);
        stack.damage=0;stack.data_mut().set_other("minecraft:custom_data",Some(Tag::compound([("tic_max_damage",Tag::Int(4)),("tic_broken",Tag::Byte(1))])));
        assert_eq!(intact(&stack),None);
    }
    #[test] fn source_modes_and_strict_distance_boundaries() {
        assert_eq!(attack("tconstruct:scythe"),Some(Attack::Circle(2.0)));
        assert_eq!(attack("tconstruct:kama"),Some(Attack::Circle(1.0)));
        assert_eq!(attack("tconstruct:sword"),Some(Attack::Sweep(1.0)));
        assert_eq!(attack("tconstruct:cleaver"),Some(Attack::Sweep(2.0)));
        assert_eq!(attack("minecraft:iron_sword"),None);
        assert!(in_distance(Attack::Circle(2.0),[0.0;3],[10.0,0.0,0.0],[1.99,0.0,0.0]));
        assert!(!in_distance(Attack::Circle(2.0),[0.0;3],[10.0,0.0,0.0],[2.0,0.0,0.0]));
        assert!(in_distance(Attack::Sweep(2.0),[10.0,0.0,0.0],[0.0;3],[3.99,0.0,0.0]));
        assert!(!in_distance(Attack::Sweep(2.0),[10.0,0.0,0.0],[0.0;3],[4.0,0.0,0.0]));
    }
}

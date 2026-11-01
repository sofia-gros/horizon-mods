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

use horizon_items::items::ItemStack;
use std::collections::HashSet;

// The source harvest consumes exactly one seed from the generated drops.
pub(super) fn consume_seed(drops: &mut [ItemStack], seeds: &HashSet<String>) -> bool {
    let Some(seed) = drops.iter_mut().find(|stack| stack.count > 0 && seeds.contains(stack.item.as_ref())) else { return false; };
    seed.count -= 1;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consumes_one_real_seed_and_preserves_other_drops() {
        let seeds = ["minecraft:wheat_seeds".to_owned()].into_iter().collect();
        let mut drops = [ItemStack::new("minecraft:wheat", 2), ItemStack::new("minecraft:wheat_seeds", 3)];
        assert!(consume_seed(&mut drops, &seeds));
        assert_eq!(drops[0].count, 2);
        assert_eq!(drops[1].count, 2);
        drops[1].count = 0;
        assert!(!consume_seed(&mut drops, &seeds));
        assert_eq!(drops[0].count, 2);
    }
}

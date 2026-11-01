// Original tool definitions supply Knockback II on Excavator and I on
// Battlesign. The source attack context applies the attack-knockback
// attribute divided by two; this is in addition to native sprint/enchantments.
pub(super) fn builtin_bonus(item: &str) -> f32 {
    match item {
        "tconstruct:excavator" => 1.0,
        "tconstruct:battlesign" => 0.5,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn source_builtin_levels() {
        assert_eq!(builtin_bonus("tconstruct:excavator"), 1.0);
        assert_eq!(builtin_bonus("tconstruct:battlesign"), 0.5);
        assert_eq!(builtin_bonus("minecraft:iron_axe"), 0.0);
    }
}

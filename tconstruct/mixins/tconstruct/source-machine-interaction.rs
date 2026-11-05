/// Until the real item filter can be persisted and safely recovered on every
/// removal path, neither Duct has a supported fluid capability. In source an
/// empty filter drains nothing; treating it as the selected controller fluid
/// bypasses both the filter and fluid-NBT matching requirement.
pub(super) fn unsupported_duct(name: &str) -> bool {
    matches!(name, "tconstruct:seared_duct" | "tconstruct:scorched_duct")
}

/// Source FaucetBlock and AbstractCastingBlock always PASS while sneaking,
/// including with both hands empty. Other machines retain the host's existing
/// held-item bypass, so sneaking can still place/use either held item.
pub(super) fn sneak_passthrough(name: &str, sneaking: bool, main_hand: bool, off_hand: bool) -> bool {
    sneaking && (main_hand || off_hand || matches!(name,
        "tconstruct:seared_faucet" | "tconstruct:scorched_faucet" |
        "tconstruct:seared_table" | "tconstruct:scorched_table" |
        "tconstruct:seared_basin" | "tconstruct:scorched_basin"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_ducts_are_closed_without_affecting_drains_or_other_blocks() {
        for name in ["tconstruct:seared_duct", "tconstruct:scorched_duct"] {
            assert!(unsupported_duct(name));
        }
        for name in ["tconstruct:seared_drain", "tconstruct:scorched_drain",
            "tconstruct:seared_melter", "minecraft:air", "other:seared_duct"] {
            assert!(!unsupported_duct(name));
        }
    }
    const TARGETS: [&str;6] = ["tconstruct:seared_faucet","tconstruct:scorched_faucet",
        "tconstruct:seared_table","tconstruct:scorched_table","tconstruct:seared_basin","tconstruct:scorched_basin"];
    #[test]
    fn faucets_and_casting_pass_with_empty_either_or_both_hands() {
        for name in TARGETS { for main in [false,true] { for off in [false,true] {
            assert!(sneak_passthrough(name,true,main,off));
            assert!(!sneak_passthrough(name,false,main,off));
        } } }
    }
    #[test]
    fn other_machine_and_vanilla_controls_keep_previous_policy() {
        for name in ["tconstruct:seared_melter","tconstruct:seared_heater",
            "tconstruct:smeltery_controller","tconstruct:foundry_controller",
            "tconstruct:seared_casting_tank","tconstruct:seared_fluid_cannon",
            "tconstruct:seared_fuel_tank","tconstruct:seared_drain","minecraft:chest","other:seared_faucet"] {
            for sneaking in [false,true] { for main in [false,true] { for off in [false,true] {
                assert_eq!(sneak_passthrough(name,sneaking,main,off),sneaking && (main || off));
            } } }
        }
    }
}

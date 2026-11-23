//! Recipe-level `tconstruct:config` conditions (the original's
//! ConfigEnabledCondition): the shipped recipes whose availability follows a
//! common config value. The host evaluates only the `neoforge:conditions`
//! key with a closed type set, so these gates cannot ride the data files;
//! they are decided here from the parsed common config and are meant to be
//! enforced through PlutoLib's crafting-match veto (deny-only admission in
//! pluto-lib's inventory-api crafting::matching registry).
//!
//! Property semantics follow ConfigEnabledCondition exactly, including its
//! quirk: the `slime_recipe_fix` property reads the glassRecipeFix config
//! value, the same as `glass_recipe_fix` (there is no separate value behind
//! the recipe condition in the original).

use crate::config::Common;

/// One shipped recipe gated by a `tconstruct:config` condition.
pub struct GatedRecipe {
    /// Full recipe id as loaded by the host.
    pub id: &'static str,
    /// The condition's `prop` name, as written in the original recipe JSON.
    pub prop: &'static str,
}

/// Every shipped recipe carrying a `tconstruct:config` condition (DATA-W3).
pub const GATED_RECIPES: &[GatedRecipe] = &[
    GatedRecipe {
        id: "tconstruct:common/flint",
        prop: "gravel_to_flint",
    },
    GatedRecipe {
        id: "tconstruct:common/glass/vanilla/beacon",
        prop: "glass_recipe_fix",
    },
    GatedRecipe {
        id: "tconstruct:common/glass/vanilla/daylight_detector",
        prop: "glass_recipe_fix",
    },
    GatedRecipe {
        id: "tconstruct:common/glass/vanilla/end_crystal",
        prop: "glass_recipe_fix",
    },
    GatedRecipe {
        id: "tconstruct:common/glass/vanilla/glass_bottle",
        prop: "glass_recipe_fix",
    },
    GatedRecipe {
        id: "tconstruct:common/slime/lead",
        prop: "slime_recipe_fix",
    },
    GatedRecipe {
        id: "tconstruct:common/slime/sticky_piston",
        prop: "slime_recipe_fix",
    },
];

/// The condition property gating a recipe, if it carries one.
pub fn config_prop(recipe_id: &str) -> Option<&'static str> {
    GATED_RECIPES
        .iter()
        .find(|gated| gated.id == recipe_id)
        .map(|gated| gated.prop)
}

/// The current value behind a condition property, or None for a property the
/// recipe conditions do not use.
pub fn prop_enabled(prop: &str, config: &Common) -> Option<bool> {
    match prop {
        "gravel_to_flint" => Some(config.add_gravel_to_flint_recipe),
        "glass_recipe_fix" => Some(config.glass_recipe_fix),
        // ConfigEnabledCondition deliberately maps this name to the
        // glassRecipeFix value as well; keep the original's behaviour.
        "slime_recipe_fix" => Some(config.glass_recipe_fix),
        _ => None,
    }
}

/// Whether a recipe is available under the current config. Recipes without a
/// config condition are always available.
pub fn recipe_enabled(recipe_id: &str, config: &Common) -> bool {
    match config_prop(recipe_id) {
        Some(prop) => prop_enabled(prop, config).unwrap_or(true),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> Common {
        Common::default()
    }

    #[test]
    fn all_seven_gated_recipes_are_listed_with_original_props() {
        let expected = [
            ("tconstruct:common/flint", "gravel_to_flint"),
            ("tconstruct:common/glass/vanilla/beacon", "glass_recipe_fix"),
            (
                "tconstruct:common/glass/vanilla/daylight_detector",
                "glass_recipe_fix",
            ),
            (
                "tconstruct:common/glass/vanilla/end_crystal",
                "glass_recipe_fix",
            ),
            (
                "tconstruct:common/glass/vanilla/glass_bottle",
                "glass_recipe_fix",
            ),
            ("tconstruct:common/slime/lead", "slime_recipe_fix"),
            ("tconstruct:common/slime/sticky_piston", "slime_recipe_fix"),
        ];
        assert_eq!(GATED_RECIPES.len(), expected.len());
        for (id, prop) in expected {
            assert_eq!(config_prop(id), Some(prop), "recipe {id}");
        }
        assert_eq!(config_prop("tconstruct:tools/materials/flint"), None);
        assert_eq!(config_prop("minecraft:beacon"), None);
    }

    #[test]
    fn defaults_enable_every_gated_recipe() {
        let config = config();
        for gated in GATED_RECIPES {
            assert!(recipe_enabled(gated.id, &config), "{}", gated.id);
        }
    }

    #[test]
    fn gravel_flag_gates_only_the_flint_recipe() {
        let mut config = config();
        config.add_gravel_to_flint_recipe = false;
        assert!(!recipe_enabled("tconstruct:common/flint", &config));
        for gated in GATED_RECIPES.iter().skip(1) {
            assert!(recipe_enabled(gated.id, &config), "{}", gated.id);
        }
    }

    #[test]
    fn glass_flag_gates_glass_and_slime_fixes() {
        let mut config = config();
        config.glass_recipe_fix = false;
        assert!(recipe_enabled("tconstruct:common/flint", &config));
        for gated in GATED_RECIPES.iter().skip(1) {
            assert!(!recipe_enabled(gated.id, &config), "{}", gated.id);
        }
    }

    #[test]
    fn slime_recipes_follow_the_glass_value_not_the_slime_field() {
        // The original condition reads glassRecipeFix for slime_recipe_fix;
        // flipping only the separate slimeRecipeFix config field must not
        // change recipe availability.
        let mut config = config();
        config.slime_recipe_fix = false;
        assert!(recipe_enabled("tconstruct:common/slime/lead", &config));
        assert!(recipe_enabled(
            "tconstruct:common/slime/sticky_piston",
            &config
        ));
    }

    #[test]
    fn ungated_recipes_ignore_config() {
        let mut config = config();
        config.add_gravel_to_flint_recipe = false;
        config.glass_recipe_fix = false;
        assert!(recipe_enabled("minecraft:beacon", &config));
        assert!(recipe_enabled("tconstruct:common/materials/flint", &config));
    }
}

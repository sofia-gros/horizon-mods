// Both source BottleBrewingRecipes derive reagents from the corresponding
// potion container mixes, without inventing a separate fixed ingredient.
fn tconstruct_splash_bottle_brewing(mut value: crate::brewing::Brewing, items: &Items) -> crate::brewing::Brewing {
    for (from, to, input, output, id) in [
        ("minecraft:potion", "minecraft:splash_potion", "minecraft:glass_bottle", "tconstruct:splash_bottle", "tconstruct:bottle_brewing/splash"),
        ("minecraft:splash_potion", "minecraft:lingering_potion", "tconstruct:splash_bottle", "tconstruct:lingering_bottle", "tconstruct:bottle_brewing/lingering"),
    ] {
        if items.get(input).is_none() || items.get(output).is_none() { continue; }
        if input == "tconstruct:splash_bottle" && items.item_tags.get("forge:bottles/splash").is_some_and(|tag| !tag.contains(input)) { continue; }
        let mut reagent = None;
        let mut ambiguous = false;
        for recipe in &value.recipes {
            if !recipe.input.items.contains(from) || recipe.output.item.as_ref() != to { continue; }
            if recipe.reagent.items.is_empty() || recipe.reagent.potions.is_some() { ambiguous = true; break; }
            if let Some(prior) = &reagent {
                if prior != &recipe.reagent.items { ambiguous = true; break; }
            } else { reagent = Some(recipe.reagent.items.clone()); }
        }
        if ambiguous { continue; }
        let Some(reagent) = reagent else { continue; };
        // Brewing::find selects the first matching recipe. Do not add an
        // overlapping conversion, including an identical one on a reload.
        if value.recipes.iter().any(|recipe| recipe.id == id
            || (recipe.input.items.contains(input) && !recipe.reagent.items.is_disjoint(&reagent))) { continue; }
        value.inputs.insert(input.to_owned());
        value.reagents.extend(reagent.iter().cloned());
        value.recipes.push(crate::brewing::BrewingRecipe {
            id: id.to_owned(),
            input: crate::brewing::PotionIngredient { items: [input.to_owned()].into_iter().collect(), potions: None },
            reagent: crate::brewing::PotionIngredient { items: reagent, potions: None },
            output: ItemStack::new(output, 1),
        });
    }
    // The fixed source registers one recipe per slime type and one for magma.
    for (reagent, output) in [
        ("tconstruct:earth_congealed_slime", "tconstruct:earth_slime_bottle"),
        ("tconstruct:sky_congealed_slime", "tconstruct:sky_slime_bottle"),
        ("tconstruct:ender_congealed_slime", "tconstruct:ender_slime_bottle"),
        ("tconstruct:ichor_congealed_slime", "tconstruct:ichor_slime_bottle"),
        ("minecraft:magma_block", "tconstruct:magma_bottle"),
    ] {
        let input = "minecraft:glass_bottle";
        if items.get(input).is_none() || items.get(reagent).is_none() || items.get(output).is_none() { continue; }
        let id = format!("tconstruct:bottle_brewing/{}", output.rsplit(':').next().unwrap());
        // Keep existing pack recipes and their order, including constrained
        // alternatives. Refuse an ambiguous overlap instead of overriding it.
        if value.recipes.iter().any(|recipe| recipe.id == id
            || (recipe.input.items.contains(input) && recipe.reagent.items.contains(reagent))) { continue; }
        value.inputs.insert(input.to_owned());
        value.reagents.insert(reagent.to_owned());
        value.recipes.push(crate::brewing::BrewingRecipe {
            id,
            input: crate::brewing::PotionIngredient { items: [input.to_owned()].into_iter().collect(), potions: None },
            reagent: crate::brewing::PotionIngredient { items: [reagent.to_owned()].into_iter().collect(), potions: None },
            output: ItemStack::new(output, 1),
        });
    }
    value
}

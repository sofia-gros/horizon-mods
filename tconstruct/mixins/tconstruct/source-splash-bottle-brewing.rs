// Both source BottleBrewingRecipes derive reagents from the corresponding
// potion container mixes, without inventing a separate fixed ingredient.
fn tconstruct_splash_bottle_brewing(mut value: StationData, items: &Items) -> StationData {
    for (from, to, input, output, id) in [
        ("minecraft:potion", "minecraft:splash_potion", "minecraft:glass_bottle", "tconstruct:splash_bottle", "tconstruct:bottle_brewing/splash"),
        ("minecraft:splash_potion", "minecraft:lingering_potion", "tconstruct:splash_bottle", "tconstruct:lingering_bottle", "tconstruct:bottle_brewing/lingering"),
    ] {
        if items.get(input).is_none() || items.get(output).is_none() { continue; }
        if input == "tconstruct:splash_bottle" && items.item_tags.get("forge:bottles/splash").is_some_and(|tag| !tag.contains(input)) { continue; }
        let mut reagent = None;
        let mut ambiguous = false;
        for recipe in &value.brewing.recipes {
            if !recipe.input.items.contains(from) || recipe.output.item.as_ref() != to { continue; }
            if recipe.reagent.items.is_empty() || recipe.reagent.potions.is_some() { ambiguous = true; break; }
            if let Some(prior) = &reagent {
                if prior != &recipe.reagent.items { ambiguous = true; break; }
            } else { reagent = Some(recipe.reagent.items.clone()); }
        }
        if ambiguous { continue; }
        let Some(reagent) = reagent else { continue; };
        value.brewing.inputs.insert(input.to_owned());
        value.brewing.reagents.extend(reagent.iter().cloned());
        value.brewing.recipes.push(crate::brewing::BrewingRecipe {
            id: id.to_owned(),
            input: crate::brewing::PotionIngredient { items: [input.to_owned()].into_iter().collect(), potions: None },
            reagent: crate::brewing::PotionIngredient { items: reagent, potions: None },
            output: ItemStack::new(output, 1),
        });
    }
    value
}

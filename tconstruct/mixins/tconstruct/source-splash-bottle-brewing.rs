// BottleBrewingRecipe derives its reagent from the source potion-to-splash
// container mix. Use the corresponding loaded host recipes rather than
// inventing a separate fixed reagent or bypassing the brewing stand.
fn tconstruct_splash_bottle_brewing(mut value: StationData, items: &Items) -> StationData {
    if items.get("tconstruct:splash_bottle").is_none() { return value; }
    let mut reagent = None;
    for recipe in &value.brewing.recipes {
        if !recipe.input.items.contains("minecraft:potion") || recipe.output.item.as_ref() != "minecraft:splash_potion" { continue; }
        if recipe.reagent.items.is_empty() || recipe.reagent.potions.is_some() { return value; }
        if let Some(prior) = &reagent {
            if prior != &recipe.reagent.items { return value; }
        } else { reagent = Some(recipe.reagent.items.clone()); }
    }
    let Some(reagent) = reagent else { return value; };
    value.brewing.inputs.insert("minecraft:glass_bottle".to_owned());
    value.brewing.reagents.extend(reagent.iter().cloned());
    value.brewing.recipes.push(crate::brewing::BrewingRecipe {
        id: "tconstruct:bottle_brewing/splash".to_owned(),
        input: crate::brewing::PotionIngredient { items: ["minecraft:glass_bottle".to_owned()].into_iter().collect(), potions: None },
        reagent: crate::brewing::PotionIngredient { items: reagent, potions: None },
        output: ItemStack::new("tconstruct:splash_bottle", 1),
    });
    value
}

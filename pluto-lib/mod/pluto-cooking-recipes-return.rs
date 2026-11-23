fn pluto_cooking_recipes_return(value: Result<Recipes>) -> Result<Recipes> {
    if let Ok(recipes) = &value {
        let table = recipes.cooking.iter().map(|cooking| crate::entry::pluto_cooking_recipes::CookingFacts {
            kind: cooking.kind.clone(),
            inputs: cooking.ingredient.items.iter().cloned().collect(),
            result: cooking.result.item.to_string(),
            result_count: cooking.result.count,
            experience: cooking.experience,
            time: cooking.time,
        }).collect();
        crate::entry::pluto_cooking_recipes::publish(table);
    }
    value
}

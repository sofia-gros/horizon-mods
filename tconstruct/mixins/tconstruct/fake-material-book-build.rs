fn tconstruct_fake_material_book_build(mut value: Book, recipes: &Recipes) -> Book {
    tconstruct_source_fake_material_book::decorate(&mut value, &recipes.crafting);
    value
}
